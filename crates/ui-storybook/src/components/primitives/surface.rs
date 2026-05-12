//! `Surface` — semantic background container (M-UI.1 / AUT-121).
//!
//! Wraps content in a single `<div>` whose class encodes the surface
//! kind (`Base` / `Elevated` / `Popover` / `Selected` / `Glass`). Every
//! recorder / library / editor / cursor surface stacks on these
//! kinds — popovers, panels, selected list rows, glassy overlays — so
//! the visual layering stays consistent across product surfaces.

use leptos::prelude::*;

/// Semantic surface kind. Drives the background, border, and shadow
/// tokens applied. Picked once per surface; doesn't change at runtime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SurfaceKind {
    /// App background — the bottom of the stack. Use for the canvas
    /// underneath everything else.
    #[default]
    Base,
    /// One step elevated — panels, cards, tray surfaces. The default
    /// background for grouped content.
    Elevated,
    /// Popover background — for tray menus, dropdowns, command menus.
    /// Has a stronger drop shadow than `Elevated`.
    Popover,
    /// Selected list-row background — highlighted state inside a menu
    /// or sidebar list.
    Selected,
    /// Translucent overlay — used for full-window overlays and the
    /// hint surfaces that float over the recording.
    Glass,
}

impl SurfaceKind {
    /// CSS class that controls the surface look.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            SurfaceKind::Base => "surface-base",
            SurfaceKind::Elevated => "surface-elevated",
            SurfaceKind::Popover => "surface-popover",
            SurfaceKind::Selected => "surface-selected",
            SurfaceKind::Glass => "surface-glass",
        }
    }
}

#[component]
pub fn Surface(
    #[prop(optional)] kind: SurfaceKind,
    #[prop(optional, into)] extra_class: String,
    children: Children,
) -> impl IntoView {
    let class = format!(
        "surface {}{}{}",
        kind.css(),
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! { <div class=class>{children()}</div> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_a_unique_class() {
        let classes = [
            SurfaceKind::Base.css(),
            SurfaceKind::Elevated.css(),
            SurfaceKind::Popover.css(),
            SurfaceKind::Selected.css(),
            SurfaceKind::Glass.css(),
        ];
        let mut sorted = classes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), classes.len(), "duplicate Surface CSS class");
    }

    #[test]
    fn each_class_starts_with_surface_prefix() {
        for k in [
            SurfaceKind::Base,
            SurfaceKind::Elevated,
            SurfaceKind::Popover,
            SurfaceKind::Selected,
            SurfaceKind::Glass,
        ] {
            assert!(
                k.css().starts_with("surface-"),
                "missing prefix: {}",
                k.css()
            );
        }
    }
}
