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
use ui_storybook::fixtures::shell::{sample_nav_items, sample_workspace_badge};

#[component]
pub fn AppShellRoot(initial: AppSection) -> impl IntoView {
    let active = RwSignal::new(initial);

    // NavigationRail click → flip the signal + rewrite the URL so a
    // page-reload restores the last visited surface within the
    // current Tauri session.
    let on_select = Callback::new(move |section: AppSection| {
        active.set(section);
        push_surface_query(section);
    });

    let nav_items = sample_nav_items(false);
    let workspace = sample_workspace_badge();

    view! {
        <AppShell
            rail=ToChildren::to_children(move || view! {
                <NavigationRail
                    items=nav_items.clone()
                    active=active.get()
                    workspace=workspace.clone()
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

/// Render the placeholder content for the currently-active surface.
///
/// Real per-surface content (real recorder, library, editor, cursor
/// studio, preferences) lives in dedicated tickets — this is the
/// M-TRAY.4 stub that proves the routing works.
#[component]
fn SurfacePane(active: RwSignal<AppSection>) -> impl IntoView {
    view! {
        <Show
            when=move || matches!(active.get(), AppSection::Record)
            fallback=move || view! { <NonRecorderSurfaces active=active /> }
        >
            <section class="app-surface app-surface--recorder">
                <h1>"Recorder"</h1>
                <crate::camera_picker::CameraPicker />
                <crate::camera_preview::CameraPreview />
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
                <h1>"Library"</h1>
                <p>"Recordings grid + sidebar. Future tickets: AUT-134 / AUT-135."</p>
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
            <section class="app-surface app-surface--editor">
                <h1>"Editor"</h1>
                <p>"Editor shell + timeline. Future tickets: AUT-136 / AUT-139."</p>
            </section>
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
