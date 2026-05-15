//! Browser entrypoint. Compiles only on `wasm32-unknown-unknown`.
//!
//! Two paths:
//!
//! - **Static** (`?chart=<id>`) — one render, present once, done.
//!   This is what every chart-only chapter iframe uses.
//! - **Animated** (`?chart=<id>&animate=<id>`) — bring up wgpu + the
//!   chart graphics ONCE, capture the chart's `wisp::NodeId`, then
//!   drive a `wisp_animation::Driver` per `requestAnimationFrame`
//!   tick that mutates the chart node's container in place and
//!   re-renders. Every M-ANIM ticket plugs in a new
//!   [`AnimationKind`] variant + a per-frame mutator closure.
//!
//! The dispatch in [`setup_animation`] is the single growth point —
//! everything else (the rAF loop, the render call, the surface
//! lifecycle) is shared by every animation kind.

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
use wisp::scene::{Container, NodeId, Transform};
use wisp_animation::{
    Animatable, Animation, AnimationRepeatExt, Driver, Ease, LinearRamp, RepeatCount,
    RepeatStrategy, Sequence, Tween,
};

use crate::ChartId;

/// Per-frame mutator closure shared by every animated path. Takes
/// the driver (caller can re-read elapsed, sample animations, etc.)
/// and a mutable borrow of the chart node's container (to apply
/// transform / alpha / blend changes).
type FrameMutator = Box<dyn FnMut(&Driver, &mut Container)>;

/// Which animation to drive against the active chart. Parsed from
/// the URL's `?animate=…` query parameter. Unknown / missing →
/// `None` → static (one-shot) render path.
///
/// Every M-ANIM ticket adds a variant here + a setup arm in
/// [`setup_animation`]. The rAF infrastructure is shared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationKind {
    /// Rotation loop through `0..2π` once per second (M-ANIM.0 /
    /// AUT-227).
    Spin,
    /// Alpha fade 0.0 → 1.0 → 0.0 yoyo over 2s, driven by
    /// `Animatable for f32` (M-ANIM.1 / AUT-228).
    Fade,
    /// One-shot 0 → 1 → 0 scale pulse via a `Tween<f32>` with
    /// `Ease::OutBack` so the chart overshoots before settling
    /// (M-ANIM.2 / AUT-229).
    TweenScale,
    /// Three-step storyline via `Sequence`: fade-in → rotate
    /// quarter-turn → fade-out (M-ANIM.3 / AUT-230).
    Storyline,
    /// Infinite mirrored-repeat (yoyo) of a scale Tween, driving
    /// the chart between scale 0.6 and 1.0 forever (M-ANIM.4 /
    /// AUT-231).
    Yoyo,
}

impl AnimationKind {
    /// Parse from a URL-param string (case-insensitive). Returns
    /// `None` for unknown values so callers can default to the
    /// static path.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        match id.to_ascii_lowercase().as_str() {
            "spin" | "rotate" => Some(Self::Spin),
            "fade" | "alpha" => Some(Self::Fade),
            "tween" | "scale" | "scale-in" => Some(Self::TweenScale),
            "storyline" | "sequence" => Some(Self::Storyline),
            "yoyo" | "mirror" | "repeat" => Some(Self::Yoyo),
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
        Some(kind) => run_animated(app, surface, surface_format, viewport, kind),
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

/// Generic animated path: build a chart + a per-frame mutator
/// closure via [`setup_animation`], then drive both forever via
/// `requestAnimationFrame`.
fn run_animated(
    mut app: Application,
    surface: wgpu::Surface<'static>,
    surface_format: wgpu::TextureFormat,
    viewport: Vec2,
    kind: AnimationKind,
) -> Result<(), String> {
    let setup = setup_animation(&mut app, viewport, kind)?;

    let renderer =
        Renderer::new(&app, surface_format).map_err(|e| format!("Renderer::new: {e}"))?;

    let state = AnimState {
        app,
        surface,
        renderer,
        chart_id: setup.chart_id,
        driver: setup.driver,
        mutator: setup.mutator,
        last_tick_ms: now_ms()?,
    };
    let state = Rc::new(RefCell::new(state));
    request_next_frame(&state)?;
    log::info!("wisp-chart-web: {kind:?} animation loop attached.");
    Ok(())
}

/// Bundle returned by [`setup_animation`].
struct AnimSetup {
    chart_id: NodeId,
    driver: Driver,
    mutator: FrameMutator,
}

/// Per-AnimationKind setup. Builds the chart, picks the driver
/// mode + animation value(s), returns the per-frame mutator closure
/// that the rAF dispatch will invoke.
///
/// This is the single growth point for the M-ANIM roadmap — every
/// new ticket adds one arm here.
fn setup_animation(
    app: &mut Application,
    viewport: Vec2,
    kind: AnimationKind,
) -> Result<AnimSetup, String> {
    let theme = wisp_chart::Theme::light();
    let root = app.stage().root();

    match kind {
        AnimationKind::Spin => {
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let anim = LinearRamp::new(0.0, TAU, Duration::from_secs(1));
            let mutator: FrameMutator = Box::new(move |d: &Driver, c: &mut Container| {
                let rotation = anim.sample(d.elapsed()) % TAU;
                c.transform = Transform::from_rotation(rotation);
            });
            Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            })
        }
        AnimationKind::Yoyo => {
            // Tween 0.6 → 1.0, wrapped with infinite mirrored-
            // repeat: scale bounces back-and-forth forever.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let pulse = Tween::new(0.6_f32, 1.0, Duration::from_millis(600))
                .ease(Ease::InOutCubic)
                .repeat_with(RepeatCount::Infinite, RepeatStrategy::MirroredRepeat);
            let mutator: FrameMutator = Box::new(move |d: &Driver, c: &mut Container| {
                let scale = pulse.sample(d.elapsed());
                c.transform = Transform::from_scale(glam::Vec2::splat(scale));
            });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Storyline => {
            // 3-step alpha sequence: fade-in → hold → fade-out.
            // Rotation runs in parallel via a separate LinearRamp
            // (single value, no sequence needed).
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let alpha_seq: Sequence<f32> = Sequence::new()
                .then(Tween::new(0.0_f32, 1.0, Duration::from_millis(700)).ease(Ease::OutCubic))
                .then(Tween::new(1.0_f32, 1.0, Duration::from_millis(600)))
                .then(Tween::new(1.0_f32, 0.0, Duration::from_millis(700)).ease(Ease::InCubic));
            let rotation = LinearRamp::new(
                0.0,
                std::f32::consts::FRAC_PI_2,
                Duration::from_millis(2_000),
            );
            let cycle_ms = 2_000.0_f32;
            let mutator: FrameMutator = Box::new(move |d: &Driver, c: &mut Container| {
                let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                let local = Duration::from_secs_f32(pos_ms / 1000.0);
                c.alpha = alpha_seq.sample(local);
                c.transform = Transform::from_rotation(rotation.sample(local));
            });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::TweenScale => {
            // Polar plot scales from 0 → 1 with an overshooting
            // OutBack ease, then yoyos back. Demonstrates the full
            // `Tween<f32>` + `Ease::OutBack` shape.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            // Cycle = grow (700 ms) + hold (200 ms) + shrink
            // (700 ms) + hold (400 ms) for a 2 s loop.
            let cycle = Duration::from_millis(2_000);
            let grow = Tween::new(0.0_f32, 1.0, Duration::from_millis(700)).ease(Ease::OutBack);
            let shrink = Tween::new(1.0_f32, 0.0, Duration::from_millis(700)).ease(Ease::InCubic);
            let mutator: FrameMutator = Box::new(move |d: &Driver, c: &mut Container| {
                let cycle_ms = cycle.as_secs_f32() * 1000.0;
                let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                let scale = if pos_ms < 700.0 {
                    grow.sample(Duration::from_secs_f32(pos_ms / 1000.0))
                } else if pos_ms < 900.0 {
                    1.0
                } else if pos_ms < 1_600.0 {
                    shrink.sample(Duration::from_secs_f32((pos_ms - 900.0) / 1000.0))
                } else {
                    0.0
                };
                c.transform = Transform::from_scale(glam::Vec2::splat(scale));
            });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Fade => {
            // Use a contour plot for variety. Fade its container's
            // alpha 0 → 1 → 0 over a 2-second yoyo cycle. The lerp
            // itself goes through `Animatable for f32`.
            let chart = crate::fixtures::contour_fixture();
            let graphics = chart.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let cycle = Duration::from_millis(2_000);
            let mutator: FrameMutator = Box::new(move |d: &Driver, c: &mut Container| {
                // Map [0, 2s) to [0, 1, 0] (yoyo). Cycle position
                // wraps; first half ramps up, second half ramps
                // down. Pure use of `Animatable::lerp(f32)`.
                let cycle_pos = (d.elapsed().as_secs_f32() % cycle.as_secs_f32())
                    / cycle.as_secs_f32();
                let alpha = if cycle_pos < 0.5 {
                    f32::lerp(&0.0, &1.0, cycle_pos * 2.0)
                } else {
                    f32::lerp(&1.0, &0.0, (cycle_pos - 0.5) * 2.0)
                };
                c.alpha = alpha;
            });
            Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            })
        }
    }
}

/// State the rAF closure pumps each frame.
struct AnimState {
    app: Application,
    surface: wgpu::Surface<'static>,
    renderer: Renderer,
    chart_id: NodeId,
    driver: Driver,
    mutator: FrameMutator,
    last_tick_ms: f64,
}

/// Schedule the next animation frame against the shared state.
fn request_next_frame(state: &Rc<RefCell<AnimState>>) -> Result<(), String> {
    let state = state.clone();
    let cb = Closure::wrap(Box::new(move || {
        if let Err(e) = step_one_frame(&state) {
            web_sys::console::error_1(&JsValue::from_str(&format!("anim step: {e}")));
            return;
        }
        if let Err(e) = request_next_frame(&state) {
            web_sys::console::error_1(&JsValue::from_str(&format!("anim reschedule: {e}")));
        }
    }) as Box<dyn FnMut()>);

    let window = web_sys::window().ok_or_else(|| "no `window` for rAF schedule".to_owned())?;
    window
        .request_animation_frame(cb.as_ref().unchecked_ref())
        .map_err(|e| format!("request_animation_frame: {e:?}"))?;
    // Closure must outlive this scope; rAF only holds a weak JS
    // reference. `forget` leaks one Closure per frame — acceptable
    // for a demo iframe; a long-lived host would recycle.
    cb.forget();
    Ok(())
}

/// Advance the driver, run the per-frame mutator against the chart
/// node's container, render, present.
fn step_one_frame(state: &Rc<RefCell<AnimState>>) -> Result<(), String> {
    let mut s = state.borrow_mut();
    let now = now_ms()?;
    let dt_ms = (now - s.last_tick_ms).max(0.0);
    s.last_tick_ms = now;
    let dt = Duration::from_secs_f64(dt_ms / 1000.0);
    s.driver.tick(dt);

    let chart_id = s.chart_id;
    // Split the mutable borrow: take the closure and the driver
    // out of `s` so we can pass `&mut Container` to the closure
    // without overlapping borrows of `s`.
    let mut mutator = std::mem::replace(&mut s.mutator, Box::new(|_, _| {}));
    let driver_snapshot = s.driver.clone();
    if let Some(node) = s.app.stage_mut().get_mut(chart_id) {
        mutator(&driver_snapshot, node.container_mut());
    }
    s.mutator = mutator;

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

/// Read the browser's high-resolution monotonic clock (ms).
fn now_ms() -> Result<f64, String> {
    let window = web_sys::window().ok_or_else(|| "no `window` for performance.now()".to_owned())?;
    let perf = window
        .performance()
        .ok_or_else(|| "no `performance` for now()".to_owned())?;
    Ok(perf.now())
}
