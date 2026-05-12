//! `RecordingControlsFooter` (M-UI.11 / AUT-131) — final tray footer
//! before the first recording starts. Two compact select pills
//! (auto-zoom + countdown) sit on the leading side; the prominent
//! red Start recording button sits on the trailing side with the
//! keyboard shortcut hint inside.
//!
//! Stateless: the parent owns the popover open state, the underlying
//! zoom/countdown values, and the recording lifecycle. The footer
//! exposes optional `Callback<()>` props so the storybook stories
//! can leave them unset.

use leptos::prelude::*;

use crate::components::recorder::recording_selects::{
    AutoZoomSelect, CountdownSelect, ShortcutBadgeGroup,
};

/// Lifecycle state of the Start recording button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StartRecordingState {
    /// Idle, ready to fire — primary red button, shortcut visible.
    #[default]
    Ready,
    /// Disabled — no capture sources selected yet, etc. Dimmed.
    Disabled,
    /// Press dispatched, waiting for the capture pipeline to spin up.
    /// Spinner glyph, button stays red but not clickable.
    Loading,
    /// Recording requires a permission the user hasn't granted yet.
    /// Renders as the warning variant with a permission CTA.
    PermissionBlocked,
}

impl StartRecordingState {
    /// Stable kebab-case slug used for `data-state` selectors + tests.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            StartRecordingState::Ready => "ready",
            StartRecordingState::Disabled => "disabled",
            StartRecordingState::Loading => "loading",
            StartRecordingState::PermissionBlocked => "permission-blocked",
        }
    }

    /// `true` when the button is non-interactive.
    #[must_use]
    pub fn is_disabled(self) -> bool {
        matches!(
            self,
            StartRecordingState::Disabled | StartRecordingState::Loading
        )
    }
}

/// View-model for the footer.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordingControlsView {
    /// Pre-formatted auto-zoom label (`"Auto-zoom 2.0×"`).
    pub auto_zoom_label: String,
    /// Pre-formatted countdown label (`"3s Countdown"`).
    pub countdown_label: String,
    /// Keyboard shortcut shown inside the Start button (`["⌘", "⇧", "2"]`).
    /// Render order is preserved.
    pub shortcuts: Vec<String>,
    /// Lifecycle of the Start recording button.
    pub start_state: StartRecordingState,
}

/// Reusable Start recording button. The full footer wraps this, but
/// the `TrayRecordPopover` and `RecordingStatusButton` (UI-12 / UI-13)
/// also reach for it directly.
#[component]
pub fn StartRecordingButton(
    /// Lifecycle state.
    #[prop(optional)]
    state: StartRecordingState,
    /// Optional keyboard shortcut to render inside the button.
    #[prop(optional)]
    shortcuts: Vec<String>,
    /// Click handler. Optional — stories leave it unset; the recorder
    /// shell wires it to the Tauri `start_recording` command.
    #[prop(optional)]
    on_start: Option<Callback<()>>,
) -> impl IntoView {
    let class = format!("start-recording-btn start-recording-{}", state.slug());
    let disabled = state.is_disabled();
    let label = match state {
        StartRecordingState::PermissionBlocked => "Grant permissions",
        StartRecordingState::Loading => "Starting…",
        _ => "Start recording",
    };
    let glyph = match state {
        StartRecordingState::PermissionBlocked => "⚠",
        StartRecordingState::Loading => "◌",
        _ => "●",
    };
    let click = move |_| {
        if let Some(cb) = on_start {
            cb.run(());
        }
    };
    let show_shortcuts = matches!(state, StartRecordingState::Ready) && !shortcuts.is_empty();
    view! {
        <button
            class=class
            type="button"
            aria-label=label
            data-state=state.slug()
            disabled=disabled
            on:click=click
        >
            <span class="start-recording-glyph" aria-hidden="true">{glyph}</span>
            <span class="start-recording-label">{label}</span>
            {show_shortcuts.then(|| view! {
                <ShortcutBadgeGroup keys=shortcuts on_action=true />
            })}
        </button>
    }
}

/// The recording-controls footer. Composes `AutoZoomSelect` +
/// `CountdownSelect` + `StartRecordingButton`.
#[component]
pub fn RecordingControlsFooter(
    /// View-model. The parent rebuilds this on every relevant signal
    /// change.
    view: RecordingControlsView,
    /// Open handler for the auto-zoom select.
    #[prop(optional, into)]
    on_auto_zoom_open: Option<Callback<()>>,
    /// Open handler for the countdown select.
    #[prop(optional, into)]
    on_countdown_open: Option<Callback<()>>,
    /// Click handler for the Start button. Forwarded to
    /// `StartRecordingButton`'s own `on_start`.
    #[prop(optional, into)]
    on_start: Option<Callback<()>>,
) -> impl IntoView {
    let zoom_label = view.auto_zoom_label.clone();
    let countdown_label = view.countdown_label.clone();
    let shortcuts = view.shortcuts.clone();
    let start_state = view.start_state;
    let zoom_click = move |_| {
        if let Some(cb) = on_auto_zoom_open {
            cb.run(());
        }
    };
    let countdown_click = move |_| {
        if let Some(cb) = on_countdown_open {
            cb.run(());
        }
    };
    let button = match on_start {
        Some(cb) => view! {
            <StartRecordingButton state=start_state shortcuts=shortcuts.clone() on_start=cb />
        }
        .into_any(),
        None => view! {
            <StartRecordingButton state=start_state shortcuts=shortcuts.clone() />
        }
        .into_any(),
    };
    view! {
        <div class="recording-controls-footer" data-state=start_state.slug()>
            <div class="recording-controls-selects">
                <span class="recording-controls-zoom" on:click=zoom_click>
                    <AutoZoomSelect label=zoom_label />
                </span>
                <span class="recording-controls-countdown" on:click=countdown_click>
                    <CountdownSelect label=countdown_label />
                </span>
            </div>
            <div class="recording-controls-action">{button}</div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_unique_and_kebab() {
        let slugs = [
            StartRecordingState::Ready.slug(),
            StartRecordingState::Disabled.slug(),
            StartRecordingState::Loading.slug(),
            StartRecordingState::PermissionBlocked.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
        for s in slugs {
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }

    #[test]
    fn loading_and_disabled_are_non_interactive() {
        assert!(!StartRecordingState::Ready.is_disabled());
        assert!(StartRecordingState::Disabled.is_disabled());
        assert!(StartRecordingState::Loading.is_disabled());
        // Permission-blocked is interactive — the click opens the
        // permission prompt; we want the parent to be able to react.
        assert!(!StartRecordingState::PermissionBlocked.is_disabled());
    }
}
