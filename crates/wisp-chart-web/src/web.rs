//! Browser entrypoint. Compiles only on `wasm32-unknown-unknown`.
//!
//! Flow:
//!
//! 1. wasm-bindgen calls `start()` once on page load.
//! 2. `start()` finds `<canvas id="wisp-chart-canvas">` in the DOM,
//!    spawns the async wgpu + wisp bring-up, returns to the event
//!    loop.
//! 3. `run()` builds a `wgpu::Instance` with `BROWSER_WEBGPU`, a
//!    `wgpu::Surface` from the canvas, adapter + device, then
//!    hands those four values into
//!    `wisp::Application::from_wgpu`. From there
//!    [`crate::render_gantt`] draws [`crate::sample_gantt`] into
//!    the surface texture via `wisp::Renderer::render_stage`.
//! 4. `frame.present()` posts the rendered Gantt to the canvas.

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
use wisp_chart::Theme;

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

/// Run the WebGPU bring-up + Gantt render.
async fn run(canvas: HtmlCanvasElement) -> Result<(), String> {
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
    log::info!("wisp-chart-web: caps.formats = {:?}", caps.formats);
    log::info!("wisp-chart-web: caps.alpha_modes = {:?}", caps.alpha_modes);
    log::info!(
        "wisp-chart-web: caps.present_modes = {:?}",
        caps.present_modes
    );

    let surface_format = caps.formats[0];
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

    // Hand the canvas-built wgpu context to `wisp::Application`.
    // wgpu types are `Arc`-backed so the clones are cheap and the
    // surface keeps working from this scope's bindings.
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

    let gantt = crate::sample_gantt();
    let theme = Theme::light();
    crate::render_gantt(
        &mut app,
        &view,
        surface_format,
        Vec2::new(width as f32, height as f32),
        &gantt,
        &theme,
    )
    .map_err(|e| format!("render_gantt: {e}"))?;

    frame.present();
    log::info!(
        "wisp-chart-web: rendered Gantt ({} rows, {} bars) — WebGPU path is live.",
        gantt.rows.len(),
        gantt.bars.len()
    );
    Ok(())
}
