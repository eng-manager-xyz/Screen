//! Story: vector-backed callout text boxes (M-TEXT.10 / AUT-84).
//!
//! Five callout shapes — label box, caption pill, number badge,
//! pointer + label, arrow + label — composed from `Graphics`
//! primitives + `CaptionBlock` / text sprites. No new wisp types; the
//! existing `Graphics::draw_rounded_rect` / `draw_ellipse` /
//! `draw_line` cover the vocabulary.

use glam::Vec2;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp::text::{CaptionBlock, TextPreset, TextTexturePipeline, WispText};
use wisp::texture::render_texture::RenderTexture;
use wisp::{Color, Container, Fill, Graphics, Sprite, Stage, Stroke};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-callouts",
        category: "Text",
        title: "Callout boxes + label + badges + arrows",
        milestone: "M-TEXT.10",
        writeup: include_str!("writeups/text_callouts.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);
    let root = stage.root();

    // ── Light backdrop sprite ──────────────────────────────────
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(app, format).expect("renderer");
    let backdrop_rt = RenderTexture::with_format(app, 256, 256, format);
    let _ = renderer.render_stage(
        app,
        backdrop_rt.view(),
        Color::rgba(0.92, 0.93, 0.95, 1.0),
        &Stage::new(),
    );
    let backdrop_bytes = backdrop_rt.read_pixels(app);
    let backdrop_tex = wisp::Texture::from_rgba(app, 256, 256, &backdrop_bytes);
    let mut backdrop = Sprite::from_texture(backdrop_tex).with_anchor(Vec2::splat(0.5));
    backdrop.container.transform.scale = Vec2::splat(2.0);
    let _ = stage.add_child(root, backdrop);

    // ── 1) Caption pill (top center) ───────────────────────────
    let pill = CaptionBlock::from_text(
        WispText::new("Now recording")
            .with_style(TextPreset::Caption.style().with_color(Color::WHITE)),
    )
    .with_width(0.7)
    .with_padding(0.04)
    .with_radius(0.10)
    .with_background(Color::rgba_u8(220, 60, 80, 240));
    let layout = pill.layout(app, &pipeline);
    let mut cont = Container::new();
    cont.transform.position = Vec2::new(-0.35, 0.75);
    let id = stage.add_child(root, cont).expect("attach");
    let _ = stage.add_child(id, layout.background);
    let _ = stage.add_child(id, layout.text_sprite);

    // ── 2) Number badge (small filled circle + number) ────────
    let mut badge_bg = Graphics::new();
    badge_bg.fill(Fill::Solid(Color::rgba_u8(45, 130, 220, 255)));
    badge_bg.draw_ellipse(Vec2::new(-0.65, 0.35), Vec2::splat(0.08));
    let _ = stage.add_child(root, badge_bg);

    let n_text =
        WispText::new("3").with_style(TextPreset::StepBadge.style().with_color(Color::WHITE));
    let n_rt = pipeline.render(app, &n_text, 192, 192);
    let mut n_sprite = Sprite::from_texture(n_rt.as_texture()).with_anchor(Vec2::splat(0.5));
    n_sprite.container.transform.position = Vec2::new(-0.65, 0.35);
    n_sprite.container.transform.scale = Vec2::new(0.12, -0.12);
    let _ = stage.add_child(root, n_sprite);

    // ── 3) Label box (rounded rect + text, center-left) ───────
    let label = CaptionBlock::from_text(
        WispText::new("Caption block")
            .with_style(TextPreset::Caption.style().with_color(Color::WHITE)),
    )
    .with_width(0.55)
    .with_padding(0.04)
    .with_radius(0.04)
    .with_background(Color::rgba_u8(20, 28, 40, 220));
    let lab_layout = label.layout(app, &pipeline);
    let mut lab_cont = Container::new();
    lab_cont.transform.position = Vec2::new(-0.45, -0.05);
    let lab_id = stage.add_child(root, lab_cont).expect("attach");
    let _ = stage.add_child(lab_id, lab_layout.background);
    let _ = stage.add_child(lab_id, lab_layout.text_sprite);

    // ── 4) Pointer + label (line connecting label to an anchor) ─
    let mut pointer = Graphics::new();
    pointer.stroke(Some(Stroke::new(0.012, Color::rgba_u8(20, 28, 40, 255))));
    pointer.fill(Fill::Solid(Color::TRANSPARENT));
    pointer.draw_line(Vec2::new(0.10, 0.05), Vec2::new(0.55, -0.20), 0.012);
    // Small filled circle at the target.
    pointer.fill(Fill::Solid(Color::rgba_u8(20, 28, 40, 255)));
    pointer.draw_ellipse(Vec2::new(0.55, -0.20), Vec2::splat(0.02));
    let _ = stage.add_child(root, pointer);

    // ── 5) Arrow + label (line + arrowhead triangle from rects) ─
    // No general-path primitive; emulate the arrowhead with three
    // overlapping `draw_line` calls forming a wedge.
    let mut arrow = Graphics::new();
    arrow.fill(Fill::Solid(Color::TRANSPARENT));
    let arrow_color = Color::rgba_u8(45, 130, 220, 255);
    arrow.stroke(Some(Stroke::new(0.014, arrow_color)));
    let arrow_tip = Vec2::new(-0.20, -0.75);
    let arrow_tail = Vec2::new(0.45, -0.45);
    arrow.draw_line(arrow_tail, arrow_tip, 0.014);
    // Arrowhead — two short lines fanning out from the tip.
    arrow.draw_line(arrow_tip, arrow_tip + Vec2::new(0.10, 0.04), 0.014);
    arrow.draw_line(arrow_tip, arrow_tip + Vec2::new(0.07, 0.10), 0.014);
    let _ = stage.add_child(root, arrow);

    // Arrow's accompanying label — a small caption right of its tail.
    let arrow_label = WispText::new("click").with_style(
        TextPreset::Caption
            .style()
            .with_color(Color::rgba_u8(45, 130, 220, 255)),
    );
    let al_rt = pipeline.render(app, &arrow_label, 384, 192);
    let mut al_sprite = Sprite::from_texture(al_rt.as_texture()).with_anchor(Vec2::new(0.0, 0.5));
    al_sprite.container.transform.position = Vec2::new(0.48, -0.42);
    al_sprite.container.transform.scale = Vec2::new(0.45, -0.18);
    let _ = stage.add_child(root, al_sprite);
}
