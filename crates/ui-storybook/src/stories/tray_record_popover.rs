//! Tray-record-popover composition stories (M-UI.12 / AUT-132).

use leptos::prelude::*;

use crate::components::recorder::{OpenRecorderPopoverKind, TrayRecordPopover};
use crate::fixtures::recorder::{
    sample_tray_record_popover, sample_tray_record_popover_start_disabled,
};

use super::{Story, StoryViewport, render};

const POPOVER_WIDTH: u16 = 460;
const POPOVER_HEIGHT: u16 = 760;

/// All tray-record-popover stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        story(
            "tray-record-popover-default",
            "Tray record popover — default",
            render_default,
        ),
        story(
            "tray-record-popover-workspace-open",
            "Tray record popover — workspace menu open",
            render_workspace_open,
        ),
        story(
            "tray-record-popover-camera-open",
            "Tray record popover — camera menu open",
            render_camera_open,
        ),
        story(
            "tray-record-popover-microphone-open",
            "Tray record popover — microphone menu open",
            render_microphone_open,
        ),
        story(
            "tray-record-popover-system-audio-open",
            "Tray record popover — system-audio expanded",
            render_system_audio_open,
        ),
        story(
            "tray-record-popover-on-screen-open",
            "Tray record popover — on-screen options open",
            render_on_screen_open,
        ),
        story(
            "tray-record-popover-start-disabled",
            "Tray record popover — Start recording disabled",
            render_start_disabled,
        ),
    ]
}

fn story(id: &'static str, title: &'static str, render: fn() -> String) -> Story {
    Story {
        id,
        category: "Recorder",
        title,
        viewport: StoryViewport::Fixed {
            width: POPOVER_WIDTH,
            height: POPOVER_HEIGHT,
        },
        render,
    }
}

fn render_default() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover(OpenRecorderPopoverKind::None) />
    })
}

fn render_workspace_open() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover(OpenRecorderPopoverKind::WorkspaceMenu) />
    })
}

fn render_camera_open() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover(OpenRecorderPopoverKind::CameraMenu) />
    })
}

fn render_microphone_open() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover(OpenRecorderPopoverKind::MicrophoneMenu) />
    })
}

fn render_system_audio_open() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover(OpenRecorderPopoverKind::SystemAudioMenu) />
    })
}

fn render_on_screen_open() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover(OpenRecorderPopoverKind::OnScreenOptions) />
    })
}

fn render_start_disabled() -> String {
    render(view! {
        <TrayRecordPopover view=sample_tray_record_popover_start_disabled() />
    })
}
