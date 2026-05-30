//! Cursor inspector — cursor styling (ED.19 / M-EDIT).
//!
//! The cursor controls: how big the pointer renders, how much its motion is
//! smoothed, whether clicks throw a ripple, whether it hides while idle, and
//! whether clicks drive auto-zoom (ED.17). All of it edits one
//! [`CursorConfig`] on the project. The composited
//! cursor **overlay** — a smoothed, scaled pointer with click ripples — is a
//! `wisp` layer driven by the captured cursor track, and lands with the
//! render-integration pass (it needs the same per-OS telemetry capture ED.17
//! is waiting on). This chunk is the authoring side, undoable through the
//! shared [`edit::History`].

use edit::EditProject;
use edit::style::CursorConfig;
use leptos::prelude::*;

use crate::style_inspector::parse_u32_field;

type ProjectSignal = Option<RwSignal<Option<EditProject>>>;
type HistoryStore = Option<StoredValue<Option<edit::History>>>;

/// Clamp a cursor size percentage to a sane authoring range (25–400 %).
#[must_use]
pub fn clamp_size_pct(v: u32) -> u32 {
    v.clamp(25, 400)
}

fn commit(project: ProjectSignal, history: HistoryStore, edit: impl FnOnce(&mut CursorConfig)) {
    if let (Some(p), Some(h)) = (project, history) {
        let mut cfg = p.get_untracked().map(|pr| pr.cursor).unwrap_or_default();
        edit(&mut cfg);
        crate::editor_edits::set_cursor(p, h, cfg);
    }
}

fn number_field(
    project: ProjectSignal,
    history: HistoryStore,
    label: &'static str,
    value: u32,
    max: u32,
    set: fn(&mut CursorConfig, u32),
) -> AnyView {
    view! {
        <label class="style-field">
            <span class="style-field-label">{label}</span>
            <input
                class="style-field-input"
                type="number"
                min="0"
                max=max.to_string()
                prop:value=value.to_string()
                on:change=move |ev| {
                    let v = parse_u32_field(&event_target_value(&ev), max);
                    commit(project, history, |c| set(c, v));
                }
            />
        </label>
    }
    .into_any()
}

fn toggle_field(
    project: ProjectSignal,
    history: HistoryStore,
    label: &'static str,
    checked: bool,
    set: fn(&mut CursorConfig, bool),
) -> AnyView {
    view! {
        <label class="cursor-toggle">
            <input
                type="checkbox"
                prop:checked=checked
                on:change=move |ev| {
                    let b = event_target_checked(&ev);
                    commit(project, history, |c| set(c, b));
                }
            />
            <span>{label}</span>
        </label>
    }
    .into_any()
}

/// The cursor inspector: size / smoothing + ripples / hide-static / auto-zoom.
#[component]
pub fn CursorInspector() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let history = use_context::<StoredValue<Option<edit::History>>>();
    view! {
        <div class="cursor-inspector">
            <div class="clip-inspector-section">
                <h3 class="clip-inspector-title">"Cursor"</h3>
                {move || {
                    let c = project.and_then(|s| s.get().map(|p| p.cursor)).unwrap_or_default();
                    view! {
                        <div class="style-fields">
                            {number_field(project, history, "Size %", c.size_pct, 400, |c, v| c.size_pct = clamp_size_pct(v))}
                            {number_field(project, history, "Smooth", c.smoothing, 100, |c, v| c.smoothing = v)}
                        </div>
                        {toggle_field(project, history, "Click ripples", c.click_ripples, |c, b| c.click_ripples = b)}
                        {toggle_field(project, history, "Hide when static", c.hide_static, |c, b| c.hide_static = b)}
                        {toggle_field(
                            project,
                            history,
                            "Auto-zoom on clicks",
                            c.auto_zoom.detect_from_cursor,
                            |c, b| c.auto_zoom.detect_from_cursor = b,
                        )}
                    }
                }}
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_pct_clamps_to_authoring_range() {
        assert_eq!(clamp_size_pct(180), 180);
        assert_eq!(clamp_size_pct(0), 25); // floor
        assert_eq!(clamp_size_pct(9999), 400); // ceiling
    }
}
