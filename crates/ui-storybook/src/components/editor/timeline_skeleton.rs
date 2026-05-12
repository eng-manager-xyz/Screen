//! `TimelineSkeleton` + transport + track rows (M-UI.19 / AUT-139).
//!
//! Layout-only timeline scaffold — transport row + per-track labels +
//! placeholder content. No editing semantics, no real keyframes
//! (those live in `DopeSheet`). The parent supplies timecode labels
//! and selection state.

use leptos::prelude::*;

/// One row in the skeleton.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineTrackView {
    /// Stable id ("video", "auto-zoom", "audio").
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// Placeholder shown when the track is empty
    /// (`"drop a clip to fill"`).
    pub placeholder: Option<&'static str>,
    /// `true` to render disabled.
    pub disabled: bool,
    /// `true` for the currently-selected track.
    pub selected: bool,
}

/// Timeline view-model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineView {
    /// Pre-formatted playhead label ("00:03").
    pub playhead_label: &'static str,
    /// Pre-formatted duration label ("01:24").
    pub duration_label: &'static str,
    /// `true` for the playing glyph; `false` for the paused glyph.
    pub is_playing: bool,
    /// Tracks in display order.
    pub tracks: Vec<TimelineTrackView>,
}

#[component]
pub fn TimelineTransport(
    /// Pre-formatted playhead label.
    #[prop(into)]
    playhead_label: String,
    /// Pre-formatted duration label.
    #[prop(into)]
    duration_label: String,
    /// `true` for the playing glyph.
    is_playing: bool,
) -> impl IntoView {
    let glyph = if is_playing { "❚❚" } else { "▶" };
    let label = if is_playing { "Pause" } else { "Play" };
    view! {
        <div class="timeline-transport" role="group" aria-label="Transport">
            <button class="timeline-transport-toggle" aria-label=label>{glyph}</button>
            <span class="timeline-timecode">{playhead_label}</span>
            <span class="timeline-timecode-sep">"/"</span>
            <span class="timeline-timecode-total">{duration_label}</span>
        </div>
    }
}

#[component]
pub fn TimelineTrackRow(track: TimelineTrackView) -> impl IntoView {
    let mut class = String::from("timeline-track-row");
    if track.selected {
        class.push_str(" timeline-track-row-selected");
    }
    if track.disabled {
        class.push_str(" timeline-track-row-disabled");
    }
    let placeholder = track.placeholder.unwrap_or("");
    view! {
        <li class=class data-id=track.id>
            <span class="timeline-track-label">{track.label}</span>
            <span class="timeline-track-body">
                <span class="timeline-track-placeholder">{placeholder}</span>
            </span>
        </li>
    }
}

#[component]
pub fn TimelineSkeleton(view: TimelineView) -> impl IntoView {
    let TimelineView {
        playhead_label,
        duration_label,
        is_playing,
        tracks,
    } = view;
    let track_rows: Vec<_> = tracks
        .into_iter()
        .map(|t| view! { <TimelineTrackRow track=t /> })
        .collect();
    view! {
        <section class="timeline-skeleton" aria-label="Timeline">
            <TimelineTransport
                playhead_label=playhead_label
                duration_label=duration_label
                is_playing=is_playing
            />
            <ul class="timeline-tracks" role="list">{track_rows}</ul>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_view_carries_state() {
        let t = TimelineTrackView {
            id: "video",
            label: "Video",
            placeholder: Some("drop a clip to fill"),
            disabled: false,
            selected: true,
        };
        assert!(t.selected);
        assert!(t.placeholder.is_some());
    }
}
