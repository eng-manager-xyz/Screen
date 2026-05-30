//! Clip inspector — per-segment controls (ED.14: speed) (M-EDIT).
//!
//! The right-hand inspector for the *selected* clip. Today it carries the
//! speed control: a row of multiplier presets that retime one segment via
//! `EditOp::SetSpeed`. Because a segment's `timescale` changes how many
//! project frames it occupies, setting speed reshapes the timeline — the
//! filmstrip re-flows and the playback clock re-syncs automatically (the
//! edit runs through the shared [`edit::History`], which pushes the new
//! duration to the backend clock). The Style + Cursor tabs (ED.18 / ED.19)
//! grow this same panel.

use edit::EditProject;
use leptos::prelude::*;

/// Speed presets offered in the inspector (playback multipliers). `1.0` is
/// real-time; `< 1` is slow-motion, `> 1` is fast-forward.
pub const SPEED_PRESETS: [f64; 5] = [0.5, 1.0, 1.5, 2.0, 4.0];

/// A compact label for a speed multiplier — `1.0 → "1×"`, `0.5 → "0.5×"`.
/// (Rust's `f64` `Display` drops a whole number's trailing `.0`.)
#[must_use]
pub fn speed_label(timescale: f64) -> String {
    format!("{timescale}×")
}

/// Whether `current` matches `preset` within float tolerance.
#[must_use]
pub fn is_active_speed(current: f64, preset: f64) -> bool {
    (current - preset).abs() < 1e-6
}

/// The speed (`timescale`) of segment `index`, if it exists.
#[must_use]
pub fn selected_segment_speed(project: &EditProject, index: usize) -> Option<f64> {
    project.segments.get(index).map(|s| s.timescale)
}

type ProjectSignal = Option<RwSignal<Option<EditProject>>>;
type HistoryStore = Option<StoredValue<Option<edit::History>>>;

/// The speed section for the selected clip `index` at its `current` speed.
fn speed_panel(
    index: usize,
    current: f64,
    project: ProjectSignal,
    history: HistoryStore,
) -> AnyView {
    let presets = SPEED_PRESETS
        .into_iter()
        .map(|preset| {
            let mut class = String::from("clip-speed-preset");
            if is_active_speed(current, preset) {
                class.push_str(" clip-speed-preset--active");
            }
            view! {
                <button
                    class=class
                    on:click=move |_| {
                        if let (Some(p), Some(h)) = (project, history) {
                            crate::editor_edits::set_speed(p, h, index, preset);
                        }
                    }
                >
                    {speed_label(preset)}
                </button>
            }
        })
        .collect_view();
    view! {
        <div class="clip-inspector-section">
            <h3 class="clip-inspector-title">"Speed"</h3>
            <p class="clip-inspector-current">
                {format!("Clip {} · {}", index + 1, speed_label(current))}
            </p>
            <div class="clip-speed-presets">{presets}</div>
        </div>
    }
    .into_any()
}

/// The per-clip inspector. Reads the clip selection, project, and edit
/// history from context; renders speed presets for the selected clip.
#[component]
pub fn ClipInspector() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let history = use_context::<StoredValue<Option<edit::History>>>();
    let selection = use_context::<RwSignal<Option<usize>>>().unwrap_or_else(|| RwSignal::new(None));
    view! {
        <div class="clip-inspector">
            {move || match selection.get() {
                None => view! {
                    <p class="clip-inspector-empty">"Select a clip to edit its speed."</p>
                }
                .into_any(),
                Some(index) => {
                    let current = project
                        .and_then(|s| s.get().as_ref().and_then(|p| selected_segment_speed(p, index)))
                        .unwrap_or(1.0);
                    speed_panel(index, current, project, history)
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edit::{ClipRef, EditProject, TimelineSegment};
    use std::path::PathBuf;

    fn project() -> EditProject {
        EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/rec.mp4"),
            1920,
            1080,
            30,
            900,
        ))
    }

    #[test]
    fn speed_labels_drop_trailing_zero() {
        assert_eq!(speed_label(1.0), "1×");
        assert_eq!(speed_label(2.0), "2×");
        assert_eq!(speed_label(0.5), "0.5×");
        assert_eq!(speed_label(1.5), "1.5×");
    }

    #[test]
    fn active_speed_uses_tolerance() {
        assert!(is_active_speed(1.0, 1.0));
        assert!(is_active_speed(2.0 + 1e-9, 2.0));
        assert!(!is_active_speed(1.0, 2.0));
    }

    #[test]
    fn selected_speed_reads_timescale() {
        let mut p = project();
        p.segments = vec![
            TimelineSegment::new(0, 300),
            TimelineSegment::with_speed(300, 900, 2.0),
        ];
        assert!((selected_segment_speed(&p, 0).unwrap() - 1.0).abs() < 1e-9);
        assert!((selected_segment_speed(&p, 1).unwrap() - 2.0).abs() < 1e-9);
        assert_eq!(selected_segment_speed(&p, 2), None);
    }
}
