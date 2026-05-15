//! Browser entrypoint. Compiles only on `wasm32-unknown-unknown`.
//!
//! Two paths:
//!
//! - **Static** (`?chart=<id>`) — one render, present once, done.
//!   This is what every chart-only chapter iframe uses.
//! - **Animated** (`?chart=<id>&animate=<id>`) — bring up wgpu + the
//!   chart graphics ONCE, capture the chart's `wisp::NodeId`, then
//!   drive a `wisp_animation::Driver` per `requestAnimationFrame`
//!   tick that mutates the chart node's rotation and re-renders.
//!   This is what every `wisp-animation` chapter iframe uses to
//!   demonstrate a primitive against a real chart.
//!
//! Today only `animate=spin` is recognised; future animations
//! (`?animate=fade`, `?animate=draw-in`, …) plug in here as new
//! enum variants on [`AnimationKind`].

#![allow(
    clippy::cast_precision_loss,
    reason = "canvas width/height come from the browser bounded by viewport size; \
              well below the f32 precision boundary."
)]

use std::cell::RefCell;
use std::f32::consts::TAU;
use std::rc::Rc;
use std::time::Duration;

use glam::Vec2;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::scene::{NodeId, Transform};
use wisp_animation::{Animation, Driver, LinearRamp};

use crate::ChartId;

/// Which animation to drive against the active chart. Parsed from
/// the URL's `?animate=…` query parameter. Unknown / missing →
/// `None` → static (one-shot) render path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationKind {
    /// Loop the chart's container rotation through `0..2π` every
    /// second. Driven by a `wisp_animation::Driver` sampling a
    /// `LinearRamp` from `0.0` to `TAU`; the rAF loop applies the
    /// sample to the chart node's `Container::transform.rotation`
    /// each frame. Showcases the M-ANIM.0 Animation+Driver pair.
    Spin,
}

impl AnimationKind {
    /// Parse from a URL-param string (case-insensitive). Returns
    /// `None` for unknown values so callers can default to the
    /// static path.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        match id.to_ascii_lowercase().as_str() {
            "spin" | "rotate" => Some(Self::Spin),
            _ => None,
        }
    }
}

/// Entry point invoked by wasm-bindgen on page load.
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

    let chart = url_param(&window, "chart")
        .as_deref()
        .and_then(ChartId::parse)
        .unwrap_or_default();
    let animation = url_param(&window, "animate")
        .as_deref()
        .and_then(AnimationKind::parse);
    log::info!("wisp-chart-web: chart = {chart:?}, animate = {animation:?}");

    wasm_bindgen_futures::spawn_local(async move {
        if let Err(e) = run(canvas, chart, animation).await {
            web_sys::console::error_1(&JsValue::from_str(&format!("wisp-chart-web: {e}")));
        }
    });
    Ok(())
}

/// Read a query-string parameter from `window.location.search`.
/// Returns `None` when the parameter is missing.
fn url_param(window: &web_sys::Window, key: &str) -> Option<String> {
    let search = window.location().search().ok()?;
    let trimmed = search.trim_start_matches('?');
    for pair in trimmed.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k.eq_ignore_ascii_case(key) {
            return Some(v.to_owned());
        }
    }
    None
}

/// Run the WebGPU bring-up + render path. Static or animated
/// depending on the parsed `animation` argument.
async fn run(
    canvas: HtmlCanvasElement,
    chart: ChartId,
    animation: Option<AnimationKind>,
) -> Result<(), String> {
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

    let viewport = Vec2::new(width as f32, height as f32);

    match animation {
        None => run_static(&mut app, &surface, surface_format, viewport, chart),
        Some(AnimationKind::Spin) => run_spin(app, surface, surface_format, viewport),
    }
}

/// One-shot render path. Builds the chart, renders one frame,
/// presents, returns. Unchanged from the pre-animation behaviour.
fn run_static(
    app: &mut Application,
    surface: &wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    viewport: Vec2,
    chart: ChartId,
) -> Result<(), String> {
    let frame = surface
        .get_current_texture()
        .map_err(|e| format!("get_current_texture: {e}"))?;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    crate::render_chart_to_view(chart, app, &view, surface_format, viewport)
        .map_err(|e| format!("render_chart_to_view: {e}"))?;

    frame.present();
    log::info!("wisp-chart-web: rendered {chart:?} — WebGPU static path is live.");
    Ok(())
}

/// Animated path: spin the polar chart's container rotation through
/// `0..2π` once per second forever. Drives a `wisp_animation::Driver`
/// in real-time mode from `requestAnimationFrame` callbacks; the
/// stage is mutated in place (no re-add) so per-frame allocation is
/// bounded.
fn run_spin(
    mut app: Application,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    viewport: Vec2,
) -> Result<(), String> {
    // Build the polar plot once and keep its NodeId. Subsequent
    // frames mutate this node's rotation; nothing else is touched.
    let polar = crate::fixtures::polar_plot_fixture();
    let theme = wisp_chart::Theme::light();
    let graphics = polar.emit_graphics(&theme, viewport);
    let root = app.stage().root();
    let chart_id: NodeId = app
        .stage_mut()
        .add_child(root, graphics)
        .ok_or_else(|| "add_child returned None — root id is stale".to_owned())?;

    let renderer =
        Renderer::new(&app, surface_format).map_err(|e| format!("Renderer::new: {e}"))?;

    // Realtime driver — host supplies dt per rAF tick. The animation
    // value itself is timeless: a 1-second linear ramp from 0 to TAU
    // that the driver wraps via modulo on each sample.
    let mut driver = Driver::realtime();
    driver.play();
    let anim = LinearRamp::new(0.0, TAU, Duration::from_secs(1));

    // Stash state in an Rc<RefCell<>> so the rAF closure can be
    // re-invoked across many frames. The closure forgets itself
    // into the global window and is never reclaimed for the page
    // lifetime — acceptable for a demo iframe.
    let state = SpinState {
        app,
        surface,
        renderer,
        chart_id,
        driver,
        anim,
        last_tick_ms: now_ms()?,
    };
    let state = Rc::new(RefCell::new(state));
    request_next_frame(&state)?;
    log::info!("wisp-chart-web: spin animation loop attached.");
    Ok(())
}

/// Holds everything the rAF callback needs to advance one frame.
struct SpinState {
    app: Application,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    chart_id: NodeId,
    driver: Driver,
    anim: LinearRamp,
    last_tick_ms: f64,
}

/// Schedule the next animation frame against the shared state.
fn request_next_frame(state: &Rc<RefCell<SpinState>>) -> Result<(), String> {
    let state = state.clone();
    let cb = Closure::wrap(Box::new(move || {
        if let Err(e) = step_one_frame(&state) {
            web_sys::console::error_1(&JsValue::from_str(&format!("spin step: {e}")));
            return;
        }
        if let Err(e) = request_next_frame(&state) {
            web_sys::console::error_1(&JsValue::from_str(&format!("spin reschedule: {e}")));
        }
    }) as Box<dyn FnMut()>);

    let window = web_sys::window().ok_or_else(|| "no `window` for rAF schedule".to_owned())?;
    window
        .request_animation_frame(cb.as_ref().unchecked_ref())
        .map_err(|e| format!("request_animation_frame: {e:?}"))?;
    // Closure must outlive this scope; rAF only holds a weak JS
    // reference. `forget` leaks one Closure per frame — acceptable
    // for a demo iframe; a long-lived host would recycle via a
    // single persistent Closure + `Rc<RefCell<Option<Closure>>>`
    // hand-off pattern.
    cb.forget();
    Ok(())
}

/// Advance the driver, mutate the chart node's rotation, render.
fn step_one_frame(state: &Rc<RefCell<SpinState>>) -> Result<(), String> {
    let mut s = state.borrow_mut();

    // Caller-supplied dt from the browser's monotonic clock.
    let now = now_ms()?;
    let dt_ms = (now - s.last_tick_ms).max(0.0);
    s.last_tick_ms = now;
    let dt = Duration::from_secs_f64(dt_ms / 1000.0);
    s.driver.tick(dt);

    // Sample the animation. The ramp clamps at TAU, so wrap the
    // result via modulo to keep the loop seamless.
    let raw = s.anim.sample(s.driver.elapsed());
    let rotation = raw % TAU;

    // Reach the chart node in place and mutate its container
    // transform. No re-emit; no allocation in the hot path.
    let chart_id = s.chart_id;
    if let Some(node) = s.app.stage_mut().get_mut(chart_id) {
        node.container_mut().transform = Transform::from_rotation(rotation);
    }

    // Wrap the driver clock once per cycle so `elapsed()` stays
    // bounded forever (otherwise it'd creep toward Duration::MAX).
    let cycle = s.anim.duration();
    if s.driver.elapsed() >= cycle {
        let wrapped = s.driver.elapsed().checked_sub(cycle).unwrap_or_default();
        s.driver.seek(wrapped);
    }

    let frame = s
        .surface
        .get_current_texture()
        .map_err(|e| format!("get_current_texture: {e}"))?;
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    let _stats = s
        .renderer
        .render_stage(&s.app, &view, wisp::Color::WHITE, s.app.stage());
    frame.present();
    Ok(())
}

/// Read the browser's high-resolution monotonic clock (in
/// milliseconds). Used as the realtime driver's dt source.
fn now_ms() -> Result<f64, String> {
    let window = web_sys::window().ok_or_else(|| "no `window` for performance.now()".to_owned())?;
    let perf = window
        .performance()
        .ok_or_else(|| "no `performance` for now()".to_owned())?;
    Ok(perf.now())
}
