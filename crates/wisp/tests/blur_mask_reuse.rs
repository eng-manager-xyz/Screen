//! AUT-45 / M-DYN.3 — explicit mask-reuse via
//! `compose_blur_through_mask`.
//!
//! The high-level `apply_privacy_blur` and `apply_privacy_blur_vector`
//! generate (or fetch from cache) the mask texture internally. Some
//! callers want to share a mask across multiple effects in the same
//! frame — privacy-blur a region, then solid-redact a portion of that
//! same region, then spotlight around it — without regenerating the
//! mask three times.
//!
//! `compose_blur_through_mask(base, radius, mask, output)` accepts the
//! mask texture as an explicit parameter. This test locks in:
//!
//! 1. The explicit-mask route produces output byte-equivalent to the
//!    high-level `apply_privacy_blur` for the same shape + radius.
//! 2. Reusing one mask texture across two effects (blur + redaction)
//!    produces correct output (no regression from sharing the mask).

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{Color, Fill, Graphics, MaskShape, RenderTexture, Stage, Vector, VectorShape};

const W: u32 = 96;
const H: u32 = 96;

fn skip_on_software_adapter() -> bool {
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        eprintln!(
            "WISP_SKIP_GPU_FILTER_TESTS set — skipping blur-mask-reuse test \
             (lavapipe loses the device on multi-bind-group filter pipelines)"
        );
        return true;
    }
    false
}

fn boot() -> (Application, Renderer) {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("app");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("renderer");
    (app, renderer)
}

fn red_blue_split(app: &Application, renderer: &Renderer) -> RenderTexture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let base = RenderTexture::with_format(app, W, H, format);
    let mut stage = Stage::new();
    let mut left = Graphics::new();
    left.fill(Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)));
    left.draw_rect(Rect::new(-1.0, -1.0, 1.0, 2.0));
    let _ = stage.add_child(stage.root(), left);
    let mut right = Graphics::new();
    right.fill(Fill::Solid(Color::rgba(0.0, 0.0, 1.0, 1.0)));
    right.draw_rect(Rect::new(0.0, -1.0, 1.0, 2.0));
    let _ = stage.add_child(stage.root(), right);
    let _ = renderer.render_stage(app, base.view(), Color::BLACK, &stage);
    base
}

#[test]
fn explicit_mask_blur_matches_high_level_call() {
    if skip_on_software_adapter() {
        return;
    }
    let (app, renderer) = boot();
    let base = red_blue_split(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let region = Rect::new(-0.4, -0.4, 0.8, 0.8);
    let radius = 8.0;

    // High-level path.
    let out_high = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_privacy_blur(&app, MaskShape::rect(region), radius, &base, &out_high);

    // Explicit-mask path: generate the mask once, pass to the
    // composition primitive directly.
    let mask = renderer.generate_mask_texture(&app, MaskShape::rect(region), W, H);
    let out_explicit = RenderTexture::with_format(&app, W, H, format);
    renderer.compose_blur_through_mask(&app, &base, radius, &mask, &out_explicit);

    let bytes_high = out_high.read_pixels(&app);
    let bytes_explicit = out_explicit.read_pixels(&app);
    assert_eq!(
        bytes_high, bytes_explicit,
        "explicit-mask route must match the high-level apply_privacy_blur"
    );
}

#[test]
fn shared_mask_across_blur_and_apply_clip_works() {
    if skip_on_software_adapter() {
        return;
    }
    let (app, renderer) = boot();
    let base = red_blue_split(&app, &renderer);
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let radius = 8.0;
    let v = Vector::new(VectorShape::rect(Rect::new(-0.4, -0.4, 0.8, 0.8)));

    // Generate the mask once.
    let mask = renderer.cached_vector_mask_texture(&app, &v, W, H);

    // Use it to drive a blur.
    let blurred = RenderTexture::with_format(&app, W, H, format);
    renderer.compose_blur_through_mask(&app, &base, radius, &mask, &blurred);

    // Use it to drive an apply_mask_to_texture from the same mask.
    let masked = RenderTexture::with_format(&app, W, H, format);
    renderer.apply_mask_to_texture(&app, &base, &mask, &masked);

    // Both outputs should have masked content inside the region. The
    // exact pixel content differs (blurred vs base-cutout), but
    // outside the mask, both should preserve / be transparent
    // respectively. Sanity check: outside-region pixel of `blurred`
    // is base red; outside-region pixel of `masked` has alpha 0.
    let outside_blur = {
        let bytes = blurred.read_pixels(&app);
        let stride = (W as usize) * 4;
        let i = 4 * stride + 4 * 4;
        [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
    };
    let outside_masked = {
        let bytes = masked.read_pixels(&app);
        let stride = (W as usize) * 4;
        let i = 4 * stride + 4 * 4;
        [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
    };

    assert_eq!(outside_blur[0], 255, "blur path: outside is base red");
    assert_eq!(outside_masked[3], 0, "mask path: outside is alpha 0");
}
