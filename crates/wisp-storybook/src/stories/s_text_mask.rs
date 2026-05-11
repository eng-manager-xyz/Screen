//! Story: text used as an alpha mask (M-TEXT.11 / AUT-85).
//!
//! Three compositions, one mask: render "WISP" into a `RenderTexture`,
//! then call `Renderer::apply_mask_to_texture(foreground, text_mask,
//! output)` three times with three foreground RTs — a gradient fill,
//! a blurred backdrop, and a spotlight highlight. The text silhouette
//! is the only common element; the foreground does the colored work.

use glam::Vec2;
use wisp::application::Application;
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::text::{TextTexturePipeline, WispText, WispTextStyle};
use wisp::texture::render_texture::RenderTexture;
use wisp::{
    BlurFilter, Color, DropShadowFilter, Fill, Graphics, Sprite, Stage, Stroke, Texture,
    WispFontWeight,
};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-mask",
        category: "Text",
        title: "Text as mask — fill / blur / spotlight",
        milestone: "M-TEXT.11",
        writeup: include_str!("writeups/text_mask.md"),
        build,
        tick: None,
    }
}

const SIZE: u32 = 256;

fn build(app: &Application, stage: &mut Stage) {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);
    let root = stage.root();

    // ── Build the text alpha mask in the renderer's format ────
    let text = WispText::new("WISP").with_style(
        WispTextStyle::default()
            .with_size(0.95)
            .with_weight(WispFontWeight::Bold)
            .with_color(Color::WHITE),
    );
    let text_rt = pipeline.render(app, &text, SIZE, SIZE);
    let text_bytes = text_rt.read_pixels(app);
    let text_tex = Texture::from_rgba(app, SIZE, SIZE, &text_bytes);

    let mask_rt = RenderTexture::with_format(app, SIZE, SIZE, format);
    let mut mask_stage = Stage::new();
    let mut mask_sprite = Sprite::from_texture(text_tex.clone()).with_anchor(Vec2::new(0.0, 0.0));
    mask_sprite.container.transform.position = Vec2::new(-1.0, 1.0);
    mask_sprite.container.transform.scale = Vec2::new(2.0, -2.0);
    let _ = mask_stage.add_child(mask_stage.root(), mask_sprite);
    let _ = renderer.render_stage(app, mask_rt.view(), Color::TRANSPARENT, &mask_stage);

    // ── Three foreground RTs ──────────────────────────────────
    let fill_rt = render_gradient_fill(app, &renderer, format);
    let blur_rt = render_blur_backdrop(app, &renderer, format);
    let spot_rt = render_spotlight(app, &renderer, format);

    // ── Three masked outputs ──────────────────────────────────
    let fill_out = RenderTexture::with_format(app, SIZE, SIZE, format);
    renderer.apply_mask_to_texture(app, &fill_rt, &mask_rt, &fill_out);

    let blur_out = RenderTexture::with_format(app, SIZE, SIZE, format);
    renderer.apply_mask_to_texture(app, &blur_rt, &mask_rt, &blur_out);

    let spot_out = RenderTexture::with_format(app, SIZE, SIZE, format);
    renderer.apply_mask_to_texture(app, &spot_rt, &mask_rt, &spot_out);

    // ── Paper-white backdrop ──────────────────────────────────
    let backdrop_rt = RenderTexture::with_format(app, SIZE, SIZE, format);
    let _ = renderer.render_stage(
        app,
        backdrop_rt.view(),
        Color::rgba(0.94, 0.95, 0.97, 1.0),
        &Stage::new(),
    );
    let backdrop_bytes = backdrop_rt.read_pixels(app);
    let backdrop_tex = Texture::from_rgba(app, SIZE, SIZE, &backdrop_bytes);
    let mut backdrop = Sprite::from_texture(backdrop_tex).with_anchor(Vec2::splat(0.5));
    backdrop.container.transform.scale = Vec2::splat(2.0);
    let _ = stage.add_child(root, backdrop);

    // ── Three rows of masked text (top: fill, mid: blur, bot: spot) ──
    place_masked(stage, root, &fill_out, app, 0.55);
    place_masked(stage, root, &blur_out, app, 0.05);
    place_masked(stage, root, &spot_out, app, -0.45);
}

fn place_masked(
    stage: &mut Stage,
    root: wisp::NodeId,
    rt: &RenderTexture,
    app: &Application,
    y: f32,
) {
    let bytes = rt.read_pixels(app);
    let tex = Texture::from_rgba(app, SIZE, SIZE, &bytes);
    let mut sprite = Sprite::from_texture(tex).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.position = Vec2::new(0.0, y);
    sprite.container.transform.scale = Vec2::new(1.8, -0.4);
    let _ = stage.add_child(root, sprite);
}

fn render_gradient_fill(
    app: &Application,
    renderer: &Renderer,
    format: wgpu::TextureFormat,
) -> RenderTexture {
    // Horizontal stripes in saturated colors — looks like a poster fill
    // through the text silhouette.
    let rt = RenderTexture::with_format(app, SIZE, SIZE, format);
    let mut s = Stage::new();
    let bands = [
        (Color::rgba_u8(255, 100, 80, 255), -1.0),
        (Color::rgba_u8(255, 200, 60, 255), -0.5),
        (Color::rgba_u8(80, 220, 120, 255), 0.0),
        (Color::rgba_u8(60, 170, 240, 255), 0.5),
    ];
    let mut g = Graphics::new();
    for (color, y0) in bands {
        g.fill(Fill::Solid(color));
        g.draw_rect(Rect::new(-1.0, y0, 2.0, 0.5));
    }
    let _ = s.add_child(s.root(), g);
    let _ = renderer.render_stage(app, rt.view(), Color::TRANSPARENT, &s);
    rt
}

fn render_blur_backdrop(
    app: &Application,
    renderer: &Renderer,
    format: wgpu::TextureFormat,
) -> RenderTexture {
    // Render a few overlapping circles then blur them — the masked
    // text reveals a soft-focus background. Skip on lavapipe (the blur
    // multi-bind-group filter loses the device); the story still
    // smoke-tests because we substitute a sharp version.
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        return render_circles(app, renderer, format);
    }
    let sharp = render_circles(app, renderer, format);
    let blurred = RenderTexture::with_format(app, SIZE, SIZE, format);
    renderer.apply_filter(app, &BlurFilter { radius: 8.0 }, &sharp, &blurred);
    blurred
}

fn render_circles(
    app: &Application,
    renderer: &Renderer,
    format: wgpu::TextureFormat,
) -> RenderTexture {
    let rt = RenderTexture::with_format(app, SIZE, SIZE, format);
    let mut s = Stage::new();
    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(60, 120, 220, 255)));
    g.draw_ellipse(Vec2::new(-0.5, 0.3), Vec2::splat(0.45));
    g.fill(Fill::Solid(Color::rgba_u8(255, 130, 70, 255)));
    g.draw_ellipse(Vec2::new(0.5, -0.2), Vec2::splat(0.45));
    g.fill(Fill::Solid(Color::rgba_u8(80, 220, 160, 255)));
    g.draw_ellipse(Vec2::new(0.0, 0.0), Vec2::splat(0.40));
    let _ = s.add_child(s.root(), g);
    let _ = renderer.render_stage(app, rt.view(), Color::rgba(0.05, 0.05, 0.08, 1.0), &s);
    rt
}

fn render_spotlight(
    app: &Application,
    renderer: &Renderer,
    format: wgpu::TextureFormat,
) -> RenderTexture {
    // Bright warm center, dark falloff at edges. The text-mask reveals
    // a glowing fragment of the spotlight.
    let rt = RenderTexture::with_format(app, SIZE, SIZE, format);
    let mut s = Stage::new();
    let mut g = Graphics::new();
    g.fill(Fill::Solid(Color::rgba_u8(255, 235, 200, 255)));
    g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    g.fill(Fill::Solid(Color::rgba_u8(255, 180, 80, 230)));
    g.stroke(Some(Stroke::new(0.04, Color::rgba_u8(255, 220, 100, 255))));
    g.draw_ellipse(Vec2::new(0.0, 0.0), Vec2::splat(0.55));
    g.stroke(None);
    g.fill(Fill::Solid(Color::rgba_u8(0, 0, 0, 60)));
    g.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
    let _ = s.add_child(s.root(), g);
    // Light blur to soften — DropShadowFilter with offset zero produces
    // a softer warm field; for lavapipe-safe path we skip the blur.
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() {
        let direct = RenderTexture::with_format(app, SIZE, SIZE, format);
        let _ = renderer.render_stage(app, direct.view(), Color::TRANSPARENT, &s);
        return direct;
    }
    let input = RenderTexture::with_format(app, SIZE, SIZE, format);
    let _ = renderer.render_stage(app, input.view(), Color::TRANSPARENT, &s);
    renderer.apply_filter(
        app,
        &DropShadowFilter {
            offset: Vec2::new(0.0, 0.0),
            blur: 4.0,
            color: Color::rgba(1.0, 0.8, 0.4, 0.6),
        },
        &input,
        &rt,
    );
    rt
}
