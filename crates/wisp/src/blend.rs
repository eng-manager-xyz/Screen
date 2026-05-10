//! Blend modes for compositing nodes.
//!
//! Two architectural buckets, mirroring `PixiJS` v8:
//!
//! - **Standard** modes are GPU-native — implemented via `wgpu::BlendState`
//!   per pipeline. A single render pass writes the final pixel; no
//!   intermediate texture is needed. Cheap.
//! - **Advanced** modes can't be expressed as a GPU blend equation
//!   because they need to *read* the backdrop and apply non-linear math
//!   (e.g. `overlay = base < 0.5 ? 2·base·blend : …`). Implemented as
//!   compositing filters via [`crate::render::Renderer::apply_advanced_blend`]:
//!   render the foreground into one render-texture, sample both
//!   backdrop and foreground in a fragment shader, write to a third
//!   render-texture. One offscreen pass per advanced-blended node.
//!
//! Setting `container.blend_mode = BlendMode::Overlay` on a node and
//! then rendering through `render_stage` doesn't *automatically*
//! trigger the offscreen path — `render_stage` falls back to the
//! standard pipeline for advanced modes and tracing-warns once. The
//! manual `Renderer::apply_advanced_blend` API is the way to get
//! correct advanced-blend results today; automatic dispatch is queued
//! for a follow-up chunk.

/// Blend mode for compositing a renderable onto its target.
///
/// Mirrors the `PixiJS` v8 catalog. See the [module docs](self) for the
/// standard-vs-advanced distinction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    // ─── Standard (GPU-native via wgpu::BlendState) ───────────────────
    /// Source-over alpha blending. `src.a * src + (1 - src.a) * dst`.
    #[default]
    Normal,
    /// `dst * src + (1 - src.a) * dst` — darkens.
    Multiply,
    /// `src + (1 - src.a) * dst` — additive.
    Add,
    /// `1 - (1 - src) * (1 - dst)` — softer additive.
    Screen,
    /// `dst - src` clamped to `[0, 1]`.
    Subtract,
    /// `min(src, dst)` per channel.
    Min,
    /// `max(src, dst)` per channel.
    Max,
    /// `dst * (1 - src.a)` — erases dst proportional to src alpha.
    Erase,

    // ─── Advanced (offscreen pass + custom shader) ────────────────────
    /// `(base < 0.5) ? 2·base·blend : 1 - 2·(1-base)·(1-blend)`.
    Overlay,
    /// Overlay with the roles swapped — `(blend < 0.5) ? …`.
    HardLight,
    /// Photoshop's W3C-defined soft-light formula.
    SoftLight,
    /// Pixel-wise pin: `(blend < 0.5) ? min(base, 2·blend) : max(base, 2·blend - 1)`.
    PinLight,
    /// `VividLight` thresholded to `{0, 1}` per channel.
    HardMix,
    /// `(blend < 0.5) ? color-burn(base, 2·blend) : color-dodge(base, 2·(blend - 0.5))`.
    VividLight,
    /// `base + 2·blend - 1` clamped.
    LinearLight,
    /// `(blend == 0) ? 0 : 1 - min(1, (1-base) / blend)`.
    ColorBurn,
    /// `(blend == 1) ? 1 : min(1, base / (1-blend))`.
    ColorDodge,
    /// `base + blend - 1` clamped.
    LinearBurn,
    /// `base + blend` clamped (a.k.a. linear-dodge / additive).
    LinearDodge,
    /// `min(base, blend)`.
    Darken,
    /// `max(base, blend)`.
    Lighten,
    /// `abs(base - blend)`.
    Difference,
    /// `base + blend - 2·base·blend`.
    Exclusion,
    /// `1 - abs(1 - base - blend)`.
    Negation,
    /// `base / blend`. Not GPU-native (needs read-modify-write on dst).
    Divide,
    /// HSL: keep saturation of foreground, hue+lum of backdrop.
    Saturation,
    /// HSL: keep hue+saturation of foreground, lum of backdrop.
    Color,
    /// HSL: keep lum of foreground, hue+saturation of backdrop.
    Luminosity,
}

impl BlendMode {
    /// Whether this mode requires the offscreen filter path (Tier C).
    /// Modes that *don't* require it return `Some(BlendState)` from
    /// [`Self::native_blend_state`].
    #[must_use]
    pub const fn is_advanced(self) -> bool {
        self.native_blend_state().is_none()
    }

    /// The wgpu blend state for this mode, if it can be expressed as a
    /// single GPU blend equation. Returns `None` for advanced modes.
    #[must_use]
    pub const fn native_blend_state(self) -> Option<wgpu::BlendState> {
        use wgpu::{BlendComponent, BlendFactor as F, BlendOperation as Op, BlendState};
        Some(match self {
            BlendMode::Normal => BlendState::ALPHA_BLENDING,
            BlendMode::Multiply => BlendState {
                color: BlendComponent {
                    src_factor: F::Dst,
                    dst_factor: F::OneMinusSrcAlpha,
                    operation: Op::Add,
                },
                alpha: BlendComponent::OVER,
            },
            BlendMode::Add => BlendState {
                color: BlendComponent {
                    src_factor: F::SrcAlpha,
                    dst_factor: F::One,
                    operation: Op::Add,
                },
                alpha: BlendComponent::OVER,
            },
            BlendMode::Screen => BlendState {
                color: BlendComponent {
                    src_factor: F::OneMinusDst,
                    dst_factor: F::One,
                    operation: Op::Add,
                },
                alpha: BlendComponent::OVER,
            },
            BlendMode::Subtract => BlendState {
                color: BlendComponent {
                    src_factor: F::SrcAlpha,
                    dst_factor: F::One,
                    operation: Op::ReverseSubtract,
                },
                alpha: BlendComponent::OVER,
            },
            BlendMode::Min => BlendState {
                color: BlendComponent {
                    src_factor: F::One,
                    dst_factor: F::One,
                    operation: Op::Min,
                },
                alpha: BlendComponent::OVER,
            },
            BlendMode::Max => BlendState {
                color: BlendComponent {
                    src_factor: F::One,
                    dst_factor: F::One,
                    operation: Op::Max,
                },
                alpha: BlendComponent::OVER,
            },
            BlendMode::Erase => BlendState {
                color: BlendComponent {
                    src_factor: F::Zero,
                    dst_factor: F::OneMinusSrcAlpha,
                    operation: Op::Add,
                },
                alpha: BlendComponent {
                    src_factor: F::Zero,
                    dst_factor: F::OneMinusSrcAlpha,
                    operation: Op::Add,
                },
            },
            // Advanced modes (Tier C) — no GPU blend equation captures them.
            BlendMode::Overlay
            | BlendMode::HardLight
            | BlendMode::SoftLight
            | BlendMode::PinLight
            | BlendMode::HardMix
            | BlendMode::VividLight
            | BlendMode::LinearLight
            | BlendMode::ColorBurn
            | BlendMode::ColorDodge
            | BlendMode::LinearBurn
            | BlendMode::LinearDodge
            | BlendMode::Darken
            | BlendMode::Lighten
            | BlendMode::Difference
            | BlendMode::Exclusion
            | BlendMode::Negation
            | BlendMode::Divide
            | BlendMode::Saturation
            | BlendMode::Color
            | BlendMode::Luminosity => return None,
        })
    }

    /// CSS-style kebab-case identifier — matches `PixiJS` v8's
    /// `Container.blendMode` string values.
    #[must_use]
    pub const fn css_name(self) -> &'static str {
        match self {
            BlendMode::Normal => "normal",
            BlendMode::Multiply => "multiply",
            BlendMode::Add => "add",
            BlendMode::Screen => "screen",
            BlendMode::Subtract => "subtract",
            BlendMode::Min => "min",
            BlendMode::Max => "max",
            BlendMode::Erase => "erase",
            BlendMode::Overlay => "overlay",
            BlendMode::HardLight => "hard-light",
            BlendMode::SoftLight => "soft-light",
            BlendMode::PinLight => "pin-light",
            BlendMode::HardMix => "hard-mix",
            BlendMode::VividLight => "vivid-light",
            BlendMode::LinearLight => "linear-light",
            BlendMode::ColorBurn => "color-burn",
            BlendMode::ColorDodge => "color-dodge",
            BlendMode::LinearBurn => "linear-burn",
            BlendMode::LinearDodge => "linear-dodge",
            BlendMode::Darken => "darken",
            BlendMode::Lighten => "lighten",
            BlendMode::Difference => "difference",
            BlendMode::Exclusion => "exclusion",
            BlendMode::Negation => "negation",
            BlendMode::Divide => "divide",
            BlendMode::Saturation => "saturation",
            BlendMode::Color => "color",
            BlendMode::Luminosity => "luminosity",
        }
    }

    /// Iterate every variant. Useful for table-driven tests + the
    /// blend-modes storybook gallery.
    #[must_use]
    pub fn all() -> impl ExactSizeIterator<Item = BlendMode> + DoubleEndedIterator {
        const ALL: [BlendMode; 28] = [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Add,
            BlendMode::Screen,
            BlendMode::Subtract,
            BlendMode::Min,
            BlendMode::Max,
            BlendMode::Erase,
            BlendMode::Overlay,
            BlendMode::HardLight,
            BlendMode::SoftLight,
            BlendMode::PinLight,
            BlendMode::HardMix,
            BlendMode::VividLight,
            BlendMode::LinearLight,
            BlendMode::ColorBurn,
            BlendMode::ColorDodge,
            BlendMode::LinearBurn,
            BlendMode::LinearDodge,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Negation,
            BlendMode::Divide,
            BlendMode::Saturation,
            BlendMode::Color,
            BlendMode::Luminosity,
        ];
        ALL.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_normal() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }

    #[test]
    fn standard_set_is_native() {
        for m in [
            BlendMode::Normal,
            BlendMode::Multiply,
            BlendMode::Add,
            BlendMode::Screen,
            BlendMode::Subtract,
            BlendMode::Min,
            BlendMode::Max,
            BlendMode::Erase,
        ] {
            assert!(m.native_blend_state().is_some(), "{m:?} should be native");
            assert!(!m.is_advanced(), "{m:?} should not be advanced");
        }
    }

    #[test]
    fn advanced_set_is_not_native() {
        for m in [
            BlendMode::Overlay,
            BlendMode::HardLight,
            BlendMode::SoftLight,
            BlendMode::PinLight,
            BlendMode::HardMix,
            BlendMode::VividLight,
            BlendMode::LinearLight,
            BlendMode::ColorBurn,
            BlendMode::ColorDodge,
            BlendMode::LinearBurn,
            BlendMode::LinearDodge,
            BlendMode::Darken,
            BlendMode::Lighten,
            BlendMode::Difference,
            BlendMode::Exclusion,
            BlendMode::Negation,
            BlendMode::Divide,
            BlendMode::Saturation,
            BlendMode::Color,
            BlendMode::Luminosity,
        ] {
            assert!(m.native_blend_state().is_none(), "{m:?} should be advanced");
            assert!(m.is_advanced(), "{m:?} should be advanced");
        }
    }

    #[test]
    fn css_names_are_unique_and_kebab() {
        let mut seen = std::collections::HashSet::new();
        for m in BlendMode::all() {
            let n = m.css_name();
            assert!(seen.insert(n), "duplicate css_name {n}");
            assert!(
                n.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "css_name {n} not kebab-case"
            );
        }
    }

    #[test]
    fn all_iterator_covers_every_variant() {
        let count = BlendMode::all().count();
        assert_eq!(count, 28, "BlendMode::all() must enumerate all variants");
    }
}
