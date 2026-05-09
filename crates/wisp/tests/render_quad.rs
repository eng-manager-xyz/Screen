//! M0.6 — integration tests for textured quad rendering.
//!
//! Pixel-readback tests are deferred to M0.11 (when `RenderTexture::read_pixels`
//! lands). For now we validate:
//!   1. `Texture::from_rgba` produces a texture with expected dimensions.
//!   2. `Texture::from_image` round-trips a generated `DynamicImage`.
//!   3. `Renderer::render_quad` does not panic / wgpu-validate-fail when
//!      drawing into an offscreen render target — i.e. the pipeline + bind
//!      groups + shader form a valid wgpu submission.

use glam::Mat4;
use image::{DynamicImage, RgbaImage};
use pollster::block_on;
use wisp::Texture;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

fn make_offscreen(app: &Application) -> (wgpu::Texture, wgpu::TextureView, wgpu::TextureFormat) {
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let texture = app.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("test offscreen target"),
        size: wgpu::Extent3d {
            width: 64,
            height: 64,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view, format)
}

#[test]
fn texture_from_rgba_has_expected_dimensions() {
    let app = boot();
    let bytes = vec![255u8; 16 * 16 * 4];
    let texture = Texture::from_rgba(&app, 16, 16, &bytes);
    assert_eq!(texture.width(), 16);
    assert_eq!(texture.height(), 16);
}

#[test]
fn texture_from_image_round_trips_dimensions() {
    let app = boot();
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(
        32,
        17,
        image::Rgba([10, 20, 30, 255]),
    ));
    let texture = Texture::from_image(&app, &img);
    assert_eq!(texture.width(), 32);
    assert_eq!(texture.height(), 17);
}

#[test]
#[should_panic(expected = "byte length mismatch")]
fn texture_from_rgba_rejects_wrong_byte_length() {
    let app = boot();
    // 8×8×4 = 256 bytes; we pass 100.
    let bytes = vec![0u8; 100];
    let _ = Texture::from_rgba(&app, 8, 8, &bytes);
}

#[test]
fn render_quad_validates_to_offscreen_target() {
    let app = boot();
    let bytes = vec![255u8; 4 * 4 * 4];
    let texture = Texture::from_rgba(&app, 4, 4, &bytes);

    let (_target, view, format) = make_offscreen(&app);
    let renderer = Renderer::new(&app, format).expect("renderer");

    // If the pipeline / bind groups / shader form an invalid submission, wgpu
    // panics during validation. Reaching the end means the path is sound.
    renderer.render_quad(
        &app,
        &view,
        wisp::Color::BLACK,
        &texture,
        Mat4::IDENTITY,
        wisp::Color::WHITE,
    );

    // Force the queue to flush so any deferred validation surfaces here.
    app.device().poll(wgpu::Maintain::Wait);
}
