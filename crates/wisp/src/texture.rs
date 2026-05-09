//! Texture types — `Texture` (image), `VideoTexture` (per-frame upload), `RenderTexture` (target).

pub mod render_texture;
pub mod video_texture;

use std::sync::Arc;

use image::DynamicImage;

use crate::application::Application;

/// GPU image texture backed by a `wgpu::Texture`.
///
/// Cheaply cloneable; the underlying GPU resource is `Arc`-wrapped.
#[derive(Clone)]
pub struct Texture {
    inner: Arc<TextureInner>,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texture")
            .field("width", &self.inner.width)
            .field("height", &self.inner.height)
            .field("id", &self.id())
            .finish()
    }
}

struct TextureInner {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
}

impl Texture {
    /// Construct from raw RGBA8 bytes in linear-srgb space.
    ///
    /// `bytes.len()` must equal `width * height * 4`.
    ///
    /// # Panics
    ///
    /// Panics if the byte slice length doesn't match `width * height * 4`.
    #[track_caller]
    #[must_use]
    pub fn from_rgba(app: &Application, width: u32, height: u32, bytes: &[u8]) -> Self {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .expect("Texture::from_rgba: dimensions overflow usize");
        assert_eq!(
            bytes.len(),
            expected,
            "Texture::from_rgba: byte length mismatch (got {got}, expected {expected} for {width}×{height} RGBA)",
            got = bytes.len(),
        );

        let device = app.device();
        let queue = app.queue();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wisp::Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp::Texture sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            inner: Arc::new(TextureInner {
                texture,
                view,
                sampler,
                width,
                height,
            }),
        }
    }

    /// Construct from a decoded image. Converts to RGBA8 if needed.
    #[must_use]
    pub fn from_image(app: &Application, img: &DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        Self::from_rgba(app, rgba.width(), rgba.height(), rgba.as_raw())
    }

    /// Construct an empty (zeroed) texture for the given format.
    ///
    /// Usage flags include `TEXTURE_BINDING | COPY_DST`, suitable for
    /// sampling and per-frame uploads (e.g. backing a [`crate::texture::video_texture`]).
    /// For render targets see [`crate::texture::render_texture`].
    #[must_use]
    pub fn empty(app: &Application, width: u32, height: u32, format: wgpu::TextureFormat) -> Self {
        let device = app.device();
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("wisp::Texture::empty"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("wisp::Texture::empty sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            inner: Arc::new(TextureInner {
                texture,
                view,
                sampler,
                width,
                height,
            }),
        }
    }

    /// Texture width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.inner.width
    }

    /// Texture height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.inner.height
    }

    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.inner.view
    }

    pub(crate) fn sampler(&self) -> &wgpu::Sampler {
        &self.inner.sampler
    }

    pub(crate) fn wgpu_texture(&self) -> &wgpu::Texture {
        &self.inner.texture
    }

    /// Identity for batching — texture-pointer-equality.
    ///
    /// Two `Texture` clones (or two views into the same `Arc`) share the same
    /// id; two independently-constructed textures don't. Used by the renderer
    /// to group sprite draws.
    pub(crate) fn id(&self) -> usize {
        Arc::as_ptr(&self.inner) as usize
    }
}
