//! Shell stories — `DropZone`, `StatusBar`. UI-02 will add `AppShell` /
//! `NavigationRail` stories.

use leptos::prelude::*;

use crate::components::shell::{DropZone, DropZoneState, StatusBar, StatusKind};

use super::{Story, StoryViewport, render};

/// All shell-surface stories, in display order.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "drop-zone-idle",
            category: "App surfaces",
            title: "Drop zone — idle",
            viewport: StoryViewport::Auto,
            render: render_drop_zone_idle,
        },
        Story {
            id: "drop-zone-active",
            category: "App surfaces",
            title: "Drop zone — active (file dragged)",
            viewport: StoryViewport::Auto,
            render: render_drop_zone_active,
        },
        Story {
            id: "status-bar-ready",
            category: "Shell",
            title: "Status bar — ready",
            viewport: StoryViewport::Auto,
            render: render_status_ready,
        },
        Story {
            id: "status-bar-busy",
            category: "Shell",
            title: "Status bar — encoding",
            viewport: StoryViewport::Auto,
            render: render_status_busy,
        },
        Story {
            id: "status-bar-error",
            category: "Shell",
            title: "Status bar — error",
            viewport: StoryViewport::Auto,
            render: render_status_error,
        },
    ]
}

fn render_drop_zone_idle() -> String {
    render(view! {
        <DropZone state=DropZoneState::Idle hint="⌘O to browse" />
    })
}

fn render_drop_zone_active() -> String {
    render(view! {
        <DropZone state=DropZoneState::Active />
    })
}

fn render_status_ready() -> String {
    render(view! {
        <StatusBar
            fps=60.0
            encoder="H.264 · idle"
            file_bytes=0
            kind=StatusKind::Ready
        />
    })
}

fn render_status_busy() -> String {
    render(view! {
        <StatusBar
            fps=60.0
            encoder="H.264 · 9.4 Mbps"
            file_bytes=24_117_248_u64
            kind=StatusKind::Busy
            detail="Encoding · 38%"
        />
    })
}

fn render_status_error() -> String {
    render(view! {
        <StatusBar
            fps=0.0
            encoder="H.264"
            file_bytes=12_582_912_u64
            kind=StatusKind::Error
            detail="VideoToolbox: out of memory"
        />
    })
}
