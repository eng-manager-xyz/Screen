//! `EditorSurface` — the live editor surface (ED.5/ED.7 / M-EDIT).
//!
//! Renders the real [`EditorShell`] chrome driven by the loaded
//! [`EditProject`], with a wired transport bar (ED.7) in the timeline slot:
//! play/pause, frame-step, jump-to-ends, a scrubber, a speed selector, a
//! `MM:SS.ff` timecode, and keyboard shortcuts — all driving the backend
//! editor session over [`crate::editor_ipc`]. The canvas / inspector slots
//! are filled by later chunks (preview ED.6, inspector ED.18).

use edit::EditProject;
use leptos::prelude::*;
use ui_storybook::components::editor::{EditorShell, EditorShellView, ToolbarActionView};

use crate::editor_ipc::{self, EditorStatus, TransportAction};

/// The editor toolbar action set (matches the reference design). When no
/// clip is loaded every action is disabled.
fn default_toolbar(loaded: bool) -> Vec<ToolbarActionView> {
    let disabled = !loaded;
    let action = |id, label, icon, selected| ToolbarActionView {
        id,
        label,
        icon,
        selected,
        disabled,
    };
    vec![
        action("aspect", "16:9", "▭", true),
        action("split", "Split", "✂", false),
        action("trim", "Trim", "⇥", false),
        action("crop", "Crop", "⛶", false),
        action("zoom", "Zoom", "⌖", false),
        action("annotate", "Annotate", "✎", false),
        action("captions", "Captions", "≡", false),
    ]
}

/// File name of the clip (the document title).
fn clip_title(project: &EditProject) -> String {
    project
        .source
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("recording")
        .to_owned()
}

/// `"1920×1080 · 30 fps · m:ss"` subtitle from the clip metadata.
fn clip_subtitle(project: &EditProject) -> String {
    let src = &project.source;
    let secs = if src.source_fps > 0 {
        src.frame_count / u64::from(src.source_fps)
    } else {
        0
    };
    let (mins, rem) = (secs / 60, secs % 60);
    format!(
        "{}×{} · {} fps · {mins}:{rem:02}",
        src.width, src.height, src.source_fps
    )
}

/// Format a frame index as `MM:SS.ff` (ff = frame within the second).
#[must_use]
pub fn format_timecode(frame: u64, fps: u32) -> String {
    let fps = u64::from(fps.max(1));
    let total_secs = frame / fps;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let frames = frame % fps;
    format!("{mins:02}:{secs:02}.{frames:02}")
}

/// Map the loaded project (or its absence) to the shell view-model.
fn shell_view_for(project: Option<&EditProject>) -> EditorShellView {
    match project {
        Some(p) => EditorShellView {
            document_title: clip_title(p),
            document_subtitle: Some(clip_subtitle(p)),
            has_clip_loaded: true,
            toolbar_actions: default_toolbar(true),
            export_enabled: true,
            share_enabled: true,
        },
        None => EditorShellView {
            document_title: String::new(),
            document_subtitle: None,
            has_clip_loaded: false,
            toolbar_actions: default_toolbar(false),
            export_enabled: false,
            share_enabled: false,
        },
    }
}

/// Transport bar (ED.7) — bound to the backend editor session via
/// [`crate::editor_ipc`]. Fine-grained reactive: only the timecode, the
/// play glyph, and the scrubber value re-render as the playhead advances.
#[component]
fn EditorTransportBar() -> impl IntoView {
    let status = use_context::<RwSignal<EditorStatus>>()
        .unwrap_or_else(|| RwSignal::new(EditorStatus::default()));
    view! {
        <div class="editor-transport" role="group" aria-label="Playback transport">
            <button class="transport-btn" title="Jump to start"
                on:click=move |_| editor_ipc::editor_transport(&TransportAction::Seek { frame: 0 })>
                "⏮"
            </button>
            <button class="transport-btn" title="Step back"
                on:click=move |_| editor_ipc::editor_transport(&TransportAction::Step { delta: -1 })>
                "‹"
            </button>
            <button class="transport-btn transport-play" title="Play / pause (Space)"
                on:click=move |_| editor_ipc::editor_transport(&TransportAction::TogglePlay)>
                {move || if status.get().playing { "⏸" } else { "▶" }}
            </button>
            <button class="transport-btn" title="Step forward"
                on:click=move |_| editor_ipc::editor_transport(&TransportAction::Step { delta: 1 })>
                "›"
            </button>
            <button class="transport-btn" title="Jump to end"
                on:click=move |_| editor_ipc::editor_transport(&TransportAction::Seek {
                    frame: status.get_untracked().duration_frames.saturating_sub(1),
                })>
                "⏭"
            </button>
            <span class="transport-timecode">
                {move || format_timecode(status.get().current_frame, status.get().fps)}
                " / "
                {move || format_timecode(status.get().duration_frames.saturating_sub(1), status.get().fps)}
            </span>
            <input
                type="range"
                class="transport-scrub"
                min="0"
                prop:max=move || status.get().duration_frames.saturating_sub(1).to_string()
                prop:value=move || status.get().current_frame.to_string()
                on:input=move |ev| {
                    if let Ok(frame) = event_target_value(&ev).parse::<u64>() {
                        editor_ipc::editor_transport(&TransportAction::Seek { frame });
                    }
                }
            />
            <select class="transport-rate" title="Playback speed"
                on:change=move |ev| {
                    if let Ok(rate) = event_target_value(&ev).parse::<f32>() {
                        editor_ipc::editor_transport(&TransportAction::SetRate { rate });
                    }
                }>
                <option value="0.5">"0.5×"</option>
                <option value="1" selected=true>"1×"</option>
                <option value="2">"2×"</option>
            </select>
        </div>
    }
}

/// Handle a keydown on the editor surface: transport (Space / arrows /
/// I·O) and edit (S split, ⌘Z undo·redo, Delete ripple) shortcuts.
fn handle_editor_keydown(
    ev: &web_sys::KeyboardEvent,
    project: Option<RwSignal<Option<EditProject>>>,
    history: Option<StoredValue<Option<edit::History>>>,
    selection: Option<RwSignal<Option<usize>>>,
    status: RwSignal<EditorStatus>,
) {
    match ev.key().as_str() {
        " " | "Spacebar" => {
            ev.prevent_default();
            editor_ipc::editor_transport(&TransportAction::TogglePlay);
        }
        "ArrowLeft" => {
            ev.prevent_default();
            let delta = if ev.shift_key() { -5 } else { -1 };
            editor_ipc::editor_transport(&TransportAction::Step { delta });
        }
        "ArrowRight" => {
            ev.prevent_default();
            let delta = if ev.shift_key() { 5 } else { 1 };
            editor_ipc::editor_transport(&TransportAction::Step { delta });
        }
        "i" | "I" => {
            let s = status.get_untracked();
            editor_ipc::editor_transport(&TransportAction::SetInOut {
                a: s.current_frame,
                b: s.out_frame.max(s.current_frame + 1),
            });
        }
        "o" | "O" => {
            let s = status.get_untracked();
            editor_ipc::editor_transport(&TransportAction::SetInOut {
                a: s.in_frame.min(s.current_frame),
                b: s.current_frame + 1,
            });
        }
        // ED.11 — the razor: split the clip under the playhead.
        "s" | "S" => {
            ev.prevent_default();
            if let (Some(p), Some(h)) = (project, history) {
                crate::editor_edits::split_at(p, h, status.get_untracked().current_frame);
            }
        }
        // ED.11 — the trim bin: undo / redo (⌘Z / ⌘⇧Z).
        "z" | "Z" if ev.meta_key() || ev.ctrl_key() => {
            ev.prevent_default();
            if let (Some(p), Some(h)) = (project, history) {
                if ev.shift_key() {
                    crate::editor_edits::redo(p, h);
                } else {
                    crate::editor_edits::undo(p, h);
                }
            }
        }
        // ED.11 — ripple-delete the selected clip (close the gap).
        "Delete" | "Backspace" => {
            ev.prevent_default();
            if let (Some(p), Some(h)) = (project, history) {
                let sel = selection.and_then(|s| s.get_untracked());
                crate::editor_edits::ripple_delete_selected(p, h, sel);
            }
        }
        _ => {}
    }
}

/// The editor surface. Reads the loaded [`EditProject`] + the playhead
/// [`EditorStatus`] from context and renders the [`EditorShell`] with a
/// wired transport. Keyboard: Space = play/pause, ←/→ step (Shift = 5),
/// I/O set in/out, S split, ⌘Z undo·redo, Delete ripple-delete.
#[component]
pub fn EditorSurface() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let status = use_context::<RwSignal<EditorStatus>>()
        .unwrap_or_else(|| RwSignal::new(EditorStatus::default()));
    let history = use_context::<StoredValue<Option<edit::History>>>();
    let selection = use_context::<RwSignal<Option<usize>>>();
    view! {
        <section
            class="app-surface app-surface--editor"
            tabindex="0"
            on:keydown=move |ev| handle_editor_keydown(&ev, project, history, selection, status)
        >
            {move || {
                let vm = match project {
                    Some(signal) => shell_view_for(signal.get().as_ref()),
                    None => shell_view_for(None),
                };
                let loaded = vm.has_clip_loaded;
                view! {
                    <EditorShell
                        view=vm
                        canvas=ToChildren::to_children(move || view! {
                            <div class="editor-canvas-empty">
                                <p class="editor-canvas-hint">
                                    {if loaded {
                                        "Preview renders here."
                                    } else {
                                        "Finish a recording to start editing it here."
                                    }}
                                </p>
                            </div>
                        })
                        inspector=ToChildren::to_children(move || view! {
                            <crate::style_inspector::StyleInspector />
                            <crate::framing_inspector::FramingInspector />
                            <crate::clip_inspector::ClipInspector />
                        })
                        timeline=ToChildren::to_children(move || view! {
                            <crate::timeline_view::TimelineRuler />
                            <crate::filmstrip::VideoFilmstrip />
                            <crate::zoom_lane::ZoomLane />
                            <crate::waveform::AudioWaveform />
                            <EditorTransportBar />
                        })
                    />
                }
            }}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edit::ClipRef;
    use std::path::PathBuf;

    fn project() -> EditProject {
        EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/Screen-2026-05-30.mp4"),
            1920,
            1080,
            30,
            900,
        ))
    }

    #[test]
    fn empty_view_has_no_clip_and_disabled_actions() {
        let v = shell_view_for(None);
        assert!(!v.has_clip_loaded);
        assert!(!v.export_enabled);
        assert!(v.toolbar_actions.iter().all(|a| a.disabled));
        assert!(v.document_subtitle.is_none());
    }

    #[test]
    fn loaded_view_titles_and_enables() {
        let p = project();
        let v = shell_view_for(Some(&p));
        assert!(v.has_clip_loaded);
        assert!(v.export_enabled);
        assert_eq!(v.document_title, "Screen-2026-05-30.mp4");
        assert_eq!(
            v.document_subtitle.as_deref(),
            Some("1920×1080 · 30 fps · 0:30")
        );
        assert!(v.toolbar_actions.iter().all(|a| !a.disabled));
        assert!(
            v.toolbar_actions
                .iter()
                .any(|a| a.id == "aspect" && a.selected)
        );
    }

    #[test]
    fn timecode_formats_mm_ss_ff() {
        assert_eq!(format_timecode(0, 30), "00:00.00");
        assert_eq!(format_timecode(75, 30), "00:02.15"); // 2s + 15 frames
        assert_eq!(format_timecode(1800, 30), "01:00.00"); // 60s
        assert_eq!(format_timecode(29, 30), "00:00.29");
        // fps 0 is treated as 1 (no div-by-zero).
        assert_eq!(format_timecode(5, 0), "00:05.00");
    }
}
