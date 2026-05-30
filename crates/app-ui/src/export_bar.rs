//! Export bar — start / progress / cancel for the edited export (ED.22).
//!
//! The cutting room's "strike a print" button. One control that drives the
//! whole export lifecycle off the [`ExportUiState`] the
//! [export event bridge](crate::editor_ipc::install_editor_export_listeners)
//! feeds: an **Export** button when idle (or after a finished / failed run),
//! a **progress bar + Cancel** while a print is being struck. The heavy
//! lifting is the backend `editor_export` command; this is the thin,
//! reactive surface over it.

use edit::EditProject;
use leptos::prelude::*;

use crate::editor_ipc::{self, ExportUiState, export_percent};

type ProjectSignal = Option<RwSignal<Option<EditProject>>>;

/// Idle / done / error view: the Export button plus an optional status line.
fn idle_view(
    project: ProjectSignal,
    state: RwSignal<ExportUiState>,
    status: Option<String>,
) -> AnyView {
    view! {
        <div class="export-bar-row">
            <button
                class="export-btn"
                on:click=move |_| {
                    if let Some(p) = project
                        && let Some(proj) = p.get_untracked()
                    {
                        state.set(ExportUiState::Running {
                            done: 0,
                            total: proj.project_duration(),
                        });
                        editor_ipc::editor_export(&proj, "mp4");
                    }
                }
            >
                "Export mp4"
            </button>
            {status.map(|msg| view! { <span class="export-status">{msg}</span> })}
        </div>
    }
    .into_any()
}

/// Running view: a progress bar + percent + Cancel.
fn running_view(done: u64, total: u64) -> AnyView {
    let pct = export_percent(done, total);
    view! {
        <div class="export-bar-row">
            <div class="export-progress">
                <div class="export-progress-fill" style=format!("width:{pct}%")></div>
            </div>
            <span class="export-pct">{format!("{pct}%")}</span>
            <button class="export-cancel" on:click=move |_| editor_ipc::editor_export_cancel()>
                "Cancel"
            </button>
        </div>
    }
    .into_any()
}

/// The export bar. Reads the project + export state from context.
#[component]
pub fn ExportBar() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let state = use_context::<RwSignal<ExportUiState>>()
        .unwrap_or_else(|| RwSignal::new(ExportUiState::Idle));
    view! {
        <div class="export-bar">
            <h3 class="clip-inspector-title">"Export"</h3>
            {move || match state.get() {
                ExportUiState::Idle => idle_view(project, state, None),
                ExportUiState::Running { done, total } => running_view(done, total),
                ExportUiState::Done { path } => {
                    idle_view(project, state, Some(format!("✓ Exported: {path}")))
                }
                ExportUiState::Error { message } => {
                    idle_view(project, state, Some(format!("✗ {message}")))
                }
            }}
        </div>
    }
}
