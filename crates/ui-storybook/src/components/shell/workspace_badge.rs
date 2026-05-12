//! `WorkspaceBadge` — top-of-rail workspace marker (M-UI.2 / AUT-122).
//!
//! Compact square tile with the active workspace's monogram. Includes a
//! small chevron glyph that signals "click for the workspace switcher".
//! Purely visual; the switcher menu is the parent's responsibility
//! (UI-05).

use leptos::prelude::*;

/// View-model for the workspace badge at the top of the rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceBadgeView {
    /// Stable id (matches the workspace fixture's id).
    pub id: &'static str,
    /// 2-letter monogram displayed inside the tile.
    pub monogram: &'static str,
    /// Workspace label, used for the accessible title.
    pub label: &'static str,
    /// Optional accent color for the tile background (CSS-color
    /// string). Defaults to the brand red when `None`.
    pub accent: Option<&'static str>,
}

#[component]
pub fn WorkspaceBadge(
    view: WorkspaceBadgeView,
    /// `true` when the workspace-switcher popover is open. Adds a
    /// `workspace-badge-open` class for the parent menu's open state.
    #[prop(optional)]
    open: bool,
) -> impl IntoView {
    let style = view
        .accent
        .map(|a| format!("background:{a}"))
        .unwrap_or_default();
    let class = if open {
        "workspace-badge workspace-badge-open"
    } else {
        "workspace-badge"
    };
    view! {
        <button class=class title=view.label aria-haspopup="menu" aria-expanded=open>
            <span class="workspace-badge-tile" style=style>{view.monogram}</span>
            <span class="workspace-badge-chevron" aria-hidden="true">"▾"</span>
        </button>
    }
}
