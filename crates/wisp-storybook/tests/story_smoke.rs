//! Story smoke + visibility tests.
//!
//! For every shipped story, we:
//!   1. Push a wgpu validation error scope.
//!   2. Build the scene.
//!   3. Tick at `t = 0.0` (gives animated stories a deterministic frame).
//!   4. Render to a 256×256 `RenderTexture`.
//!   5. Pop the error scope and assert no validation errors.
//!   6. Read pixels and assert at least one channel deviates from the
//!      clear color (proves the story drew something visible).
//!
//! This is the "no console errors at runtime" gate the user asked for.

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Stage};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 256;
const CLEAR: [u8; 4] = [18, 18, 22, 255];

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

fn clear_color() -> Color {
    Color::rgba_u8(CLEAR[0], CLEAR[1], CLEAR[2], CLEAR[3])
}

/// Stories whose multi-bind-group filter pipelines lavapipe (mesa's
/// software Vulkan, the only Vulkan available on GitHub-hosted Linux
/// runners) loses the device on. Real adapters (Metal locally and on
/// macos-latest CI) build them fine. The CI workflow sets
/// `WISP_SKIP_GPU_FILTER_TESTS=1` on the Linux job; this filter
/// transparently drops those stories so the rest can still run.
///
/// **Adding a new story?** If its `build()` ever calls
/// `apply_filter(BlurFilter, …)`, `apply_filter(DropShadowFilter, …)`,
/// `apply_filter(MotionBlurFilter, …)`, `apply_privacy_blur*`, or any
/// other wrapper that touches `crate::filter::blur::run_blur_pass`,
/// add the story id here. `DropShadowFilter` and `MotionBlurFilter`
/// **both** call `run_blur_pass` internally — they're transitively
/// the same problem as a raw `BlurFilter`.
const LAVAPIPE_INCOMPATIBLE: &[&str] = &[
    "filter-blur",
    "filter-drop-shadow",
    "filter-motion-blur",
    // M-MASK.2..4 stories build their preview RTs via `apply_privacy_blur`
    // / `apply_privacy_blur_data`, which run a `BlurFilter` pass under the
    // hood — same multi-bind-group pipeline that lavapipe loses the device
    // on. The non-blur mask stories (solid-redaction, spotlight,
    // dim-outside, webcam-shapes, ellipse-mask, path-mask, clip-rounded)
    // stay in the smoke set and run on Linux just fine.
    "privacy-blur-rect",
    "privacy-blur-rounded",
    "privacy-blur-strength",
    // M-TEXT.8 — `apply_filter(DropShadowFilter)` ×2 (shadow + glow).
    // `DropShadowFilter` performs alpha-extract → run_blur_pass →
    // composite, so it pulls the same blur pipeline that lavapipe
    // can't keep alive. The story's `build()` runs unconditionally; no
    // env-var guard is possible without losing the visual demo.
    "text-shadow-glow",
];

fn stories_for_env() -> Vec<wisp_storybook::story::Story> {
    let all = wisp_storybook::stories::all_stories();
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        eprintln!(
            "WISP_SKIP_GPU_FILTER_TESTS set — filtering {} lavapipe-incompatible \
             stories (validated on macos-latest with real Metal)",
            LAVAPIPE_INCOMPATIBLE.len()
        );
        all.into_iter()
            .filter(|s| !LAVAPIPE_INCOMPATIBLE.contains(&s.id))
            .collect()
    } else {
        all
    }
}

#[test]
fn every_story_renders_without_validation_errors() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, SIZE, SIZE, FORMAT);
    let renderer = Renderer::new(&app, FORMAT).expect("renderer");

    for story in stories_for_env() {
        app.device().push_error_scope(wgpu::ErrorFilter::Validation);

        let mut stage = Stage::new();
        (story.build)(&app, &mut stage);
        story.tick(&mut stage, 0.0);
        let _stats = renderer.render_stage(&app, rt.view(), clear_color(), &stage);
        // Force the queue so any deferred validation surfaces here.
        app.device().poll(wgpu::Maintain::Wait);

        let err = block_on(app.device().pop_error_scope());
        assert!(
            err.is_none(),
            "story `{}` ({}) raised wgpu validation: {:?}",
            story.title,
            story.milestone,
            err,
        );
    }
}

#[test]
fn every_story_draws_visible_pixels() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, SIZE, SIZE, FORMAT);
    let renderer = Renderer::new(&app, FORMAT).expect("renderer");

    for story in stories_for_env() {
        let mut stage = Stage::new();
        (story.build)(&app, &mut stage);
        story.tick(&mut stage, 0.0);
        let _ = renderer.render_stage(&app, rt.view(), clear_color(), &stage);

        let bytes = rt.read_pixels(&app);
        // Find at least one pixel that diverges meaningfully from the clear color.
        let mut visible_pixels = 0u32;
        for chunk in bytes.chunks_exact(4) {
            let dr = chunk[0].abs_diff(CLEAR[0]);
            let dg = chunk[1].abs_diff(CLEAR[1]);
            let db = chunk[2].abs_diff(CLEAR[2]);
            if dr > 16 || dg > 16 || db > 16 {
                visible_pixels = visible_pixels.saturating_add(1);
            }
        }
        assert!(
            visible_pixels > 50,
            "story `{}` ({}) drew effectively nothing — only {visible_pixels} pixels diverged from the clear color",
            story.title,
            story.milestone,
        );
    }
}
