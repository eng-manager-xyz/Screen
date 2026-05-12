//! `DopeSheet` — net-new component, the editor's timeline.
//!
//! A dope sheet is the workhorse of any keyframe editor: rows are tracks
//! (video, cursor, audio, captions, …), columns are time, dots are keyframes,
//! and a vertical line marks the playhead. We intentionally keep this purely
//! presentational for now — interaction (drag-to-move, snap-to-frame, scrub)
//! lives behind a future signal-driven variant.
//!
//! Layout: a left labels column at fixed width, then a flexible timeline area
//! that contains a ruler row plus one row per track. Keyframes are absolutely
//! positioned by percentage of total duration, so the same component scales
//! cleanly to any container width without re-layout math.

use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct DopeSheetTrack {
    pub label: String,
    pub kind: TrackKind,
    pub keyframes: Vec<DopeSheetKeyframe>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TrackKind {
    #[default]
    Video,
    Cursor,
    Audio,
    Caption,
    Effect,
}

impl TrackKind {
    fn css(self) -> &'static str {
        match self {
            TrackKind::Video => "track-video",
            TrackKind::Cursor => "track-cursor",
            TrackKind::Audio => "track-audio",
            TrackKind::Caption => "track-caption",
            TrackKind::Effect => "track-effect",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            TrackKind::Video => "▣",
            TrackKind::Cursor => "↗",
            TrackKind::Audio => "♪",
            TrackKind::Caption => "T",
            TrackKind::Effect => "✦",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DopeSheetKeyframe {
    /// Time in seconds (clamped to `duration_seconds` at render time).
    pub time: f32,
    pub kind: KeyframeKind,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyframeKind {
    #[default]
    Hold,
    Linear,
    Ease,
    Marker,
}

impl KeyframeKind {
    fn css(self) -> &'static str {
        match self {
            KeyframeKind::Hold => "kf-hold",
            KeyframeKind::Linear => "kf-linear",
            KeyframeKind::Ease => "kf-ease",
            KeyframeKind::Marker => "kf-marker",
        }
    }
}

#[component]
pub fn DopeSheet(
    tracks: Vec<DopeSheetTrack>,
    #[prop(optional)] duration_seconds: f32,
    #[prop(optional)] playhead_seconds: f32,
) -> impl IntoView {
    let duration = if duration_seconds <= 0.0 {
        8.0
    } else {
        duration_seconds
    };
    let playhead_pct = (playhead_seconds / duration).clamp(0.0, 1.0) * 100.0;

    // Whole-second tick marks on the ruler.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "duration is small + non-negative; ceil keeps the trailing tick"
    )]
    let tick_count = duration.ceil() as usize + 1;
    let ticks: Vec<(usize, f32)> = (0..tick_count)
        .map(|i| {
            #[allow(
                clippy::cast_precision_loss,
                reason = "tick_count fits in f32 without loss"
            )]
            let pct = (i as f32 / duration).clamp(0.0, 1.0) * 100.0;
            (i, pct)
        })
        .collect();

    let track_views = tracks
        .into_iter()
        .map(|track| {
            let kind_css = track.kind.css();
            let glyph = track.kind.glyph();
            let label = track.label.clone();
            let keyframes = track
                .keyframes
                .into_iter()
                .map(|kf| {
                    let pct = (kf.time / duration).clamp(0.0, 1.0) * 100.0;
                    let class = format!("dope-keyframe {}", kf.kind.css());
                    let style = format!("left: {pct:.2}%");
                    view! { <div class=class style=style></div> }
                })
                .collect_view();
            view! {
                <div class=format!("dope-row {kind_css}")>
                    <div class="dope-label">
                        <span class="dope-label-glyph">{glyph}</span>
                        <span class="dope-label-text">{label}</span>
                    </div>
                    <div class="dope-track">
                        {keyframes}
                    </div>
                </div>
            }
        })
        .collect_view();

    let tick_views = ticks
        .into_iter()
        .map(|(i, pct)| {
            let style = format!("left: {pct:.2}%");
            view! {
                <div class="dope-tick" style=style>
                    <span class="dope-tick-label">{format!("{i}s")}</span>
                </div>
            }
        })
        .collect_view();

    let playhead_style = format!("left: {playhead_pct:.2}%");

    view! {
        <div class="dope-sheet">
            <div class="dope-row dope-ruler">
                <div class="dope-label dope-label-spacer"></div>
                <div class="dope-track dope-ruler-track">
                    {tick_views}
                </div>
            </div>
            {track_views}
            <div class="dope-playhead" style=playhead_style></div>
        </div>
    }
}
