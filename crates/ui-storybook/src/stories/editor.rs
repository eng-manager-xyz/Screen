//! Editor stories — `DopeSheet`, `PlayerControls`, editor compositions.
//! UI-16..19 expand this with `EditorShell` / `WispCanvasHost` /
//! `InspectorPanel` / `TimelineSkeleton`.

use leptos::prelude::*;

use crate::components::editor::{DopeSheet, PlayState, PlayerControls};
use crate::components::primitives::{Card, CardBody, CardHeader};
use crate::fixtures::editor::{sample_dope_sheet_dense, sample_dope_sheet_tracks};

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
