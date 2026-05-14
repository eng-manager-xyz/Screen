//! Browser entrypoint. Compiles only on `wasm32-unknown-unknown`.
//!
//! Flow:
//!
//! 1. wasm-bindgen calls `start()` once on page load.
//! 2. `start()` parses the URL's `?chart=<id>` query parameter into
//!    a [`crate::ChartId`] (default `Gantt`), finds
//!    `<canvas id="wisp-chart-canvas">` in the DOM, and spawns the
//!    async wgpu bring-up.
//! 3. `run()` builds the wgpu surface from the canvas, hands the
//!    context to a `wisp::Application`, then calls
//!    [`crate::render_chart_to_view`] which dispatches to the
//!    right per-chart fixture + `emit_graphics` path.
//! 4. `frame.present()` posts the rendered chart to the canvas.
//!
//! Pages embedded in iframes typically pass `?chart=line` etc.

#![allow(
    clippy::cast_precision_loss,
    reason = "canvas width/height come from the browser bounded by viewport size; \
              well below the f32 precision boundary."
)]

use glam::Vec2;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wisp::application::{AppConfig, Application};

use crate::ChartId;

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

    // Parse `?chart=<id>` from the current URL. Default to Gantt
    // so the bare `index.html` keeps working.
    let chart = chart_id_from_url(&window).unwrap_or_default();
    log::info!("wisp-chart-web: chart = {chart:?}");

    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = run(canvas, chart).await {
            web_sys::console::error_1(&JsValue::from_str(&format!("wisp-chart-web: {e}")));
        }
    });
    Ok(())
}

/// Read `?chart=<id>` from `window.location.search`. Returns
/// `None` when the parameter is missing or its value is not a
/// known chart id.
fn chart_id_from_url(window: &web_sys::Window) -> Option<ChartId> {
    let search = window.location().search().ok()?;
    let trimmed = search.trim_start_matches('?');
    for pair in trimmed.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key.eq_ignore_ascii_case("chart") {
            return ChartId::parse(value);
        }
    }
    None
}

/// Run the WebGPU bring-up + chart render.
async fn run(canvas: HtmlCanvasElement, chart: ChartId) -> Result<(), String> {
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);
    log::info!("wisp-chart-web: canvas is {width}x{height}");

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .map_err(|e| format!("create_surface: {e}"))?;

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
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .map_err(|e| format!("request_device: {e}"))?;

    let caps = surface.get_capabilities(&adapter);
    let surface_format = caps.formats[0];
    let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        caps.alpha_modes[0]
    };

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

    let mut app = Application::from_wgpu(
        instance.clone(),
        adapter.clone(),
        device.clone(),
        queue.clone(),
        AppConfig {
            width,
            height,
            ..Default::default()
        },
    );

    let frame = surface
        .get_current_texture()
        .map_err(|e| format!("get_current_texture: {e}"))?;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    crate::render_chart_to_view(
        chart,
        &mut app,
        &view,
        surface_format,
        Vec2::new(width as f32, height as f32),
    )
    .map_err(|e| format!("render_chart_to_view: {e}"))?;

    frame.present();
    log::info!("wisp-chart-web: rendered {chart:?} — WebGPU path is live.");
    Ok(())
}
