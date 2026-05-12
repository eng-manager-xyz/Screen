//! Recording-footer select pills + shortcut badge group (M-UI.11 /
//! AUT-131).
//!
//! `AutoZoomSelect` and `CountdownSelect` are thin wrappers around
//! UI-04 `SelectPill` that bake in a leading glyph + the expected
//! label format so the parent (and the `TrayRecordPopover` in UI-12)
//! doesn't have to redeclare them.
//!
//! `ShortcutBadgeGroup` renders the keyboard hint inside the red
//! Start button — dark chips on a red field. It's distinct from
//! UI-01 `Kbd`: the visual treatment is inverted (light text on the
//! action color, not the neutral surface treatment `Kbd` uses).

use leptos::prelude::*;

use crate::components::primitives::SelectPill;

/// Compact pill that opens the auto-zoom strength menu. Visual only —
/// the popover content is owned by the parent.
#[component]
pub fn AutoZoomSelect(
    /// Pre-formatted label shown inside the pill. Examples:
    /// `"Auto-zoom 2.0×"`, `"Auto-zoom off"`. The footer fixture builds
    /// this via `format_auto_zoom_label`.
    #[prop(into)]
    label: String,
    /// `true` when the parent's popover is open.
    #[prop(optional)]
    open: bool,
    /// `true` to render disabled.
    #[prop(optional)]
    disabled: bool,
) -> impl IntoView {
    view! {
        <SelectPill icon="⊙".to_string() label=label open=open disabled=disabled />
    }
}

/// Compact pill that opens the countdown-length menu.
#[component]
pub fn CountdownSelect(
    /// Pre-formatted label. Examples: `"3s Countdown"`, `"No countdown"`.
    /// Build via `format_countdown_label`.
    #[prop(into)]
    label: String,
    #[prop(optional)] open: bool,
    #[prop(optional)] disabled: bool,
) -> impl IntoView {
    view! {
        <SelectPill icon="⏱".to_string() label=label open=open disabled=disabled />
    }
}

/// Group of small keyboard-shortcut badges (light-on-red, used inside
/// the Start button).
#[component]
pub fn ShortcutBadgeGroup(
    /// Keys to render in display order.
    keys: Vec<String>,
    /// `true` when sitting on a colored action surface (red Start
    /// button). Drives the inverted color treatment.
    #[prop(optional)]
    on_action: bool,
) -> impl IntoView {
    let class = if on_action {
        "shortcut-badges shortcut-badges-on-action"
    } else {
        "shortcut-badges"
    };
    view! {
        <span class=class aria-hidden="true">
            {keys.into_iter()
                .map(|k| view! { <span class="shortcut-badge">{k}</span> })
                .collect_view()}
        </span>
    }
}

/// Format `"Auto-zoom 2.0×"` / `"Auto-zoom off"`. `None` → off.
#[must_use]
pub fn format_auto_zoom_label(zoom: Option<f32>) -> String {
    match zoom {
        None => "Auto-zoom off".to_owned(),
        Some(z) => format!("Auto-zoom {z:.1}×"),
    }
}

/// Format `"3s Countdown"` / `"No countdown"`. `0` → off.
#[must_use]
pub fn format_countdown_label(seconds: u8) -> String {
    if seconds == 0 {
        "No countdown".to_owned()
    } else {
        format!("{seconds}s Countdown")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_zoom_label_formats_one_decimal() {
        assert_eq!(format_auto_zoom_label(None), "Auto-zoom off");
        assert_eq!(format_auto_zoom_label(Some(1.0)), "Auto-zoom 1.0×");
        assert_eq!(format_auto_zoom_label(Some(2.0)), "Auto-zoom 2.0×");
        assert_eq!(format_auto_zoom_label(Some(2.5)), "Auto-zoom 2.5×");
    }

    #[test]
    fn countdown_label_zero_is_off() {
        assert_eq!(format_countdown_label(0), "No countdown");
        assert_eq!(format_countdown_label(3), "3s Countdown");
        assert_eq!(format_countdown_label(10), "10s Countdown");
    }
}
