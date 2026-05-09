//! Animated rotating sprite with a drop shadow. M0.20 verification.
//!
//! Run with: `cargo run -p wisp --example hello_sprite`. Esc / window close to exit.

use std::sync::Arc;
use std::time::Instant;

use glam::Vec2;
use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, DropShadowFilter, RenderTexture, Sprite, Stage, Texture};

fn checker_texture(app: &Application) -> Texture {
    let size = 64u32;
    let mut bytes = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let cell_x = (x / 8) % 2;
            let cell_y = (y / 8) % 2;
            let dark = (cell_x ^ cell_y) == 1;
            if dark {
                bytes.extend_from_slice(&[140, 200, 255, 255]);
            } else {
                bytes.extend_from_slice(&[40, 60, 90, 255]);
            }
        }
    }
    Texture::from_rgba(app, size, size, &bytes)
}

struct State {
    wisp: Application,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    window: Arc<Window>,
    started: Instant,
    texture: Texture,
}

impl State {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wisp · hello_sprite"))
                .expect("create window"),
        );
        let size = window.inner_size();

        let wisp = pollster::block_on(Application::new(AppConfig {
            width: size.width,
            height: size.height,
            ..Default::default()
        }))
        .expect("init wisp");

        let surface = wisp
            .instance()
            .create_surface(window.clone())
            .expect("create surface");

        let caps = surface.get_capabilities(wisp.adapter());
        let format = caps
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(caps.formats[0]);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: caps.present_modes[0],
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(wisp.device(), &surface_config);

        let renderer = Renderer::new(&wisp, format).expect("renderer");
        let texture = checker_texture(&wisp);

        Self {
            wisp,
            surface,
            surface_config,
            renderer,
            window,
            started: Instant::now(),
            texture,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.surface_config.width = w.max(1);
        self.surface_config.height = h.max(1);
        self.surface
            .configure(self.wisp.device(), &self.surface_config);
    }

    fn render(&self) {
        let Ok(frame) = self.surface.get_current_texture() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // Pre-render the sprite into a RenderTexture, drop-shadow it, then
        // present the result via render_quad.
        let format = self.surface_config.format;
        let rt_in = RenderTexture::with_format(&self.wisp, 512, 512, format);
        let rt_out = RenderTexture::with_format(&self.wisp, 512, 512, format);

        let t = self.started.elapsed().as_secs_f32();
        let mut sprite = Sprite::from_texture(self.texture.clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.scale = Vec2::splat(0.6);
        sprite.container.transform.rotation = t * 0.4;
        let mut stage = Stage::new();
        let _ = stage.add_child(stage.root(), sprite);
        let _ = self
            .renderer
            .render_stage(&self.wisp, rt_in.view(), Color::TRANSPARENT, &stage);

        let shadow = DropShadowFilter {
            offset: Vec2::new(8.0, 10.0),
            blur: 6.0,
            color: Color::rgba(0.0, 0.0, 0.0, 0.55),
        };
        self.renderer
            .apply_filter(&self.wisp, &shadow, &rt_in, &rt_out);

        // Present the post-filter texture as a single quad.
        let bytes = rt_out.read_pixels(&self.wisp);
        let presentable = Texture::from_rgba(&self.wisp, 512, 512, &bytes);
        self.renderer.render_quad(
            &self.wisp,
            &view,
            Color::rgba_u8(20, 20, 30, 255),
            &presentable,
            glam::Mat4::from_scale(glam::Vec3::new(0.8, 0.8, 1.0)),
            Color::WHITE,
        );

        frame.present();
    }
}

#[derive(Default)]
struct App {
    state: Option<State>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.state = Some(State::new(event_loop));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.render();
                state.window.request_redraw();
            }
            _ => {}
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=info")),
        )
        .init();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run");
}
