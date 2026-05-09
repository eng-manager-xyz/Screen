//! M0.19 — Mesh perspective rendering integration test.

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, Mesh, RenderTexture, Stage, Texture};

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

#[test]
fn mesh_renders_with_zero_rotation_as_textured_quad() {
    let app = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let rt = RenderTexture::with_format(&app, 32, 32, format);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let mut bytes = Vec::with_capacity(4 * 4 * 4);
    for _ in 0..(4 * 4) {
        bytes.extend_from_slice(&[255, 200, 60, 255]);
    }
    let texture = Texture::from_rgba(&app, 4, 4, &bytes);

    let mut mesh = Mesh::from_texture(texture).with_perspective(0.0);
    mesh.container.transform.scale = Vec2::splat(0.6);
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), mesh);

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.meshes_drawn, 1);
    assert_eq!(stats.draw_calls, 1);

    let pixels = rt.read_pixels(&app);
    let center = (16 * 32 + 16) * 4;
    assert!(
        pixels[center] > 100,
        "mesh center should be the texture color; got R={}",
        pixels[center]
    );
}

#[test]
fn meshes_sharing_texture_batch_into_one_draw_call() {
    let app = boot();
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let rt = RenderTexture::with_format(&app, 64, 64, format);
    let renderer = Renderer::new(&app, format).expect("renderer");

    let mut bytes = Vec::with_capacity(4 * 4 * 4);
    for _ in 0..(4 * 4) {
        bytes.extend_from_slice(&[255, 255, 255, 255]);
    }
    let texture = Texture::from_rgba(&app, 4, 4, &bytes);

    let mut stage = Stage::new();
    let root = stage.root();
    for i in 0u8..5 {
        let mut mesh = Mesh::from_texture(texture.clone());
        mesh.container.transform.scale = Vec2::splat(0.2);
        mesh.container.transform.position = Vec2::new(f32::from(i) * 0.2 - 0.4, 0.0);
        mesh.rotation_y = f32::from(i) * 0.3;
        let _ = stage.add_child(root, mesh);
    }

    let stats = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);
    assert_eq!(stats.meshes_drawn, 5);
    assert_eq!(stats.draw_calls, 1);
}
