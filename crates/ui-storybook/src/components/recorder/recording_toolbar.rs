//! `RecordingToolbar` — the top-of-app surface during capture.
//!
//! Three states drive everything: `Idle` (big primary "Record" button,
//! source picker placeholder), `Recording` (red pulsing indicator, timer,
//! Pause + Stop), `Paused` (paused badge, Resume + Stop). The timer is a
//! prop the parent ticks; the component is purely presentational.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RecordingState {
    #[default]
    Idle,
    Recording,
    Paused,
}

impl RecordingState {
    fn css(self) -> &'static str {
        match self {
            RecordingState::Idle => "rec-idle",
            RecordingState::Recording => "rec-recording",
            RecordingState::Paused => "rec-paused",
        }
    }

    fn status_label(self) -> &'static str {
        match self {
            RecordingState::Idle => "Ready",
            RecordingState::Recording => "Recording",
            RecordingState::Paused => "Paused",
        }
    }
}

#[component]
pub fn RecordingToolbar(
    /// Current capture state.
    #[prop(optional)]
    state: RecordingState,
    /// Elapsed recording time in seconds.
    #[prop(optional)]
    elapsed_seconds: f32,
    /// Display name of the current capture source (e.g. "Built-in Display").
    #[prop(optional, into)]
    source: String,
) -> impl IntoView {
    let class = format!("recording-toolbar {}", state.css());
    let status = state.status_label();
    let timer = format_clock(elapsed_seconds.max(0.0));
    let source_label = if source.is_empty() {
        "Pick a source".to_string()
    } else {
        source
    };

    let actions = match state {
        RecordingState::Idle => view! {
            <button class="rec-action rec-action-primary" type="button">
                <span class="rec-dot rec-dot-static"></span>
                <span>"Start recording"</span>
            </button>
        }
        .into_any(),
        RecordingState::Recording => view! {
            <button class="rec-action rec-action-secondary" type="button">"Pause"</button>
            <button class="rec-action rec-action-stop" type="button">"Stop"</button>
        }
        .into_any(),
        RecordingState::Paused => view! {
            <button class="rec-action rec-action-primary" type="button">
                <span>"Resume"</span>
            </button>
            <button class="rec-action rec-action-stop" type="button">"Stop"</button>
        }
        .into_any(),
    };

    view! {
        <div class=class role="toolbar" aria-label="Recording controls">
            <div class="rec-status">
                <span class="rec-dot"></span>
                <span class="rec-status-label">{status}</span>
            </div>
            <div class="rec-timer">{timer}</div>
            <div class="rec-source">
                <span class="rec-source-glyph">"⌬"</span>
                <span class="rec-source-label">{source_label}</span>
            </div>
            <div class="rec-actions">{actions}</div>
        </div>
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "elapsed is positive and small, fits in u32"
)]
fn format_clock(seconds: f32) -> String {
    let total = seconds.round() as u32;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}
