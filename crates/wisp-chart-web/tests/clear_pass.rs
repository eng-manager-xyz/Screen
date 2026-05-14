//! Native readback test for [`wisp_chart_web::clear_with_color`].
//!
//! Runs the helper against an offscreen `RenderTexture` on whichever
//! wgpu backend the host exposes (Metal on macOS, Vulkan / lavapipe
//! on Linux CI, DX12 on Windows) and asserts that every pixel is the
//! distinctive demo purple
//! ([`wisp_chart_web::DEMO_CLEAR_RGBA8`] = `[153, 51, 204, 255]`).
//! Catches "the render code wrote wrong pixels" regressions — the
//! layer that `cargo check` cannot.
//!
//! Does **not** validate the `BROWSER_WEBGPU` surface-presentation
//! path. That's `tests/headless_webgpu.rs`'s job and is local-only.
//!
//! On success the test also writes
//! `_docs/wisp-chart-book/src/assets/wisp-chart-web/demo-purple.png`
//! — the PR-visible, byte-identical snapshot of what the demo will
//! show in the browser. Regenerating that PNG IS the commit-time
//! evidence that the assertion fired against real pixels, not just
//! the constant it compared to.

#![cfg(not(target_arch = "wasm32"))]

use std::path::PathBuf;

use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp_chart_web::{DEMO_CLEAR_COLOR, DEMO_CLEAR_RGBA8, clear_with_color};

/// Snapshot size — large enough to be visibly identifiable in a
/// GitHub PR preview, small enough that the committed PNG stays
/// well under 1 KB (PNG run-length / filter compression on a
/// uniform fill is highly effective).
const W: u32 = 256;
const H: u32 = 256;

#[test]
fn clear_pass_paints_every_pixel_demo_purple() {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");

    // Mirror the demo's choice of an `Rgba8Unorm` colour target —
    // the established wisp test pattern (clip_circle.rs et al.).
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);

    app.device().push_error_scope(wgpu::ErrorFilter::Validation);
    clear_with_color(app.device(), app.queue(), rt.view(), DEMO_CLEAR_COLOR);
    let validation = block_on(app.device().pop_error_scope());
    assert!(
        validation.is_none(),
        "wgpu validation error during clear_with_color: {validation:?}"
    );

    let bytes = rt.read_pixels(&app);
    let expected_len = (W * H * 4) as usize;
    assert_eq!(
        bytes.len(),
        expected_len,
        "unexpected readback length: expected {expected_len}, got {}",
        bytes.len()
    );

    // Every pixel must be the demo purple — the helper clears the
    // entire view, no exception. Failing this means either the
    // clear op no-op'd or the colour-conversion path is wrong.
    for (i, chunk) in bytes.chunks_exact(4).enumerate() {
        assert_eq!(
            chunk, DEMO_CLEAR_RGBA8,
            "pixel {i} expected {DEMO_CLEAR_RGBA8:?}, got {chunk:?}"
        );
    }

    // Write the rendered output to the committed snapshot path so
    // the PR diff shows the real bytes. `CARGO_MANIFEST_DIR` is
    // `crates/wisp-chart-web` at test time.
    let snapshot_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/wisp-chart-book/src/assets/wisp-chart-web/demo-purple.png");
    if let Some(parent) = snapshot_path.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot_path, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write demo-purple.png");
}
