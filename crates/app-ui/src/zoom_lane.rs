//! Zoom lane — author cinematic punch-ins on the timeline (ED.12 / M-EDIT).
//!
//! The lane that sits the zoom regions ([`edit::zoom::ZoomSegment`]) beneath
//! the video track, one block per region, laid out by the same
//! fraction-of-duration math as the [filmstrip](crate::filmstrip) so every
//! lane stays pixel-aligned. Adding a block here writes a `ZoomSegment` the
//! [zoom engine](../../edit/zoom_anim/index.html) (ED.16) compiles to a
//! push-in at preview + export. Dragging a block's body / edges to
//! move + resize joins the deferred gesture pass; this chunk is the
//! responsive layout + add / select / remove.

use edit::EditProject;
use edit::zoom::ZoomId;
use leptos::prelude::*;

use crate::editor_ipc::EditorStatus;

/// A laid-out zoom region: its fractional position across the project plus
/// its amount and a label.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoomBlock {
    /// Stable id of the underlying [`edit::zoom::ZoomSegment`].
    pub id: ZoomId,
    /// Left edge as a fraction `0..=1` of the project duration.
    pub start_fraction: f64,
    /// Width as a fraction `0..=1` of the project duration.
    pub width_fraction: f64,
    /// Zoom factor at the hold (e.g. `1.6`).
    pub amount: f64,
    /// Amount label, e.g. `"1.6×"`.
    pub label: String,
}

#[allow(
    clippy::cast_precision_loss,
    reason = "frame counts are well under 2^52; u64→f64 is lossless at these magnitudes"
)]
fn fraction(part: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    part as f64 / total as f64
}

/// Lay out the project's zoom regions as proportional blocks (left + width
/// fractions of the project duration), in list order.
#[must_use]
pub fn zoom_spans(project: &EditProject) -> Vec<ZoomBlock> {
    let total = project.project_duration();
    project
        .zooms
        .iter()
        .map(|z| ZoomBlock {
            id: z.id,
            start_fraction: fraction(z.start, total),
            width_fraction: fraction(z.len(), total),
            amount: z.amount,
            label: format!("{:.1}×", z.amount),
        })
        .collect()
}

/// The zoom lane: the project's zoom regions as selectable blocks, plus a
/// "+ Zoom" affordance that drops a default region at the playhead. Reads
/// the project, edit history, playhead, and zoom selection from context.
#[component]
pub fn ZoomLane() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let history = use_context::<StoredValue<Option<edit::History>>>();
    let status = use_context::<RwSignal<EditorStatus>>()
        .unwrap_or_else(|| RwSignal::new(EditorStatus::default()));
    let selection =
        use_context::<RwSignal<Option<ZoomId>>>().unwrap_or_else(|| RwSignal::new(None));
    view! {
        <div class="timeline-lane timeline-lane--zoom" aria-label="Zoom track">
            <button
                class="zoom-lane-add"
                title="Add a zoom at the playhead"
                on:click=move |_| {
                    if let (Some(p), Some(h)) = (project, history) {
                        crate::editor_edits::add_zoom_default(p, h, status.get_untracked().current_frame);
                    }
                }
            >
                "+ Zoom"
            </button>
            <button
                class="zoom-lane-add zoom-lane-add--cursor"
                title="Add a zoom that punches in on the cursor at the playhead"
                on:click=move |_| {
                    if let (Some(p), Some(h)) = (project, history) {
                        crate::editor_edits::add_zoom_at_cursor(p, h, status.get_untracked().current_frame);
                    }
                }
            >
                "+ Cursor"
            </button>
            {move || {
                let spans = project
                    .and_then(|signal| signal.get().as_ref().map(zoom_spans))
                    .unwrap_or_default();
                spans
                    .into_iter()
                    .map(|block| {
                        let id = block.id;
                        let is_selected = move || selection.get() == Some(id);
                        let style = format!(
                            "left:{:.3}%;width:{:.3}%",
                            block.start_fraction * 100.0,
                            block.width_fraction * 100.0
                        );
                        view! {
                            <div
                                class="zoom-block"
                                class:zoom-block--selected=is_selected
                                style=style
                                on:click=move |_| selection.set(Some(id))
                            >
                                <span class="zoom-block-label">{block.label}</span>
                                <button
                                    class="zoom-block-remove"
                                    title="Remove zoom"
                                    on:click=move |ev| {
                                        ev.stop_propagation();
                                        if let (Some(p), Some(h)) = (project, history) {
                                            crate::editor_edits::remove_zoom(p, h, id);
                                            selection.set(None);
                                        }
                                    }
                                >
                                    "×"
                                </button>
                            </div>
                        }
                    })
                    .collect_view()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edit::zoom::ZoomSegment;
    use edit::{ClipRef, EditProject};
    use std::path::PathBuf;

    fn project() -> EditProject {
        // 900 project frames @ 30 fps.
        EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/rec.mp4"),
            1920,
            1080,
            30,
            900,
        ))
    }

    #[test]
    fn empty_when_no_zooms() {
        assert!(zoom_spans(&project()).is_empty());
    }

    #[test]
    fn zoom_spans_are_proportional() {
        let mut p = project();
        p.zooms = vec![
            ZoomSegment::manual(ZoomId(1), 0, 300, 1.5),
            ZoomSegment::manual(ZoomId(2), 450, 900, 2.0),
        ];
        let spans = zoom_spans(&p);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].id, ZoomId(1));
        assert!((spans[0].start_fraction).abs() < 1e-9);
        assert!((spans[0].width_fraction - 1.0 / 3.0).abs() < 1e-6);
        assert_eq!(spans[0].label, "1.5×");
        // Second block: starts halfway, runs to the end.
        assert!((spans[1].start_fraction - 0.5).abs() < 1e-6);
        assert!((spans[1].width_fraction - 0.5).abs() < 1e-6);
        assert_eq!(spans[1].label, "2.0×");
    }
}
