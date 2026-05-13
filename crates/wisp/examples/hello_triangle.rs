//! Renders a hardcoded RGB triangle in a winit window. M0.5 verification.
//!
//! Run with: `cargo run -p screen-wisp --example hello_triangle`
//! Press Esc or close the window to exit.

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{KeyEvent, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use wisp::Color;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;

struct State {
    wisp: Application,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    window: Arc<Window>,
}

impl State {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("wisp · hello_triangle"))
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

        Self {
            wisp,
            surface,
            surface_config,
            renderer,
            window,
        }
    }

    fn resize(&mut self, w: u32, h: u32) {
        self.surface_config.width = w.max(1);
        self.surface_config.height = h.max(1);
        self.surface
            .configure(self.wisp.device(), &self.surface_config);
    }

    fn render(&self) {
        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(e) => {
                tracing::warn!(error = ?e, "skip frame");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.renderer.render(&self.wisp, &view, Color::BLACK);
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
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=debug,info")),
        )
        .init();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App::default();
    event_loop.run_app(&mut app).expect("run");
}
