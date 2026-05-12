//! Shell stories — `DropZone`, `StatusBar`, plus M-UI.2 `NavigationRail`
//! + `AppShell` compositions.

use leptos::prelude::*;

use crate::components::shell::{
    AppSection, AppShell, DropZone, DropZoneState, NavigationRail, StatusBar, StatusKind,
};
use crate::fixtures::shell::{sample_nav_items, sample_user_avatar, sample_workspace_badge};

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
        // ─── M-UI.2 navigation rail / app shell ──────────────
        Story {
            id: "nav-rail-record-active",
            category: "Shell",
            title: "Nav rail — Record active",
            viewport: StoryViewport::Fixed {
                width: 88,
                height: 460,
            },
            render: render_nav_record,
        },
        Story {
            id: "nav-rail-library-active",
            category: "Shell",
            title: "Nav rail — Library active",
            viewport: StoryViewport::Fixed {
                width: 88,
                height: 460,
            },
            render: render_nav_library,
        },
        Story {
            id: "nav-rail-editor-active",
            category: "Shell",
            title: "Nav rail — Editor active",
            viewport: StoryViewport::Fixed {
                width: 88,
                height: 460,
            },
            render: render_nav_editor,
        },
        Story {
            id: "nav-rail-cursor-active",
            category: "Shell",
            title: "Nav rail — Cursor active",
            viewport: StoryViewport::Fixed {
                width: 88,
                height: 460,
            },
            render: render_nav_cursor,
        },
        Story {
            id: "nav-rail-with-counts",
            category: "Shell",
            title: "Nav rail — with notification count",
            viewport: StoryViewport::Fixed {
                width: 88,
                height: 460,
            },
            render: render_nav_with_counts,
        },
        Story {
            id: "app-shell-three-pane",
            category: "Compositions",
            title: "App shell — three-pane editor layout",
            viewport: StoryViewport::Fixed {
                width: 760,
                height: 480,
            },
            render: render_app_shell_three_pane,
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

fn render_nav_rail(active: AppSection, extra_count: bool) -> String {
    let workspace = sample_workspace_badge();
    let user = sample_user_avatar();
    let items = sample_nav_items(extra_count);
    render(view! {
        <div style="width:88px; height:460px; background:#0b0b0d; border:1px solid var(--line-subtle); border-radius:8px;">
            <NavigationRail items=items active=active workspace=workspace user=user />
        </div>
    })
}

fn render_nav_record() -> String {
    render_nav_rail(AppSection::Record, false)
}
fn render_nav_library() -> String {
    render_nav_rail(AppSection::Library, false)
}
fn render_nav_editor() -> String {
    render_nav_rail(AppSection::Editor, false)
}
fn render_nav_cursor() -> String {
    render_nav_rail(AppSection::Cursor, false)
}
fn render_nav_with_counts() -> String {
    render_nav_rail(AppSection::Library, true)
}

fn render_app_shell_three_pane() -> String {
    let workspace = sample_workspace_badge();
    let user = sample_user_avatar();
    let items = sample_nav_items(false);
    render(view! {
        <div class="shell-story">
            <AppShell
                rail=ToChildren::to_children(move || view! {
                    <NavigationRail
                        items=items
                        active=AppSection::Editor
                        workspace=workspace
                        user=user
                    />
                })
                titlebar=ToChildren::to_children(move || view! {
                    <span>"screen-studio · editor · Recording 02"</span>
                })
                main=ToChildren::to_children(move || view! {
                    <div class="shell-story-main-placeholder">
                        "Main pane — editor canvas / library grid / cursor preview goes here"
                    </div>
                })
                inspector=ToChildren::to_children(move || view! {
                    <div class="shell-story-inspector-placeholder">
                        <h3 style="font-size:13px;margin:0 0 8px;color:var(--text-primary)">"Inspector"</h3>
                        <p>"Property rows render here in UI-18."</p>
                    </div>
                })
                footer=ToChildren::to_children(move || view! {
                    <span>"Encoding · 38%   ·   FPS 60"</span>
                })
            />
        </div>
    })
}
