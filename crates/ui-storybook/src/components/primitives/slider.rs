//! `Slider` — value slider (M-UI.4 / AUT-124).
//!
//! Visual only — renders a track, fill, thumb, and an optional label /
//! readout. No drag handling. The parent component owns the value and
//! re-renders with the new value when its input handler fires.

use leptos::prelude::*;

#[component]
pub fn Slider(
    /// Current value. Clamped into `[min, max]` for the visual percent.
    value: f32,
    /// Lower bound. Default 0.0.
    #[prop(optional, default = 0.0)]
    min: f32,
    /// Upper bound. Default 1.0.
    #[prop(optional, default = 1.0)]
    max: f32,
    /// `true` to render disabled.
    #[prop(optional)]
    disabled: bool,
    /// Optional leading label.
    #[prop(optional, into)]
    label: Option<String>,
    /// Optional trailing readout string (e.g. `"42 px"`).
    #[prop(optional, into)]
    readout: Option<String>,
) -> impl IntoView {
    let percent = slider_percent(value, min, max);
    let mut class = String::from("slider");
    if disabled {
        class.push_str(" slider-disabled");
    }
    let fill_style = format!("width:{percent:.2}%");
    let thumb_style = format!("left:{percent:.2}%");
    view! {
        <div class=class>
            {label.map(|l| view! { <span class="slider-label">{l}</span> })}
            <div class="slider-track">
                <div class="slider-fill" style=fill_style></div>
                <div class="slider-thumb" style=thumb_style></div>
            </div>
            {readout.map(|r| view! { <span class="slider-readout">{r}</span> })}
        </div>
    }
}

/// Project `value` into `[0, 100]` against `[min, max]`. Clamps so
/// out-of-range values pin to the endpoints; if `min == max` the value
/// projects to `0%` to avoid divide-by-zero.
#[must_use]
pub fn slider_percent(value: f32, min: f32, max: f32) -> f32 {
    if max <= min {
        return 0.0;
    }
    ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_zero_at_min() {
        assert!((slider_percent(0.0, 0.0, 1.0) - 0.0).abs() < 1e-4);
        assert!((slider_percent(-5.0, 0.0, 1.0) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn percent_one_hundred_at_max() {
        assert!((slider_percent(1.0, 0.0, 1.0) - 100.0).abs() < 1e-4);
        assert!((slider_percent(2.0, 0.0, 1.0) - 100.0).abs() < 1e-4);
    }

    #[test]
    fn percent_midway() {
        assert!((slider_percent(0.5, 0.0, 1.0) - 50.0).abs() < 1e-4);
        assert!((slider_percent(5.0, 0.0, 10.0) - 50.0).abs() < 1e-4);
        assert!((slider_percent(15.0, 10.0, 20.0) - 50.0).abs() < 1e-4);
    }

    #[test]
    fn zero_range_returns_zero() {
        assert!((slider_percent(1.0, 5.0, 5.0) - 0.0).abs() < 1e-4);
    }
}
