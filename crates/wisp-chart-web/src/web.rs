//! Browser entrypoint. Compiles only on `wasm32-unknown-unknown`.
//!
//! Flow:
//!
//! 1. wasm-bindgen calls `start()` once on page load.
//! 2. `start()` finds `<canvas id="wisp-chart-canvas">` in the DOM,
//!    spawns the async wgpu bring-up, returns to the event loop.
//! 3. `run()` creates a `wgpu::Instance` with `BROWSER_WEBGPU`,
//!    builds a `wgpu::Surface` from the canvas, requests an
//!    adapter + device, configures the surface, and clears to the
//!    demo purple via
//!    `crate::clear_with_color(_, _, _, DEMO_CLEAR_COLOR)`.
//! 4. The actual chart drawing replaces step 3's clear pass when
//!    the M-CHART.0 render chunk lands.

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

    // BROWSER_WEBGPU honours WebGPU's native limits — the WebGL2
    // downlevel set is wrong here and was the legacy of copying
    // from a generic wgpu example. Use `downlevel_defaults()` so
    // the device matches what Chrome / Firefox actually expose.
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("wisp-chart-web device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| format!("request_device: {e}"))?;

    // Surface capabilities for diagnostic logging.
    let caps = surface.get_capabilities(&adapter);
    log::info!("wisp-chart-web: caps.formats = {:?}", caps.formats);
    log::info!("wisp-chart-web: caps.alpha_modes = {:?}", caps.alpha_modes);
    log::info!(
        "wisp-chart-web: caps.present_modes = {:?}",
        caps.present_modes
    );

    // BROWSER_WEBGPU surfaces don't expose an sRGB-tagged format
    // (Chrome returns `Bgra8Unorm` / `Rgba8Unorm`). Pick the first
    // entry — that's the canvas-preferred format.
    let surface_format = caps.formats[0];

    // Pick alpha mode explicitly. Chrome typically exposes
    // [Opaque, PreMultiplied]; we want Opaque so the canvas's CSS
    // background never bleeds through. Fall back to whatever's
    // first if Opaque isn't offered.
    let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        caps.alpha_modes[0]
    };
    log::info!("wisp-chart-web: chose format={surface_format:?}, alpha_mode={alpha_mode:?}");

    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );

    // Render one frame: clear to white via the shared helper. (When
    // `Gantt::render` ships, this becomes "build scene tree → draw
    // scene tree" but the helper's surface-presentation contract
    // stays the same.)
    let frame = surface
        .get_current_texture()
        .map_err(|e| format!("get_current_texture: {e}"))?;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    crate::clear_with_color(&device, &queue, &view, crate::DEMO_CLEAR_COLOR);
    frame.present();
    log::info!(
        "wisp-chart-web: cleared canvas to demo purple {:?} — WebGPU path is live.",
        crate::DEMO_CLEAR_COLOR
    );
    Ok(())
}
