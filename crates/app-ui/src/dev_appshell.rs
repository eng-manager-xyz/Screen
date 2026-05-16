//! M-TRAY.1 / AUT-250 — feature-gated `AppShell` CSR preview.
//!
//! When the `tray-appshell-preview` Cargo feature is enabled, the
//! [`run`](crate::run) entry point mounts [`DevAppShellPreview`]
//! instead of the default `<App />` drop-zone shell. The preview is a
//! developer affordance — it proves the [`ui_storybook::components`]
//! shell tree compiles, mounts, and paints under CSR (`wasm32-unknown-
//! unknown` + Leptos `csr` feature) before M-TRAY.3 wires it for real.
//!
//! Run with:
//!
//! ```sh
//! cd crates/app-ui && trunk serve --features tray-appshell-preview
//! ```
//!
//! Default builds compile this module out entirely. The wasm32 build
//! smoke in `gate.yml` exercises the on-feature path.

use leptos::prelude::*;
use ui_storybook::components::shell::{
    AppSection, AppShell, NavigationRail, StatusBar, StatusKind,
};
use ui_storybook::fixtures::shell::{sample_nav_items, sample_workspace_badge};

/// Mount the AppShell with the shipped storybook fixtures and a static
/// placeholder main pane. NavigationRail clicks are inert today — that
/// wiring lands in M-TRAY.4 (AUT-253) after `NavigationRail` gains the
/// `on_select: Callback<AppSection>` prop the audit doc flagged.
///
/// Slot wiring uses `ToChildren::to_children` per the canonical
/// pattern in `crates/ui-storybook/src/stories/shell.rs:196-222` and
/// CLAUDE.md "Leptos discipline" — passing a bare closure to a
/// `Children` prop trips a type error in Leptos 0.8.
#[component]
pub fn DevAppShellPreview() -> impl IntoView {
    let nav_items = sample_nav_items(false);
    let workspace = sample_workspace_badge();
    view! {
        <AppShell
            rail=ToChildren::to_children(move || view! {
                <NavigationRail
                    items=nav_items.clone()
                    active=AppSection::Record
                    workspace=workspace.clone()
                />
            })
            main=ToChildren::to_children(move || view! {
                <section class="dev-appshell-main">
                    <h1>"AppShell CSR preview"</h1>
                    <p>
                        "M-TRAY.1 (AUT-250) developer affordance. Renders the "
                        "ui-storybook AppShell + NavigationRail with shipped "
                        "fixtures to prove CSR mount works. NavRail clicks "
                        "are inert until M-TRAY.4 wires the on_select callback."
                    </p>
                </section>
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
