//! Cache of generated mask textures (M-DYN.2 / AUT-44).
//!
//! Keyed on the `MaskShape` data + output dimensions + invert flag.
//! `f32` fields are bit-cast to `u32` so equality is exact-bit and
//! `Hash` works (avoids the NaN-aware floating-point quirks of
//! [`std::cmp::Eq`]). Same canonical NaN bits hash identically — fine
//! for our use case where callers re-pass the same shape value across
//! frames.
//!
//! Eviction is FIFO, capped at [`MAX_ENTRIES`]. The cap is a balance
//! between memory (each mask texture is `w × h × 4` bytes of GPU
//! memory) and reuse efficiency. 64 entries × 256² × 4 bytes ≈ 16 MB
//! upper bound — small even on integrated GPUs.
//!
//! Path masks are intentionally **not** cached in V1: hashing a
//! `Vec<glam::Vec2>` is non-trivial and most freehand-mask use cases
//! mutate the polygon between frames anyway. Callers needing path
//! caching should use [`Renderer::generate_path_mask_texture`][gpmt]
//! and manage the cache externally.
//!
//! [gpmt]: crate::render::Renderer::generate_path_mask_texture

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::scene::clip::MaskShape;
use crate::texture::render_texture::RenderTexture;

/// Maximum number of cached mask textures before FIFO eviction.
pub(crate) const MAX_ENTRIES: usize = 64;

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
enum ShapeKey {
    Rect {
        rect_bits: [u32; 4],
    },
    RoundedRect {
        rect_bits: [u32; 4],
        radius_bits: u32,
    },
    Circle {
        center_bits: [u32; 2],
        radius_bits: u32,
    },
    Ellipse {
        center_bits: [u32; 2],
        half_bits: [u32; 2],
    },
}

#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub(crate) struct MaskKey {
    shape: ShapeKey,
    w: u32,
    h: u32,
    invert: bool,
}

impl MaskKey {
    pub(crate) fn new(shape: MaskShape, w: u32, h: u32, invert: bool) -> Self {
        let shape = match shape {
            MaskShape::Rect { rect } => ShapeKey::Rect {
                rect_bits: [
                    rect.min.x.to_bits(),
                    rect.min.y.to_bits(),
                    rect.size.x.to_bits(),
                    rect.size.y.to_bits(),
                ],
            },
            MaskShape::RoundedRect { rect, radius } => ShapeKey::RoundedRect {
                rect_bits: [
                    rect.min.x.to_bits(),
                    rect.min.y.to_bits(),
                    rect.size.x.to_bits(),
                    rect.size.y.to_bits(),
                ],
                radius_bits: radius.to_bits(),
            },
            MaskShape::Circle { center, radius } => ShapeKey::Circle {
                center_bits: [center.x.to_bits(), center.y.to_bits()],
                radius_bits: radius.to_bits(),
            },
            MaskShape::Ellipse {
                center,
                half_extents,
            } => ShapeKey::Ellipse {
                center_bits: [center.x.to_bits(), center.y.to_bits()],
                half_bits: [half_extents.x.to_bits(), half_extents.y.to_bits()],
            },
        };
        Self {
            shape,
            w,
            h,
            invert,
        }
    }
}

/// FIFO-bounded cache of generated mask textures.
pub(crate) struct MaskCache {
    map: HashMap<MaskKey, Arc<RenderTexture>>,
    order: VecDeque<MaskKey>,
    hits: u64,
    misses: u64,
}

impl MaskCache {
    pub(crate) fn new() -> Self {
        Self {
            map: HashMap::new(),
            order: VecDeque::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get the cached texture for `key`, generating it via `generate`
    /// (which is only called on a miss).
    pub(crate) fn get_or_insert<F>(&mut self, key: MaskKey, generate: F) -> Arc<RenderTexture>
    where
        F: FnOnce() -> RenderTexture,
    {
        if let Some(existing) = self.map.get(&key) {
            self.hits += 1;
            return Arc::clone(existing);
        }
        self.misses += 1;

        let rt = Arc::new(generate());
        if self.map.len() >= MAX_ENTRIES
            && let Some(oldest) = self.order.pop_front()
        {
            self.map.remove(&oldest);
        }
        self.map.insert(key, Arc::clone(&rt));
        self.order.push_back(key);
        rt
    }

    pub(crate) fn stats(&self) -> (u64, u64) {
        (self.hits, self.misses)
    }

    pub(crate) fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
    }
}

/// Type alias for the `RefCell`-wrapped cache as stored on
/// [`crate::render::Renderer`].
pub(crate) type MaskCacheCell = RefCell<MaskCache>;
