//! `RecordingStatusButton` + `CountdownBadge` (M-UI.13 / AUT-133) —
//! compact tray/menu-bar status pill that replaces the
//! `StartRecordingButton` after the user starts recording.
//!
//! Stateless: the parent passes a [`CompactRecordingState`] each
//! frame. No internal timers, no tick-driven labels — countdown
//! seconds and elapsed-labels are computed upstream so the component
//! stays SSR-stable.

use leptos::prelude::*;

/// Lifecycle state of the compact recording pill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompactRecordingState {
    /// Pre-recording countdown — parent ticks down `seconds_remaining`.
    Countdown {
        /// Seconds until capture begins.
        seconds_remaining: u8,
    },
    /// Active recording. The parent supplies the formatted
    /// elapsed-label (`"00:03"`, `"01:42:18"`).
    Recording {
        /// Pre-formatted clock label.
        elapsed_label: String,
    },
    /// Paused — the elapsed label freezes.
    Paused {
        /// Pre-formatted clock label.
        elapsed_label: String,
    },
    /// Stop dispatched, waiting for the encoder to finalize.
    Stopping,
    /// Encoder finished, recording is saved.
    Stopped,
    /// Recording aborted with a user-visible message.
    Error {
        /// Short message displayed to the user.
        message: String,
    },
}

impl CompactRecordingState {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(&self) -> &'static str {
        match self {
            CompactRecordingState::Countdown { .. } => "countdown",
            CompactRecordingState::Recording { .. } => "recording",
            CompactRecordingState::Paused { .. } => "paused",
            CompactRecordingState::Stopping => "stopping",
            CompactRecordingState::Stopped => "stopped",
            CompactRecordingState::Error { .. } => "error",
        }
    }
}

/// Format the countdown label — `3`, `1`, `0`. Returned as a
/// `&'static str` for the eight bounded values; falls back to a
/// leaked string for larger seconds (rare, since real countdowns are
/// 0–10s).
#[must_use]
pub fn format_countdown_seconds(s: u8) -> String {
    s.to_string()
}

/// Numeric badge shown in the countdown state. Styled separately
/// (large monospace digit on a red field).
#[component]
pub fn CountdownBadge(
    /// Seconds remaining. The component renders the value verbatim.
    seconds: u8,
) -> impl IntoView {
    let label = format_countdown_seconds(seconds);
    view! {
        <span class="countdown-badge" role="timer" aria-label="Recording starts in {seconds} seconds">
            <span class="countdown-badge-digit">{label}</span>
        </span>
    }
}

/// Compact recording status pill — countdown / live / paused /
/// stopping / stopped / error. Designed for the system tray, the
/// menu-bar overlay, and any other small surface that needs the
/// status without the full recorder footer.
#[component]
pub fn RecordingStatusButton(
    /// Current lifecycle state.
    state: CompactRecordingState,
    /// Keyboard shortcut to display inside the pill. Optional —
    /// stop-only flows can leave this empty.
    #[prop(optional)]
    shortcuts: Vec<String>,
    /// Pause click handler.
    #[prop(optional)]
    on_pause: Option<Callback<()>>,
    /// Resume click handler (used when state is Paused).
    #[prop(optional)]
    on_resume: Option<Callback<()>>,
    /// Stop click handler.
    #[prop(optional)]
    on_stop: Option<Callback<()>>,
) -> impl IntoView {
    let class = format!("recording-status-btn recording-status-{}", state.slug());
    let inner = render_inner(state, shortcuts, on_pause, on_resume, on_stop);
    view! {
        <div class=class role="group" aria-label="Recording status">
            {inner}
        </div>
    }
}

fn render_inner(
    state: CompactRecordingState,
    shortcuts: Vec<String>,
    on_pause: Option<Callback<()>>,
    on_resume: Option<Callback<()>>,
    on_stop: Option<Callback<()>>,
) -> AnyView {
    let pause_click = move |_| {
        if let Some(cb) = on_pause {
            cb.run(());
        }
    };
    let resume_click = move |_| {
        if let Some(cb) = on_resume {
            cb.run(());
        }
    };
    let stop_click = move |_| {
        if let Some(cb) = on_stop {
            cb.run(());
        }
    };
    match state {
        CompactRecordingState::Countdown { seconds_remaining } => view! {
            <>
                <CountdownBadge seconds=seconds_remaining />
                <span class="recording-status-text">"Starting…"</span>
            </>
        }
        .into_any(),
        CompactRecordingState::Recording { elapsed_label } => view! {
            <>
                <span class="recording-status-dot recording-status-dot-live" aria-hidden="true"></span>
                <span class="recording-status-text">{elapsed_label}</span>
                {render_shortcuts(&shortcuts)}
                <button class="recording-status-action" type="button" on:click=pause_click aria-label="Pause">"⏸"</button>
                <button class="recording-status-action recording-status-action-stop" type="button" on:click=stop_click aria-label="Stop">"■"</button>
            </>
        }
        .into_any(),
        CompactRecordingState::Paused { elapsed_label } => view! {
            <>
                <span class="recording-status-dot recording-status-dot-paused" aria-hidden="true"></span>
                <span class="recording-status-text">{elapsed_label} " · Paused"</span>
                <button class="recording-status-action" type="button" on:click=resume_click aria-label="Resume">"▶"</button>
                <button class="recording-status-action recording-status-action-stop" type="button" on:click=stop_click aria-label="Stop">"■"</button>
            </>
        }
        .into_any(),
        CompactRecordingState::Stopping => view! {
            <>
                <span class="recording-status-glyph">"◌"</span>
                <span class="recording-status-text">"Saving…"</span>
            </>
        }
        .into_any(),
        CompactRecordingState::Stopped => view! {
            <>
                <span class="recording-status-glyph">"✓"</span>
                <span class="recording-status-text">"Saved"</span>
            </>
        }
        .into_any(),
        CompactRecordingState::Error { message } => view! {
            <>
                <span class="recording-status-glyph">"⚠"</span>
                <span class="recording-status-text">{message}</span>
            </>
        }
        .into_any(),
    }
}

fn render_shortcuts(shortcuts: &[String]) -> Option<impl IntoView> {
    if shortcuts.is_empty() {
        None
    } else {
        let chips: Vec<_> = shortcuts
            .iter()
            .map(|k| {
                let key = k.clone();
                view! { <span class="recording-status-kbd">{key}</span> }
            })
            .collect();
        Some(view! { <span class="recording-status-shortcuts">{chips}</span> })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_unique_and_kebab_case() {
        let states = [
            CompactRecordingState::Countdown {
                seconds_remaining: 3,
            },
            CompactRecordingState::Recording {
                elapsed_label: "00:03".into(),
            },
            CompactRecordingState::Paused {
                elapsed_label: "00:03".into(),
            },
            CompactRecordingState::Stopping,
            CompactRecordingState::Stopped,
            CompactRecordingState::Error {
                message: "Disk full".into(),
            },
        ];
        let slugs: Vec<_> = states.iter().map(CompactRecordingState::slug).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
        for s in slugs {
            assert!(s.chars().all(|c| c.is_ascii_lowercase()));
        }
    }

    #[test]
    fn countdown_seconds_round_trip() {
        assert_eq!(format_countdown_seconds(3), "3");
        assert_eq!(format_countdown_seconds(0), "0");
        assert_eq!(format_countdown_seconds(10), "10");
    }

    #[test]
    fn elapsed_label_passes_through_verbatim() {
        let s = CompactRecordingState::Recording {
            elapsed_label: "01:42:18".into(),
        };
        if let CompactRecordingState::Recording { elapsed_label } = s {
            assert_eq!(elapsed_label, "01:42:18");
        }
    }
}
