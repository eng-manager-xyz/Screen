//! `wisp-chart-web` — Trunk-driven WebGPU demo for `wisp-chart`.
//!
//! The same `wisp-chart` crate compiles for native AND
//! `wasm32-unknown-unknown`. This crate is the browser-facing
//! consumer: it owns the HTML page, finds a `<canvas>`, builds a
//! `wgpu::Surface` from it via the `BROWSER_WEBGPU` backend, then
//! hands the wgpu context to a `wisp::Application` and lets
//! `wisp::Renderer` draw whatever `wisp-chart` emits.
//!
//! Today the demo renders [`sample_gantt`] — a small Gantt fixture
//! laid out on a white background — as the smoke signal that the
//! whole pipeline (canvas → wgpu → wisp → wisp-chart) is wired up.
//!
//! Build with Trunk:
//!
//! ```bash
//! cd crates/wisp-chart-web
//! trunk build --release
//! ```
//!
//! Or run the dev server:
//!
//! ```bash
//! just dev-wisp-chart-demo
//! ```
//!
//! # Tests
//!
//! Two layered tests prove the render path actually writes pixels
//! — the chunk-3 commit only proved `cargo check`, which is why
//! the original grey-canvas regression slipped through.
//!
//! - `tests/render_gantt.rs` (native, always-on) — runs the same
//!   `Gantt::emit_graphics` + `Renderer::render_stage` path against
//!   an offscreen `RenderTexture` on whichever wgpu backend the
//!   host exposes (Metal / Vulkan / DX12), reads pixels back, and
//!   asserts (1) background pixel matches `Theme.bg`,
//!   (2) centre pixel of a known bar matches the explicit
//!   `Person.color` override registered in [`sample_gantt`]. Also
//!   writes the rendered output to
//!   `_docs/wisp-chart-book/src/assets/wisp-chart-web/gantt-demo.png`
//!   as PR-visible proof.
//! - `tests/headless_webgpu.rs` (wasm32, local-only) — drives a
//!   real headless Chrome via `wasm-bindgen-test`, runs the same
//!   render path against a canvas-backed surface, copies the
//!   surface texture back, and asserts the same pixels. Run with
//!   `WASM_BINDGEN_TEST_TIMEOUT=60 cargo test --target
//!   wasm32-unknown-unknown -p wisp-chart-web` after `brew install
//!   chromedriver`.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod easing_grid;
pub mod fixtures;
pub mod render;

pub use render::{ChartId, render_chart_to_view};

use glam::Vec2;
use jiff::civil::date;
use wgpu::TextureView;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp_chart::gantt::{Bar, DateRange, Row};
use wisp_chart::{Color as ChartColor, Gantt, Person, PersonMap, Theme};

// Re-export wisp-chart so downstream consumers of this crate
// (rare — almost everyone consumes wisp-chart directly) can grab
// it via one Cargo dep line.
pub use wisp_chart;

/// The demo Gantt fixture rendered both in the browser and in
/// the tests.
///
/// Snapshot of the wisp roadmap as it stood when this demo
/// shipped: four milestone rows (M-VEC, M-DYN, M-TEXT, M-BOOL)
/// across 2026 with bars on the months the milestone was the
/// active focus. Two named owners with explicit colour
/// overrides so the tests can pin centre-pixel colour
/// assertions against a known palette entry rather than against
/// a hash that might shift if the palette policy changes.
#[must_use]
pub fn sample_gantt() -> Gantt {
    let mut people = PersonMap::default();
    // Wong navy — owner "Matt".
    people.insert(Person {
        name: "Matt".into(),
        color: ChartColor::from_hex("#0072b2").unwrap(),
    });
    // Wong vermillion — owner "Alice".
    people.insert(Person {
        name: "Alice".into(),
        color: ChartColor::from_hex("#d55e00").unwrap(),
    });

    Gantt {
        range: DateRange::year(2026),
        rows: vec![
            Row::new("vec", "M-VEC"),
            Row::new("dyn", "M-DYN"),
            Row::new("text", "M-TEXT"),
            Row::new("bool", "M-BOOL"),
        ],
        bars: vec![
            Bar::new("vec", date(2026, 1, 15)..date(2026, 4, 1), "Matt"),
            Bar::new("vec", date(2026, 5, 1)..date(2026, 6, 15), "Alice"),
            Bar::new("dyn", date(2026, 3, 1)..date(2026, 7, 1), "Alice"),
            Bar::new("text", date(2026, 5, 15)..date(2026, 9, 1), "Matt"),
            Bar::new("bool", date(2026, 8, 1)..date(2026, 12, 15), "Matt"),
        ],
        people,
        markers: Vec::new(),
    }
}

/// Planning-size Gantt fixture used by WG.8 (AUT-328) — dozens
/// of rows across two Group parents, multi-lane assignments,
/// allocation caps, a tech-lead marker, holidays, quarter ticks,
/// a current-date overlay, and a year-end slowdown overlay.
///
/// Shaped after the H2 planning view's typical density so the
/// wisp-chart-web demo proves the API stack works at real scale.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "Fixture builder — every line constructs one stable row/bar/marker; splitting would force re-passing the PersonMap + date helpers and reduce readability"
)]
pub fn sample_gantt_planning() -> Gantt {
    use wisp_chart::gantt::{GanttMarker, GanttRole, RowKind};

    let mut people = PersonMap::default();
    for (name, hex) in [
        ("Matt", "#0072b2"),
        ("Alice", "#d55e00"),
        ("Bob", "#009e73"),
        ("Carol", "#cc79a7"),
        ("Dave", "#56b4e9"),
        ("Eve", "#e69f00"),
    ] {
        people.insert(Person {
            name: name.into(),
            color: ChartColor::from_hex(hex).unwrap(),
        });
    }

    let mut rows = Vec::new();
    let mut bars = Vec::new();

    // ─── M-CHART group — staggered Q1/Q2 work ──
    rows.push(
        Row::new("chart", "M-CHART")
            .with_kind(RowKind::Group)
            .with_subtitle("Chart milestones"),
    );
    let chart_milestones: &[(&str, &str, i8, i8, &str, GanttRole, f32)] = &[
        // (id, label, start_month, end_month, owner, role, alloc%)
        (
            "chart.line",
            "Line + area",
            1,
            4,
            "Matt",
            GanttRole::TechLead,
            75.0,
        ),
        (
            "chart.bar",
            "Grouped + stacked bar",
            2,
            4,
            "Alice",
            GanttRole::TechLead,
            50.0,
        ),
        (
            "chart.heatmap",
            "Heatmaps",
            3,
            5,
            "Bob",
            GanttRole::TechLead,
            60.0,
        ),
        (
            "chart.pie",
            "Pie + donut + sunburst",
            4,
            6,
            "Carol",
            GanttRole::TechLead,
            50.0,
        ),
        (
            "chart.gantt",
            "Gantt v2 (this PR)",
            5,
            11,
            "Matt",
            GanttRole::TechLead,
            80.0,
        ),
    ];
    for (id, label, start_m, end_m, owner, role, alloc) in chart_milestones {
        rows.push(
            Row::new(*id, *label)
                .with_parent("chart")
                .with_effort_label(format!("{} wk", (end_m - start_m) * 4))
                .with_estimated_weeks(f32::from((end_m - start_m) * 4)),
        );
        let start = date(2026, *start_m, 1);
        let end = date(2026, *end_m, 1);
        bars.push(
            Bar::new(*id, start..end, *owner)
                .with_allocation_pct(*alloc)
                .with_role(*role),
        );
    }
    // Second lane on "Gantt v2" for multi-person testing.
    bars.push(
        Bar::new("chart.gantt", date(2026, 5, 1)..date(2026, 11, 1), "Bob")
            .with_lane(1)
            .with_allocation_pct(50.0),
    );

    // ─── M-INTERACTION group — staggered Q2/Q3/Q4 work ──
    rows.push(
        Row::new("inter", "M-INTERACTION")
            .with_kind(RowKind::Group)
            .with_subtitle("Pointer + camera + animation triggers"),
    );
    let inter_milestones: &[(&str, &str, i8, i8, &str)] = &[
        ("inter.pointer", "Pointer dispatch", 5, 8, "Carol"),
        ("inter.hit", "Hit-test backends", 6, 8, "Dave"),
        ("inter.camera", "Orbit + pan/zoom", 7, 10, "Eve"),
        ("inter.adapter", "winit + web adapters", 8, 10, "Carol"),
        ("inter.gantt", "Gantt pan/popover/kinetic", 9, 12, "Dave"),
    ];
    for (id, label, start_m, end_m, owner) in inter_milestones {
        rows.push(
            Row::new(*id, *label)
                .with_parent("inter")
                .with_effort_label(format!("{} wk", (end_m - start_m) * 4))
                .with_estimated_weeks(f32::from((end_m - start_m) * 4)),
        );
        let start = date(2026, *start_m, 15);
        let end = date(2026, *end_m, 15);
        bars.push(Bar::new(*id, start..end, *owner).with_allocation_pct(60.0));
    }

    let markers = vec![
        GanttMarker::CurrentDate {
            date: date(2026, 6, 15),
        },
        GanttMarker::QuarterStart {
            date: date(2026, 4, 1),
            label: Some("Q2 2026".into()),
        },
        GanttMarker::QuarterStart {
            date: date(2026, 7, 1),
            label: Some("Q3 2026".into()),
        },
        GanttMarker::QuarterStart {
            date: date(2026, 10, 1),
            label: Some("Q4 2026".into()),
        },
        GanttMarker::Holiday {
            range: DateRange::day(date(2026, 7, 4)),
            label: "Independence Day".into(),
        },
        GanttMarker::Holiday {
            range: DateRange::from_range(date(2026, 11, 26)..date(2026, 11, 28)),
            label: "Thanksgiving".into(),
        },
        GanttMarker::PlanningOverlay {
            range: DateRange::from_range(date(2026, 12, 20)..date(2027, 1, 1)),
            label: "Year-end slowdown".into(),
            color: ChartColor {
                r: 0.95,
                g: 0.92,
                b: 0.80,
                a: 0.45,
            },
        },
    ];

    Gantt {
        range: DateRange::year(2026),
        rows,
        bars,
        people,
        markers,
    }
}

/// Render `gantt` through the new four-pane scene graph (WG.4)
/// with chrome (WG.2) + laned bars (WG.3). Targets `target_view`
/// via a fresh `wisp::Renderer` keyed on `surface_format`.
///
/// Unlike [`render_gantt`], this path:
/// - Emits the WG.2 chrome (header bg, gridlines, marker overlays).
/// - Uses WG.3 lane-aware bar layout.
/// - Renders via WG.4's `GanttScene` (corner / header / gutter /
///   body panes). Per-pane scissor + pan offset wiring is the
///   host's job — this function paints all four panes back-to-back
///   onto a single surface for the demo's static-image use case.
///
/// # Errors
///
/// Returns a wisp [`Error`](wisp::Error) if `Renderer::new` fails.
pub fn render_gantt_planning(
    app: &mut Application,
    target_view: &TextureView,
    surface_format: wgpu::TextureFormat,
    viewport_px: Vec2,
    gantt: &Gantt,
    theme: &Theme,
) -> Result<(), wisp::Error> {
    let renderer = Renderer::new(app, surface_format)?;
    let scene = gantt.emit_scene(theme, viewport_px);
    let root = app.stage().root();
    // Paint in pane order: corner first (top-most), then header,
    // gutter, body — matches the host's z-order when the body
    // pans behind frozen panes.
    let _ = app.stage_mut().add_child(root, scene.body);
    let _ = app.stage_mut().add_child(root, scene.gutter);
    let _ = app.stage_mut().add_child(root, scene.header);
    let _ = app.stage_mut().add_child(root, scene.corner);

    let _stats = renderer.render_stage(
        app,
        target_view,
        wisp::Color::rgba(1.0, 0.0, 1.0, 1.0),
        app.stage(),
    );
    Ok(())
}

/// Render `gantt` into `target_view` via a fresh
/// `wisp::Renderer` keyed on `surface_format`.
///
/// Target-agnostic — `target_view` may be a canvas surface
/// texture view (browser demo) or an offscreen `RenderTexture`
/// view (native test). `viewport_px` is the destination's
/// `(width, height)`; layout math + NDC conversion key off it.
///
/// `clear` is the colour written before the scene tree paints;
/// since [`Gantt::emit_graphics`] starts with a full-frame
/// background rect of `theme.bg`, the clear colour is only
/// visible if the renderer fails partway through a bar — useful
/// as a low-grade smoke signal but not user-visible normally.
///
/// # Errors
///
/// Returns a wisp [`Error`](wisp::Error) if `Renderer::new` fails
/// (rare — happens when the device can't compile the pipelines
/// for `surface_format`).
pub fn render_gantt(
    app: &mut Application,
    target_view: &TextureView,
    surface_format: wgpu::TextureFormat,
    viewport_px: Vec2,
    gantt: &Gantt,
    theme: &Theme,
) -> Result<(), wisp::Error> {
    let renderer = Renderer::new(app, surface_format)?;
    let graphics = gantt.emit_graphics(theme, viewport_px);
    let root = app.stage().root();
    let _ = app.stage_mut().add_child(root, graphics);

    let _stats = renderer.render_stage(
        app,
        target_view,
        // Bright magenta clear — if any of the chart paint is
        // missing, you'll see it. Bars + bg should fully cover.
        wisp::Color::rgba(1.0, 0.0, 1.0, 1.0),
        app.stage(),
    );
    Ok(())
}

// Everything browser-flavoured is scoped to the wasm32 target.
// On native targets this crate is a no-op `rlib` (compiles, no
// public functions). The `cargo check --workspace` gate on
// macOS/Ubuntu/Windows runners stays green without doing any
// browser work.
#[cfg(target_arch = "wasm32")]
mod web;
