//! `cargo run -p preview --example render_offscreen`
//!
//! Headless equivalent of the `preview` binary: same wisp embedding-host
//! codepath ([`wisp::application::Application::from_wgpu`]), driven against
//! an offscreen [`wisp::RenderTexture`] instead of a winit-backed surface.
//!
//! Two purposes:
//!
//! 1. **Asset generator** for `_docs/book/src/preview/chunks/preview-window.md`.
//!    Writes letterboxed PNG snapshots to `_docs/book/src/assets/preview/`.
//! 2. **Smoke test** for the `from_wgpu` codepath. Booting the same render
//!    path in CI without winit/X11/Wayland lets us catch wisp-level
//!    regressions independently of the OS window plumbing.
//!
//! The host-supplied wgpu setup (`Instance` → `Adapter` → `Device` → `Queue`)
//! mirrors `main.rs`, with `compatible_surface = None` because there is no
//! surface to satisfy.

use std::path::{Path, PathBuf};
use std::time::Duration;

use decode::VideoStream;
use decode::gstreamer_pipe::GstreamerPipeStream;
use glam::Vec2;
use playback::{PlayState, Player};
use pollster::block_on;
use preview::aspect_fit_scale;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Sprite, Stage};

const SURFACE_W: u32 = 800;
const SURFACE_H: u32 = 450;
const FRAME_BUDGET: u32 = 5;

fn resolve_input() -> PathBuf {
    std::env::args().nth(1).map_or_else(
        || PathBuf::from("crates/decode/tests/fixtures/sample.mp4"),
        PathBuf::from,
    )
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("preview=info,wisp=warn")),
        )
        .init();

    let input = resolve_input();
    let stream = GstreamerPipeStream::open(&input).expect("open video");
    let video_w = stream.width();
    let video_h = stream.height();
    println!(
        "opened {} ({video_w}×{video_h} @ {:.2} fps)",
        input.display(),
        stream.frame_rate()
    );

    // Build wgpu the way `main.rs` does — except `compatible_surface = None`
    // because we render into a `RenderTexture`, not a winit surface.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: None,
        force_fallback_adapter: false,
    }))
    .expect("request adapter");
    let (device, queue) = block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("preview::render_offscreen::device"),
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

    let out_dir = Path::new("_docs/book/src/assets/preview");
    std::fs::create_dir_all(out_dir).ok();

    let dt = Duration::from_secs_f64(1.0 / 30.0);
    let mut written = 0u32;
    let mut tick = 0u32;
    while written < FRAME_BUDGET {
        let pulled = player.tick(&app, dt);
        if pulled > 0 {
            let scale = aspect_fit_scale(SURFACE_W, SURFACE_H, video_w, video_h);
            let mut sprite =
                Sprite::from_texture(player.texture().clone()).with_anchor(Vec2::splat(0.5));
            sprite.container.transform.scale = scale;

            let mut stage = Stage::new();
            let _ = stage.add_child(stage.root(), sprite);
            let _ = renderer.render_stage(&app, rt.view(), Color::BLACK, &stage);

            let bytes = rt.read_pixels(&app);
            let img = image::RgbaImage::from_raw(rt.width(), rt.height(), bytes).expect("dims");
            let path = out_dir.join(format!("preview_{written:02}.png"));
            img.save(&path).expect("save png");
            println!("  ✓ wrote {}", path.display());
            written += 1;
        }
        if player.state() == PlayState::Ended {
            break;
        }
        tick += 1;
        if tick > 600 {
            eprintln!("safety break — tick > 600");
            break;
        }
    }

    println!("done. wrote {written} frames.");
}
