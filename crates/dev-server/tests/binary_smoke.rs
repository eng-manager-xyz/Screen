//! Binary-level smoke test for the `dev-server` CLI.
//!
//! Spawns `target/<profile>/dev-server` as a child process against a
//! tempdir of static assets, waits for it to bind, hits it over
//! HTTP, and asserts the response shape. This is the test that would
//! have caught the shutdown-signal regression (the SIGTERM-handler
//! `.map(|s| async move {...})` form dropped the inner Future, so
//! the terminate branch completed immediately and the server killed
//! itself at startup — the existing `smoke.rs` tests only exercised
//! the *library* path and missed it entirely).
//!
//! **Anti-regression principle:** the lib tests cover the HTTP +
//! middleware shape; this file covers the binary entrypoint (main
//! signal wiring, CLI args, watcher boot). If `just dev` breaks at
//! startup again, this test goes red before the user notices.

use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn binary_path() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map_or_else(|_| workspace_root().join("target"), PathBuf::from);
    // Nextest defaults the build profile to `debug`.
    target.join("debug").join("dev-server")
}

/// Pick a port the OS just confirmed is free by binding briefly and
/// returning the assigned port. Small TOCTOU window before the
/// dev-server binds in practice but acceptable for tests.
fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().unwrap().port()
}

/// RAII guard — kills the child + reaps the zombie when this drops.
/// Without it, a test panic leaves the binary running and the next
/// run conflicts on the inherited port.
struct ServerChild(Child);
impl Drop for ServerChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[tokio::test]
async fn dev_server_binary_boots_and_serves_assets() {
    let bin = binary_path();
    assert!(
        bin.is_file(),
        "dev-server binary missing at {} — run `cargo build -p dev-server` first.",
        bin.display()
    );

    // Static-asset fixture so the test doesn't depend on
    // `_docs/book/src/assets/ui` being populated.
    let assets = tempfile::tempdir().expect("assets tempdir");
    let watch = tempfile::tempdir().expect("watch tempdir");
    std::fs::write(
        assets.path().join("index.html"),
        "<!doctype html><title>fixture</title><body>OK_FIXTURE_MARKER</body>",
    )
    .unwrap();

    let port = free_port();
    let child = Command::new(&bin)
        .arg("--assets")
        .arg(assets.path())
        .arg("--watch")
        .arg(watch.path())
        .arg("--port")
        .arg(port.to_string())
        .arg("--host")
        .arg("127.0.0.1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn dev-server");
    let _guard = ServerChild(child);

    // Poll for the server to start serving. ~3 seconds total budget.
    let deadline = Instant::now() + Duration::from_secs(3);
    let url = format!("http://127.0.0.1:{port}/");
    let client = reqwest::Client::new();
    let mut last_err: Option<String> = None;
    let body = loop {
        assert!(
            Instant::now() <= deadline,
            "dev-server never served HTTP 200 within 3 s — last error: {last_err:?}"
        );
        match client.get(&url).send().await {
            Ok(resp) if resp.status() == 200 => match resp.text().await {
                Ok(body) => break body,
                Err(e) => last_err = Some(format!("body read: {e}")),
            },
            Ok(resp) => last_err = Some(format!("HTTP {}", resp.status())),
            Err(e) => last_err = Some(e.to_string()),
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    };

    // The static-asset marker proves the server served the actual
    // file from the assets dir.
    assert!(
        body.contains("OK_FIXTURE_MARKER"),
        "response body missing fixture marker; got: {body:?}"
    );

    // The live-reload client script is injected by the middleware
    // into every text/html response. Its presence is the contract
    // surface every dev-loop browser depends on.
    assert!(
        body.contains("new WebSocket") || body.contains("ws://"),
        "live-reload client not injected; got first 200 chars: {:?}",
        body.chars().take(200).collect::<String>()
    );
}
