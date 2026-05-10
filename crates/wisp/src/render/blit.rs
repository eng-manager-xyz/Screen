//! Fullscreen-sampler "blit" pipeline — copy a `RenderTexture` onto an
//! arbitrary target view.
//!
//! Used by the auto-dispatch advanced-blend renderer at the end of each
//! frame to flush the internal `dest_rt` (where compositing happens)
//! onto the caller-supplied view (typically a window surface or another
//! `RenderTexture`).

use crate::application::Application;
use crate::texture::render_texture::RenderTexture;

pub(crate) struct BlitPipeline {
    /// `BlendState::REPLACE` — copies source pixels exactly. Used for
    /// the final `dest_rt` → view flush at end of slow-path render.
    pipeline: wgpu::RenderPipeline,
    /// `BlendState::ALPHA_BLENDING` — source-over composite. Used by
    /// the clip dispatch path to layer a masked RT on top of the
    /// in-progress destination.
    pipeline_over: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

impl BlitPipeline {
    pub(crate) fn new(app: &Application, output_format: wgpu::TextureFormat) -> Self {
        let device = app.device();

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wisp::blit bg layout"),
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wisp::blit pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wisp::blit shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/blit.wgsl").into()),
        });

        let make_pipeline = |label: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
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
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };
        let pipeline = make_pipeline("wisp::blit pipeline", wgpu::BlendState::REPLACE);
        let pipeline_over = make_pipeline(
            "wisp::blit pipeline (over)",
            wgpu::BlendState::ALPHA_BLENDING,
        );

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp::blit sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Self {
            pipeline,
            pipeline_over,
            bind_group_layout,
            sampler,
        }
    }

    /// Sample `src` and write it onto `target_view`, replacing the
    /// destination pixels. Used for the final `dest_rt` → view flush.
    pub(crate) fn blit(
        &self,
        app: &Application,
        src: &RenderTexture,
        target_view: &wgpu::TextureView,
    ) {
        self.run(app, src, target_view, &self.pipeline, /*clear=*/ true);
    }

    /// Sample `src` and source-over composite onto `target_rt`,
    /// preserving the destination's existing content under transparent
    /// regions of `src`. Used by the clip dispatcher to layer a masked
    /// RT on top of the in-progress destination.
    pub(crate) fn compose_over(
        &self,
        app: &Application,
        src: &RenderTexture,
        target_rt: &RenderTexture,
    ) {
        self.run(
            app,
            src,
            target_rt.view(),
            &self.pipeline_over,
            /*clear=*/ false,
        );
    }

    fn run(
        &self,
        app: &Application,
        src: &RenderTexture,
        target_view: &wgpu::TextureView,
        pipeline: &wgpu::RenderPipeline,
        clear: bool,
    ) {
        let bg = app.device().create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wisp::blit bg"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src.view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp::blit encoder"),
            });
        {
            let load = if clear {
                wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
            } else {
                wgpu::LoadOp::Load
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp::blit pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load,
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
