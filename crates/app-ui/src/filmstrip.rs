//! Video track filmstrip + clip selection (ED.9 / M-EDIT).
//!
//! [`segment_spans`] is the pure layout: each [`edit::TimelineSegment`]
//! becomes a proportional span (`start_fraction` / `width_fraction`) of the
//! project so the video lane lays out responsively at any width. The
//! [`VideoFilmstrip`] component renders those spans as selectable clip
//! blocks; the selection (a `RwSignal<Option<usize>>` in context) drives
//! the inspector (ED.18).
//!
//! Per-clip thumbnail images decode + downscale through `EditorVideoStream`
//! and land with the render-integration pass; this chunk is the responsive
//! segment layout + selection + duration labels.

use edit::EditProject;
use leptos::prelude::*;

/// A laid-out segment: its fractional position across the project plus its
/// source range and a duration label.
#[derive(Clone, Debug, PartialEq)]
pub struct SegmentSpan {
    /// Index into [`EditProject::segments`].
    pub index: usize,
    /// Left edge as a fraction `0..=1` of the project duration.
    pub start_fraction: f64,
    /// Width as a fraction `0..=1` of the project duration.
    pub width_fraction: f64,
    /// Source in-point (frame).
    pub source_start: u64,
    /// Source out-point (frame).
    pub source_end: u64,
    /// Playback speed multiplier.
    pub timescale: f64,
    /// `m:ss` duration label (project time).
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

fn label_for(project_len: u64, fps: u32) -> String {
    let secs = project_len / u64::from(fps.max(1));
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// Lay out the project's segments as proportional spans (left + width
/// fractions), in timeline order.
#[must_use]
pub fn segment_spans(project: &EditProject) -> Vec<SegmentSpan> {
    let total = project.project_duration();
    let fps = project.project_fps;
    let mut acc = 0u64;
    project
        .segments
        .iter()
        .enumerate()
        .map(|(index, seg)| {
            let project_len = seg.project_len();
            let span = SegmentSpan {
                index,
                start_fraction: fraction(acc, total),
                width_fraction: fraction(project_len, total),
                source_start: seg.source_start,
                source_end: seg.source_end,
                timescale: seg.timescale,
                label: label_for(project_len, fps),
            };
            acc += project_len;
            span
        })
        .collect()
}

/// The video lane: the project's segments as selectable clip blocks,
/// positioned by fraction so the lane is responsive. Click selects a clip
/// (drives the inspector). Reads the project + selection from context.
#[component]
pub fn VideoFilmstrip() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let selection = use_context::<RwSignal<Option<usize>>>().unwrap_or_else(|| RwSignal::new(None));
    view! {
        <div class="timeline-lane timeline-lane--video" aria-label="Video track">
            {move || {
                let spans = project
                    .and_then(|signal| signal.get().as_ref().map(segment_spans))
                    .unwrap_or_default();
                spans
                    .into_iter()
                    .map(|span| {
                        let index = span.index;
                        let is_selected = move || selection.get() == Some(index);
                        let style = format!(
                            "left:{:.3}%;width:{:.3}%",
                            span.start_fraction * 100.0,
                            span.width_fraction * 100.0
                        );
                        view! {
                            <button
                                class="filmstrip-clip"
                                class:filmstrip-clip--selected=is_selected
                                style=style
                                on:click=move |_| selection.set(Some(index))
                            >
                                <span class="filmstrip-clip-label">{span.label}</span>
                            </button>
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
    use edit::{ClipRef, EditProject, TimelineSegment};
    use std::path::PathBuf;

    fn project_with(segments: Vec<TimelineSegment>) -> EditProject {
        let mut p = EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/rec.mp4"),
            1920,
            1080,
            30,
            900,
        ));
        p.segments = segments;
        p
    }

    #[test]
    fn single_segment_spans_full_width() {
        let p = project_with(vec![TimelineSegment::new(0, 900)]);
        let spans = segment_spans(&p);
        assert_eq!(spans.len(), 1);
        assert!((spans[0].start_fraction).abs() < 1e-9);
        assert!((spans[0].width_fraction - 1.0).abs() < 1e-9);
        assert_eq!(spans[0].label, "0:30"); // 900 frames @ 30 fps
    }

    #[test]
    fn segments_are_proportional_and_contiguous() {
        // 300 + 600 = 900 project frames → 1/3 and 2/3.
        let p = project_with(vec![
            TimelineSegment::new(0, 300),
            TimelineSegment::new(300, 900),
        ]);
        let spans = segment_spans(&p);
        assert_eq!(spans.len(), 2);
        assert!((spans[0].start_fraction - 0.0).abs() < 1e-9);
        assert!((spans[0].width_fraction - 1.0 / 3.0).abs() < 1e-6);
        // Second clip starts where the first ends.
        assert!((spans[1].start_fraction - 1.0 / 3.0).abs() < 1e-6);
        assert!((spans[1].width_fraction - 2.0 / 3.0).abs() < 1e-6);
        // Spans tile the lane (start + width of the last == 1.0).
        let end = spans[1].start_fraction + spans[1].width_fraction;
        assert!((end - 1.0).abs() < 1e-6);
        assert_eq!(spans[0].index, 0);
        assert_eq!(spans[1].index, 1);
    }

    #[test]
    fn speed_segment_width_reflects_project_length() {
        // A 2× segment occupies half its source span in project time.
        let p = project_with(vec![
            TimelineSegment::new(0, 300),               // 300 project frames
            TimelineSegment::with_speed(300, 900, 2.0), // 600 src → 300 project frames
        ]);
        let spans = segment_spans(&p);
        // Both occupy 300 project frames → equal widths.
        assert!((spans[0].width_fraction - spans[1].width_fraction).abs() < 1e-6);
    }
}
