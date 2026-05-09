//! M0.16 — filter pipeline integration tests.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{BlurFilter, Color, RenderTexture, Sprite, Stage, Texture};

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

fn solid_texture(app: &Application, w: u32, h: u32, rgba: [u8; 4]) -> Texture {
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..(w * h) {
        bytes.extend_from_slice(&rgba);
    }
    Texture::from_rgba(app, w, h, &bytes)
}

#[test]
fn blur_filter_smears_edge() {
    let app = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let input = RenderTexture::with_format(&app, 32, 32, format);
    let output = RenderTexture::with_format(&app, 32, 32, format);
    let renderer = Renderer::new(&app, format).expect("renderer");

    // Render a small white square in the middle of the input RT.
    let texture = solid_texture(&app, 4, 4, [255, 255, 255, 255]);
    let mut sprite = Sprite::from_texture(texture).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = Vec2::splat(0.4);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), sprite);
    let _ = renderer.render_stage(&app, input.view(), Color::BLACK, &stage);

    let pre = input.read_pixels(&app);
    // Use a tight-radius blur so the kernel stays within the rendered square
    // and the center remains bright.
    renderer.apply_filter(&app, &BlurFilter::new(1.0), &input, &output);
    let post = output.read_pixels(&app);

    // Pixel one row outside the rendered square (row 22 ≈ NDC y=-0.41,
    // sprite spans -0.4..+0.4). Must be 0 in the sharp input; non-zero after blur.
    let outside_idx = (22 * 32 + 16) * 4;
    assert_eq!(pre[outside_idx], 0, "input outside square should be black");
    assert!(
        post[outside_idx] > 0,
        "blur should leak some non-zero brightness outside the square; got {}",
        post[outside_idx]
    );

    // With a 1-texel blur kernel the center samples remain entirely inside
    // the rendered square — output center should stay near-white.
    let center_idx = (16 * 32 + 16) * 4;
    assert!(
        post[center_idx] > 200,
        "blur should keep the center bright; got {}",
        post[center_idx]
    );

    // Differential check: output != input at the boundary (proves blur
    // actually ran, regardless of the radius-vs-square-size geometry).
    let diff: u32 = pre
        .iter()
        .zip(post.iter())
        .map(|(a, b)| u32::from(a.abs_diff(*b)))
        .sum();
    assert!(
        diff > 1000,
        "blur should noticeably change the image; sum-of-diffs = {diff}"
    );
}

#[test]
fn blur_passes_count_is_two() {
    let f = BlurFilter::new(2.0);
    assert_eq!(<BlurFilter as wisp::Filter>::passes(&f), 2);
}

#[test]
fn drop_shadow_renders_visible_shadow() {
    let app = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let input = RenderTexture::with_format(&app, 64, 64, format);
    let output = RenderTexture::with_format(&app, 64, 64, format);
    let renderer = Renderer::new(&app, format).expect("renderer");

    // White square in the middle of input.
    let texture = solid_texture(&app, 4, 4, [255, 255, 255, 255]);
    let mut sprite = Sprite::from_texture(texture).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = Vec2::splat(0.4);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), sprite);
    let _ = renderer.render_stage(&app, input.view(), Color::TRANSPARENT, &stage);

    let filter = wisp::DropShadowFilter {
        offset: glam::Vec2::new(8.0, 8.0),
        blur: 3.0,
        color: Color::rgba(0.0, 0.0, 0.0, 0.8),
    };
    renderer.apply_filter(&app, &filter, &input, &output);
    let bytes = output.read_pixels(&app);

    // Shadow should appear offset from the source. Square spans roughly
    // cols/rows 19..45 in a 64-row image (NDC ±0.4). Offset (8,8) shifts
    // the shadow down-right by 8 texels; pixel (46, 46) is just past the
    // source's bottom-right corner, inside the shadow's bright halo.
    let shadow_idx = (46 * 64 + 46) * 4;
    assert!(
        bytes[shadow_idx + 3] > 20,
        "shadow alpha should be visible at shadow halo; got {}",
        bytes[shadow_idx + 3]
    );

    // Source pixel (center) should remain bright (RGB ~255) in the composite.
    let center_idx = (32 * 64 + 32) * 4;
    assert!(
        bytes[center_idx] > 200,
        "source center should remain bright over shadow; got R={}",
        bytes[center_idx]
    );
}
