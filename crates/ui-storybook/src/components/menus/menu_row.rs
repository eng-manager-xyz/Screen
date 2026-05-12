//! `MenuRow` — one row inside a popover menu (M-UI.3 / AUT-123).
//!
//! Supports the recurring shape across workspace switcher, device
//! pickers, system-audio picker, and on-screen options: leading icon
//! tile, title, optional subtitle, badges, optional trailing slot, and
//! a kind-driven visual state.

use leptos::prelude::*;

use crate::components::primitives::{Badge, BadgeKind};

/// Visual kind for a menu row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuRowKind {
    /// Normal row.
    #[default]
    Default,
    /// Highlighted — currently selected.
    Selected,
    /// Action — slightly emphasized leading text. Used for "Add
    /// workspace" / "Pair device".
    Action,
    /// Destructive — red text. Used for "Remove workspace" /
    /// "Forget device".
    Danger,
    /// Visually disabled — non-interactive.
    Disabled,
}

impl MenuRowKind {
    /// CSS class for the kind.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            MenuRowKind::Default => "menu-row-default",
            MenuRowKind::Selected => "menu-row-selected",
            MenuRowKind::Action => "menu-row-action",
            MenuRowKind::Danger => "menu-row-danger",
            MenuRowKind::Disabled => "menu-row-disabled",
        }
    }
}

/// Badge attached to a menu row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuBadgeView {
    /// Display text.
    pub label: &'static str,
    /// Visual kind.
    pub kind: BadgeKind,
}

#[component]
pub fn MenuRow(
    #[prop(optional)] kind: MenuRowKind,
    /// Optional leading slot — an `IconTile` / avatar / small glyph.
    #[prop(optional)]
    leading: Option<Children>,
    /// Primary row text.
    #[prop(into)]
    title: String,
    /// Optional secondary text under the title.
    #[prop(optional, into)]
    subtitle: Option<String>,
    /// Badges rendered between the text and the trailing slot.
    #[prop(optional)]
    badges: Vec<MenuBadgeView>,
    /// Optional trailing slot — kbd shortcut, count, or chevron.
    #[prop(optional)]
    trailing: Option<Children>,
) -> impl IntoView {
    let disabled = kind == MenuRowKind::Disabled;
    let class = format!("menu-row {}", kind.css());
    view! {
        <li class="menu-row-item" role="none">
            <button
                class=class
                role="menuitem"
                aria-disabled=disabled
                disabled=disabled
            >
                {leading.map(|l| view! { <span class="menu-row-leading">{l()}</span> })}
                <span class="menu-row-text">
                    <span class="menu-row-title">{title}</span>
                    {subtitle.map(|s| view! { <span class="menu-row-subtitle">{s}</span> })}
                </span>
                {(!badges.is_empty()).then(|| view! {
                    <span class="menu-row-badges">
                        {badges.into_iter()
                            .map(|b| view! { <Badge kind=b.kind>{b.label}</Badge> })
                            .collect_view()}
                    </span>
                })}
                {trailing.map(|t| view! { <span class="menu-row-trailing">{t()}</span> })}
                {(kind == MenuRowKind::Selected).then(|| view! {
                    <span class="menu-row-check" aria-hidden="true">"✓"</span>
                })}
            </button>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_unique_class() {
        let classes = [
            MenuRowKind::Default.css(),
            MenuRowKind::Selected.css(),
            MenuRowKind::Action.css(),
            MenuRowKind::Danger.css(),
            MenuRowKind::Disabled.css(),
        ];
        let mut sorted = classes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), classes.len());
    }
}
