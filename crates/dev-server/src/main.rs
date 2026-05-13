//! `dev-server` — local HTTP file server with WebSocket live reload and
//! file-watcher-driven rebuilds. Designed for the `just dev` and
//! `just dev-remote` flows: laptop runs the server on `127.0.0.1:3000`,
//! Tailscale Serve forwards from there to a phone-reachable HTTPS URL.
//!
//! ```bash
//! cargo run -p dev-server -- \
//!     --assets _docs/book/src/assets/ui \
//!     --watch crates/ui-storybook/src crates/ui-storybook/assets \
//!     --port 3000
//! ```
//!
//! Architecture lives in `lib.rs` + sibling modules so the integration
//! tests in `tests/smoke.rs` can spin up the same router.

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use dev_server::build_router;
use dev_server::live_reload::ReloadBroadcaster;
use dev_server::watcher::{WatcherConfig, spawn as spawn_watcher};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Parser)]
#[command(
    name = "dev-server",
    about = "Local HTTP dev server with WebSocket live reload + file-watcher-driven rebuilds.",
    version
)]
struct Args {
    /// Directory of static assets to serve.
    #[arg(long)]
    assets: PathBuf,

    /// One or more directories to watch for changes. Saving anything
    /// under one of these paths triggers `--rebuild-cmd` (or, for the
    /// CSS fast path, a direct copy). Pass the flag multiple times or
    /// comma-separated.
    #[arg(long, value_delimiter = ',', num_args = 1..)]
    watch: Vec<PathBuf>,

    /// Rebuild command tokens. Default: the existing storybook export
    /// binary. Override to swap in DEV-07's persistent worker once it
    /// lands as the default.
    #[arg(
        long,
        value_delimiter = ' ',
        default_value = "cargo run -p ui-storybook --bin ui-export-stories"
    )]
    rebuild_cmd: Vec<String>,

    /// Source path of the CSS file that drives the fast-path. When a
    /// change to this file is the *only* event in a debounced batch,
    /// the server copies it to `--css-dest` and broadcasts reload
    /// without re-running the full rebuild command.
    #[arg(long, default_value = "crates/ui-storybook/assets/style.css")]
    css_source: PathBuf,

    /// Destination path inside `--assets` where the CSS lives.
    /// Defaults to `<assets>/style.css`.
    #[arg(long)]
    css_dest: Option<PathBuf>,

    /// Disable file watching entirely (handy for tests + smoke checks).
    #[arg(long)]
    no_watch: bool,

    /// Host/IP to bind. Default `127.0.0.1` (loopback only — Tailscale
    /// Serve forwards from loopback). Flip to `0.0.0.0` for LAN access.
    #[arg(long, default_value = "127.0.0.1")]
    host: IpAddr,

    /// TCP port. Default `3000`.
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Debounce window for filesystem events, in milliseconds.
    #[arg(long, default_value_t = 250)]
    debounce_ms: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    let args = Args::parse();
    let socket = SocketAddr::new(args.host, args.port);
    let assets = args
        .assets
        .canonicalize()
        .with_context(|| format!("--assets path does not exist: {}", args.assets.display()))?;
    let broadcaster = ReloadBroadcaster::default();
    let app = build_router(&assets, broadcaster.clone());

    if args.no_watch {
        tracing::info!("watcher: disabled via --no-watch");
    } else {
        let css_dest = args
            .css_dest
            .clone()
            .unwrap_or_else(|| assets.join("style.css"));
        let cfg = WatcherConfig {
            watch_paths: args.watch.clone(),
            rebuild_cmd: args.rebuild_cmd.clone(),
            css_source: args.css_source.clone(),
            css_dest,
            debounce: std::time::Duration::from_millis(args.debounce_ms),
        };
        spawn_watcher(cfg, broadcaster.clone()).context("spawn watcher")?;
    }

    tracing::info!("dev-server: serving {assets:?} at http://{socket}/");
    let listener = tokio::net::TcpListener::bind(&socket)
        .await
        .with_context(|| format!("bind {socket}"))?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum serve")?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    // SIGTERM on unix. Previous form was `signal(...).map(|mut s|
    // async move { s.recv().await; })` which wraps an async block
    // inside a `Result<impl Future>` and then drops the Result —
    // the inner future never gets awaited. Net effect: this branch
    // completed immediately at startup and `select!` returned on
    // the first iteration, killing the server before it served a
    // single request. The bug masked itself as "shutdown signal
    // received" in the logs.
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(e) => {
                tracing::warn!(error = ?e, "dev-server: failed to install SIGTERM handler; only Ctrl-C will stop the server");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {}
        () = terminate => {}
    }
    tracing::info!("dev-server: shutdown signal received");
}
