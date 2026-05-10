//! `PlayerControls` — transport bar for the player view.
//!
//! Composed of: play/pause button, scrub track with progress fill + handle,
//! and a `current / total` time display. Pure presentational — `position`
//! is a `0.0..=1.0` fraction; the parent owns the signal that drives it.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayState {
    #[default]
    Paused,
    Playing,
}

#[component]
pub fn PlayerControls(
    /// Play/pause state — drives the glyph and aria-label of the toggle.
    #[prop(optional)]
    state: PlayState,
    /// Scrub position as `0.0..=1.0`.
    #[prop(optional)]
    position: f32,
    /// Total duration in seconds (used for the right-hand label).
    #[prop(optional)]
    duration_seconds: f32,
    /// Optional click handler for the play/pause toggle. Pure-presentation
    /// stories leave this `None` (default); the recorder shell (`app-ui`)
    /// passes a callback that invokes the Tauri `player_play` /
    /// `player_pause` commands.
    #[prop(optional, into)]
    on_toggle: Option<Callback<()>>,
) -> impl IntoView {
    let position = position.clamp(0.0, 1.0);
    let duration = if duration_seconds <= 0.0 {
        60.0
    } else {
        duration_seconds
    };
    let current = position * duration;

    let pct = position * 100.0;
    let fill_style = format!("width: {pct:.2}%");
    let handle_style = format!("left: {pct:.2}%");

    let (toggle_glyph, toggle_label, toggle_class) = match state {
        PlayState::Paused => ("▶", "Play", "player-toggle player-toggle-paused"),
        PlayState::Playing => ("❚❚", "Pause", "player-toggle player-toggle-playing"),
    };

    let toggle_click = move |_| {
        if let Some(cb) = on_toggle {
            cb.run(());
        }
    };

    view! {
        <div class="player-controls" role="group" aria-label="Player transport">
            <button class=toggle_class type="button" aria-label=toggle_label on:click=toggle_click>
                <span class="player-toggle-glyph">{toggle_glyph}</span>
            </button>

            <div class="player-time" data-role="current">{format_time(current)}</div>

            <div class="player-scrub">
                <div class="player-scrub-track">
                    <div class="player-scrub-fill" style=fill_style></div>
                    <div class="player-scrub-handle" style=handle_style></div>
                </div>
            </div>

            <div class="player-time" data-role="total">{format_time(duration)}</div>
        </div>
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "seconds is positive and bounded by duration_seconds, fits in u32"
)]
fn format_time(seconds: f32) -> String {
    let total = seconds.max(0.0).round() as u32;
    let m = total / 60;
    let s = total % 60;
    format!("{m}:{s:02}")
}
