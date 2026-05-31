//! Editor drop-zone smoke gates.
//!
//! Proves the Record→Edit promise: a video **dropped into** the editor (the
//! `open_in_editor` / drop-zone load path) becomes an [`EditProject`] on
//! which the entire editor feature set works — every timeline + dopesheet +
//! framing operation, undo/redo, auto-zoom from telemetry, project
//! persistence, and the export-frame walk.
//!
//! These are integration *smoke* gates: they guard that the features
//! **compose correctly on a dropped clip**, which the per-feature unit tests
//! can't. The drop handler builds its project with
//! [`project_from_metadata`] (after a `gst-discoverer` probe), so the edit
//! gates drive that exact project shape — and since every edit operation is
//! pure (no decode / GPU), those gates run on **every** platform. Two extra
//! gates exercise the parts that genuinely touch the media stack (the real
//! `GStreamer` probe-load and the `wgpu` export walk) and are guarded so a
//! tool-less / adapter-less environment skips them cleanly. macOS is the
//! truth runner where all five execute.
//!
//! ## Coverage
//!
//! - ED.1/ED.5 — `EditProject` / `project_from_metadata` (the drop output).
//! - ED.2 — undo/redo. ED.8 — segment↔project time mapping.
//! - ED.11 — split / trim / ripple. ED.12 — zoom add/move/remove.
//! - ED.13 — dopesheet keyframes + Easy-Ease. ED.14 — per-segment speed.
//! - ED.15/ED.18 — crop / aspect / background. ED.16 — zoom animation
//!   sampling. ED.17 — auto-zoom from clicks. ED.19 — cursor styling.
//! - ED.23 — `.screenproj` round-trip.
//! - ED.3 — real `EditorVideoStream` probe-load (gst-guarded).
//! - ED.20/ED.21 — export-frame generator walk (gst + wgpu guarded).

use std::path::{Path, PathBuf};

use decode::EditorVideoStream;
use decode::gstreamer_pipe::gstreamer_available;
use edit::style::{AspectRatio, BackgroundConfig, CropRect, CursorConfig};
use edit::telemetry::{ClickEvent, auto_zoom_segments};
use edit::zoom::{EditEase, ZoomId, ZoomSegment};
use edit::zoom_anim::{active_zoom_at, default_ramp_frames, zoom_keyframes};
use edit::{AutoZoomConfig, EditOp, EditProject, History, TrimEdge};
use screen_app::editor_command::project_from_metadata;
use screen_app::editor_export::ExportFrameGenerator;

const FIXTURE: &str = "../decode/tests/fixtures/sample.mp4";

/// The project a dropped clip produces. `project_from_metadata` is exactly
/// what `open_in_editor` builds after probing the dropped file; here we feed
/// it representative 1080p / 30 fps / 10 s metadata so the (pure) edit gates
/// run on every platform without a decode. The real probe-load is covered by
/// [`gstreamer_probe_loads_the_dropped_clip`].
fn dropped_clip_project() -> EditProject {
    project_from_metadata(PathBuf::from(FIXTURE), 1920, 1080, 30.0, 300)
}

#[test]
fn a_dropped_clip_is_a_full_length_editable_project() {
    let p = dropped_clip_project();
    // A freshly dropped clip is one full-length, real-time segment with no
    // edits applied — ready to edit.
    assert_eq!(
        p.segments.len(),
        1,
        "drop loads a single full-length segment"
    );
    assert!(p.source.width > 0 && p.source.height > 0, "real dimensions");
    assert_eq!(
        p.project_duration(),
        300,
        "full clip length on the timeline"
    );
    assert!(p.zooms.is_empty() && p.crop.is_none());
    assert_eq!(p.aspect, AspectRatio::Wide);
    p.check_invariants()
        .expect("a freshly dropped project is valid");
}

#[test]
fn timeline_editing_and_undo_redo_on_a_dropped_clip() {
    // ED.11 split / trim, ED.14 speed, ED.8 time mapping, ED.2 undo/redo.
    let p = dropped_clip_project();
    let dur0 = p.project_duration();
    let mut hist = History::new(p);

    // The razor: split the clip under the playhead into two.
    hist.apply(&EditOp::Split { at: dur0 / 2 }).unwrap();
    assert_eq!(hist.project().segments.len(), 2, "split → two segments");
    hist.project().check_invariants().unwrap();

    // Speed the first segment 2×: the timeline shortens (ED.8 time mapping).
    hist.apply(&EditOp::SetSpeed {
        index: 0,
        timescale: 2.0,
    })
    .unwrap();
    assert!(
        hist.project().project_duration() < dur0,
        "a 2× segment shortens the project"
    );

    // Trim the second segment's out-point inward.
    let seg1_end = hist.project().segments[1].source_end;
    hist.apply(&EditOp::Trim {
        index: 1,
        edge: TrimEdge::End,
        to: seg1_end.saturating_sub(5),
    })
    .unwrap();
    hist.project().check_invariants().unwrap();

    // Undo back to the freshly-dropped state, then redo to the edit (the
    // trim bin loses nothing).
    let edited = hist.project().clone();
    while hist.undo() {}
    assert_eq!(hist.project().segments.len(), 1, "undo → back to the drop");
    assert_eq!(hist.project().project_duration(), dur0);
    while hist.redo() {}
    assert_eq!(*hist.project(), edited, "redo restores the edit");
}

#[test]
fn zoom_lane_and_dopesheet_on_a_dropped_clip() {
    // ED.12 zoom add/move/remove, ED.13 dopesheet keyframes + ease,
    // ED.16 zoom-animation sampling.
    let p = dropped_clip_project();
    let fps = p.project_fps;
    let mut hist = History::new(p);

    hist.apply(&EditOp::AddZoom {
        zoom: ZoomSegment::manual(ZoomId(0), 10, 40, 1.8),
    })
    .unwrap();
    hist.apply(&EditOp::AddZoom {
        zoom: ZoomSegment::manual(ZoomId(0), 60, 90, 2.2),
    })
    .unwrap();
    let first_zoom = hist.project().zooms[0].id;
    hist.apply(&EditOp::MoveZoom {
        id: first_zoom,
        start: 5,
        end: 35,
    })
    .unwrap();
    hist.apply(&EditOp::SetZoomEase {
        id: first_zoom,
        ease: EditEase::InOutSine,
    })
    .unwrap();
    assert_eq!(hist.project().zooms.len(), 2, "two zoom regions authored");
    hist.project().check_invariants().unwrap();

    // The dopesheet view + the zoom engine agree: keyframes bracket the hold
    // at identity and peak above it; the sampled transform punches in inside
    // the window and is identity at its edge / outside it.
    let z0 = hist.project().zooms[0];
    let kfs = zoom_keyframes(&z0, default_ramp_frames(fps));
    assert!(kfs.len() >= 3, "at least start / peak / end keyframes");
    assert!(
        (kfs.first().unwrap().scale - 1.0).abs() < 1e-6,
        "opens at identity"
    );
    assert!(
        (kfs.last().unwrap().scale - 1.0).abs() < 1e-6,
        "closes at identity"
    );
    assert!(kfs.iter().any(|k| k.scale > 1.0), "peaks above identity");
    let mid = u64::midpoint(z0.start, z0.end);
    assert!(
        active_zoom_at(hist.project(), mid).scale > 1.0,
        "zoomed mid-window"
    );
    assert!(
        (active_zoom_at(hist.project(), z0.start).scale - 1.0).abs() < 1e-6,
        "identity at the zoom's leading edge"
    );
    assert!(
        active_zoom_at(hist.project(), z0.end + 5).is_identity(),
        "no zoom just past the window"
    );

    // Removing a zoom is undoable.
    hist.apply(&EditOp::RemoveZoom { id: first_zoom }).unwrap();
    assert_eq!(hist.project().zooms.len(), 1);
    hist.undo();
    assert_eq!(
        hist.project().zooms.len(),
        2,
        "undo restores the removed zoom"
    );
}

#[test]
fn framing_and_screenproj_round_trip_on_a_dropped_clip() {
    // ED.15/ED.18 crop / aspect / background, ED.19 cursor, ED.23 persistence.
    let p = dropped_clip_project();
    let mut hist = History::new(p);

    hist.apply(&EditOp::SetCrop {
        rect: CropRect {
            x: 0.1,
            y: 0.1,
            width: 0.8,
            height: 0.8,
        },
    })
    .unwrap();
    assert!(hist.project().crop.is_some(), "crop applied");
    hist.apply(&EditOp::SetAspect {
        ratio: AspectRatio::Vertical,
    })
    .unwrap();
    assert_eq!(hist.project().aspect, AspectRatio::Vertical);
    hist.apply(&EditOp::SetBackground {
        config: BackgroundConfig {
            padding: 96,
            ..BackgroundConfig::default()
        },
    })
    .unwrap();
    assert_eq!(hist.project().background.padding, 96);
    hist.apply(&EditOp::SetCursor {
        cursor: CursorConfig {
            size_pct: 220,
            ..CursorConfig::default()
        },
    })
    .unwrap();
    assert_eq!(hist.project().cursor.size_pct, 220);
    hist.project().check_invariants().unwrap();

    // The framed project round-trips through `.screenproj` losslessly.
    let json = edit::persist::to_screenproj(hist.project()).unwrap();
    let reloaded = edit::persist::from_screenproj(&json).unwrap();
    assert_eq!(
        reloaded,
        *hist.project(),
        "framed project persists losslessly"
    );
}

#[test]
fn auto_zoom_from_click_telemetry_yields_editable_zooms() {
    let p = dropped_clip_project();
    let fps = p.project_fps;
    // ED.17 — a click log over the dropped clip's timeline yields editable
    // zoom regions. Two clusters (~2 s apart) → two non-overlapping zooms.
    let clicks = [
        ClickEvent::new(u64::from(fps), 0.25, 0.40),
        ClickEvent::new(u64::from(fps) + 10, 0.30, 0.45),
        ClickEvent::new(u64::from(fps) * 3, 0.70, 0.60),
    ];
    let autos = auto_zoom_segments(&clicks, fps, &AutoZoomConfig::default());
    assert!(!autos.is_empty(), "clicks produce auto-zoom regions");
    assert!(
        autos.windows(2).all(|w| w[0].end <= w[1].start),
        "auto-zoom windows never overlap"
    );

    // They are ordinary editable zooms — feeding them back through the
    // command stack is accepted and keeps the project valid.
    let mut hist = History::new(p);
    for z in &autos {
        hist.apply(&EditOp::AddZoom { zoom: *z }).unwrap();
    }
    assert_eq!(hist.project().zooms.len(), autos.len());
    hist.project().check_invariants().unwrap();
}

#[test]
fn gstreamer_probe_loads_the_dropped_clip() {
    // ED.3 — the real drop-load path: probe the fixture and build its
    // project. gst-guarded (macOS truth runner exercises it).
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping real probe-load gate");
        return;
    }
    let probe = EditorVideoStream::open(Path::new(FIXTURE)).expect("probe the dropped clip");
    let project = project_from_metadata(
        PathBuf::from(FIXTURE),
        probe.width(),
        probe.height(),
        probe.frame_rate(),
        probe.frame_count().unwrap_or(0).max(2),
    );
    assert_eq!(project.source.width, probe.width());
    assert_eq!(project.source.height, probe.height());
    assert_eq!(
        project.segments.len(),
        1,
        "probe-loaded clip is one segment"
    );
    assert!(project.project_duration() > 0);
    project.check_invariants().unwrap();
}

#[test]
fn the_edited_dropped_clip_exports_composed_frames() {
    // ED.20/ED.21 — walk an *edited* timeline of the real dropped clip into
    // composed BGRA. Needs gst (decode) + wgpu (compose); guarded.
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping export-walk gate");
        return;
    }
    let Ok(probe) = EditorVideoStream::open(Path::new(FIXTURE)) else {
        eprintln!("could not probe the fixture — skipping export-walk gate");
        return;
    };
    let project = project_from_metadata(
        PathBuf::from(FIXTURE),
        probe.width(),
        probe.height(),
        probe.frame_rate(),
        probe.frame_count().unwrap_or(0).max(2),
    );
    drop(probe);

    let mut hist = History::new(project);
    hist.apply(&EditOp::SetSpeed {
        index: 0,
        timescale: 2.0,
    })
    .unwrap();

    let mut generator = match ExportFrameGenerator::new(hist.project().clone(), Path::new(FIXTURE))
    {
        Ok(g) => g,
        // No wgpu adapter (headless / software) — skip rather than fail.
        Err(err) => {
            eprintln!("export compose pipeline unavailable ({err}) — skipping");
            return;
        }
    };
    assert!(
        generator.frame_count() > 0,
        "the edited timeline has frames"
    );

    let mut produced = 0u32;
    while let Some(frame) = generator.next_frame() {
        assert!(!frame.frame.bytes.is_empty(), "each frame composes to BGRA");
        produced += 1;
        if produced >= 8 {
            break; // a smoke walk — the first frames prove the path
        }
    }
    assert!(produced > 0, "the timeline walk produced composed frames");
    assert_eq!(
        generator.spawn_count(),
        1,
        "a forward walk never re-spawns the decode pipe"
    );
}
