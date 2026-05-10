//! `BlendPipelineMap` — pre-built `RenderPipeline` per standard blend mode.
//!
//! Each of wisp's 6 pipelines (sprite, quad, triangle, mesh, text,
//! graphics) takes the same shader/vertex/bind-group setup and varies
//! *only* in the `wgpu::BlendState`. Rather than building one pipeline
//! per mode by hand (× 6 pipelines × 8 native modes = 48 pipeline
//! construction sites), this helper accepts a builder closure that
//! takes a `BlendState` and returns a `RenderPipeline`. We pre-build
//! one pipeline per native [`BlendMode`] at construction time.
//!
//! Advanced modes (Tier C — Overlay, `ColorBurn`, …) aren't represented
//! in the map — they require the offscreen filter pipeline. When a
//! caller asks for an advanced mode via [`BlendPipelineMap::get`], we
//! fall back to `Normal`. This fallback is INTENTIONAL when
//! [`Renderer::render_stage`](crate::render::Renderer::render_stage)
//! renders an advanced-blend subtree into a foreground RT — the leaf's
//! pure colors land in the foreground, then the parent's advanced blend
//! is applied via [`apply_advanced_blend`](crate::render::Renderer::apply_advanced_blend).
//! No warning is emitted because auto-dispatch makes it correct by
//! default.

use std::collections::HashMap;

use crate::blend::BlendMode;

/// Map from a [`BlendMode`] to its pre-built `RenderPipeline`.
pub(crate) struct BlendPipelineMap {
    inner: HashMap<BlendMode, wgpu::RenderPipeline>,
}

impl BlendPipelineMap {
    /// Build one pipeline per native [`BlendMode`] by invoking `build`
    /// with that mode's [`wgpu::BlendState`].
    pub(crate) fn new<F>(mut build: F) -> Self
    where
        F: FnMut(wgpu::BlendState) -> wgpu::RenderPipeline,
    {
        let mut inner = HashMap::new();
        for mode in BlendMode::all() {
            if let Some(blend) = mode.native_blend_state() {
                inner.insert(mode, build(blend));
            }
        }
        Self { inner }
    }

    /// Look up the pipeline for `mode`. Advanced modes silently fall
    /// back to `Normal` — by design, since `render_stage`'s
    /// auto-dispatch path renders advanced-blend subtrees into a
    /// foreground RT with Normal blending, then composites via
    /// `apply_advanced_blend`.
    pub(crate) fn get(&self, mode: BlendMode) -> &wgpu::RenderPipeline {
        if let Some(p) = self.inner.get(&mode) {
            return p;
        }
        self.inner
            .get(&BlendMode::Normal)
            .expect("BlendPipelineMap always builds Normal")
    }
}
