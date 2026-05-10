//! `RenderTexture` — texture configured as a render target with a pixel-readback path.
//!
//! Used by:
//! - the export pipeline to render a scene to disk-bound bytes (M0.21);
//! - filter passes (M0.16) that ping-pong between two render targets;
//! - integration tests that need to verify rendered output by inspecting pixels.

use std::sync::Arc;

use crate::application::Application;
use crate::texture::Texture;

/// GPU texture configured for rendering into and reading back.
///
/// Usage flags: `RENDER_ATTACHMENT | COPY_SRC | TEXTURE_BINDING`.
#[derive(Clone)]
pub struct RenderTexture {
    inner: Arc<RenderTextureInner>,
}

struct RenderTextureInner {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl RenderTexture {
    /// Allocate a new `RenderTexture` of the given dimensions in `Rgba8UnormSrgb`.
    ///
    /// Use [`RenderTexture::with_format`] for other formats.
    #[must_use]
    pub fn new(app: &Application, width: u32, height: u32) -> Self {
        Self::with_format(app, width, height, wgpu::TextureFormat::Rgba8UnormSrgb)
    }

    /// Allocate with an explicit color format.
    #[must_use]
    pub fn with_format(
        app: &Application,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let device = app.device();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wisp::RenderTexture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp::RenderTexture sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            inner: Arc::new(RenderTextureInner {
                texture,
                view,
                sampler,
                width,
                height,
                format,
            }),
        }
    }

    /// `wgpu::TextureView` suitable for use as a render-pass color attachment.
    #[must_use]
    pub fn view(&self) -> &wgpu::TextureView {
        &self.inner.view
    }

    /// Sampler for downstream passes that read from this render target.
    #[must_use]
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.inner.sampler
    }

    /// Color format.
    #[must_use]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.inner.format
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.inner.width
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.inner.height
    }

    /// Wrap this render target as a sampled [`Texture`] for use in
    /// the sprite pipeline.
    ///
    /// Shares the underlying wgpu resources — no GPU copy. The render
    /// texture's `TEXTURE_BINDING` usage flag is already set, so
    /// sampling is well-defined.
    ///
    /// Useful for compositing text/mask/filter outputs into a scene:
    /// render into a `RenderTexture`, then `as_texture()` + attach to
    /// a [`crate::scene::Sprite`].
    #[must_use]
    pub fn as_texture(&self) -> Texture {
        Texture::from_render_texture_parts(
            self.inner.texture.clone(),
            self.inner.view.clone(),
            self.inner.sampler.clone(),
            self.inner.width,
            self.inner.height,
        )
    }

    /// Read back the rendered pixels as a tightly-packed RGBA8 buffer.
    ///
    /// For `Rgba8UnormSrgb` / `Bgra8UnormSrgb` formats the returned buffer is
    /// `width * height * 4` bytes; row-padding from
    /// `COPY_BYTES_PER_ROW_ALIGNMENT` is stripped before return.
    ///
    /// Blocks the current thread on `device.poll(Maintain::Wait)`.
    #[must_use]
    pub fn read_pixels(&self, app: &Application) -> Vec<u8> {
        let device = app.device();
        let queue = app.queue();
        let unpadded_bytes_per_row = self.inner.width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = u64::from(padded_bytes_per_row) * u64::from(self.inner.height);

        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wisp::RenderTexture readback"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("wisp::RenderTexture::read_pixels encoder"),
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.inner.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.inner.height),
                },
            },
            wgpu::Extent3d {
                width: self.inner.width,
                height: self.inner.height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));

        let (sender, receiver) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .expect("readback channel dropped")
            .expect("readback map failed");

        let mapped = staging.slice(..).get_mapped_range();
        let unpadded_size = (unpadded_bytes_per_row as usize) * (self.inner.height as usize);
        let mut output = Vec::with_capacity(unpadded_size);
        for row in 0..self.inner.height {
            let start = (row * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            output.extend_from_slice(&mapped[start..end]);
        }
        drop(mapped);
        staging.unmap();
        output
    }
}

impl std::fmt::Debug for RenderTexture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderTexture")
            .field("width", &self.inner.width)
            .field("height", &self.inner.height)
            .field("format", &self.inner.format)
            .finish()
    }
}
