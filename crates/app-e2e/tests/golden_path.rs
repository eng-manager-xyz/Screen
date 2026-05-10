//! Golden-path e2e: the user drops a video, clicks play, then pause.
//!
//! Runs via `just e2e` (Linux) — `cargo nextest run --workspace` skips
//! this crate by default since it requires `tauri-driver` and a real
//! `WebView`. See `_docs/book/src/app-ui/testing.md`.

use std::time::Duration;

use anyhow::Result;
use app_e2e::E2eApp;
use fantoccini::Locator;
use serde_json::json;

/// Element-wait deadline. Generous because cold-boot of a Tauri app
/// + Trunk-served WASM bundle takes a few seconds on first run.
const ELEMENT_WAIT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn golden_path_drop_play_pause() -> Result<()> {
    let app = E2eApp::start().await?;
    let driver = app.client();

    // The fixture path needs to be absolute when handed to `player_open`
    // — the app's cwd is wherever tauri-driver spawned it. Resolve from
    // the workspace root we ascended to in `screen_app_binary_path()`.
    let workspace_root = std::env::current_dir()?
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").is_dir())
        .ok_or_else(|| anyhow::anyhow!("could not find workspace root from cwd"))?
        .to_path_buf();
    let fixture = workspace_root.join("crates/decode/tests/fixtures/sample.mp4");
    assert!(fixture.exists(), "fixture missing: {}", fixture.display());

    // 1. Synthesize a file drop via the debug-only Tauri command.
    let fixture_str = fixture.to_string_lossy().to_string();
    let _: serde_json::Value = driver
        .execute_async(
            r"
            const [path, callback] = arguments;
            window.__TAURI__.core.invoke('__test_drop_file', { path })
                .then(callback);
            ",
            vec![json!(fixture_str)],
        )
        .await?;

    // 2. Wait for the player view to appear (drop-zone replaced).
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-controls"))
        .await?;

    // 3. Click the toggle — should flip from paused to playing.
    let toggle = driver.find(Locator::Css(".player-toggle")).await?;
    toggle.click().await?;
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-toggle-playing"))
        .await?;

    // 4. Click again — flip back to paused.
    let toggle = driver.find(Locator::Css(".player-toggle")).await?;
    toggle.click().await?;
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-toggle-paused"))
        .await?;

    Ok(())
}
