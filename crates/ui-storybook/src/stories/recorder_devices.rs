//! Capture-source + device-picker stories (M-UI.8 / AUT-128).

use leptos::prelude::*;

use crate::components::recorder::{
    CaptureSourceKind, CaptureSourceRow, DevicePickerMenu, DevicePickerState,
};
use crate::fixtures::devices::{
    sample_camera_options, sample_capture_source_camera, sample_capture_source_microphone,
    sample_microphone_options,
};

use super::{Story, StoryViewport, render};

/// All capture-source + device-picker stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "capture-source-camera-collapsed",
            category: "Recorder",
            title: "Capture source — camera (collapsed)",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 80,
            },
            render: render_camera_collapsed,
        },
        Story {
            id: "capture-source-microphone-collapsed",
            category: "Recorder",
            title: "Capture source — microphone with live meter",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 80,
            },
            render: render_microphone_collapsed,
        },
        Story {
            id: "device-picker-camera-open",
            category: "Recorder",
            title: "Device picker — cameras (open)",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 360,
            },
            render: render_camera_open,
        },
        Story {
            id: "device-picker-microphone-open",
            category: "Recorder",
            title: "Device picker — microphones (open)",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 360,
            },
            render: render_microphone_open,
        },
        Story {
            id: "device-picker-empty",
            category: "Recorder",
            title: "Device picker — no devices detected",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 280,
            },
            render: render_empty,
        },
        Story {
            id: "device-picker-permission-needed",
            category: "Recorder",
            title: "Device picker — permission needed",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 280,
            },
            render: render_permission_needed,
        },
    ]
}

fn render_camera_collapsed() -> String {
    render(view! {
        <div style="padding:12px">
            <CaptureSourceRow view=sample_capture_source_camera(true, false) />
        </div>
    })
}

fn render_microphone_collapsed() -> String {
    render(view! {
        <div style="padding:12px">
            <CaptureSourceRow view=sample_capture_source_microphone(true, false, Some(0.45)) />
        </div>
    })
}

fn render_camera_open() -> String {
    render(view! {
        <DevicePickerMenu
            kind=CaptureSourceKind::Camera
            devices=sample_camera_options()
        />
    })
}

fn render_microphone_open() -> String {
    render(view! {
        <DevicePickerMenu
            kind=CaptureSourceKind::Microphone
            devices=sample_microphone_options()
        />
    })
}

fn render_empty() -> String {
    render(view! {
        <DevicePickerMenu
            kind=CaptureSourceKind::Camera
            devices=vec![]
            state=DevicePickerState::Empty
        />
    })
}

fn render_permission_needed() -> String {
    render(view! {
        <DevicePickerMenu
            kind=CaptureSourceKind::Microphone
            devices=vec![]
            state=DevicePickerState::PermissionNeeded
        />
    })
}
