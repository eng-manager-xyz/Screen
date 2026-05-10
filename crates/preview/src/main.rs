//! `preview` — native winit + wgpu window that wisp renders into.
//!
//! Plays an MP4 end-to-end through the
//! [`decode`] → [`playback`] → [`wisp`] stack:
//!
//! ```text
//! cargo run -p preview -- crates/decode/tests/fixtures/sample.mp4
//! ```
//!
//! No CLI args = uses the committed test fixture so the binary is
//! runnable without the user hunting for a video.
//!
//! # Architecture
//!
//! The window owns its own wgpu `Instance` / `Adapter` / `Device` / `Queue`
//! and surface. Those four are handed to [`wisp::Application::from_wgpu`]
//! so wisp's `Renderer` and `Sprite` pipeline drive the surface directly.
//! Each frame:
//!
//! - `Player::tick(dt)` advances and uploads the next decoded frame to
//!   `VideoTexture` if the wallclock has caught up to its PTS.
//! - We build a one-sprite `Stage` from the player's current texture.
//! - `Renderer::render_stage` draws into the surface texture view.
//! - `surface_texture.present()` shows it.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use decode::VideoStream;
use decode::gstreamer_pipe::GstreamerPipeStream;
use glam::Vec2;
use playback::{PlayState, Player};
use preview::aspect_fit_scale;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, Sprite, Stage};

const DEFAULT_FIXTURE: &str = "crates/decode/tests/fixtures/sample.mp4";

fn resolve_input() -> PathBuf {
    std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from(DEFAULT_FIXTURE), PathBuf::from)
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("preview=info,wisp=warn")),
        )
        .init();

    let event_loop = EventLoop::new().expect("create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = PreviewApp::new(resolve_input());
    event_loop.run_app(&mut app).expect("run event loop");
}

struct PreviewApp {
    video_path: PathBuf,
    state: Option<RenderState>,
    last_frame_at: Option<Instant>,
}

impl PreviewApp {
    fn new(video_path: PathBuf) -> Self {
        Self {
            video_path,
            state: None,
            last_frame_at: None,
        }
    }
}

struct RenderState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    app: Application,
    renderer: Renderer,
    player: Player,
}

impl RenderState {
    fn build(event_loop: &ActiveEventLoop, video_path: &Path) -> Self {
        // Open the stream first — its dimensions drive the initial window size.
        let stream = GstreamerPipeStream::open(video_path).expect("open video");
        tracing::info!(
            width = stream.width(),
            height = stream.height(),
            fps = stream.frame_rate(),
            count = ?stream.frame_count_hint(),
            "video opened"
        );
        let win_w = stream.width().max(640);
        let win_h = stream.height().max(360);

        let attrs = WindowAttributes::default()
            .with_title("screen — preview")
            .with_inner_size(LogicalSize::new(f64::from(win_w), f64::from(win_h)));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("create winit window"),
        );

        // Build wgpu against the new window's surface.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("create wgpu surface");
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("request adapter");
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("preview::device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .expect("request device");

        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(caps.formats[0]);

        let physical = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: physical.width.max(1),
            height: physical.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Hand wgpu off to wisp.
        let wisp_app = Application::from_wgpu(
            instance,
            adapter,
            device,
            queue,
            AppConfig {
                width: physical.width,
                height: physical.height,
                ..AppConfig::default()
            },
        );
        let renderer = Renderer::new(&wisp_app, surface_format).expect("renderer");

        let mut player = Player::new(&wisp_app, Box::new(stream));
        player.play();

        Self {
            window,
            surface,
            surface_config,
            app: wisp_app,
            renderer,
            player,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface
            .configure(self.app.device(), &self.surface_config);
    }

    fn render(&mut self, dt: std::time::Duration) {
        let _pulled = self.player.tick(&self.app, dt);
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(err) => {
                tracing::warn!(?err, "surface acquire failed; reconfiguring");
                self.surface
                    .configure(self.app.device(), &self.surface_config);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Build a one-sprite stage from the player's current texture.
        let mut sprite =
            Sprite::from_texture(self.player.texture().clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.scale = aspect_fit_scale(
            self.surface_config.width,
            self.surface_config.height,
            self.player.texture().width(),
            self.player.texture().height(),
        );

        let mut stage = Stage::new();
        let _ = stage.add_child(stage.root(), sprite);

        let _stats = self
            .renderer
            .render_stage(&self.app, &view, Color::BLACK, &stage);

        frame.present();
    }
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }
        self.state = Some(RenderState::build(event_loop, &self.video_path));
        self.last_frame_at = Some(Instant::now());
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = self
                    .last_frame_at
                    .map_or(std::time::Duration::from_millis(16), |prev| {
                        now.duration_since(prev)
                    });
                self.last_frame_at = Some(now);
                state.render(dt);
                if state.player.state() == PlayState::Ended {
                    tracing::info!("playback ended; closing window");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}
