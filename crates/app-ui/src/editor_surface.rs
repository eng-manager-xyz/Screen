//! `EditorSurface` — the live editor surface (ED.5 / M-EDIT).
//!
//! Replaces the `?surface=editor` placeholder with the real
//! [`EditorShell`] chrome, driven by the loaded [`EditProject`] from a
//! Leptos context signal. The canvas / timeline / inspector slots are
//! filled by later chunks (preview ED.6, transport ED.7, timeline ED.8,
//! inspector ED.18); this chunk activates the surface and the
//! Record→Edit handoff.

use edit::EditProject;
use leptos::prelude::*;
use ui_storybook::components::editor::{EditorShell, EditorShellView, ToolbarActionView};

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

/// The editor surface. Reads the loaded [`EditProject`] from context (set
/// by the `open_in_editor` handoff) and renders the [`EditorShell`].
#[component]
pub fn EditorSurface() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    view! {
        <section class="app-surface app-surface--editor">
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
                                        "Preview & timeline come online next."
                                    } else {
                                        "Finish a recording to start editing it here."
                                    }}
                                </p>
                            </div>
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
        // The aspect chip starts selected (matches the design).
        assert!(
            v.toolbar_actions
                .iter()
                .any(|a| a.id == "aspect" && a.selected)
        );
    }
}
