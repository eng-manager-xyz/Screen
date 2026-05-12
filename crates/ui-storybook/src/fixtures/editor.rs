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
