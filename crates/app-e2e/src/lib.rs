//! Tier-2 e2e harness — spawns `tauri-driver` + the `screen-app` binary,
//! returns a connected [`fantoccini::Client`] to the test.
//!
//! See `_docs/book/src/app-ui/testing.md` for the full three-tier
//! strategy. This crate is excluded from `just gate` (no tauri-driver
//! installed by default); it runs via `just e2e`, which on Linux
//! wraps everything in `xvfb-run` and on macOS prints a skip message.
//!
//! ## Local prerequisites (Linux)
//!
//! ```bash
//! cargo install --locked tauri-driver
//! sudo apt-get install -y webkit2gtk-driver xvfb
//! just e2e
//! ```
//!
//! ## Lifecycle
//!
//! ```text
//! E2eApp::start()
//!   ├─ cargo build -p screen-app   (idempotent — fast on rebuild)
//!   ├─ spawn `tauri-driver`        (proxies WebDriver to the app)
//!   ├─ wait for tauri-driver to bind on port 4444
//!   ├─ build fantoccini Client with `tauri:options.application = <bin>`
//!   └─ tauri-driver spawns the app, fantoccini drives it
//!
//! E2eApp::Drop
//!   ├─ client.close().await       (best-effort)
//!   └─ tauri_driver.kill()
//! ```

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use fantoccini::{Client, ClientBuilder};
use serde_json::json;
use tokio::time::sleep;

const TAURI_DRIVER_PORT: u16 = 4444;
const TAURI_DRIVER_READY_TIMEOUT: Duration = Duration::from_secs(15);
const TAURI_DRIVER_POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Handle to a running e2e session. Drop kills the underlying processes.
pub struct E2eApp {
    /// `tauri-driver` child. Killed on Drop.
    tauri_driver: Child,
    /// Connected `WebDriver` client. The app itself is owned by tauri-driver
    /// and dies with it.
    client: Option<Client>,
}

impl E2eApp {
    /// Start the e2e harness:
    /// - `cargo build -p screen-app`
    /// - spawn `tauri-driver`
    /// - connect a fantoccini client pointed at the freshly-built binary
    ///
    /// # Errors
    ///
    /// Returns the underlying error if any step fails — most commonly
    /// "tauri-driver not on PATH" (install via
    /// `cargo install --locked tauri-driver`) or "`WebKitGTK` driver not
    /// available" (Linux: `apt install webkit2gtk-driver`).
    pub async fn start() -> Result<Self> {
        build_screen_app().context("cargo build -p screen-app")?;

        let mut tauri_driver = spawn_tauri_driver()?;
        wait_for_port(TAURI_DRIVER_PORT, TAURI_DRIVER_READY_TIMEOUT).await?;

        let bin_path = screen_app_binary_path()?;
        let client = build_client(&bin_path).await.inspect_err(|_err| {
            // Best-effort cleanup if the client build failed — leaving
            // tauri-driver alive would block port 4444 on the next run.
            let _ = tauri_driver.kill();
        })?;

        Ok(Self {
            tauri_driver,
            client: Some(client),
        })
    }

    /// The connected `WebDriver` client. Use `find` / `execute` / etc.
    #[must_use]
    pub fn client(&self) -> &Client {
        self.client
            .as_ref()
            .expect("E2eApp.client() called after Drop")
    }
}

impl Drop for E2eApp {
    fn drop(&mut self) {
        // Close the WebDriver session first (asks tauri-driver to shut
        // the app cleanly); fall back to killing tauri-driver outright.
        if let Some(client) = self.client.take() {
            let _ = std::thread::spawn(move || {
                let runtime = tokio::runtime::Runtime::new();
                if let Ok(rt) = runtime {
                    let _ = rt.block_on(client.close());
                }
            })
            .join();
        }
        let _ = self.tauri_driver.kill();
    }
}

/// Build `screen-app` so `target/debug/screen-app` exists. Idempotent —
/// cargo skips work when nothing has changed.
fn build_screen_app() -> Result<()> {
    let status = Command::new("cargo")
        .args(["build", "-p", "screen-app", "--bin", "screen-app"])
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("failed to invoke cargo")?;
    if !status.success() {
        return Err(anyhow!("cargo build -p screen-app failed: {status}"));
    }
    Ok(())
}

/// Spawn `tauri-driver` on the default port. Returns the child handle.
fn spawn_tauri_driver() -> Result<Child> {
    Command::new("tauri-driver")
        .args(["--port", &TAURI_DRIVER_PORT.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            "failed to spawn `tauri-driver` — install via \
             `cargo install --locked tauri-driver` and ensure it's on PATH"
        })
}

/// Poll `localhost:<port>` until something is listening, or time out.
async fn wait_for_port(port: u16, timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(anyhow!(
                "tauri-driver did not bind on port {port} within {timeout:?}"
            ));
        }
        sleep(TAURI_DRIVER_POLL_INTERVAL).await;
    }
}

/// Resolve the path to the freshly-built `screen-app` binary. Walks up
/// from `CARGO_MANIFEST_DIR` to the workspace root, then descends into
/// `target/debug/`.
fn screen_app_binary_path() -> Result<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| anyhow!("cannot ascend from CARGO_MANIFEST_DIR to workspace root"))?;
    let bin_name = if cfg!(windows) {
        "screen-app.exe"
    } else {
        "screen-app"
    };
    let path = workspace_root.join("target").join("debug").join(bin_name);
    if !path.exists() {
        return Err(anyhow!(
            "screen-app binary not found at {} — did `cargo build -p screen-app` succeed?",
            path.display()
        ));
    }
    Ok(path)
}

/// Build a fantoccini `Client` with the Tauri-specific capabilities.
async fn build_client(bin_path: &std::path::Path) -> Result<Client> {
    let capabilities = json!({
        "tauri:options": {
            "application": bin_path.to_string_lossy(),
        },
    });
    let map: serde_json::Map<String, serde_json::Value> =
        capabilities.as_object().expect("object").clone();

    let client = ClientBuilder::native()
        .capabilities(map)
        .connect(&format!("http://localhost:{TAURI_DRIVER_PORT}"))
        .await
        .context("fantoccini failed to connect to tauri-driver")?;
    Ok(client)
}
