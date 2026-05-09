//! Blend modes for compositing nodes.

/// Blend mode for compositing a renderable onto its target.
///
/// Only [`BlendMode::Normal`] is wired up in M0.3; other variants are
/// declared so the public API doesn't churn when they're enabled by the
/// renderer pipeline (M0.5 onwards).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BlendMode {
    /// Standard alpha blending: `src.a * src + (1 - src.a) * dst`.
    #[default]
    Normal,
    /// `src * dst` — darkens.
    Multiply,
    /// `src + dst` (clamped to 1.0) — lightens.
    Add,
    /// `1 - (1 - src) * (1 - dst)` — softer additive.
    Screen,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_normal() {
        assert_eq!(BlendMode::default(), BlendMode::Normal);
    }
}
