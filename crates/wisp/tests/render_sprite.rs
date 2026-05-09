//! M0.9 — integration tests for the sprite batcher.
//!
//! Anti-regression target: 100 sprites that share a texture must batch into
//! exactly one draw call. Hidden parents must skip their sprite descendants.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, Sprite, Stage, Texture};

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

fn make_solid_texture(app: &Application, w: u32, h: u32, rgba: [u8; 4]) -> Texture {
    let mut bytes = Vec::with_capacity((w * h * 4) as usize);
    for _ in 0..(w * h) {
        bytes.extend_from_slice(&rgba);
    }
    Texture::from_rgba(app, w, h, &bytes)
}

#[test]
fn empty_stage_produces_no_draw_calls() {
    let app = boot();
    let (_t, view, format) = make_offscreen(&app);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let stage = Stage::new();
    let stats = renderer.render_stage(&app, &view, Color::BLACK, &stage);

    assert_eq!(stats.draw_calls, 0);
    assert_eq!(stats.sprites_drawn, 0);
}

#[test]
fn one_sprite_one_draw_call() {
    let app = boot();
    let texture = make_solid_texture(&app, 4, 4, [255, 0, 0, 255]);
    let (_t, view, format) = make_offscreen(&app);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let mut stage = Stage::new();
    let root = stage.root();
    let _ = stage
        .add_child(root, Sprite::from_texture(texture))
        .expect("added");

    let stats = renderer.render_stage(&app, &view, Color::BLACK, &stage);

    assert_eq!(stats.draw_calls, 1);
    assert_eq!(stats.sprites_drawn, 1);
}

/// **The headline M0.9 anti-regression test.**
///
/// 100 sprites all sharing the same texture must batch into one draw call.
#[test]
fn one_hundred_sprites_share_texture_one_draw_call() {
    let app = boot();
    let texture = make_solid_texture(&app, 4, 4, [255, 255, 255, 255]);
    let (_t, view, format) = make_offscreen(&app);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let mut stage = Stage::new();
    let root = stage.root();
    for i in 0u16..100 {
        let mut sprite = Sprite::from_texture(texture.clone());
        // Spread across NDC; doesn't matter visually for this test.
        let f = f32::from(i) / 100.0;
        sprite.container.transform.position = Vec2::new(f * 2.0 - 1.0, f * 2.0 - 1.0);
        let _ = stage.add_child(root, sprite).expect("added");
    }

    let stats = renderer.render_stage(&app, &view, Color::BLACK, &stage);

    assert_eq!(
        stats.draw_calls, 1,
        "100 sprites sharing a texture must batch into 1 draw call"
    );
    assert_eq!(stats.sprites_drawn, 100);
}

#[test]
fn two_textures_produce_two_draw_calls() {
    let app = boot();
    let red = make_solid_texture(&app, 4, 4, [255, 0, 0, 255]);
    let blue = make_solid_texture(&app, 4, 4, [0, 0, 255, 255]);
    let (_t, view, format) = make_offscreen(&app);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let mut stage = Stage::new();
    let root = stage.root();
    for _ in 0..50 {
        let _ = stage.add_child(root, Sprite::from_texture(red.clone()));
        let _ = stage.add_child(root, Sprite::from_texture(blue.clone()));
    }

    let stats = renderer.render_stage(&app, &view, Color::BLACK, &stage);

    assert_eq!(stats.draw_calls, 2);
    assert_eq!(stats.sprites_drawn, 100);
}

#[test]
fn hidden_container_skips_sprite_descendants() {
    let app = boot();
    let texture = make_solid_texture(&app, 4, 4, [128, 128, 128, 255]);
    let (_t, view, format) = make_offscreen(&app);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let mut stage = Stage::new();
    let root = stage.root();
    let hidden_parent = stage
        .add_child(root, wisp::Container::default())
        .expect("added");
    if let Some(node) = stage.get_mut(hidden_parent) {
        node.container_mut().visible = false;
    }
    // Add 10 sprites under the hidden parent.
    for _ in 0..10 {
        let _ = stage
            .add_child(hidden_parent, Sprite::from_texture(texture.clone()))
            .expect("added");
    }
    // Add 3 visible sprites under the root.
    for _ in 0..3 {
        let _ = stage
            .add_child(root, Sprite::from_texture(texture.clone()))
            .expect("added");
    }

    let stats = renderer.render_stage(&app, &view, Color::BLACK, &stage);

    assert_eq!(
        stats.sprites_drawn, 3,
        "hidden parent must skip its 10 descendants"
    );
    assert_eq!(stats.draw_calls, 1);
}
