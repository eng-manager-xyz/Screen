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
//! fall back to `Normal` and `tracing::warn!` once (deduped by mode).

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

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

    /// Look up the pipeline for `mode`. For advanced modes we fall back
    /// to `Normal` and log a warning once per mode.
    pub(crate) fn get(&self, mode: BlendMode) -> &wgpu::RenderPipeline {
        if let Some(p) = self.inner.get(&mode) {
            return p;
        }
        warn_advanced_fallback_once(mode);
        self.inner
            .get(&BlendMode::Normal)
            .expect("BlendPipelineMap always builds Normal")
    }
}

/// Suppress duplicate warnings for the same advanced mode — a stage
/// with 100 sprites all using `Overlay` would otherwise spam stderr 100
/// times per frame.
fn warn_advanced_fallback_once(mode: BlendMode) {
    static SEEN: Mutex<Option<HashSet<BlendMode>>> = Mutex::new(None);
    let mut guard = SEEN.lock().expect("warn_advanced_fallback_once mutex");
    let set = guard.get_or_insert_with(HashSet::new);
    if set.insert(mode) {
        tracing::warn!(
            mode = mode.css_name(),
            "advanced blend mode used in render_stage; falling back to Normal. \
             Use Renderer::apply_advanced_blend for correct compositing."
        );
    }
}
