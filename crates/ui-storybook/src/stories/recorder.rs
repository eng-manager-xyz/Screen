//! Recorder stories — `RecordingToolbar`. UI-06..13 expand this list with
//! capture-mode tabs, device pickers, overlay options, recording footer,
//! and the tray popover composition.

use leptos::prelude::*;

use crate::components::recorder::{CaptureModeTabs, RecordingState, RecordingToolbar};
use crate::fixtures::recorder::CaptureMode;

use super::{Story, StoryViewport, render};

/// All recorder-surface stories, in display order.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "capture-mode-screen",
            category: "Recorder",
            title: "Capture mode — Screen",
            viewport: StoryViewport::Auto,
            render: render_capture_screen,
        },
        Story {
            id: "capture-mode-window",
            category: "Recorder",
            title: "Capture mode — Window",
            viewport: StoryViewport::Auto,
            render: render_capture_window,
        },
        Story {
            id: "capture-mode-area",
            category: "Recorder",
            title: "Capture mode — Area",
            viewport: StoryViewport::Auto,
            render: render_capture_area,
        },
        Story {
            id: "capture-mode-disabled-area",
            category: "Recorder",
            title: "Capture mode — Area disabled (permissions)",
            viewport: StoryViewport::Auto,
            render: render_capture_disabled_area,
        },
        Story {
            id: "recording-toolbar-idle",
            category: "Recorder",
            title: "Recording toolbar — idle (ready)",
            viewport: StoryViewport::Auto,
            render: render_recording_idle,
        },
        Story {
            id: "recording-toolbar-recording",
            category: "Recorder",
            title: "Recording toolbar — recording",
            viewport: StoryViewport::Auto,
            render: render_recording_recording,
        },
        Story {
            id: "recording-toolbar-paused",
            category: "Recorder",
            title: "Recording toolbar — paused",
            viewport: StoryViewport::Auto,
            render: render_recording_paused,
        },
    ]
}

fn render_recording_idle() -> String {
    render(view! {
        <RecordingToolbar
            state=RecordingState::Idle
            elapsed_seconds=0.0
            source="Built-in Display"
        />
    })
}

fn render_recording_recording() -> String {
    render(view! {
        <RecordingToolbar
            state=RecordingState::Recording
            elapsed_seconds=137.0
            source="Built-in Display"
        />
    })
}

fn render_recording_paused() -> String {
    render(view! {
        <RecordingToolbar
            state=RecordingState::Paused
            elapsed_seconds=137.0
            source="Built-in Display"
        />
    })
}

fn render_capture_screen() -> String {
    render(view! {
        <div style="padding:14px">
            <CaptureModeTabs selected=CaptureMode::Screen />
        </div>
    })
}

fn render_capture_window() -> String {
    render(view! {
        <div style="padding:14px">
            <CaptureModeTabs selected=CaptureMode::Window />
        </div>
    })
}

fn render_capture_area() -> String {
    render(view! {
        <div style="padding:14px">
            <CaptureModeTabs selected=CaptureMode::Area />
        </div>
    })
}

fn render_capture_disabled_area() -> String {
    render(view! {
        <div style="padding:14px">
            <CaptureModeTabs selected=CaptureMode::Screen disabled_modes=vec![CaptureMode::Area] />
        </div>
    })
}
