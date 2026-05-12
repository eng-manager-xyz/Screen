//! `AppShell` — top-level layout with slots for rail / main / inspector /
//! titlebar / footer (M-UI.2 / AUT-122).
//!
//! Structural component only — owns no state, picks no router. Each
//! product screen (library, editor, cursor, prefs) composes one
//! `AppShell` with the panes it needs. `rail` and `main` are required;
//! `titlebar`, `inspector`, and `footer` are opt-in via
//! `Option<Children>`.

use leptos::prelude::*;

#[component]
pub fn AppShell(
    /// Left-edge navigation rail. Typically `NavigationRail`.
    rail: Children,
    /// The primary surface (record setup, library grid, editor canvas,
    /// cursor studio).
    main: Children,
    /// Optional right-edge inspector / property panel.
    #[prop(optional)]
    inspector: Option<Children>,
    /// Optional top window-chrome strip.
    #[prop(optional)]
    titlebar: Option<Children>,
    /// Optional bottom status / recording-controls footer.
    #[prop(optional)]
    footer: Option<Children>,
    #[prop(optional, into)] extra_class: String,
) -> impl IntoView {
    let class = format!(
        "app-shell{}{}",
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! {
        <div class=class>
            {titlebar.map(|t| view! { <header class="app-shell-titlebar">{t()}</header> })}
            <div class="app-shell-body">
                <aside class="app-shell-rail">{rail()}</aside>
                <main class="app-shell-main">{main()}</main>
                {inspector.map(|i| view! { <aside class="app-shell-inspector">{i()}</aside> })}
            </div>
            {footer.map(|f| view! { <footer class="app-shell-footer">{f()}</footer> })}
        </div>
    }
}
