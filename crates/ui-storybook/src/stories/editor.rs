//! Editor stories — `DopeSheet`, `PlayerControls`, editor compositions.
//! UI-16..19 expand this with `EditorShell` / `WispCanvasHost` /
//! `InspectorPanel` / `TimelineSkeleton`.

use leptos::prelude::*;

use crate::components::editor::{
    CanvasBackendView, DopeSheet, EditorDropZoneCanvas, EditorShell, InspectorPanel, PlayState,
    PlayerControls, PropertyControlView, PropertyRowView, PropertySection, PropertySectionView,
    TimelineSkeleton, WispCanvasHost,
};
use crate::components::primitives::{Card, CardBody, CardHeader};
use crate::fixtures::editor::{
    sample_dope_sheet_dense, sample_dope_sheet_tracks, sample_editor_drop_zone,
    sample_editor_drop_zone_no_recent, sample_editor_shell, sample_editor_shell_export_disabled,
    sample_inspector_cursor_tab, sample_inspector_disabled_section, sample_inspector_style_tab,
    sample_timeline_skeleton, sample_timeline_skeleton_empty, sample_timeline_skeleton_playing,
};

use super::{Story, StoryViewport, render};

fn fixed(width: u16, height: u16) -> StoryViewport {
    StoryViewport::Fixed { width, height }
}

fn s(
    id: &'static str,
    category: &'static str,
    title: &'static str,
    viewport: StoryViewport,
    render: fn() -> String,
) -> Story {
    Story {
        id,
        category,
        title,
        viewport,
        render,
    }
}

fn legacy_editor_stories() -> Vec<Story> {
    let auto = StoryViewport::Auto;
    vec![
        s(
            "dope-sheet-basic",
            "Editor",
            "Dope sheet — multi-track",
            auto,
            render_dope_sheet_basic,
        ),
        s(
            "dope-sheet-dense",
            "Editor",
            "Dope sheet — dense keyframes",
            auto,
            render_dope_sheet_dense_story,
        ),
        s(
            "card-with-dope-sheet",
            "Compositions",
            "Editor panel — card wrapping dope sheet",
            auto,
            render_editor_panel,
        ),
        s(
            "player-controls-paused",
            "Player",
            "Player controls — paused at start",
            auto,
            render_player_paused,
        ),
        s(
            "player-controls-playing",
            "Player",
            "Player controls — playing mid-clip",
            auto,
            render_player_playing,
        ),
        s(
            "player-controls-near-end",
            "Player",
            "Player controls — near end of clip",
            auto,
            render_player_near_end,
        ),
        s(
            "editor-mock",
            "Compositions",
            "Editor mock — drop zone result + player + dope sheet",
            auto,
            render_editor_mock,
        ),
    ]
}

fn editor_shell_stories() -> Vec<Story> {
    let shell = fixed(960, 600);
    vec![
        s(
            "editor-shell-empty",
            "Editor",
            "Editor shell — empty (no clip)",
            shell,
            render_editor_shell_empty,
        ),
        s(
            "editor-shell-clip-loaded",
            "Editor",
            "Editor shell — clip loaded",
            shell,
            render_editor_shell_loaded,
        ),
        s(
            "editor-toolbar-states",
            "Editor",
            "Editor toolbar — selected + disabled mix",
            fixed(720, 80),
            render_editor_toolbar_states,
        ),
        s(
            "editor-shell-export-disabled",
            "Editor",
            "Editor shell — export disabled",
            shell,
            render_editor_shell_export_disabled,
        ),
    ]
}

fn timeline_stories() -> Vec<Story> {
    let strip = fixed(960, 200);
    vec![
        s(
            "timeline-empty",
            "Editor",
            "Timeline — empty (no placeholders)",
            strip,
            render_timeline_empty,
        ),
        s(
            "timeline-with-placeholders",
            "Editor",
            "Timeline — with placeholders",
            strip,
            render_timeline_with_placeholders,
        ),
        s(
            "timeline-playing",
            "Editor",
            "Timeline — playing",
            strip,
            render_timeline_playing,
        ),
        s(
            "timeline-selected-track",
            "Editor",
            "Timeline — selected video track",
            strip,
            render_timeline_selected,
        ),
    ]
}

fn inspector_stories() -> Vec<Story> {
    let panel = fixed(320, 480);
    vec![
        s(
            "inspector-style-tab",
            "Inspector",
            "Inspector — Style tab",
            panel,
            render_inspector_style,
        ),
        s(
            "inspector-cursor-tab",
            "Inspector",
            "Inspector — Cursor tab",
            panel,
            render_inspector_cursor,
        ),
        s(
            "property-row-slider",
            "Inspector",
            "Property row — slider",
            fixed(320, 80),
            render_row_slider,
        ),
        s(
            "property-row-toggle",
            "Inspector",
            "Property row — toggle",
            fixed(320, 80),
            render_row_toggle,
        ),
        s(
            "property-row-color-swatches",
            "Inspector",
            "Property row — color swatches",
            fixed(320, 80),
            render_row_swatches,
        ),
        s(
            "inspector-disabled-section",
            "Inspector",
            "Inspector — disabled rows",
            panel,
            render_inspector_disabled,
        ),
    ]
}

fn editor_canvas_stories() -> Vec<Story> {
    let drop_zone = fixed(760, 520);
    let canvas = fixed(600, 360);
    vec![
        s(
            "editor-drop-zone-empty",
            "Editor",
            "Editor drop zone — empty",
            drop_zone,
            render_drop_zone_empty,
        ),
        s(
            "editor-drop-zone-drag-active",
            "Editor",
            "Editor drop zone — drag active",
            drop_zone,
            render_drop_zone_drag,
        ),
        s(
            "editor-drop-zone-with-recent",
            "Editor",
            "Editor drop zone — with recent clips",
            drop_zone,
            render_drop_zone_with_recent,
        ),
        s(
            "wisp-canvas-host-fallback",
            "Editor",
            "Wisp canvas host — CSS fallback",
            canvas,
            render_canvas_host_fallback,
        ),
        s(
            "wisp-canvas-host-asset",
            "Editor",
            "Wisp canvas host — runtime unavailable",
            canvas,
            render_canvas_host_unavailable,
        ),
    ]
}

/// All editor-surface stories, in display order.
#[must_use]
pub fn stories() -> Vec<Story> {
    let mut out = legacy_editor_stories();
    out.extend(editor_shell_stories());
    out.extend(editor_canvas_stories());
    out.extend(inspector_stories());
    out.extend(timeline_stories());
    out
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

fn render_drop_zone_empty() -> String {
    render(view! { <EditorDropZoneCanvas view=sample_editor_drop_zone_no_recent() /> })
}

fn render_drop_zone_drag() -> String {
    render(view! { <EditorDropZoneCanvas view=sample_editor_drop_zone(true) /> })
}

fn render_drop_zone_with_recent() -> String {
    render(view! { <EditorDropZoneCanvas view=sample_editor_drop_zone(false) /> })
}

fn render_canvas_host_fallback() -> String {
    render(view! { <WispCanvasHost backend=CanvasBackendView::CssFallback /> })
}

fn render_canvas_host_unavailable() -> String {
    render(view! { <WispCanvasHost backend=CanvasBackendView::WispRuntimeUnavailable /> })
}

fn render_inspector_style() -> String {
    render(view! { <InspectorPanel view=sample_inspector_style_tab() /> })
}

fn render_inspector_cursor() -> String {
    render(view! { <InspectorPanel view=sample_inspector_cursor_tab() /> })
}

fn render_inspector_disabled() -> String {
    render(view! { <InspectorPanel view=sample_inspector_disabled_section() /> })
}

fn render_row_slider() -> String {
    let section = PropertySectionView {
        title: "EXAMPLE",
        rows: vec![PropertyRowView {
            label: "Auto-zoom",
            value: Some("2.0×"),
            disabled: false,
            control: PropertyControlView::SliderPercent { percent: 55 },
        }],
    };
    render(view! { <PropertySection section=section /> })
}

fn render_row_toggle() -> String {
    let section = PropertySectionView {
        title: "EXAMPLE",
        rows: vec![PropertyRowView {
            label: "Drop shadow",
            value: None,
            disabled: false,
            control: PropertyControlView::Toggle { on: true },
        }],
    };
    render(view! { <PropertySection section=section /> })
}

fn render_timeline_empty() -> String {
    render(view! { <TimelineSkeleton view=sample_timeline_skeleton_empty() /> })
}

fn render_timeline_with_placeholders() -> String {
    render(view! { <TimelineSkeleton view=sample_timeline_skeleton() /> })
}

fn render_timeline_playing() -> String {
    render(view! { <TimelineSkeleton view=sample_timeline_skeleton_playing() /> })
}

fn render_timeline_selected() -> String {
    let mut v = sample_timeline_skeleton();
    if let Some(t) = v.tracks.first_mut() {
        t.selected = true;
    }
    render(view! { <TimelineSkeleton view=v /> })
}

fn render_row_swatches() -> String {
    let section = PropertySectionView {
        title: "EXAMPLE",
        rows: vec![PropertyRowView {
            label: "Color",
            value: None,
            disabled: false,
            control: PropertyControlView::ColorSwatches {
                swatches: vec!["#fafafa", "#facc15", "#38bdf8", "#a78bfa", "#f97316"],
                selected: 2,
            },
        }],
    };
    render(view! { <PropertySection section=section /> })
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
