//! Editor stories — `DopeSheet`, `PlayerControls`, editor compositions.
//! UI-16..19 expand this with `EditorShell` / `WispCanvasHost` /
//! `InspectorPanel` / `TimelineSkeleton`.

use leptos::prelude::*;

use crate::components::editor::{DopeSheet, EditorShell, PlayState, PlayerControls};
use crate::components::primitives::{Card, CardBody, CardHeader};
use crate::fixtures::editor::{
    sample_dope_sheet_dense, sample_dope_sheet_tracks, sample_editor_shell,
    sample_editor_shell_export_disabled,
};

use super::{Story, StoryViewport, render};

/// All editor-surface stories, in display order.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "dope-sheet-basic",
            category: "Editor",
            title: "Dope sheet — multi-track",
            viewport: StoryViewport::Auto,
            render: render_dope_sheet_basic,
        },
        Story {
            id: "dope-sheet-dense",
            category: "Editor",
            title: "Dope sheet — dense keyframes",
            viewport: StoryViewport::Auto,
            render: render_dope_sheet_dense_story,
        },
        Story {
            id: "card-with-dope-sheet",
            category: "Compositions",
            title: "Editor panel — card wrapping dope sheet",
            viewport: StoryViewport::Auto,
            render: render_editor_panel,
        },
        Story {
            id: "player-controls-paused",
            category: "Player",
            title: "Player controls — paused at start",
            viewport: StoryViewport::Auto,
            render: render_player_paused,
        },
        Story {
            id: "player-controls-playing",
            category: "Player",
            title: "Player controls — playing mid-clip",
            viewport: StoryViewport::Auto,
            render: render_player_playing,
        },
        Story {
            id: "player-controls-near-end",
            category: "Player",
            title: "Player controls — near end of clip",
            viewport: StoryViewport::Auto,
            render: render_player_near_end,
        },
        Story {
            id: "editor-mock",
            category: "Compositions",
            title: "Editor mock — drop zone result + player + dope sheet",
            viewport: StoryViewport::Auto,
            render: render_editor_mock,
        },
        Story {
            id: "editor-shell-empty",
            category: "Editor",
            title: "Editor shell — empty (no clip)",
            viewport: StoryViewport::Fixed {
                width: 960,
                height: 600,
            },
            render: render_editor_shell_empty,
        },
        Story {
            id: "editor-shell-clip-loaded",
            category: "Editor",
            title: "Editor shell — clip loaded",
            viewport: StoryViewport::Fixed {
                width: 960,
                height: 600,
            },
            render: render_editor_shell_loaded,
        },
        Story {
            id: "editor-toolbar-states",
            category: "Editor",
            title: "Editor toolbar — selected + disabled mix",
            viewport: StoryViewport::Fixed {
                width: 720,
                height: 80,
            },
            render: render_editor_toolbar_states,
        },
        Story {
            id: "editor-shell-export-disabled",
            category: "Editor",
            title: "Editor shell — export disabled",
            viewport: StoryViewport::Fixed {
                width: 960,
                height: 600,
            },
            render: render_editor_shell_export_disabled,
        },
    ]
}

fn render_dope_sheet_basic() -> String {
    render(view! {
        <DopeSheet
            tracks=sample_dope_sheet_tracks()
            duration_seconds=8.0
            playhead_seconds=3.4
        />
    })
}

fn render_dope_sheet_dense_story() -> String {
    render(view! {
        <DopeSheet
            tracks=sample_dope_sheet_dense()
            duration_seconds=8.0
            playhead_seconds=5.1
        />
    })
}

fn render_editor_panel() -> String {
    render(view! {
        <Card>
            <CardHeader title="Timeline" subtitle="4 tracks · 8.0s" />
            <CardBody>
                <DopeSheet
                    tracks=sample_dope_sheet_tracks()
                    duration_seconds=8.0
                    playhead_seconds=2.0
                />
            </CardBody>
        </Card>
    })
}

fn render_player_paused() -> String {
    render(view! {
        <PlayerControls state=PlayState::Paused position=0.0 duration_seconds=84.0 />
    })
}

fn render_player_playing() -> String {
    render(view! {
        <PlayerControls state=PlayState::Playing position=0.32 duration_seconds=84.0 />
    })
}

fn render_player_near_end() -> String {
    render(view! {
        <PlayerControls state=PlayState::Playing position=0.94 duration_seconds=84.0 />
    })
}

fn render_editor_shell_empty() -> String {
    render(view! { <EditorShell view=sample_editor_shell(false) /> })
}

fn render_editor_shell_loaded() -> String {
    render(view! { <EditorShell view=sample_editor_shell(true) /> })
}

fn render_editor_shell_export_disabled() -> String {
    render(view! { <EditorShell view=sample_editor_shell_export_disabled() /> })
}

fn render_editor_toolbar_states() -> String {
    use crate::components::editor::EditorToolbar;
    let v = sample_editor_shell(true);
    render(view! {
        <EditorToolbar actions=v.toolbar_actions export_enabled=true share_enabled=false />
    })
}

fn render_editor_mock() -> String {
    render(view! {
        <div class="editor-mock">
            <Card>
                <CardHeader title="Recording 02" subtitle="Captured 2026-05-09 · 1m 24s" />
                <CardBody>
                    <div class="editor-mock-preview">
                        <div class="editor-mock-screen">
                            <div class="editor-mock-screen-label">"Player surface"</div>
                        </div>
                    </div>
                    <PlayerControls
                        state=PlayState::Playing
                        position=0.42
                        duration_seconds=84.0
                    />
                </CardBody>
            </Card>
            <Card>
                <CardHeader title="Timeline" subtitle="4 tracks · 8.0s window" />
                <CardBody>
                    <DopeSheet
                        tracks=sample_dope_sheet_tracks()
                        duration_seconds=8.0
                        playhead_seconds=3.4
                    />
                </CardBody>
            </Card>
        </div>
    })
}
