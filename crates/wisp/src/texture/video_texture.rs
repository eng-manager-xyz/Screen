//! `VideoTexture` — texture backed by per-frame BGRA uploads.
//!
//! Used by the recorder to feed decoded screen-capture frames into the scene.
//! Wraps a [`crate::texture::Texture`] with `Bgra8UnormSrgb` format and
//! `TEXTURE_BINDING | COPY_DST` usage; each frame the host calls
//! [`VideoTexture::upload_bgra`].

use crate::application::Application;
use crate::texture::Texture;

/// GPU texture backed by per-frame BGRA8 uploads.
#[derive(Clone, Debug)]
pub struct VideoTexture {
    texture: Texture,
}

impl VideoTexture {
    /// Allocate a new `VideoTexture` with the given dimensions.
    #[must_use]
    pub fn new(app: &Application, width: u32, height: u32) -> Self {
        Self {
            texture: Texture::empty(app, width, height, wgpu::TextureFormat::Bgra8UnormSrgb),
        }
    }

    /// Upload a frame of BGRA8 bytes. `bytes.len()` must equal `width * height * 4`.
    ///
    /// # Panics
    ///
    /// Panics if the byte slice length doesn't match the texture dimensions.
    pub fn upload_bgra(&self, app: &Application, bytes: &[u8]) {
        let expected = (self.texture.width() as usize)
            .checked_mul(self.texture.height() as usize)
            .and_then(|n| n.checked_mul(4))
            .expect("VideoTexture: dimensions overflow usize");
        assert_eq!(
            bytes.len(),
            expected,
            "VideoTexture::upload_bgra: byte length mismatch"
        );

        app.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: self.texture.wgpu_texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.texture.width() * 4),
                rows_per_image: Some(self.texture.height()),
            },
            wgpu::Extent3d {
                width: self.texture.width(),
                height: self.texture.height(),
                depth_or_array_layers: 1,
            },
        );
    }

    /// Borrow the underlying [`Texture`] for sampling.
    #[must_use]
    pub fn texture(&self) -> &Texture {
        &self.texture
    }

    /// Width in pixels.
    #[must_use]
    pub fn width(&self) -> u32 {
        self.texture.width()
    }

    /// Height in pixels.
    #[must_use]
    pub fn height(&self) -> u32 {
        self.texture.height()
    }
}
