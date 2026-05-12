//! `Meter` — audio-level bars (M-UI.4 / AUT-124).
//!
//! Renders `bar_count` segments, lit from the left up to the integer
//! count corresponding to `level` (clamped to `[0, 1]`). Pure
//! presentation; the parent feeds in the current normalized level.

use leptos::prelude::*;

#[component]
pub fn Meter(
    /// Normalized level in `[0, 1]`. Out-of-range values are clamped.
    level: f32,
    /// Number of segments to render.
    #[prop(optional, default = 12)]
    bar_count: u8,
    /// `true` to render in the danger color (clipping / over).
    #[prop(optional)]
    danger: bool,
) -> impl IntoView {
    let lit = lit_segments(level, bar_count);
    let mut class = String::from("meter");
    if danger {
        class.push_str(" meter-danger");
    }
    view! {
        <div class=class role="meter" aria-valuemin="0" aria-valuemax="1" aria-valuenow=level.clamp(0.0, 1.0)>
            {(0..bar_count).map(|i| {
                let on = i < lit;
                let bar_class = if on { "meter-bar meter-bar-on" } else { "meter-bar" };
                view! { <span class=bar_class></span> }
            }).collect_view()}
        </div>
    }
}

/// How many of `bar_count` segments to light at `level`.
#[must_use]
pub fn lit_segments(level: f32, bar_count: u8) -> u8 {
    let l = level.clamp(0.0, 1.0);
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "bar_count <= 255 fits f32 + result is bound [0, bar_count]"
    )]
    let lit = (l * f32::from(bar_count)).round() as u8;
    lit.min(bar_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_level_lights_no_bars() {
        assert_eq!(lit_segments(0.0, 12), 0);
        assert_eq!(lit_segments(-0.5, 12), 0);
    }

    #[test]
    fn full_level_lights_all_bars() {
        assert_eq!(lit_segments(1.0, 12), 12);
        assert_eq!(lit_segments(1.5, 12), 12);
    }

    #[test]
    fn half_level_lights_half() {
        assert_eq!(lit_segments(0.5, 12), 6);
    }

    #[test]
    fn rounds_to_nearest() {
        // 0.55 * 10 = 5.5 → 6
        assert_eq!(lit_segments(0.55, 10), 6);
        // 0.54 * 10 = 5.4 → 5
        assert_eq!(lit_segments(0.54, 10), 5);
    }
}
