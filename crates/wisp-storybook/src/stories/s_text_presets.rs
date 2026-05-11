//! Story: gallery of `TextPreset` styles in one scene (M-TEXT.12 /
//! AUT-86). Each preset renders its label using its own style so the
//! gallery reads as the preset names themselves.

use glam::Vec2;
use wisp::application::Application;
use wisp::text::{TextPreset, TextTexturePipeline, WispText};
use wisp::{Sprite, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "text-presets",
        category: "Text",
        title: "Text style presets",
        milestone: "M-TEXT.12",
        writeup: include_str!("writeups/text_presets.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    let pipeline = TextTexturePipeline::new(app, wgpu::TextureFormat::Rgba8UnormSrgb);
    let root = stage.root();

    // 7 presets stacked vertically. Storybook export is 256×256, so
    // each row gets ≈0.27 NDC of vertical space.
    let presets = TextPreset::all();
    let row_count = u8::try_from(presets.len()).unwrap_or(1);
    let row_height = 2.0 / f32::from(row_count); // NDC ∈ [-1, +1]

    for (i, preset) in presets.iter().enumerate() {
        let style = preset.style();
        let text = WispText::new(preset.label()).with_style(style);
        let rt = pipeline.render(app, &text, 1024, 224);

        // Place this row at center-y for slot i, top→bottom.
        let idx = u8::try_from(i).unwrap_or(0);
        let center_y = 1.0 - row_height * (f32::from(idx) + 0.5);
        let mut sprite = Sprite::from_texture(rt.as_texture()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = Vec2::new(0.0, center_y);
        // Each row is 1024×224 ≈ 4.5:1, so width-scale 1.7 fits and the
        // matching height-scale ≈ 1.7 / 4.5 ≈ 0.38 preserves aspect. We
        // intentionally squash a bit so all rows fit cleanly.
        sprite.container.transform.scale = Vec2::new(1.7, -row_height * 0.85);
        let _ = stage.add_child(root, sprite);
    }
}
