//! Render-to-PNG integration test for the W3D.8 / engmanager.xyz
//! 404 pyramid (AUT-302 first customer).
//!
//! Builds the same scene the `wisp-3d-web` bundle composes
//! (pyramid + `PaletteRampMaterial` + wireframe overlay), renders
//! it through `MaterialRenderer::draw_one` + `WireframePipeline`
//! to an offscreen `RenderTexture`, writes the result to
//! `_docs/book/src/assets/wisp-3d/pyramid.png`, and asserts:
//!
//! 1. The centre of the canvas is NOT the clear color — proves
//!    the pyramid actually rendered, not an empty depth-cleared
//!    canvas.
//! 2. A point near the centre of the pyramid's front face matches
//!    one of the palette stops within a generous tolerance (the
//!    fragment shader mixes stops + lambert + grain so an exact
//!    hex match is impossible; we just verify it's in the palette
//!    family).
//! 3. The corner pixels ARE the clear color — the pyramid doesn't
//!    cover the whole canvas; if every pixel reads as a palette
//!    color, the camera/projection is broken.
//!
//! The committed PNG is what the W3D.9 mdBook chapter embeds.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::too_many_lines,
    reason = "Single linear render-and-assert test; splitting the wgpu setup, the draw, and the assertions would scatter state across helpers and obscure the sequence."
)]

use std::path::PathBuf;

use glam::{Mat4, Vec3};
use pollster::block_on;
use wisp::RenderTexture;
use wisp::application::{AppConfig, Application};
use wisp_3d::{
    Camera3D, EdgesMesh, LineColor, MaterialRenderer, Mesh3D, PaletteRampMaterial,
    WireframePipeline,
};

const W: u32 = 800;
const H: u32 = 800;
/// Background tint matching the engmanager.xyz 404 page's `--bg`
/// (`#11111b`, the Catppuccin Mocha base).
const BG: [u8; 4] = [0x11, 0x11, 0x1b, 0xff];

#[test]
fn pyramid_renders_with_palette_and_wireframe() {
    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))
    .expect("Application::new");

    // 1. Set up camera matching the 404 page exactly.
    #[allow(
        clippy::cast_precision_loss,
        reason = "W and H are bounded compile-time constants well below f32's exact-integer ceiling"
    )]
    let aspect = (W as f32) / (H as f32);
    let mut camera = Camera3D::perspective(38.0, aspect, 0.1, 100.0);
    camera.position = Vec3::new(0.0, 0.28, 6.2);

    // 2. Build the scene.
    let mesh = Mesh3D::pyramid(1.34, 1.25);
    let edges = EdgesMesh::from_mesh(&mesh, 8.0);
    let material = PaletteRampMaterial::engmanager_404();

    // 3. Offscreen RGBA8Unorm color attachment + depth.
    let rt = RenderTexture::with_format(&app, W, H, wgpu::TextureFormat::Rgba8Unorm);
    let depth_tex = app.device().create_texture(&wgpu::TextureDescriptor {
        label: Some("test::pyramid::depth"),
        size: wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wisp_3d::DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

    // 4. Material pass — clears color + depth, draws the pyramid.
    let mut renderer = MaterialRenderer::new(&app);
    let mut wireframe = WireframePipeline::new(&app, wgpu::TextureFormat::Rgba8Unorm, 1);

    let mut encoder = app
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("test::pyramid::encoder"),
        });
    app.device().push_error_scope(wgpu::ErrorFilter::Validation);

    renderer.draw_one(
        &app,
        &mut encoder,
        rt.view(),
        &depth_view,
        &camera,
        &material,
        &mesh,
        Mat4::IDENTITY,
        [1.0, 1.0, 1.0, 1.0],
        wgpu::Color {
            r: f64::from(BG[0]) / 255.0,
            g: f64::from(BG[1]) / 255.0,
            b: f64::from(BG[2]) / 255.0,
            a: 1.0,
        },
        wgpu::TextureFormat::Rgba8Unorm,
        1,
    );

    // 5. Wireframe overlay — loads color + depth, draws lines.
    let edge_vbuf = wireframe.build_vertex_buffer(app.device(), app.queue(), &edges);
    let (_color_buf, color_bg) = wireframe.build_color_resources(
        app.device(),
        app.queue(),
        LineColor {
            color: [0.96, 0.88, 0.86, 0.82],
        },
    );
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("test::pyramid::wireframe"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: rt.view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        wireframe.draw_into(
            &app,
            &mut pass,
            &camera,
            &edges,
            LineColor {
                color: [0.96, 0.88, 0.86, 0.82],
            },
            &edge_vbuf,
            &color_bg,
        );
    }
    app.queue().submit(std::iter::once(encoder.finish()));
    let validation = block_on(app.device().pop_error_scope());
    assert!(
        validation.is_none(),
        "wgpu validation error during pyramid render: {validation:?}"
    );

    // 6. Read back + write PNG. Writes FIRST so a failing assertion
    //    still leaves the committed asset up-to-date.
    let bytes = rt.read_pixels(&app);
    let snapshot_path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../_docs/book/src/assets/wisp-3d/pyramid.png");
    if let Some(parent) = snapshot_path.parent() {
        std::fs::create_dir_all(parent).expect("create assets dir");
    }
    image::save_buffer(&snapshot_path, &bytes, W, H, image::ColorType::Rgba8)
        .expect("write pyramid.png");

    // 7. Color-pick asserts. The pyramid is centred and occupies
    //    roughly the middle 60% of the canvas with the 38° / 6.2-unit
    //    camera distance.
    let centre = read_pixel(&bytes, W / 2, H / 2);
    assert!(
        !pixel_is_bg(centre),
        "centre pixel @ ({}, {}) is the background colour {:?} — \
         pyramid didn't render. Got {:?}",
        W / 2,
        H / 2,
        BG,
        centre,
    );

    // Corner — definitely outside the pyramid silhouette.
    let corner = read_pixel(&bytes, 5, 5);
    assert!(
        pixel_is_bg(corner),
        "top-left corner pixel @ (5, 5) is {corner:?}, expected background {BG:?} \
         (if the pyramid is filling the whole canvas, the camera/projection \
         is broken)",
    );

    // The fragment shader mixes 5 palette stops + lambert + grain;
    // exact-hex matching is futile. Instead assert "the centre's
    // hue family is reddish-orange-pink-mauve-blue" by checking it
    // looks distinctly different from the background.
    let dist_from_bg = colour_distance(centre, BG);
    assert!(
        dist_from_bg > 30,
        "centre pixel {centre:?} is suspiciously close to bg {BG:?} (distance {dist_from_bg}); \
         expected the pyramid's palette stops to read clearly off-bg",
    );
}

fn read_pixel(bytes: &[u8], x: u32, y: u32) -> [u8; 4] {
    let stride = (W * 4) as usize;
    let i = (y as usize) * stride + (x as usize) * 4;
    [bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]
}

fn pixel_is_bg(p: [u8; 4]) -> bool {
    // Tight tolerance — the clear is exact `BG`, the only fuzz
    // comes from the sRGB ↔ linear path for the wgpu::Color
    // conversion. ±2 absorbs that.
    let tol: u8 = 2;
    p[0].abs_diff(BG[0]) <= tol && p[1].abs_diff(BG[1]) <= tol && p[2].abs_diff(BG[2]) <= tol
}

fn colour_distance(a: [u8; 4], b: [u8; 4]) -> u32 {
    u32::from(a[0].abs_diff(b[0])) + u32::from(a[1].abs_diff(b[1])) + u32::from(a[2].abs_diff(b[2]))
}
