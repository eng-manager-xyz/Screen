//! Editor fixtures — timeline tracks, inspector property rows, dope-sheet
//! keyframes. Filled in across UI-16..19. The dope-sheet fixture lifts the
//! previously-inline `sample_tracks()` so multiple editor stories can share
//! it.

use crate::components::editor::{DopeSheetKeyframe, DopeSheetTrack, KeyframeKind, TrackKind};

/// Multi-track sample used by `dope-sheet-basic` and the editor compositions.
#[must_use]
pub fn sample_dope_sheet_tracks() -> Vec<DopeSheetTrack> {
    vec![
        DopeSheetTrack {
            label: "Video".into(),
            kind: TrackKind::Video,
            keyframes: vec![
                DopeSheetKeyframe {
                    time: 0.0,
                    kind: KeyframeKind::Hold,
                },
                DopeSheetKeyframe {
                    time: 2.5,
                    kind: KeyframeKind::Ease,
                },
                DopeSheetKeyframe {
                    time: 6.0,
                    kind: KeyframeKind::Linear,
                },
            ],
        },
        DopeSheetTrack {
            label: "Cursor".into(),
            kind: TrackKind::Cursor,
            keyframes: vec![
                DopeSheetKeyframe {
                    time: 1.2,
                    kind: KeyframeKind::Linear,
                },
                DopeSheetKeyframe {
                    time: 4.5,
                    kind: KeyframeKind::Linear,
                },
            ],
        },
        DopeSheetTrack {
            label: "Audio".into(),
            kind: TrackKind::Audio,
            keyframes: vec![
                DopeSheetKeyframe {
                    time: 0.0,
                    kind: KeyframeKind::Hold,
                },
                DopeSheetKeyframe {
                    time: 4.0,
                    kind: KeyframeKind::Hold,
                },
                DopeSheetKeyframe {
                    time: 7.5,
                    kind: KeyframeKind::Hold,
                },
            ],
        },
        DopeSheetTrack {
            label: "Caption".into(),
            kind: TrackKind::Caption,
            keyframes: vec![DopeSheetKeyframe {
                time: 3.0,
                kind: KeyframeKind::Marker,
            }],
        },
    ]
}

/// Dense variant — adds a noisy effect track with twelve keyframes for the
/// `dope-sheet-dense` story.
#[must_use]
pub fn sample_dope_sheet_dense() -> Vec<DopeSheetTrack> {
    let mut tracks = sample_dope_sheet_tracks();
    tracks.push(DopeSheetTrack {
        label: "Zoom".into(),
        kind: TrackKind::Effect,
        keyframes: (0..12)
            .map(|i| DopeSheetKeyframe {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "i ≤ 12 fits in f32 without loss"
                )]
                time: i as f32 * 0.65,
                kind: if i % 3 == 0 {
                    KeyframeKind::Ease
                } else {
                    KeyframeKind::Linear
                },
            })
            .collect(),
    });
    tracks
}

/// Sample editor-shell view (UI-16).
#[must_use]
pub fn sample_editor_shell(has_clip_loaded: bool) -> crate::components::editor::EditorShellView {
    use crate::components::editor::{EditorShellView, ToolbarActionView};
    let actions = vec![
        ToolbarActionView {
            id: "16-9",
            label: "16:9",
            icon: "▭",
            selected: true,
            disabled: false,
        },
        ToolbarActionView {
            id: "crop",
            label: "Crop",
            icon: "✂",
            selected: false,
            disabled: false,
        },
        ToolbarActionView {
            id: "annotate",
            label: "Annotate",
            icon: "✎",
            selected: false,
            disabled: !has_clip_loaded,
        },
        ToolbarActionView {
            id: "trim",
            label: "Trim",
            icon: "⎯",
            selected: false,
            disabled: !has_clip_loaded,
        },
    ];
    EditorShellView {
        document_title: "Demo · auth login".into(),
        document_subtitle: Some("Captured 2026-05-09 · 1m 24s".into()),
        has_clip_loaded,
        toolbar_actions: actions,
        export_enabled: has_clip_loaded,
        share_enabled: has_clip_loaded,
    }
}

/// Variant — export disabled.
#[must_use]
pub fn sample_editor_shell_export_disabled() -> crate::components::editor::EditorShellView {
    let mut v = sample_editor_shell(true);
    v.export_enabled = false;
    v
}

/// Sample drop-zone view (UI-17). The backend defaults to `CssFallback`
/// so SSR snapshots stay stable.
#[must_use]
pub fn sample_editor_drop_zone(drag_active: bool) -> crate::components::editor::EditorDropZoneView {
    use crate::components::editor::{CanvasBackendView, DropZoneActionView, EditorDropZoneView};
    EditorDropZoneView {
        backend: CanvasBackendView::CssFallback,
        headline: "Drop a clip to start editing",
        subtext: "or press ⌘⇧2 to record one now",
        actions: vec![
            DropZoneActionView {
                id: "record",
                label: "Record screen",
                description: "Open the tray recorder",
                icon: "●",
            },
            DropZoneActionView {
                id: "library",
                label: "Open library",
                description: "Pick a recent recording",
                icon: "▦",
            },
            DropZoneActionView {
                id: "file",
                label: "Import file",
                description: "Choose an MP4 / MOV",
                icon: "📁",
            },
        ],
        recent_clips: sample_recent_clips(),
        drag_active,
    }
}

fn sample_recent_clips() -> Vec<crate::components::editor::RecentClipView> {
    use crate::components::editor::RecentClipView;
    vec![
        RecentClipView {
            id: "rec-01",
            title: "Demo · auth login",
            duration_label: "1m 24s",
            thumbnail_css: "linear-gradient(135deg,#1e3a8a,#7c3aed)",
        },
        RecentClipView {
            id: "rec-02",
            title: "Standup recap",
            duration_label: "4m 02s",
            thumbnail_css: "linear-gradient(135deg,#0f766e,#facc15)",
        },
        RecentClipView {
            id: "rec-03",
            title: "Bug repro · payments",
            duration_label: "0m 38s",
            thumbnail_css: "linear-gradient(135deg,#7f1d1d,#f97316)",
        },
    ]
}

/// Variant — empty recent strip.
#[must_use]
pub fn sample_editor_drop_zone_no_recent() -> crate::components::editor::EditorDropZoneView {
    let mut v = sample_editor_drop_zone(false);
    v.recent_clips.clear();
    v
}

/// Sample inspector view (UI-18) — Style tab with appearance + motion sections.
#[must_use]
pub fn sample_inspector_style_tab() -> crate::components::editor::InspectorPanelView {
    use crate::components::editor::{
        InspectorPanelView, InspectorTab, PropertyControlView, PropertyRowView, PropertySectionView,
    };
    let appearance = PropertySectionView {
        title: "APPEARANCE",
        rows: vec![
            PropertyRowView {
                label: "Background",
                value: Some("Studio dark"),
                disabled: false,
                control: PropertyControlView::SelectPill {
                    current_label: "Studio dark",
                },
            },
            PropertyRowView {
                label: "Corner radius",
                value: Some("12 px"),
                disabled: false,
                control: PropertyControlView::SliderPercent { percent: 35 },
            },
            PropertyRowView {
                label: "Drop shadow",
                value: None,
                disabled: false,
                control: PropertyControlView::Toggle { on: true },
            },
        ],
    };
    let motion = PropertySectionView {
        title: "MOTION",
        rows: vec![
            PropertyRowView {
                label: "Auto-zoom",
                value: Some("2.0×"),
                disabled: false,
                control: PropertyControlView::SliderPercent { percent: 50 },
            },
            PropertyRowView {
                label: "Smooth pan",
                value: None,
                disabled: false,
                control: PropertyControlView::Toggle { on: true },
            },
        ],
    };
    InspectorPanelView {
        tabs: vec![
            InspectorTab::Style,
            InspectorTab::Cursor,
            InspectorTab::Audio,
            InspectorTab::Captions,
            InspectorTab::Ai,
        ],
        active: InspectorTab::Style,
        sections: vec![appearance, motion],
    }
}

/// Variant — cursor tab active with appearance swatches.
#[must_use]
pub fn sample_inspector_cursor_tab() -> crate::components::editor::InspectorPanelView {
    use crate::components::editor::{
        InspectorTab, PropertyControlView, PropertyRowView, PropertySectionView,
    };
    let mut v = sample_inspector_style_tab();
    v.active = InspectorTab::Cursor;
    v.sections = vec![
        PropertySectionView {
            title: "APPEARANCE",
            rows: vec![
                PropertyRowView {
                    label: "Size",
                    value: Some("120 %"),
                    disabled: false,
                    control: PropertyControlView::SliderPercent { percent: 60 },
                },
                PropertyRowView {
                    label: "Color",
                    value: None,
                    disabled: false,
                    control: PropertyControlView::ColorSwatches {
                        swatches: vec!["#fafafa", "#facc15", "#38bdf8", "#a78bfa", "#f97316"],
                        selected: 2,
                    },
                },
                PropertyRowView {
                    label: "Halo",
                    value: None,
                    disabled: false,
                    control: PropertyControlView::Toggle { on: true },
                },
            ],
        },
        PropertySectionView {
            title: "CLICK EFFECT",
            rows: vec![PropertyRowView {
                label: "Effect",
                value: Some("Ring"),
                disabled: false,
                control: PropertyControlView::SelectPill {
                    current_label: "Ring",
                },
            }],
        },
    ];
    v
}

/// Variant — disabled rows (read-only).
#[must_use]
pub fn sample_inspector_disabled_section() -> crate::components::editor::InspectorPanelView {
    let mut v = sample_inspector_style_tab();
    for s in &mut v.sections {
        for r in &mut s.rows {
            r.disabled = true;
        }
    }
    v
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dope_sheet_tracks_have_four_rows() {
        assert_eq!(sample_dope_sheet_tracks().len(), 4);
    }

    #[test]
    fn dope_sheet_dense_adds_a_zoom_track() {
        let tracks = sample_dope_sheet_dense();
        assert_eq!(tracks.len(), 5);
        assert_eq!(tracks.last().unwrap().keyframes.len(), 12);
    }
}
