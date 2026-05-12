//! Compact recording-status stories (M-UI.13 / AUT-133).

use leptos::prelude::*;

use crate::components::recorder::{CompactRecordingState, RecordingStatusButton};

use super::{Story, StoryViewport, render};

const W: u16 = 380;
const H: u16 = 64;

/// All recording-status stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        story(
            "recording-status-countdown-3",
            "Compact status — countdown 3s",
            render_countdown_3,
        ),
        story(
            "recording-status-countdown-1",
            "Compact status — countdown 1s",
            render_countdown_1,
        ),
        story(
            "recording-status-recording",
            "Compact status — recording",
            render_recording,
        ),
        story(
            "recording-status-paused",
            "Compact status — paused",
            render_paused,
        ),
        story(
            "recording-status-stopping",
            "Compact status — stopping",
            render_stopping,
        ),
        story(
            "recording-status-error",
            "Compact status — error",
            render_error,
        ),
    ]
}

fn story(id: &'static str, title: &'static str, render: fn() -> String) -> Story {
    Story {
        id,
        category: "Recorder",
        title,
        viewport: StoryViewport::Fixed {
            width: W,
            height: H,
        },
        render,
    }
}

fn render_countdown_3() -> String {
    render(view! {
        <RecordingStatusButton state=CompactRecordingState::Countdown { seconds_remaining: 3 } />
    })
}

fn render_countdown_1() -> String {
    render(view! {
        <RecordingStatusButton state=CompactRecordingState::Countdown { seconds_remaining: 1 } />
    })
}

fn render_recording() -> String {
    render(view! {
        <RecordingStatusButton
            state=CompactRecordingState::Recording { elapsed_label: "00:42".to_owned() }
            shortcuts=vec!["⌘".to_owned(), "⇧".to_owned(), "2".to_owned()]
        />
    })
}

fn render_paused() -> String {
    render(view! {
        <RecordingStatusButton
            state=CompactRecordingState::Paused { elapsed_label: "01:18".to_owned() }
        />
    })
}

fn render_stopping() -> String {
    render(view! {
        <RecordingStatusButton state=CompactRecordingState::Stopping />
    })
}

fn render_error() -> String {
    render(view! {
        <RecordingStatusButton
            state=CompactRecordingState::Error { message: "Disk full — recording stopped".to_owned() }
        />
    })
}
