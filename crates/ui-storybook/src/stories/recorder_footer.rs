//! Recording-controls-footer stories (M-UI.11 / AUT-131).

use leptos::prelude::*;

use crate::components::recorder::{RecordingControlsFooter, StartRecordingState};
use crate::fixtures::recorder::{sample_recording_controls, sample_recording_controls_compact};

use super::{Story, StoryViewport, render};

/// All recording-controls-footer stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "recording-footer-ready",
            category: "Recorder",
            title: "Recording footer — ready",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 96,
            },
            render: render_ready,
        },
        Story {
            id: "recording-footer-disabled",
            category: "Recorder",
            title: "Recording footer — disabled",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 96,
            },
            render: render_disabled,
        },
        Story {
            id: "recording-footer-loading",
            category: "Recorder",
            title: "Recording footer — loading",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 96,
            },
            render: render_loading,
        },
        Story {
            id: "recording-footer-permission-blocked",
            category: "Recorder",
            title: "Recording footer — permission blocked",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 96,
            },
            render: render_permission_blocked,
        },
        Story {
            id: "recording-footer-compact",
            category: "Recorder",
            title: "Recording footer — compact (no zoom, no countdown)",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 96,
            },
            render: render_compact,
        },
    ]
}

fn render_ready() -> String {
    render(view! {
        <RecordingControlsFooter view=sample_recording_controls(StartRecordingState::Ready) />
    })
}

fn render_disabled() -> String {
    render(view! {
        <RecordingControlsFooter view=sample_recording_controls(StartRecordingState::Disabled) />
    })
}

fn render_loading() -> String {
    render(view! {
        <RecordingControlsFooter view=sample_recording_controls(StartRecordingState::Loading) />
    })
}

fn render_permission_blocked() -> String {
    render(view! {
        <RecordingControlsFooter view=sample_recording_controls(StartRecordingState::PermissionBlocked) />
    })
}

fn render_compact() -> String {
    render(view! {
        <RecordingControlsFooter view=sample_recording_controls_compact() />
    })
}
