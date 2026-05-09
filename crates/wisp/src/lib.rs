//! `wisp` — 2D scene graph + filter chain on `wgpu`.
//!
//! Pixi-shaped public API for compositing recorded screen content. See
//! `_docs/recorder-features-and-render-api.md` for the design surface and
//! `_docs/milestone-0-renderer.md` for the build plan.
//!
//! # Module map
//!
//! - [`application`] — `Application`, `AppConfig`, `Stage` (root context).
//! - [`scene`] — `Container`, `Sprite`, `Graphics`, `Text`, `Mesh`, transforms, clip masks.
//! - [`texture`] — `Texture`, `VideoTexture`, `RenderTexture`.
//! - [`filter`] — `Filter` trait + built-in filters (blur, drop shadow, motion blur, color matrix).
//! - [`render`] — internal renderer, batcher, pipeline cache, filter pass orchestrator.
//! - [`math`] — re-exports from `glam` plus `Rect`.
//! - [`color`] — `Color` (RGBA f32).
//! - [`blend`] — `BlendMode`.

pub mod application;
pub mod blend;
pub mod color;
pub mod filter;
pub mod math;
pub mod render;
pub mod scene;
pub mod texture;

mod error;

pub use blend::BlendMode;
pub use color::Color;
pub use error::Error;
pub use filter::{
    BlurFilter, ColorMatrixFilter, DropShadowFilter, Filter, FilterContext, MotionBlurFilter,
};
pub use math::{Mat3, Mat4, Rect, Vec2, Vec3, Vec4};
pub use scene::{
    Container, Fill, Font, Graphics, Mesh, Node, NodeId, Sprite, Stage, Stroke, Text, Transform,
};
pub use texture::Texture;
pub use texture::render_texture::RenderTexture;
pub use texture::video_texture::VideoTexture;

/// Convenience alias for `Result<T, wisp::Error>`.
pub type Result<T> = core::result::Result<T, Error>;
