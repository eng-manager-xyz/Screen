//! `NavigationRail` — left-edge nav (M-UI.2 / AUT-122).
//!
//! Structural component, not a router. The selected section is passed
//! in from above via `active`; the component never owns that state.
//! Items render with icon, label, and optional notification count.

use leptos::prelude::*;

use super::user_avatar::{UserAvatar, UserAvatarView};
use super::workspace_badge::{WorkspaceBadge, WorkspaceBadgeView};

/// Top-level app section the rail can route to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppSection {
    /// Record setup + tray popover.
    Record,
    /// Recordings library.
    Library,
    /// Editor (drop zone + timeline + inspector).
    Editor,
    /// Cursor Studio.
    Cursor,
    /// Preferences.
    Prefs,
}

impl AppSection {
    /// Stable kebab-case slug used in CSS classes + data attributes.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            AppSection::Record => "record",
            AppSection::Library => "library",
            AppSection::Editor => "editor",
            AppSection::Cursor => "cursor",
            AppSection::Prefs => "prefs",
        }
    }
}

/// View-model for one nav item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavItemView {
    /// Which section this item routes to.
    pub section: AppSection,
    /// Visible label under the icon.
    pub label: &'static str,
    /// Single-glyph icon (the rail uses simple Unicode glyphs in the
    /// storybook; the production app can swap to an icon font).
    pub icon: &'static str,
    /// Optional notification badge count (rendered when `Some` and
    /// `> 0`).
    pub count: Option<u32>,
    /// `true` to render the item dimmed and non-clickable.
    pub disabled: bool,
}

#[component]
pub fn NavigationRail(
    /// List of nav items in display order (top to bottom).
    items: Vec<NavItemView>,
    /// The currently-active section. The matching item gets the
    /// `nav-rail-item-active` class.
    active: AppSection,
    /// Top-of-rail workspace marker.
    workspace: WorkspaceBadgeView,
    /// Optional bottom-of-rail user avatar.
    #[prop(optional)]
    user: Option<UserAvatarView>,
    /// `true` when the workspace switcher menu is open — controls the
    /// badge's `open` state.
    #[prop(optional)]
    workspace_open: bool,
    /// Optional callback fired when the user clicks an enabled rail
    /// item (M-TRAY.2 / AUT-251). The argument is the clicked item's
    /// [`AppSection`]. M-TRAY.4 (AUT-253) wires this in
    /// `crates/app-ui` to drive the active-surface signal; the
    /// storybook stories leave it `None` so SSR snapshots stay
    /// identical to the pre-callback baseline. Disabled items never
    /// fire the callback.
    #[prop(optional)]
    on_select: Option<Callback<AppSection>>,
) -> impl IntoView {
    view! {
        <nav class="nav-rail" aria-label="Primary">
            <div class="nav-rail-top">
                <WorkspaceBadge view=workspace open=workspace_open />
            </div>
            <ul class="nav-rail-items" role="tablist">
                {items.into_iter()
                    .map(|item| render_item(item, active, on_select))
                    .collect_view()}
            </ul>
            <div class="nav-rail-bottom">
                {user.map(|u| view! { <UserAvatar view=u /> })}
            </div>
        </nav>
    }
}

fn render_item(
    item: NavItemView,
    active: AppSection,
    on_select: Option<Callback<AppSection>>,
) -> impl IntoView {
    let is_active = item.section == active;
    let mut class = String::from("nav-rail-item");
    class.push_str(" nav-rail-item-");
    class.push_str(item.section.slug());
    if is_active {
        class.push_str(" nav-rail-item-active");
    }
    if item.disabled {
        class.push_str(" nav-rail-item-disabled");
    }
    let count_view = item
        .count
        .filter(|c| *c > 0)
        .map(|c| view! { <span class="nav-rail-count">{c.to_string()}</span> });
    // Click handler: fires the callback with the item's section when
    // present + the item isn't disabled. `Callback<T>` is `Copy` so
    // both `on_select` and `item_section` can be moved into the
    // closure freely.
    let item_section = item.section;
    let item_disabled = item.disabled;
    let on_click = move |_| {
        if !item_disabled && let Some(cb) = on_select {
            cb.run(item_section);
        }
    };
    view! {
        <li>
            <button
                class=class
                role="tab"
                aria-selected=is_active
                aria-disabled=item.disabled
                disabled=item.disabled
                data-section=item.section.slug()
                on:click=on_click
            >
                <span class="nav-rail-icon" aria-hidden="true">{item.icon}</span>
                <span class="nav-rail-label">{item.label}</span>
                {count_view}
            </button>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_section_has_a_unique_slug() {
        let slugs = [
            AppSection::Record.slug(),
            AppSection::Library.slug(),
            AppSection::Editor.slug(),
            AppSection::Cursor.slug(),
            AppSection::Prefs.slug(),
        ];
        let mut sorted: Vec<_> = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }

    #[test]
    fn slugs_are_kebab_case_and_lowercase() {
        for s in [
            AppSection::Record,
            AppSection::Library,
            AppSection::Editor,
            AppSection::Cursor,
            AppSection::Prefs,
        ] {
            let slug = s.slug();
            assert!(slug.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
            assert!(!slug.is_empty());
        }
    }
}
