//! `wisp-chart-web` — Trunk-driven WebGPU demo for `wisp-chart`.
//!
//! The same `wisp-chart` crate compiles for native AND
//! `wasm32-unknown-unknown`. This crate is the browser-facing
//! consumer: it owns the HTML page, finds a `<canvas>`, creates
//! a `wgpu::Surface` from it with the `BROWSER_WEBGPU` backend,
//! and draws whatever `wisp-chart` returns.
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
//! Today the demo clears the canvas to a deliberate vivid purple
//! ([`DEMO_CLEAR_COLOR`]) so anyone reloading the page can tell at
//! a glance whether WebGPU committed pixels — a near-white clear
//! is too easy to confuse with a default canvas backdrop. The
//! actual chart rendering lands in M-CHART.0 chunk 3 (when
//! `Gantt::render(&Theme) -> SceneNode` ships); until then this
//! purple is the smoke signal.
//!
//! # Tests
//!
//! Two layered tests prove the render path actually writes pixels
//! (the chunk-3 commit only proved `cargo check`, which is why the
//! grey-canvas regression slipped through):
//!
//! - `tests/clear_pass.rs` (native, always-on) — runs
//!   [`clear_to_white`] against an offscreen `RenderTexture` on
//!   whichever wgpu backend the runner exposes (Metal on macOS,
//!   Vulkan / lavapipe on Linux, DX12 on Windows), reads the
//!   centre pixel back, and asserts opaque white. Catches
//!   regressions of "the render code wrote wrong pixels".
//! - `tests/headless_webgpu.rs` (wasm32, local-only) — runs the
//!   same helper inside a real headless Chrome via
//!   `wasm-bindgen-test`, against a canvas-backed `wgpu::Surface`
//!   with `Backends::BROWSER_WEBGPU`. Catches regressions of
//!   "the surface-presentation path silently no-ops" — the bug
//!   class this very crate hit on first deploy. Not wired into
//!   `just gate`; invoke locally with
//!   `WASM_BINDGEN_TEST_TIMEOUT=60 cargo test --target wasm32-unknown-unknown -p wisp-chart-web`
//!   after `brew install chromedriver`.

#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export wisp-chart so downstream consumers of this crate
// (rare — almost everyone consumes wisp-chart directly) can grab
// it via one Cargo dep line.
pub use wisp_chart;

/// Clear colour used by the production demo + both tests.
///
/// Vivid purple. Chosen for three reasons:
///
/// 1. **Distinct from "broken" states.** Grey (`#fafafa` canvas
///    default), white (common clear-on-error), and full
///    transparent all look nothing like this — a glance is enough
///    to confirm WebGPU committed pixels.
/// 2. **Round byte values.** `0.6 / 0.2 / 0.8` map exactly to
///    `153 / 51 / 204` in `Rgba8Unorm`; there's no
///    half-pixel-rounding ambiguity in the assertions.
/// 3. **sRGB-neutral enough.** No channel is at 0.0 or 1.0 where
///    sRGB encoding curves are steepest, so a future surface
///    format switch is less likely to change the exact bytes the
///    tests check.
pub const DEMO_CLEAR_COLOR: wgpu::Color = wgpu::Color {
    r: 0.6,
    g: 0.2,
    b: 0.8,
    a: 1.0,
};

/// `[153, 51, 204, 255]` — [`DEMO_CLEAR_COLOR`] as `Rgba8Unorm`
/// bytes. The native readback test compares against this directly.
/// The headless surface test swaps R↔B if the canvas-preferred
/// format is `Bgra8Unorm`.
pub const DEMO_CLEAR_RGBA8: [u8; 4] = [153, 51, 204, 255];

/// Clears `view` to `color` in a single render pass.
///
/// Target-agnostic — `view` may be a canvas-backed surface view
/// (browser demo) or an offscreen `RenderTexture` view (native
/// test). The pass uses `LoadOp::Clear` + `StoreOp::Store` so the
/// destination contains the requested colour after submission.
///
/// Submitting alone is enough for offscreen targets. The
/// browser-side caller is still responsible for `frame.present()`
/// on the surface texture afterward.
pub fn clear_with_color(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    color: wgpu::Color,
) {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("wisp-chart-web clear encoder"),
    });
    {
        let _rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("wisp-chart-web clear pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    queue.submit(std::iter::once(encoder.finish()));
}

// Everything browser-flavoured is scoped to the wasm32 target.
// On native targets this crate is a no-op `rlib` (compiles, no
// public functions). The `cargo check --workspace` gate on
// macOS/Ubuntu/Windows runners stays green without doing any
// browser work.
#[cfg(target_arch = "wasm32")]
mod web;
