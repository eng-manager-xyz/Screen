//! AUT-226 / M-TEXT.20 — verify `wisp::Text` rotation under
//! `Container::transform`.
//!
//! Y-axis titles on charts need vertical (rotated -90°) text. Same
//! goes for radar axis labels, calendar month strips, and any
//! other rotated label. This test exists to prove the existing
//! scene-walk + text pipeline propagates `Container::transform`'s
//! rotation through to glyph placement so chart code can rely on
//! it without further wisp-side enablers.
//!
//! Strategy: render the same string twice — unrotated, and rotated
//! -π/2 — to an offscreen `Rgba8Unorm` `RenderTexture`. Compute
//! the bounding box of red ink in each, and assert the aspect
//! ratios flip (horizontal text → wide bbox; rotated text → tall
//! bbox). Robust against minor sub-pixel placement differences;
//! sensitive to the orientation actually changing.

#![allow(
    clippy::cast_precision_loss,
    reason = "bbox coords + viewport dims are bounded by W=128, H=128 — \
              well below the f32 precision boundary."
)]

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, Font, RenderTexture, Stage, Text};

const W: u32 = 128;
const H: u32 = 128;

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

/// Bounding box of pixels whose red channel exceeds `threshold`.
/// Returns `None` if no pixels qualify.
fn red_bbox(bytes: &[u8], threshold: u8) -> Option<(u32, u32, u32, u32)> {
    let mut x_min = W;
    let mut x_max = 0u32;
    let mut y_min = H;
    let mut y_max = 0u32;
    let mut any = false;
    for y in 0..H {
        for x in 0..W {
            let i = ((y * W + x) * 4) as usize;
            if bytes[i] > threshold {
                x_min = x_min.min(x);
                x_max = x_max.max(x);
                y_min = y_min.min(y);
                y_max = y_max.max(y);
                any = true;
            }
        }
    }
    if any {
        Some((x_min, x_max, y_min, y_max))
    } else {
        None
    }
}

fn render_text_with_rotation(rotation_radians: f32) -> Vec<u8> {
    let app = boot();
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let renderer = Renderer::new(&app, rt.format()).expect("renderer");

    let font = Font::bitmap_8x8(&app);
    let mut text = Text::new(font, "ABCDE").with_cell_size(0.04);
    text.color = Color::rgba_u8(255, 0, 0, 255);
    // Pull the string left so the unrotated layout (which extends
    // rightward from its origin) is roughly centred. After rotation
    // it extends downward / leftward instead, which the bbox test
    // below picks up.
    text.container.transform.position = glam::Vec2::new(-0.5, 0.1);
    text.container.transform.rotation = rotation_radians;

    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), text);
    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    rt.read_pixels(&app)
}

#[test]
fn rotation_zero_produces_wide_text_bbox() {
    let bytes = render_text_with_rotation(0.0);
    let bbox = red_bbox(&bytes, 50).expect("expected red ink in unrotated render");
    let width = bbox.1 - bbox.0 + 1;
    let height = bbox.3 - bbox.2 + 1;
    assert!(
        width > height,
        "unrotated 5-char string should be wider than tall: bbox={bbox:?} (w={width}, h={height})"
    );
}

#[test]
fn rotation_negative_pi_over_two_produces_tall_text_bbox() {
    let bytes = render_text_with_rotation(-std::f32::consts::FRAC_PI_2);
    let bbox = red_bbox(&bytes, 50).expect("expected red ink in rotated render");
    let width = bbox.1 - bbox.0 + 1;
    let height = bbox.3 - bbox.2 + 1;
    assert!(
        height > width,
        "rotated -π/2 5-char string should be taller than wide: bbox={bbox:?} \
         (w={width}, h={height}) — if this fails, Container::transform.rotation \
         is not propagating to wisp::Text glyph placement"
    );
}

#[test]
fn rotation_changes_bbox_orientation() {
    let h = render_text_with_rotation(0.0);
    let v = render_text_with_rotation(-std::f32::consts::FRAC_PI_2);
    let hb = red_bbox(&h, 50).expect("unrotated bbox");
    let vb = red_bbox(&v, 50).expect("rotated bbox");
    let h_aspect = (hb.1 - hb.0 + 1) as f32 / (hb.3 - hb.2 + 1) as f32;
    let v_aspect = (vb.1 - vb.0 + 1) as f32 / (vb.3 - vb.2 + 1) as f32;
    // Unrotated should be wide (aspect > 1); rotated should be tall
    // (aspect < 1). Together: their ratio should be very different.
    assert!(
        h_aspect > 1.5,
        "unrotated aspect should be > 1.5 (wide); got {h_aspect:.2}"
    );
    assert!(
        v_aspect < 0.67,
        "rotated aspect should be < 0.67 (tall); got {v_aspect:.2} \
         — Container::transform.rotation not propagating to Text"
    );
}
