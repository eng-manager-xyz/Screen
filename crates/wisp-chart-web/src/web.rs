//! Browser entrypoint. Compiles only on `wasm32-unknown-unknown`.
//!
//! Flow:
//!
//! 1. wasm-bindgen calls `start()` once on page load.
//! 2. `start()` finds `<canvas id="wisp-chart-canvas">` in the DOM,
//!    spawns the async wgpu bring-up, returns to the event loop.
//! 3. `run()` creates a `wgpu::Instance` with `BROWSER_WEBGPU`,
//!    builds a `wgpu::Surface` from the canvas, requests an adapter
//!    + device, configures the surface, and clears to white.
//! 4. The actual chart drawing replaces step 3's "clear to white"
//!    when the M-CHART.0 render chunk lands.

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

/// Entry point invoked by wasm-bindgen on page load.
///
/// Returns a `JsValue` error if the canvas isn't found or wgpu
/// bring-up fails synchronously. Async failures inside `run()`
/// are logged via `console.error`.
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    log::info!("wisp-chart-web: starting…");

    let window =
        web_sys::window().ok_or_else(|| JsValue::from_str("no `window` in this context"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("no `document` in this context"))?;
    let canvas: HtmlCanvasElement = document
        .get_element_by_id("wisp-chart-canvas")
        .ok_or_else(|| JsValue::from_str("missing <canvas id=\"wisp-chart-canvas\">"))?
        .dyn_into()
        .map_err(|_| JsValue::from_str("#wisp-chart-canvas is not a <canvas>"))?;

    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = run(canvas).await {
            web_sys::console::error_1(&JsValue::from_str(&format!("wisp-chart-web: {e}")));
        }
    });
    Ok(())
}

/// Run the WebGPU bring-up + render loop.
async fn run(canvas: HtmlCanvasElement) -> Result<(), String> {
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    log::info!("wisp-chart-web: canvas is {width}x{height}");

    // Instance: pick the BROWSER_WEBGPU backend explicitly. WebGL
    // fallback is deliberately NOT enabled — this demo targets
    // WebGPU specifically. Browsers without WebGPU error here with
    // an actionable message.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    // Surface from canvas. `SurfaceTarget::Canvas` accepts the
    // owned canvas; wgpu wraps it for the BROWSER_WEBGPU backend.
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| format!("create_surface: {e}"))?;

    // Adapter — high-performance preference is a hint; the browser
    // ultimately picks. wgpu 24's `request_adapter` returns Option,
    // not Result.
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .ok_or_else(|| "request_adapter: no compatible WebGPU adapter".to_owned())?;
    log::info!("wisp-chart-web: adapter = {:?}", adapter.get_info());

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("wisp-chart-web device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| format!("request_device: {e}"))?;

    // Configure surface — sRGB output format keeps the clear-colour
    // path linear-aware.
    let caps = surface.get_capabilities(&adapter);
    let surface_format = caps
        .formats
        .iter()
        .copied()
        .find(wgpu::TextureFormat::is_srgb)
        .unwrap_or(caps.formats[0]);
    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );

    // Render one frame: clear to white. (When `Gantt::render`
    // ships, this becomes "build scene tree → draw scene tree".)
    let frame = surface
        .get_current_texture()
        .map_err(|e| format!("get_current_texture: {e}"))?;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wisp-chart-web encoder"),
    });
    {
        let _rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wisp-chart-web clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    queue.submit(std::iter::once(encoder.finish()));
    frame.present();
    log::info!("wisp-chart-web: cleared canvas to white — WebGPU path is live.");
    Ok(())
}
