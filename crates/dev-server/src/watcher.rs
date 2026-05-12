//! File watcher + rebuild orchestration (M-DEV.2 / AUT-146).
//!
//! Watches the configured directories, debounces filesystem events, and
//! on a change either:
//!
//! - **CSS fast path** — if every event in the batch is on `style.css`,
//!   copies the source CSS into the assets directory and broadcasts a
//!   reload. Sub-100ms.
//! - **Full rebuild** — otherwise, spawns `--rebuild-cmd` (default
//!   `cargo run -p ui-storybook --bin ui-export-stories`), waits for it
//!   to finish successfully, then broadcasts a reload. On failure we log
//!   and don't broadcast (so the browser stays on the last-good state).
//!
//! Rebuilds coalesce: if events arrive while a rebuild is running, a
//! single follow-up rebuild kicks off when the current one finishes.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use notify::RecursiveMode;
use notify_debouncer_mini::{DebounceEventResult, DebouncedEventKind, new_debouncer};
use tokio::process::Command;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use crate::live_reload::ReloadBroadcaster;

/// Configuration for the watcher task. Built from `main.rs` CLI args.
pub struct WatcherConfig {
    /// Paths to watch recursively.
    pub watch_paths: Vec<PathBuf>,
    /// Rebuild command (shell-tokenised). Default:
    /// `["cargo", "run", "-p", "ui-storybook", "--bin", "ui-export-stories"]`.
    pub rebuild_cmd: Vec<String>,
    /// Path to the source CSS file (the "fast path" trigger).
    pub css_source: PathBuf,
    /// Path to the CSS destination in the served assets directory.
    pub css_dest: PathBuf,
    /// Debounce window applied to incoming filesystem events.
    pub debounce: Duration,
}

/// Spawn the watcher task on the current tokio runtime.
///
/// Returns a `JoinHandle` so the caller can await graceful shutdown.
///
/// # Errors
/// Returns an error if the notify watcher cannot be created (filesystem
/// inotify limits, missing paths, etc.).
pub fn spawn(
    config: WatcherConfig,
    broadcaster: ReloadBroadcaster,
) -> Result<tokio::task::JoinHandle<()>> {
    info!(
        "watcher: starting on {} path(s), debounce {:?}, css fast-path: {:?} → {:?}",
        config.watch_paths.len(),
        config.debounce,
        config.css_source,
        config.css_dest,
    );

    // The notify-debouncer-mini calls our handler from its own thread.
    // We forward batched events into an `mpsc` channel so the rebuild
    // loop can run on the tokio runtime.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<DebouncedEvent>>();
    let tx_for_handler = tx.clone();
    let mut debouncer = new_debouncer(config.debounce, move |res: DebounceEventResult| match res {
        Ok(events) => {
            let mapped: Vec<DebouncedEvent> = events
                .into_iter()
                .map(|e| DebouncedEvent {
                    path: e.path,
                    kind: e.kind,
                })
                .collect();
            if !mapped.is_empty() {
                let _ = tx_for_handler.send(mapped);
            }
        }
        Err(err) => warn!("watcher: debouncer error: {err}"),
    })
    .context("create notify debouncer")?;

    let watcher = debouncer.watcher();
    for path in &config.watch_paths {
        if !path.exists() {
            warn!(
                "watcher: path does not exist (skipping): {}",
                path.display()
            );
            continue;
        }
        watcher
            .watch(path, RecursiveMode::Recursive)
            .with_context(|| format!("watch {}", path.display()))?;
        info!("watcher: now watching {}", path.display());
    }

    let rebuild_state = Arc::new(Mutex::new(RebuildState::Idle));
    let config = Arc::new(config);
    let css_source = config.css_source.clone();
    let css_dest = config.css_dest.clone();

    let handle = tokio::spawn(async move {
        // Hold the debouncer alive for the lifetime of the watcher task.
        let _debouncer = debouncer;
        while let Some(events) = rx.recv().await {
            let is_css_only = events
                .iter()
                .all(|e| paths_equal(&e.path, &css_source) || paths_equal(&e.path, &css_dest));
            let state = rebuild_state.clone();
            let cfg = config.clone();
            let bc = broadcaster.clone();
            let css_src = css_source.clone();
            let css_target = css_dest.clone();
            if is_css_only {
                tokio::spawn(async move {
                    handle_css_only(&css_src, &css_target, &bc).await;
                });
            } else {
                tokio::spawn(async move {
                    handle_full_rebuild(state, &cfg, &bc).await;
                });
            }
        }
    });

    Ok(handle)
}

#[derive(Debug)]
struct DebouncedEvent {
    path: PathBuf,
    #[allow(dead_code)]
    kind: DebouncedEventKind,
}

enum RebuildState {
    Idle,
    Running,
    Queued, // rebuild in flight; another batch arrived; queue exactly one
}

async fn handle_css_only(css_source: &Path, css_dest: &Path, broadcaster: &ReloadBroadcaster) {
    let start = Instant::now();
    match tokio::fs::copy(css_source, css_dest).await {
        Ok(_) => {
            broadcaster.send_reload();
            info!(
                "watcher: css fast-path → reload broadcast ({} ms)",
                start.elapsed().as_millis()
            );
        }
        Err(e) => warn!("watcher: css fast-path copy failed: {e}"),
    }
}

async fn handle_full_rebuild(
    state: Arc<Mutex<RebuildState>>,
    config: &WatcherConfig,
    broadcaster: &ReloadBroadcaster,
) {
    // Coalesce: if a rebuild is already running, queue exactly one more.
    {
        let mut guard = state.lock().await;
        match *guard {
            RebuildState::Running => {
                *guard = RebuildState::Queued;
                debug!("watcher: rebuild already running; queued one follow-up");
                return;
            }
            RebuildState::Queued => {
                debug!("watcher: rebuild already running with one queued; coalescing");
                return;
            }
            RebuildState::Idle => *guard = RebuildState::Running,
        }
    }
    loop {
        let start = Instant::now();
        info!("watcher: rebuilding via {:?}", config.rebuild_cmd);
        let status = run_rebuild(&config.rebuild_cmd).await;
        let elapsed = start.elapsed();
        match status {
            Ok(true) => {
                broadcaster.send_reload();
                info!(
                    "watcher: rebuild ok in {} ms → reload broadcast",
                    elapsed.as_millis()
                );
            }
            Ok(false) => {
                warn!(
                    "watcher: rebuild exited non-zero after {} ms; NOT broadcasting",
                    elapsed.as_millis()
                );
            }
            Err(e) => error!("watcher: rebuild failed to spawn: {e}"),
        }
        let mut guard = state.lock().await;
        if matches!(*guard, RebuildState::Queued) {
            *guard = RebuildState::Running;
            debug!("watcher: running queued follow-up rebuild");
        } else {
            *guard = RebuildState::Idle;
            return;
        }
    }
}

async fn run_rebuild(cmd: &[String]) -> Result<bool> {
    if cmd.is_empty() {
        anyhow::bail!("--rebuild-cmd is empty");
    }
    let mut child = Command::new(&cmd[0])
        .args(&cmd[1..])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawn rebuild command {cmd:?}"))?;
    let status = child.wait().await.context("await rebuild child")?;
    Ok(status.success())
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    a.canonicalize()
        .ok()
        .zip(b.canonicalize().ok())
        .map_or_else(|| a == b, |(a, b)| a == b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn paths_equal_handles_missing_files() {
        // Two paths that don't exist: fall back to direct comparison.
        let a = PathBuf::from("/tmp/does/not/exist/a.css");
        let b = PathBuf::from("/tmp/does/not/exist/a.css");
        assert!(paths_equal(&a, &b));
        let c = PathBuf::from("/tmp/does/not/exist/b.css");
        assert!(!paths_equal(&a, &c));
    }
}
