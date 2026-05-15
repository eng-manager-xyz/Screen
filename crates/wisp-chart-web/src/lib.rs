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
    }
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
