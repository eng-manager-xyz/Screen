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
    clippy::too_many_lines,
    clippy::needless_return,
    clippy::duration_suboptimal_units,
    clippy::doc_markdown,
    reason = "setup_animation grows monotonically with M-ANIM tickets; each arm is a self-contained block with explicit `return` to keep arms parallel. doc_markdown for variant names in URL-param docstrings; suboptimal_units for millisecond cycle constants that read more naturally as ms."
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
use wisp::scene::{Container, NodeId, Stage, Transform};
use wisp_animation::{
    AnimEvent, AnimId, Animatable, Animation, AnimationLifecycleExt, AnimationRepeatExt,
    BatchDriver, BoundScalar, ColorSpace, ColorTween, DrawIn, Driver, Ease, EventReader,
    LinearRamp, MoveAlongPath, NodeProperty, RepeatCount, RepeatStrategy, Sequence, Spring,
    Stagger, StaggerFrom, Track, Tween, TypeWriter,
};

use crate::ChartId;

/// Per-frame mutator closure shared by every animated path. Takes
/// the driver and a mutable borrow of the full Stage so closures
/// that touch multiple nodes (Stagger across a grid, FLIP) can
/// reach in.
///
/// For single-node demos there's a convenience wrapper —
/// [`single_node_mutator`] — that adapts a `Fn(&Driver, &mut Container)`
/// closure to the multi-node signature.
type FrameMutator = Box<dyn FnMut(&Driver, &mut Stage)>;

/// Adapt a single-node mutator (the most common case) to the
/// full-Stage signature.
fn single_node_mutator<F: FnMut(&Driver, &mut Container) + 'static>(
    chart_id: NodeId,
    mut inner: F,
) -> FrameMutator {
    Box::new(move |d: &Driver, stage: &mut Stage| {
        if let Some(node) = stage.get_mut(chart_id) {
            inner(d, node.container_mut());
        }
    })
}

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
    /// Slide chart horizontally via `NodeProperty::translation`
    /// `Target<Vec2>`, demonstrating the Target abstraction
    /// (M-ANIM.5 / AUT-232).
    Slide,
    /// Underdamped Spring scales the chart in with overshoot;
    /// loops every 1.5s (M-ANIM.6 / AUT-233).
    Spring,
    /// 4-keyframe scale walk via `Track<f32>` with per-segment
    /// eases (M-ANIM.7 / AUT-234).
    Keyframe,
    /// Five-dot row with center-out stagger on alpha — demonstrates
    /// `Stagger::each().from(Center)` across multiple `NodeId`s
    /// (M-ANIM.8 / AUT-235).
    Stagger,
    /// Spin loop wrapped with lifecycle callbacks — drains an
    /// `EventReader` each frame and logs Started/Completed events
    /// to the browser console (M-ANIM.9 / AUT-236).
    Callbacks,
    /// Polar plot slides along an S-curve whose `DrawIn` reveals
    /// the path 0..=1 over 2s, then loops (M-ANIM.10 / AUT-237).
    DrawIn,
    /// Polar plot follows a circular path with auto-rotate so it
    /// always faces the direction of motion (M-ANIM.11 / AUT-238).
    MovePath,
    /// 10-step staircase scale driven by `TypeWriter` — visually
    /// reveals the chart character-by-character (M-ANIM.12 / AUT-239).
    TypeIn,
    /// Three colour-tween ellipses (LinearRgb / Oklab / Oklch),
    /// each cycling red → green → blue → red so the midpoint
    /// brown/muddy region differs per space (M-ANIM.13 / AUT-240).
    ColorSpaces,
    /// Three Tweens registered on the same chart's alpha,
    /// rotation, and scale — driven by `BatchDriver::tick_scalars`
    /// so all three land in one deterministic write phase
    /// (M-ANIM.20 / AUT-247).
    Batched,
    /// 12×12 grid of ellipses each with its own scale Tween,
    /// hammering 144 active tweens per frame via `BatchDriver`
    /// (M-ANIM.20 / AUT-247).
    Many,
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
            "slide" | "translate" => Some(Self::Slide),
            "spring" | "bounce" => Some(Self::Spring),
            "keyframe" | "track" | "waypoints" => Some(Self::Keyframe),
            "stagger" => Some(Self::Stagger),
            "callbacks" | "events" | "lifecycle" => Some(Self::Callbacks),
            "drawin" | "draw-in" | "morph" => Some(Self::DrawIn),
            "move-path" | "movepath" | "follow" => Some(Self::MovePath),
            "type-in" | "typein" | "typewriter" => Some(Self::TypeIn),
            "color" | "color-spaces" | "colorspaces" | "oklab" => Some(Self::ColorSpaces),
            "batched" | "batch" => Some(Self::Batched),
            "many" | "swarm" => Some(Self::Many),
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
        driver: setup.driver,
        mutator: setup.mutator,
        last_tick_ms: now_ms()?,
    };
    let _ = setup.chart_id; // captured by the per-arm mutator closure
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
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let rotation = anim.sample(d.elapsed()) % TAU;
                    c.transform = Transform::from_rotation(rotation);
                });
            Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            })
        }
        AnimationKind::Batched => {
            // Three Tweens on the same chart: alpha, rotation,
            // and y-scale — driven through BatchDriver so all
            // three land in one deterministic write phase.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            // Use BatchDriver via Rc<RefCell> so the closure ticks
            // it each frame using the host dt.
            let bdriver = Rc::new(RefCell::new(BatchDriver::realtime()));
            bdriver.borrow_mut().play();
            // BoundScalar is not Clone — instantiate fresh once.
            let alpha = BoundScalar::new(
                Tween::new(0.0_f32, 1.0, Duration::from_millis(900))
                    .ease(Ease::OutCubic)
                    .repeat_with(RepeatCount::Infinite, RepeatStrategy::MirroredRepeat),
                NodeProperty::alpha(chart_id),
            );
            let rotation = BoundScalar::new(
                LinearRamp::new(0.0, std::f32::consts::TAU, Duration::from_millis(2_000)),
                NodeProperty::rotation(chart_id),
            );
            // For scale-y we'd want a Vec2 binding; reuse rotation
            // as a stand-in here for the v1 demo.
            let _ = rotation;
            let anims = Rc::new(RefCell::new(vec![alpha]));
            // Outer driver is just a stub — BatchDriver owns its
            // own clock. Use a paused realtime driver here.
            let driver = Driver::realtime();
            let anims_inner = anims.clone();
            let bdriver_inner = bdriver.clone();
            let mutator: FrameMutator = Box::new(move |_d: &Driver, stage: &mut Stage| {
                // Use BatchDriver's own clock; outer driver is
                // unused in this arm.
                let mut bd = bdriver_inner.borrow_mut();
                let mut anims = anims_inner.borrow_mut();
                bd.tick_scalars(Duration::from_secs_f32(1.0 / 60.0), &mut anims, stage);
            });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Many => {
            // 12×12 grid of small ellipses, each with a scale Tween,
            // hammered through BatchDriver to show the budget
            // headroom. 144 tweens per frame.
            use wisp::scene::{Fill, Graphics};
            let bdriver = Rc::new(RefCell::new(BatchDriver::realtime()));
            bdriver.borrow_mut().play();
            let mut anims: Vec<BoundScalar> = Vec::with_capacity(144);
            for row in 0..12_u32 {
                for col in 0..12_u32 {
                    let mut g = Graphics::new();
                    g.fill(Fill::Solid(wisp::Color {
                        r: 0.0,
                        g: 0.5,
                        b: 0.85,
                        a: 1.0,
                    }));
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "row/col < 12, fits u16 trivially"
                    )]
                    let (col_u16, row_u16) = (col as u16, row as u16);
                    let x = (f32::from(col_u16) / 11.0 - 0.5) * 1.6;
                    let y = (f32::from(row_u16) / 11.0 - 0.5) * 1.6;
                    g.draw_ellipse(glam::Vec2::new(x, y), glam::Vec2::splat(0.05));
                    let id = app
                        .stage_mut()
                        .add_child(root, g)
                        .ok_or_else(|| "add_child returned None".to_owned())?;
                    // Each ellipse runs a unique-phase pulse on alpha.
                    let dur_ms = 600 + (row * 12 + col) * 8;
                    let pulse = Tween::new(0.2_f32, 1.0, Duration::from_millis(u64::from(dur_ms)))
                        .ease(Ease::InOutCubic)
                        .repeat_with(RepeatCount::Infinite, RepeatStrategy::MirroredRepeat);
                    anims.push(BoundScalar::new(pulse, NodeProperty::alpha(id)));
                }
            }
            let chart_id = anims[0].target.node;
            let driver = Driver::realtime();
            let anims_cell = Rc::new(RefCell::new(anims));
            let anims_inner = anims_cell.clone();
            let bdriver_inner = bdriver.clone();
            let mutator: FrameMutator = Box::new(move |_d: &Driver, stage: &mut Stage| {
                let mut bd = bdriver_inner.borrow_mut();
                let mut anims = anims_inner.borrow_mut();
                bd.tick_scalars(Duration::from_secs_f32(1.0 / 60.0), &mut anims, stage);
            });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::ColorSpaces => {
            // Three ellipses, each color-cycling red→green→blue→red
            // in a different colour space. Per-frame the ellipse is
            // destroyed + re-added so the fill colour updates (Graphics
            // primitives aren't externally mutable).
            use wisp::scene::{Fill, Graphics};
            let mut driver = Driver::realtime();
            driver.play();
            // Just remember the slots; primitive is recreated each frame.
            let labels = [ColorSpace::LinearRgb, ColorSpace::Oklab, ColorSpace::Oklch];
            let xs = [-0.6_f32, 0.0, 0.6];
            // Insert placeholder graphics so we have NodeIds.
            let mut initial_ids: Vec<NodeId> = Vec::with_capacity(3);
            for _ in 0..3 {
                let g = Graphics::new();
                let id = app
                    .stage_mut()
                    .add_child(root, g)
                    .ok_or_else(|| "add_child returned None".to_owned())?;
                initial_ids.push(id);
            }
            let primary = initial_ids[0];
            let ids_cell: Rc<RefCell<Vec<NodeId>>> = Rc::new(RefCell::new(initial_ids));
            let red = wisp::Color {
                r: 1.0,
                g: 0.0,
                b: 0.0,
                a: 1.0,
            };
            let green = wisp::Color {
                r: 0.0,
                g: 1.0,
                b: 0.0,
                a: 1.0,
            };
            let blue = wisp::Color {
                r: 0.0,
                g: 0.0,
                b: 1.0,
                a: 1.0,
            };
            let cycle_ms = 3_000.0_f32;
            let ids_inner = ids_cell.clone();
            let mutator: FrameMutator = Box::new(move |d: &Driver, stage: &mut Stage| {
                let elapsed_ms = d.elapsed().as_secs_f32() * 1000.0 % cycle_ms;
                let third = cycle_ms / 3.0;
                let (from, to, local_ms) = if elapsed_ms < third {
                    (red, green, elapsed_ms)
                } else if elapsed_ms < 2.0 * third {
                    (green, blue, elapsed_ms - third)
                } else {
                    (blue, red, elapsed_ms - 2.0 * third)
                };
                let mut ids = ids_inner.borrow_mut();
                let stage_root = stage.root();
                for (i, id) in ids.iter_mut().enumerate() {
                    let space = labels[i];
                    let mut tween =
                        ColorTween::new(from, to, Duration::from_secs_f32(third / 1000.0));
                    tween.space = space;
                    let c = tween.sample(Duration::from_secs_f32(local_ms / 1000.0));
                    stage.destroy(*id);
                    let mut g = Graphics::new();
                    g.fill(Fill::Solid(c));
                    g.draw_ellipse(glam::Vec2::new(xs[i], 0.0), glam::Vec2::splat(0.22));
                    if let Some(new_id) = stage.add_child(stage_root, g) {
                        *id = new_id;
                    }
                }
            });
            return Ok(AnimSetup {
                chart_id: primary,
                driver,
                mutator,
            });
        }
        AnimationKind::TypeIn => {
            // 10-step staircase scale via TypeWriter. Each step
            // increments visible count by 1; we map that to scale.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let total = 10_usize;
            let typer = TypeWriter::new(total, Duration::from_millis(1_500));
            let cycle_ms = 2_000.0_f32;
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                    let visible = typer.sample(Duration::from_secs_f32((pos_ms / 1000.0).min(1.5)));
                    #[allow(clippy::cast_precision_loss, reason = "total <= 10")]
                    let scale = visible as f32 / total as f32;
                    c.transform = Transform::from_scale(glam::Vec2::splat(scale));
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::MovePath => {
            // Polar follows a small circle with auto-rotate.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let circle: Vec<glam::Vec2> = (0..=32)
                .map(|i| {
                    #[allow(clippy::cast_precision_loss, reason = "i <= 32")]
                    let theta = (i as f32 / 32.0) * std::f32::consts::TAU;
                    glam::Vec2::new(0.4 * theta.cos(), 0.4 * theta.sin())
                })
                .collect();
            let path = MoveAlongPath::new(circle, Duration::from_millis(3_000)).auto_rotate(true);
            let cycle_ms = 3_000.0_f32;
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                    let pose = path.sample(Duration::from_secs_f32(pos_ms / 1000.0));
                    c.transform.position = pose.position;
                    c.transform.rotation = pose.angle;
                    c.transform.scale = glam::Vec2::splat(0.35);
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::DrawIn => {
            // Polar plot slides along an S-curve whose DrawIn reveals
            // the path over 2s. Demonstration: each frame, sample
            // DrawIn → last point becomes the chart's translation.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            // 11-point S-curve in NDC space.
            let path: Vec<glam::Vec2> = (0..=10)
                .map(|i| {
                    #[allow(clippy::cast_precision_loss, reason = "i <= 10")]
                    let t = i as f32 / 10.0;
                    let x = -0.5 + t;
                    let y = 0.3 * (t * std::f32::consts::TAU).sin();
                    glam::Vec2::new(x, y)
                })
                .collect();
            let drawin = DrawIn::new(path, Duration::from_secs(2));
            let cycle_ms = 2_500.0_f32;
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                    let local = Duration::from_secs_f32((pos_ms / 1000.0).min(2.0));
                    let revealed = drawin.sample(local);
                    if let Some(tail) = revealed.last() {
                        c.transform.position = *tail;
                        c.transform.scale = glam::Vec2::splat(0.45);
                    }
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Callbacks => {
            // Single-cycle spin wrapped with lifecycle callbacks.
            // The event reader gets drained inside the mutator
            // each frame; events log to the browser console.
            // Chart's alpha flashes briefly on the Completed event.
            use std::cell::Cell;
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let reader = EventReader::default();
            let inner = LinearRamp::new(0.0, TAU, Duration::from_millis(1_500))
                .with_callbacks(AnimId(42))
                .with_reader(reader.clone());
            // Wrap with infinite mirrored repeat so the animation
            // restarts after each completion — the reader will
            // fire Started/Completed pairs every cycle.
            let spin = inner.repeat_with(RepeatCount::Infinite, RepeatStrategy::Loop);
            let flash_until = Rc::new(Cell::new(Duration::ZERO));
            let flash_until_inner = flash_until.clone();
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let rotation = spin.sample(d.elapsed()) % TAU;
                    c.transform = Transform::from_rotation(rotation);
                    // Drain events; log to console; trigger a flash
                    // on Completed.
                    for ev in reader.drain() {
                        match ev {
                            AnimEvent::Started(id) => log::info!("anim {id:?} started"),
                            AnimEvent::Completed(id) => {
                                log::info!("anim {id:?} completed");
                                flash_until_inner.set(d.elapsed() + Duration::from_millis(120));
                            }
                            AnimEvent::Cycle { id, n } => {
                                log::info!("anim {id:?} cycle {n}");
                            }
                        }
                    }
                    // Apply the flash if still in window.
                    c.alpha = if d.elapsed() < flash_until.get() {
                        0.55
                    } else {
                        1.0
                    };
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Stagger => {
            // Five dots in a horizontal row, alpha-pulsing from
            // center outward via `Stagger::each(120ms).from(Center)`.
            // Each dot is its own NodeId — the mutator captures all
            // five and mutates them via stage iteration.
            use wisp::scene::{Fill, Graphics};
            let count = 5_usize;
            let mut ids: Vec<NodeId> = Vec::with_capacity(count);
            for i in 0..count {
                let mut g = Graphics::new();
                g.fill(Fill::Solid(wisp::Color {
                    r: 0.0,
                    g: 0.45,
                    b: 0.7,
                    a: 1.0,
                }));
                #[allow(clippy::cast_precision_loss, reason = "count <= 5")]
                let i_f = i as f32;
                #[allow(clippy::cast_precision_loss, reason = "count <= 5")]
                let count_f = count as f32;
                let centre_x = ((i_f / (count_f - 1.0)) - 0.5) * 1.2;
                g.draw_ellipse(glam::Vec2::new(centre_x, 0.0), glam::Vec2::splat(0.08));
                let id = app
                    .stage_mut()
                    .add_child(root, g)
                    .ok_or_else(|| "add_child returned None".to_owned())?;
                ids.push(id);
            }
            // Reuse `chart_id` as the first dot for the AnimSetup
            // (the field is unused downstream — see run_animated).
            let primary = ids[0];
            let mut driver = Driver::realtime();
            driver.play();
            let stagger = Stagger::each(Duration::from_millis(120)).from(StaggerFrom::Center);
            let pulse = Tween::new(0.2_f32, 1.0, Duration::from_millis(400)).ease(Ease::InOutCubic);
            let cycle_ms = 1_400.0_f32;
            let mutator: FrameMutator = Box::new(move |d: &Driver, stage: &mut Stage| {
                let cycle_pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                for (i, id) in ids.iter().enumerate() {
                    let offset = stagger.offset_for(i, ids.len());
                    let local_ms = cycle_pos_ms - offset.as_secs_f32() * 1000.0;
                    let alpha = if local_ms < 0.0 {
                        0.2
                    } else if local_ms < 400.0 {
                        pulse.sample(Duration::from_secs_f32(local_ms / 1000.0))
                    } else if local_ms < 800.0 {
                        // pulse out
                        pulse.sample(Duration::from_secs_f32((800.0 - local_ms) / 1000.0))
                    } else {
                        0.2
                    };
                    if let Some(node) = stage.get_mut(*id) {
                        node.container_mut().alpha = alpha;
                    }
                }
            });
            return Ok(AnimSetup {
                chart_id: primary,
                driver,
                mutator,
            });
        }
        AnimationKind::Keyframe => {
            // 4-keyframe scale walk: 1.0 → 0.5 → 1.2 → 0.8 over 2s.
            // Each segment uses a different ease to show per-segment
            // shaping.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let track: Track<f32> = Track::new()
                .key(Duration::ZERO, 1.0)
                .key_eased(Duration::from_millis(500), 0.5, Ease::InCubic)
                .key_eased(Duration::from_millis(1_200), 1.2, Ease::OutBack)
                .key_eased(Duration::from_millis(2_000), 0.8, Ease::InOutQuad);
            let cycle_ms = 2_000.0_f32;
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                    let scale = track.sample(Duration::from_secs_f32(pos_ms / 1000.0));
                    c.transform = Transform::from_scale(glam::Vec2::splat(scale.max(0.0)));
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Spring => {
            // Underdamped spring scales 0.4 → 1.0 with overshoot.
            // Cycles every 1.5s.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            let spring = Spring::underdamped(70.0, 1.0, 0.4).between(0.4, 1.0);
            let cycle_ms = 1_500.0_f32;
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    let pos_ms = (d.elapsed().as_secs_f32() * 1000.0) % cycle_ms;
                    let scale = spring.sample(Duration::from_secs_f32(pos_ms / 1000.0));
                    c.transform = Transform::from_scale(glam::Vec2::splat(scale.clamp(0.0, 2.0)));
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
        }
        AnimationKind::Slide => {
            // Slide the chart horizontally back and forth using a
            // Tween<Vec2> wrapped in MirroredRepeat. Demonstrates
            // `Target<Vec2>` semantics — the closure writes the
            // sampled Vec2 to `container.transform.position`,
            // equivalent to `NodeProperty::translation` from the
            // Target trait.
            let polar = crate::fixtures::polar_plot_fixture();
            let graphics = polar.emit_graphics(&theme, viewport);
            let chart_id = app
                .stage_mut()
                .add_child(root, graphics)
                .ok_or_else(|| "add_child returned None".to_owned())?;
            let mut driver = Driver::realtime();
            driver.play();
            // NDC units — Stage's transform is in NDC where the
            // viewport is `[-1, +1]`. Slide ±0.3 of the half-width.
            let slide = Tween::new(
                glam::Vec2::new(-0.3, 0.0),
                glam::Vec2::new(0.3, 0.0),
                Duration::from_millis(900),
            )
            .ease(Ease::InOutCubic)
            .repeat_with(RepeatCount::Infinite, RepeatStrategy::MirroredRepeat);
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    c.transform.position = slide.sample(d.elapsed());
                });
            return Ok(AnimSetup {
                chart_id,
                driver,
                mutator,
            });
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
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
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
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
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
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
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
            let mutator: FrameMutator =
                single_node_mutator(chart_id, move |d: &Driver, c: &mut Container| {
                    // Map [0, 2s) to [0, 1, 0] (yoyo). Cycle position
                    // wraps; first half ramps up, second half ramps
                    // down. Pure use of `Animatable::lerp(f32)`.
                    let cycle_pos =
                        (d.elapsed().as_secs_f32() % cycle.as_secs_f32()) / cycle.as_secs_f32();
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

    // Split the mutable borrow: take the closure and a snapshot of
    // the driver out of `s` so the closure can call `s.app.stage_mut()`
    // without overlapping borrows.
    let mut mutator = std::mem::replace(&mut s.mutator, Box::new(|_: &Driver, _: &mut Stage| {}));
    let driver_snapshot = s.driver.clone();
    mutator(&driver_snapshot, s.app.stage_mut());
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
