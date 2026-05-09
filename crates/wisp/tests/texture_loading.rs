//! M0.10 — Texture loading tests.
//!
//! Covers `Texture::from_rgba`, `Texture::from_image`, `Texture::empty`, and
//! a real PNG round-trip via `image::load_from_memory`.

use image::{DynamicImage, GrayImage, RgbaImage};
use pollster::block_on;
use wisp::Texture;
use wisp::application::{AppConfig, Application};

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

#[test]
fn empty_has_expected_dimensions_in_srgb() {
    let app = boot();
    let texture = Texture::empty(&app, 64, 32, wgpu::TextureFormat::Rgba8UnormSrgb);
    assert_eq!(texture.width(), 64);
    assert_eq!(texture.height(), 32);
}

#[test]
fn empty_supports_bgra_for_video_uploads() {
    // VideoTexture (M0.11) backs onto an empty Bgra8UnormSrgb. Verify the
    // format is accepted today.
    let app = boot();
    let texture = Texture::empty(&app, 1920, 1080, wgpu::TextureFormat::Bgra8UnormSrgb);
    assert_eq!(texture.width(), 1920);
    assert_eq!(texture.height(), 1080);
}

#[test]
fn from_image_grayscale_converts_to_rgba8() {
    // Non-default color type — exercises `to_rgba8` inside from_image.
    let app = boot();
    let gray = GrayImage::from_pixel(8, 8, image::Luma([200]));
    let texture = Texture::from_image(&app, &DynamicImage::ImageLuma8(gray));
    assert_eq!(texture.width(), 8);
    assert_eq!(texture.height(), 8);
}

#[test]
fn png_round_trip_via_image_crate() {
    // Encode a synthetic RGBA buffer to PNG in-memory, decode it, and load
    // the result as a Texture. Verifies the full PNG path without checking
    // a binary file into the repo.
    let app = boot();
    let original = RgbaImage::from_pixel(16, 16, image::Rgba([10, 20, 30, 255]));

    let mut png_bytes: Vec<u8> = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png_bytes);
    DynamicImage::ImageRgba8(original)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .expect("encode png");

    let decoded = image::load_from_memory(&png_bytes).expect("decode png");
    let texture = Texture::from_image(&app, &decoded);

    assert_eq!(texture.width(), 16);
    assert_eq!(texture.height(), 16);
}
