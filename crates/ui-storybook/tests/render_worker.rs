//! Smoke test for the long-lived `render-worker` binary
//! (DEV-07 / AUT-150).
//!
//! Spawns the worker as a subprocess, writes JSON commands on stdin,
//! reads JSON replies on stdout, and asserts the protocol. This is the
//! regression gate that catches:
//!
//! - The worker dying on first command.
//! - The reply schema drifting (e.g. someone renames `story_count` →
//!   the test fails on the JSON shape, not silently).
//! - `export_subset` losing its compatibility with `WorkerCommand::Rerender`
//!   payloads.

use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use tempfile::TempDir;

fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Pre-build the binary on the test runner so the timing inside the
/// test is just the spawn + IPC, not cargo's compile.
fn ensure_worker_built() {
    let root = workspace_root();
    let status = Command::new(env!("CARGO"))
        .args(["build", "-p", "ui-storybook", "--bin", "render-worker"])
        .current_dir(&root)
        .status()
        .expect("cargo build render-worker");
    assert!(status.success(), "render-worker pre-build");
}

fn worker_path() -> PathBuf {
    let root = workspace_root();
    // Honour CARGO_TARGET_DIR if set; default is `<root>/target`.
    let target =
        std::env::var("CARGO_TARGET_DIR").map_or_else(|_| root.join("target"), PathBuf::from);
    target.join("debug").join("render-worker")
}

#[test]
fn worker_responds_to_hello() {
    ensure_worker_built();
    let tmp = TempDir::new().unwrap();
    let mut child = Command::new(worker_path())
        .args(["--assets", tmp.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn render-worker");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, r#"{{"cmd":"hello"}}"#).unwrap();
        writeln!(stdin, r#"{{"cmd":"shutdown"}}"#).unwrap();
        stdin.flush().unwrap();
    }
    let stdout = child.stdout.take().expect("stdout");
    let reader = BufReader::new(stdout);
    let mut lines = Vec::new();
    for line in reader.lines() {
        match line {
            Ok(l) => lines.push(l),
            Err(_) => break,
        }
    }
    let status = child.wait_timeout(Duration::from_secs(20));
    assert!(status, "worker exited cleanly within 20 s");
    let hello = lines
        .iter()
        .find(|l| l.contains("\"reply\":\"hello\""))
        .unwrap_or_else(|| panic!("no hello reply found: {lines:?}"));
    assert!(
        hello.contains("\"story_count\""),
        "hello reply missing story_count: {hello}"
    );
}

#[test]
fn worker_rerenders_a_subset() {
    ensure_worker_built();
    let tmp = TempDir::new().unwrap();
    let mut child = Command::new(worker_path())
        .args(["--assets", tmp.path().to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn render-worker");
    {
        let stdin = child.stdin.as_mut().expect("stdin");
        // Pick a story id that's stable across the registry; every
        // workspace ships button-variants (UI-04 / AUT-124).
        writeln!(stdin, r#"{{"cmd":"rerender","ids":["button-variants"]}}"#).unwrap();
        writeln!(stdin, r#"{{"cmd":"shutdown"}}"#).unwrap();
        stdin.flush().unwrap();
    }
    let stdout = child.stdout.take().expect("stdout");
    let lines: Vec<String> = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .collect();
    let ok = child.wait_timeout(Duration::from_secs(20));
    assert!(ok, "worker exited cleanly within 20 s");
    assert!(
        tmp.path().join("button-variants.html").exists(),
        "expected story file"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("\"reply\":\"done\"") && l.contains("button-variants")),
        "missing done reply: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("\"reply\":\"batch_done\"")),
        "missing batch_done reply: {lines:?}"
    );
}

trait WaitTimeout {
    fn wait_timeout(&mut self, dur: Duration) -> bool;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(&mut self, dur: Duration) -> bool {
        let start = std::time::Instant::now();
        loop {
            match self.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => {
                    if start.elapsed() >= dur {
                        let _ = self.kill();
                        return false;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => return false,
            }
        }
    }
}
