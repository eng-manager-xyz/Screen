//! Advanced (Tier C) blend modes — implemented as offscreen filter
//! passes that sample backdrop + foreground and run a per-mode blend
//! function in the fragment shader.
//!
//! The shared template lives at `shaders/advanced_blend.wgsl`. For each
//! advanced [`BlendMode`], [`blend_fn_body`] returns the per-mode
//! `blend_fn(base, blend) -> vec3<f32>` snippet that gets substituted
//! into the template before pipeline compilation.
//!
//! [`AdvancedBlendPipelines`] pre-builds one pipeline per advanced mode
//! at construction time. [`AdvancedBlendPipelines::apply`] runs the right one.

use std::collections::HashMap;

use crate::application::Application;
use crate::blend::BlendMode;
use crate::texture::render_texture::RenderTexture;

const TEMPLATE: &str = include_str!("../../shaders/advanced_blend.wgsl");
const PLACEHOLDER: &str = "// __BLEND_FN_PLACEHOLDER__";

/// Cache of one [`wgpu::RenderPipeline`] per advanced [`BlendMode`].
pub(crate) struct AdvancedBlendPipelines {
    pipelines: HashMap<BlendMode, wgpu::RenderPipeline>,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl AdvancedBlendPipelines {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::advanced_blend bg layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp::advanced_blend pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp::advanced_blend sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let mut pipelines = HashMap::new();
        for mode in BlendMode::all() {
            if !mode.is_advanced() {
                continue;
            }
            let body = blend_fn_body(mode);
            let wgsl = TEMPLATE.replace(PLACEHOLDER, body);
            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("wisp::advanced_blend shader"),
                source: wgpu::ShaderSource::Wgsl(wgsl.into()),
            });

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("wisp::advanced_blend pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("main_vs"),
                    buffers: &[],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("main_fs"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: output_format,
                        // The shader returns a fully-resolved composite;
                        // no GPU blend equation needed.
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });
            pipelines.insert(mode, pipeline);
        }

        Self {
            pipelines,
            bind_group_layout,
            sampler,
        }
    }

    /// Compose `backdrop` + `foreground` via `mode`'s blend function,
    /// writing the result into `output`. All three render-textures must
    /// share the same dimensions.
    pub(crate) fn apply(
        &self,
        app: &Application,
        mode: BlendMode,
        backdrop: &RenderTexture,
        foreground: &RenderTexture,
        output: &RenderTexture,
    ) {
        debug_assert!(
            mode.is_advanced(),
            "AdvancedBlendPipelines::apply called with native mode {mode:?}"
        );
        let pipeline = self
            .pipelines
            .get(&mode)
            .unwrap_or_else(|| panic!("no pipeline registered for {mode:?}"));

        let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::advanced_blend bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(backdrop.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(foreground.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::advanced_blend encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::advanced_blend pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: output.view(),
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.draw(0..3, 0..1);
        }

        app.queue().submit(std::iter::once(encoder.finish()));
    }
}

/// Per-mode `blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32>`.
///
/// Matches `PixiJS` v8's reference shader implementations from
/// `pixijs/src/advanced-blend-modes/*.ts`.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "20 advanced blend modes × multi-line WGSL bodies; flat match reads better than 20 helper fns"
)]
fn blend_fn_body(mode: BlendMode) -> &'static str {
    match mode {
        // ─── Mostly-elementwise ──────────────────────────────────────
        BlendMode::Darken => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return min(base, blend);
            }"
        }
        BlendMode::Lighten => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return max(base, blend);
            }"
        }
        BlendMode::Difference => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return abs(base - blend);
            }"
        }
        BlendMode::Exclusion => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return base + blend - 2.0 * base * blend;
            }"
        }
        BlendMode::Negation => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return vec3<f32>(1.0) - abs(vec3<f32>(1.0) - base - blend);
            }"
        }
        BlendMode::LinearDodge => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return clamp(base + blend, vec3<f32>(0.0), vec3<f32>(1.0));
            }"
        }
        BlendMode::LinearBurn => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return clamp(base + blend - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0));
            }"
        }
        BlendMode::LinearLight => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return clamp(base + 2.0 * blend - vec3<f32>(1.0), vec3<f32>(0.0), vec3<f32>(1.0));
            }"
        }
        BlendMode::Divide => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let safe = max(blend, vec3<f32>(1e-6));
                return clamp(base / safe, vec3<f32>(0.0), vec3<f32>(1.0));
            }"
        }

        // ─── Piecewise per channel ──────────────────────────────────
        BlendMode::Overlay => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let dark = 2.0 * base * blend;
                let light = vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - base) * (vec3<f32>(1.0) - blend);
                return select(light, dark, base < vec3<f32>(0.5));
            }"
        }
        BlendMode::HardLight => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let dark = 2.0 * base * blend;
                let light = vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - base) * (vec3<f32>(1.0) - blend);
                return select(light, dark, blend < vec3<f32>(0.5));
            }"
        }
        BlendMode::SoftLight => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let two_blend = 2.0 * blend;
                let darken = base - (vec3<f32>(1.0) - two_blend) * base * (vec3<f32>(1.0) - base);
                let one = vec3<f32>(1.0);
                let denom = select(
                    sqrt(base),
                    ((16.0 * base - 12.0) * base + 3.0) * base,
                    base <= vec3<f32>(0.25)
                );
                let lighten = base + (two_blend - one) * (denom - base);
                return select(lighten, darken, blend <= vec3<f32>(0.5));
            }"
        }
        BlendMode::PinLight => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let dark = min(base, 2.0 * blend);
                let light = max(base, 2.0 * blend - vec3<f32>(1.0));
                return select(light, dark, blend < vec3<f32>(0.5));
            }"
        }
        BlendMode::HardMix => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                // VividLight thresholded: ≥ 0.5 → 1, < 0.5 → 0.
                return step(vec3<f32>(0.5), base + blend - vec3<f32>(0.5));
            }"
        }
        BlendMode::VividLight => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let two_blend = 2.0 * blend;
                let safe_lo = max(two_blend, vec3<f32>(1e-6));
                let burn = vec3<f32>(1.0) - min(vec3<f32>(1.0), (vec3<f32>(1.0) - base) / safe_lo);
                let safe_hi = max(vec3<f32>(1.0) - (two_blend - vec3<f32>(1.0)), vec3<f32>(1e-6));
                let dodge = min(vec3<f32>(1.0), base / safe_hi);
                return select(dodge, burn, blend < vec3<f32>(0.5));
            }"
        }
        BlendMode::ColorBurn => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let safe = max(blend, vec3<f32>(1e-6));
                let result = vec3<f32>(1.0) - min(vec3<f32>(1.0), (vec3<f32>(1.0) - base) / safe);
                return select(result, vec3<f32>(0.0), blend <= vec3<f32>(0.0));
            }"
        }
        BlendMode::ColorDodge => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                let safe = max(vec3<f32>(1.0) - blend, vec3<f32>(1e-6));
                let result = min(vec3<f32>(1.0), base / safe);
                return select(result, vec3<f32>(1.0), blend >= vec3<f32>(1.0));
            }"
        }

        // ─── HSL family ──────────────────────────────────────────────
        BlendMode::Saturation => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return set_lum(set_sat(base, sat(blend)), lum(base));
            }"
        }
        BlendMode::Color => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return set_lum(blend, lum(base));
            }"
        }
        BlendMode::Luminosity => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return set_lum(base, lum(blend));
            }"
        }

        // Native modes — should never reach apply().
        BlendMode::Normal
        | BlendMode::Multiply
        | BlendMode::Add
        | BlendMode::Screen
        | BlendMode::Subtract
        | BlendMode::Min
        | BlendMode::Max
        | BlendMode::Erase => {
            "fn blend_fn(base: vec3<f32>, blend: vec3<f32>) -> vec3<f32> {
                return blend;
            }"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_advanced_mode_has_a_blend_fn() {
        // Smoke test that the placeholder substitution produces parseable
        // strings for each advanced mode. Doesn't compile WGSL — that's
        // covered by the integration test in `tests/blend_modes_advanced.rs`
        // which does build the full pipeline.
        for mode in BlendMode::all().filter(|m| m.is_advanced()) {
            let body = blend_fn_body(mode);
            let wgsl = TEMPLATE.replace(PLACEHOLDER, body);
            assert!(
                wgsl.contains("fn blend_fn"),
                "missing blend_fn for {mode:?}"
            );
            assert!(
                !wgsl.contains(PLACEHOLDER),
                "placeholder left in WGSL for {mode:?}"
            );
        }
    }
}
