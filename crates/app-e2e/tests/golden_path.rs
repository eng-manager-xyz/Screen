//! Golden-path e2e: the user drops a video, sees the `<video>` element
//! load it, then clicks play and pause.
//!
//! Runs via `just e2e` (Linux) — `cargo nextest run --workspace` skips
//! this crate by default since it requires `tauri-driver` and a real
//! `WebView`. See `_docs/book/src/app-ui/testing.md`.

use std::time::Duration;

use anyhow::{Result, anyhow};
use app_e2e::E2eApp;
use fantoccini::{Client, Locator};
use serde_json::json;

/// Element-wait deadline. Generous because cold-boot of a Tauri app
/// + Trunk-served WASM bundle takes a few seconds on first run.
const ELEMENT_WAIT: Duration = Duration::from_secs(10);

/// Poll cadence for `wait_video_paused`.
const VIDEO_POLL: Duration = Duration::from_millis(100);

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
        .ok_or_else(|| anyhow!("could not find workspace root from cwd"))?
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

    // 3. Assert the <video> element rendered with a Tauri-converted
    //    src URL. This guards M-PLAY.3's `convert_file_src` plumbing —
    //    if the JS bridge or the Leptos signal binding regressed, the
    //    element either wouldn't render or would have an empty src.
    let video = driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css("video.player-video"))
        .await?;
    let src = video
        .attr("src")
        .await?
        .ok_or_else(|| anyhow!("<video> rendered but src attribute missing"))?;
    assert!(
        !src.is_empty() && src.contains("asset"),
        "expected asset:// or http://asset.localhost URL, got {src:?}"
    );

    // 4. Click the toggle — should flip from paused to playing on both
    //    the Tauri state machine (toggle class) AND the <video> element.
    let toggle = driver.find(Locator::Css(".player-toggle")).await?;
    toggle.click().await?;
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-toggle-playing"))
        .await?;
    wait_video_paused(driver, false).await?;

    // 5. Click again — flip back to paused on both.
    let toggle = driver.find(Locator::Css(".player-toggle")).await?;
    toggle.click().await?;
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-toggle-paused"))
        .await?;
    wait_video_paused(driver, true).await?;

    Ok(())
}

/// Poll `video.player-video.paused` until it equals `want_paused` or
/// the deadline expires. `WebKit`'s `play()` is async (returns a Promise)
/// so the property doesn't flip exactly on click — give it a beat.
async fn wait_video_paused(driver: &Client, want_paused: bool) -> Result<()> {
    let deadline = std::time::Instant::now() + ELEMENT_WAIT;
    while std::time::Instant::now() < deadline {
        let result = driver
            .execute(
                "return document.querySelector('video.player-video')?.paused ?? true",
                vec![],
            )
            .await?;
        let paused = result.as_bool().unwrap_or(true);
        if paused == want_paused {
            return Ok(());
        }
        tokio::time::sleep(VIDEO_POLL).await;
    }
    Err(anyhow!(
        "video.paused did not reach {want_paused} within {ELEMENT_WAIT:?}"
    ))
}

#[tokio::test]
async fn drag_then_drop_transitions_to_playback() -> Result<()> {
    // The full happy-path the user actually performs: cursor with a
    // file enters the window → drop-zone shows the active visual →
    // user releases → app transitions to the player view → user
    // clicks play → video plays.
    //
    // Uses the user-supplied real-MP4 fixture at
    // `crates/app/video/sample.mp4` (a real H.264 video, not the
    // synthetic gradient at decode/tests/fixtures). If the drag
    // wiring landed correctly in M-POLISH.1, this scenario passes
    // end-to-end on Linux CI.
    let app = E2eApp::start().await?;
    let driver = app.client();

    let workspace_root = std::env::current_dir()?
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists() && p.join("crates").is_dir())
        .ok_or_else(|| anyhow!("could not find workspace root from cwd"))?
        .to_path_buf();
    let fixture = workspace_root.join("crates/app/video/sample.mp4");
    assert!(
        fixture.exists(),
        "user-supplied fixture missing: {}",
        fixture.display()
    );

    // 1. Idle drop-zone is the starting state.
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".drop-zone-idle"))
        .await?;

    // 2. Drag enters → active visual appears.
    let _: serde_json::Value = driver
        .execute_async(
            r"
            const [callback] = arguments;
            window.__TAURI__.core.invoke('__test_drag_enter').then(callback);
            ",
            vec![],
        )
        .await?;
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".drop-zone-active"))
        .await?;

    // 3. Drop the file → transitions to player view. Tauri's Drop
    //    handler also emits file-drag-leave so the active visual
    //    resets — guards a regression where the active class would
    //    stick after a successful drop.
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

    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-controls"))
        .await?;

    // 4. The <video> element is mounted with a Tauri-converted src.
    let video = driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css("video.player-video"))
        .await?;
    let src = video
        .attr("src")
        .await?
        .ok_or_else(|| anyhow!("<video> rendered but src attribute missing"))?;
    assert!(
        !src.is_empty() && src.contains("asset"),
        "expected asset:// or http://asset.localhost URL, got {src:?}"
    );

    // 5. Click play → both the toggle class flips AND the video
    //    element actually starts playing.
    let toggle = driver.find(Locator::Css(".player-toggle")).await?;
    toggle.click().await?;
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".player-toggle-playing"))
        .await?;
    wait_video_paused(driver, false).await?;

    Ok(())
}

#[tokio::test]
async fn drag_enter_leave_toggles_active_class() -> Result<()> {
    let app = E2eApp::start().await?;
    let driver = app.client();

    // Wait for the initial drop-zone (idle).
    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".drop-zone-idle"))
        .await?;

    // 1. Synthesize a drag-enter via the debug command.
    let _: serde_json::Value = driver
        .execute_async(
            r"
            const [callback] = arguments;
            window.__TAURI__.core.invoke('__test_drag_enter').then(callback);
            ",
            vec![],
        )
        .await?;

    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".drop-zone-active"))
        .await?;

    // 2. Synthesize a drag-leave — should revert.
    let _: serde_json::Value = driver
        .execute_async(
            r"
            const [callback] = arguments;
            window.__TAURI__.core.invoke('__test_drag_leave').then(callback);
            ",
            vec![],
        )
        .await?;

    driver
        .wait()
        .at_most(ELEMENT_WAIT)
        .for_element(Locator::Css(".drop-zone-idle"))
        .await?;

    Ok(())
}
