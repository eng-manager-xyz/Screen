//! Owner-colour palette + deterministic auto-assignment.
//!
//! Default palette is the Wong 8-colour colourblind-friendly set
//! per AUT-180. Auto-assignment hashes the owner's name to a
//! palette index; explicit overrides via `PersonMap` win.

use crate::color::Color;

/// Wong's 8-colour palette, colourblind-friendly.
///
/// Order: blue · vermillion · bluish green · reddish purple ·
/// yellow · sky blue · orange · black.
pub const WONG: &[&str] = &[
    "#0072b2", "#d55e00", "#009e73", "#cc79a7", "#f0e442", "#56b4e9", "#e69f00", "#000000",
];

/// Strategy for assigning a chart-axis colour to an owner.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum OwnerPalette {
    /// Wong's 8-colour palette, hash-by-name.
    #[default]
    Wong,
    /// Hash-by-name against a custom palette. Caller-supplied.
    Custom(Vec<Color>),
    /// Explicit overrides win; remaining owners hash against the
    /// fallback palette.
    AutoWithOverrides(Vec<Color>),
}

impl OwnerPalette {
    /// Resolve `name` → colour. Hashes `name` to an index when
    /// no explicit override is registered.
    #[must_use]
    pub fn color_for(&self, name: &str) -> Color {
        let palette = self.entries();
        // Modulo in u64 space, then `try_from` for the safe cast
        // back to usize. The modulo result is bounded by
        // `palette.len()` which is itself a `usize`, so the
        // conversion always succeeds.
        let n = u64::try_from(palette.len()).expect("palette.len() fits in u64");
        let idx = usize::try_from(hash_name(name) % n).expect("modulo bounded by usize");
        palette[idx]
    }

    /// Return the palette entries as `Color`. For `Wong`, the
    /// pinned Wong palette is hex-decoded once on demand.
    fn entries(&self) -> Vec<Color> {
        match self {
            Self::Wong => WONG.iter().filter_map(|h| Color::from_hex(h)).collect(),
            Self::Custom(v) | Self::AutoWithOverrides(v) => v.clone(),
        }
    }
}

/// Stable per-byte hash. Tiny FNV-1a so two engines (native +
/// wasm) produce identical palette indices for the same name.
///
/// Returns `u64` (not `usize`) so the same name produces the same
/// hash bits on 32-bit targets (wasm32) and 64-bit targets — callers
/// take `% palette.len()` themselves.
fn hash_name(name: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wong_decodes_to_eight_colours() {
        let p = OwnerPalette::Wong;
        assert_eq!(p.entries().len(), 8);
    }

    #[test]
    fn color_for_is_deterministic() {
        let p = OwnerPalette::default();
        assert_eq!(p.color_for("Matt"), p.color_for("Matt"));
        assert_eq!(p.color_for("Alice"), p.color_for("Alice"));
    }

    #[test]
    fn different_names_usually_get_different_colours() {
        // With only 8 colours and a few names, collisions happen
        // but at least two of the canonical fixture names must
        // differ.
        let p = OwnerPalette::default();
        let names = ["Matt", "Alice", "Bob", "Carol"];
        let colours: Vec<_> = names.iter().map(|n| p.color_for(n)).collect();
        let distinct: std::collections::HashSet<_> = colours
            .iter()
            .map(|c| format!("{:.4},{:.4},{:.4}", c.r, c.g, c.b))
            .collect();
        assert!(distinct.len() >= 2, "expected ≥2 distinct colours");
    }

    #[test]
    fn empty_name_does_not_panic() {
        let p = OwnerPalette::default();
        let _ = p.color_for("");
    }
}
