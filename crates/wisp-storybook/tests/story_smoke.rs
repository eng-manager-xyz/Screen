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

#[test]
fn every_story_renders_without_validation_errors() {
    let app = boot();
    let rt = RenderTexture::with_format(&app, SIZE, SIZE, FORMAT);
    let renderer = Renderer::new(&app, FORMAT).expect("renderer");

    for story in wisp_storybook::stories::all_stories() {
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

    for story in wisp_storybook::stories::all_stories() {
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
