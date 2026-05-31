//! Zoom dopesheet — keyframes + Easy Ease for the selected zoom (ED.13).
//!
//! The animator's dope sheet: the selected [zoom](crate::zoom_lane)'s scale
//! curve laid out as keyframes (identity → full → identity), plus a row of
//! easing presets. The default and one-click favorite is **Easy Ease**
//! (`InOutCubic`) — accelerate off the wide shot, settle into the detail.
//! The keyframe model is the pure [`zoom_keyframes`]
//! (the `Track`-shaped view of the engine's ramp); choosing an ease commits
//! a `SetZoomEase` through the shared [`edit::History`]. The marker plot is
//! authoring/inspection; the eased motion itself renders via the ED.16
//! engine.

use edit::EditProject;
use edit::zoom::{EditEase, ZoomId, ZoomSegment};
use edit::zoom_anim::{default_ramp_frames, zoom_keyframes};
use leptos::prelude::*;

type ProjectSignal = Option<RwSignal<Option<EditProject>>>;
type HistoryStore = Option<StoredValue<Option<edit::History>>>;

/// Ease presets, in display order — Easy Ease (`InOutCubic`) leads.
pub const EASES: [(EditEase, &str); 5] = [
    (EditEase::InOutCubic, "Easy Ease"),
    (EditEase::Linear, "Linear"),
    (EditEase::InCubic, "In"),
    (EditEase::OutCubic, "Out"),
    (EditEase::InOutSine, "Sine"),
];

/// The zoom with `id`, if present. Pure.
#[must_use]
pub fn selected_zoom(project: &EditProject, id: ZoomId) -> Option<ZoomSegment> {
    project.zooms.iter().find(|z| z.id == id).copied()
}

fn frac(part: u64, total: u64) -> f64 {
    let n = u32::try_from(part).unwrap_or(u32::MAX);
    let d = u32::try_from(total.max(1)).unwrap_or(u32::MAX);
    f64::from(n) / f64::from(d)
}

fn keyframe_markers(seg: ZoomSegment, fps: u32) -> AnyView {
    let len = seg.len().max(1);
    zoom_keyframes(&seg, default_ramp_frames(fps))
        .into_iter()
        .map(|kf| {
            let pos = frac(kf.frame.saturating_sub(seg.start), len) * 100.0;
            view! {
                <span
                    class="dopesheet-key"
                    style=format!("left:{pos:.2}%")
                    title=format!("frame {} · {:.2}×", kf.frame, kf.scale)
                ></span>
            }
        })
        .collect_view()
        .into_any()
}

fn dopesheet_body(
    seg: ZoomSegment,
    fps: u32,
    id: ZoomId,
    project: ProjectSignal,
    history: HistoryStore,
) -> AnyView {
    let current = seg.ease;
    let eases = EASES
        .into_iter()
        .map(|(ease, label)| {
            let mut class = String::from("ease-btn");
            if ease == current {
                class.push_str(" ease-btn--active");
            }
            view! {
                <button
                    class=class
                    on:click=move |_| {
                        if let (Some(p), Some(h)) = (project, history) {
                            crate::editor_edits::set_zoom_ease(p, h, id, ease);
                        }
                    }
                >
                    {label}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="dopesheet-track">{keyframe_markers(seg, fps)}</div>
        <div class="dopesheet-eases">{eases}</div>
    }
    .into_any()
}

/// The zoom dopesheet. Reads the zoom selection (ED.12), project, and edit
/// history from context; tunes the selected zoom's keyframes + ease.
#[component]
pub fn ZoomDopesheet() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let history = use_context::<StoredValue<Option<edit::History>>>();
    let selection =
        use_context::<RwSignal<Option<ZoomId>>>().unwrap_or_else(|| RwSignal::new(None));
    view! {
        <div class="dopesheet" aria-label="Zoom dopesheet">
            {move || {
                let selected = selection
                    .get()
                    .and_then(|id| {
                        project
                            .and_then(|s| s.get())
                            .and_then(|p| selected_zoom(&p, id).map(|seg| (id, seg, p.project_fps)))
                    });
                match selected {
                    Some((id, seg, fps)) => dopesheet_body(seg, fps, id, project, history),
                    None => view! {
                        <p class="dopesheet-hint">
                            "Select a zoom block to tune its keyframes + ease."
                        </p>
                    }
                    .into_any(),
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edit::ClipRef;
    use std::path::PathBuf;

    #[test]
    fn eases_lead_with_easy_ease() {
        assert_eq!(EASES.len(), 5);
        assert_eq!(EASES[0].0, EditEase::InOutCubic);
        assert_eq!(EASES[0].1, "Easy Ease");
    }

    #[test]
    fn selected_zoom_finds_by_id() {
        let mut p = EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/a.mp4"),
            1920,
            1080,
            30,
            900,
        ));
        p.zooms = vec![
            ZoomSegment::manual(ZoomId(1), 0, 100, 1.5),
            ZoomSegment::manual(ZoomId(2), 200, 300, 2.0),
        ];
        assert_eq!(selected_zoom(&p, ZoomId(2)).map(|z| z.start), Some(200));
        assert!(selected_zoom(&p, ZoomId(9)).is_none());
    }
}
