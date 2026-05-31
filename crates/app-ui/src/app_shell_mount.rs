//! `AppShellRoot` — root component for the tray-launched main window
//! (M-TRAY.3 / AUT-252) and the in-app `NavigationRail` surface
//! routing (M-TRAY.4 / AUT-253).
//!
//! Owns the `RwSignal<AppSection>` that drives both the rail's
//! `active` prop and the right-pane `match` over surface placeholders.
//! Clicking a rail item flips the signal (via the `on_select`
//! callback added in M-TRAY.2 / AUT-251) and the URL is rewritten to
//! match via `history.replaceState`.
//!
//! Per the M-TRAY.1 audit doc, `AppShell` itself stays pure
//! slot-composition — the section signal lives here in the consumer
//! crate, not inside `ui_storybook::components::shell::AppShell`.

use leptos::prelude::*;
use ui_storybook::components::shell::{
    AppSection, AppShell, NavigationRail, StatusBar, StatusKind,
};
use ui_storybook::fixtures::shell::{sample_nav_items, sample_user_avatar, sample_workspace_badge};

#[component]
pub fn AppShellRoot(initial: AppSection) -> impl IntoView {
    let active = RwSignal::new(initial);

    // ED.5 / M-EDIT — the loaded editor project, shared with the editor
    // surface via context. The Record→Edit handoff (`open_in_editor`)
    // pushes a project here; an effect jumps to the editor when one loads.
    let editor_project = RwSignal::new(None::<edit::EditProject>);
    provide_context(editor_project);
    crate::editor_ipc::install_editor_project_listener(editor_project);
    Effect::new(move |_| {
        if editor_project.get().is_some() {
            active.set(AppSection::Editor);
        }
    });

    // ED.7 — the playhead status from the backend editor session, plus a
    // perpetual host-injected tick that advances the clock while playing.
    // Created once here (the app root), so editor-surface re-mounts can't
    // spawn duplicate tick loops; leaked because it has app lifetime.
    let editor_status = RwSignal::new(crate::editor_ipc::EditorStatus::default());
    provide_context(editor_status);
    crate::editor_ipc::install_editor_status_listener(editor_status);
    gloo_timers::callback::Interval::new(33, move || {
        if editor_status.get_untracked().playing {
            crate::editor_ipc::editor_transport(&crate::editor_ipc::TransportAction::Tick {
                dt_ms: 33,
            });
        }
    })
    .forget();

    // ED.9 — the selected clip index, shared with the video filmstrip and
    // (ED.18) the inspector.
    let editor_selection = RwSignal::new(None::<usize>);
    provide_context(editor_selection);

    // ED.10 — audio waveform peak buckets (populated when the source audio
    // is decoded; empty renders a quiet baseline).
    let editor_peaks = RwSignal::new(Vec::<crate::waveform::WaveBucket>::new());
    provide_context(editor_peaks);

    // ED.11 — the edit history (undo/redo stacks + current project state),
    // resolved against the loaded clip on first edit.
    let editor_history = StoredValue::new(None::<edit::History>);
    provide_context(editor_history);

    // ED.12 — which zoom region is selected on the zoom lane.
    let editor_zoom_selection = RwSignal::new(None::<edit::zoom::ZoomId>);
    provide_context(editor_zoom_selection);

    // ED.22 — export lifecycle state, fed by the export event bridge.
    let editor_export = RwSignal::new(crate::editor_ipc::ExportUiState::Idle);
    provide_context(editor_export);
    crate::editor_ipc::install_editor_export_listeners(editor_export);

    // ED.23 — last-saved `.screenproj` path, set by the `editor-saved` event.
    let editor_saved = RwSignal::new(None::<String>);
    provide_context(editor_saved);
    crate::editor_ipc::install_editor_saved_listener(editor_saved);

    // ED.24 — recordings library entries, fed by the `recordings-listed` event.
    let editor_recordings = RwSignal::new(Vec::<crate::editor_ipc::RecordingEntry>::new());
    provide_context(editor_recordings);
    crate::editor_ipc::install_recordings_listener(editor_recordings);

    // NavigationRail click → flip the signal + rewrite the URL so a
    // page-reload restores the last visited surface within the
    // current Tauri session.
    let on_select = Callback::new(move |section: AppSection| {
        active.set(section);
        push_surface_query(section);
    });

    let nav_items = sample_nav_items(false);
    let workspace = sample_workspace_badge();
    let user = sample_user_avatar();

    view! {
        <AppShell
            rail=ToChildren::to_children(move || view! {
                <NavigationRail
                    items=nav_items.clone()
                    active=active.get()
                    workspace=workspace.clone()
                    user=user.clone()
                    on_select=on_select
                />
            })
            main=ToChildren::to_children(move || view! {
                <SurfacePane active=active />
            })
            footer=ToChildren::to_children(move || view! {
                <StatusBar
                    fps=60.0_f32
                    encoder="—"
                    file_bytes=0_u64
                    kind=StatusKind::Ready
                />
            })
        />
    }
}

/// Render the content for the currently-active surface. Record, Library,
/// and Editor are live ([`RecorderPage`](crate::recorder_page::RecorderPage),
/// [`RecordingsLibrary`](crate::recordings_library::RecordingsLibrary),
/// [`EditorSurface`](crate::editor_surface::EditorSurface)); Cursor and
/// Preferences are still placeholders.
#[component]
fn SurfacePane(active: RwSignal<AppSection>) -> impl IntoView {
    view! {
        <Show
            when=move || matches!(active.get(), AppSection::Record)
            fallback=move || view! { <NonRecorderSurfaces active=active /> }
        >
            <section class="app-surface app-surface--recorder">
                <crate::recorder_page::RecorderPage />
            </section>
        </Show>
    }
}

#[component]
fn NonRecorderSurfaces(active: RwSignal<AppSection>) -> impl IntoView {
    view! {
        <Show
            when=move || matches!(active.get(), AppSection::Library)
            fallback=move || view! { <EditorOrLater active=active /> }
        >
            <section class="app-surface app-surface--library">
                <crate::recordings_library::RecordingsLibrary active=active />
            </section>
        </Show>
    }
}

#[component]
fn EditorOrLater(active: RwSignal<AppSection>) -> impl IntoView {
    view! {
        <Show
            when=move || matches!(active.get(), AppSection::Editor)
            fallback=move || view! { <CursorOrPrefs active=active /> }
        >
            <crate::editor_surface::EditorSurface />
        </Show>
    }
}

#[component]
fn CursorOrPrefs(active: RwSignal<AppSection>) -> impl IntoView {
    view! {
        <Show
            when=move || matches!(active.get(), AppSection::Cursor)
            fallback=move || view! {
                <section class="app-surface app-surface--prefs">
                    <h1>"Preferences"</h1>
                    <p>"App preferences. Future ticket."</p>
                </section>
            }
        >
            <section class="app-surface app-surface--cursor">
                <h1>"Cursor Studio"</h1>
                <p>"Cursor style picker. Future ticket: AUT-140."</p>
            </section>
        </Show>
    }
}

/// Push `?surface=<slug>` onto the URL via `history.replaceState` so
/// a reload reopens at the same surface. Used by [`AppShellRoot`]'s
/// `on_select` callback (M-TRAY.4 / AUT-253).
fn push_surface_query(section: AppSection) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(history) = window.history() else {
        return;
    };
    let slug = crate::surface_to_query(section);
    let new_url = format!("?surface={slug}");
    // Empty state value, empty title → just rewrite the query.
    let _ = history.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some(&new_url));
}

#[cfg(test)]
mod tests {
    // M-TRAY.3 / M-TRAY.4 unit tests live on the pure-Rust helpers in
    // `crate::lib` — `parse_surface_from_query` + `surface_to_query`.
    // See those modules for the round-trip coverage.
}
