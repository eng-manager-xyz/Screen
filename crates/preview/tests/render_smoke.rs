//! Headless smoke test for the `from_wgpu` embedding-host codepath.
//!
//! The `preview` binary opens a winit window and hands its wgpu surface to
//! [`wisp::application::Application::from_wgpu`]. CI can't run a winit
//! window, but the same codepath can be exercised against an offscreen
//! `RenderTexture`. This test guards against a regression in:
//!
//! - the `from_wgpu` constructor itself (host-supplied wgpu objects),
//! - the `decode → Player → VideoTexture → Sprite → render_stage` pipeline,
//! - the rendered output being non-trivial (clear color was overwritten by
//!   the video sprite).
//!
//! The pure-math companion ([`preview::aspect_fit_scale`]) is unit-tested
//! in the library half.

use std::path::PathBuf;
use std::time::Duration;

use decode::VideoStream;
use decode::gstreamer_pipe::{GstreamerPipeStream, gstreamer_available};
use glam::Vec2;
use playback::Player;
use pollster::block_on;
use preview::aspect_fit_scale;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage};

const SURFACE_W: u32 = 256;
const SURFACE_H: u32 = 144;

#[test]
fn from_wgpu_path_renders_a_real_frame() {
    if !gstreamer_available() {
        eprintln!(
            "gst-launch-1.0/gst-discoverer-1.0 not on PATH — skipping render_smoke. \
             PATH={}",
            std::env::var("PATH").unwrap_or_else(|_| "<unset>".into())
        );
        return;
    }
    let fixture = PathBuf::from("../decode/tests/fixtures/sample.mp4");
    assert!(
        fixture.exists(),
        "fixture not found at {} (cwd is the crate dir during tests)",
        fixture.display()
    );

    let stream = GstreamerPipeStream::open(&fixture).expect("open fixture");
    let video_w = stream.width();
    let video_h = stream.height();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("request adapter");
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("preview::test::device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .expect("request device");

    let app = Application::from_wgpu(
        instance,
        adapter,
        device,
        queue,
        AppConfig {
            width: SURFACE_W,
            height: SURFACE_H,
            ..AppConfig::default()
        },
    );

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let renderer = Renderer::new(&app, format).expect("renderer");
    let rt = RenderTexture::with_format(&app, SURFACE_W, SURFACE_H, format);

    let mut player = Player::new(&app, Box::new(stream));
    player.play();

    // Pump until the player has uploaded at least one decoded frame.
    let dt = Duration::from_secs_f64(1.0 / 30.0);
    let mut pulled = 0;
    for _ in 0..120 {
        pulled += player.tick(&app, dt);
        if pulled > 0 {
            break;
        }
    }
    assert!(pulled > 0, "player never uploaded a frame after 120 ticks");

    let scale = aspect_fit_scale(SURFACE_W, SURFACE_H, video_w, video_h);
    let mut sprite = Sprite::from_texture(player.texture().clone()).with_anchor(Vec2::splat(0.5));
    sprite.container.transform.scale = scale;
    let mut stage = Stage::new();
    let _ = stage.add_child(stage.root(), sprite);
    let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

    let bytes = rt.read_pixels(&app);
    assert_eq!(
        u32::try_from(bytes.len()).expect("readback fits in u32"),
        SURFACE_W * SURFACE_H * 4
    );

    // The clear color is BLACK; if the sprite rendered, at least *some* pixel
    // is not `[0, 0, 0, 255]`. The fixture is a colorful gradient so this is
    // a tight assertion despite being phrased loosely.
    let non_clear = bytes
        .chunks_exact(4)
        .any(|p| p[0] != 0 || p[1] != 0 || p[2] != 0);
    assert!(non_clear, "rendered surface is entirely the clear color");

    // And the image isn't a single solid color either — guards a "render
    // succeeded but uploaded a stale/uniform texture" failure mode.
    let first = bytes.chunks_exact(4).next().unwrap();
    let varied = bytes.chunks_exact(4).any(|p| p != first);
    assert!(varied, "rendered surface is uniform (single color)");
}
