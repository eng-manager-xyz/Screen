//! `WorkspaceSwitcherMenu` — popover anchored to the rail's
//! `WorkspaceBadge` (M-UI.5 / AUT-125). Pure composition of UI-03 menu
//! primitives.

use leptos::prelude::*;

use crate::components::menus::{
    MenuBadgeView, MenuFooter, MenuList, MenuRow, MenuRowKind, MenuSection, PopoverPlacement,
    PopoverSurface,
};
use crate::components::primitives::{BadgeKind, IconTile, IconTileKind};

/// View-model for one workspace row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceView {
    /// Stable id.
    pub id: &'static str,
    /// 2-letter monogram.
    pub initials: &'static str,
    /// Display name.
    pub name: &'static str,
    /// Optional plan label rendered as a badge ("Pro", "Team", "Free").
    pub plan_badge: Option<&'static str>,
    /// Number of members. Rendered as "N members" / "1 member".
    pub member_count: u32,
    /// Optional CSS background color for the monogram tile.
    pub color: Option<&'static str>,
}

#[component]
pub fn WorkspaceSwitcherMenu(
    /// All workspaces in display order.
    workspaces: Vec<WorkspaceView>,
    /// Currently-selected workspace id. Pass `""` for the
    /// no-selection variant.
    #[prop(into)]
    selected_id: String,
) -> impl IntoView {
    // Pre-build the rows so the selected-id comparison runs once
    // up-front and the resulting `View` values are `'static`.
    let rows: Vec<_> = workspaces
        .into_iter()
        .map(|w| {
            let is_selected = w.id == selected_id;
            render_row(w, is_selected)
        })
        .collect();
    view! {
        <PopoverSurface
            placement=PopoverPlacement::BottomLeft
            width_px=320_u16
            title="Workspaces".to_string()
            description="Switch between your personal and team workspaces.".to_string()
            footer=ToChildren::to_children(|| view! {
                <MenuFooter>
                    <span style="font-size:11px;color:var(--text-tertiary)">"Workspaces sync across devices."</span>
                </MenuFooter>
            })
        >
            <MenuList label="Workspaces">
                <MenuSection heading="Your workspaces".to_string()>
                    {rows}
                </MenuSection>
                <MenuSection heading="Actions".to_string()>
                    <MenuRow
                        kind=MenuRowKind::Action
                        leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Action>"+"</IconTile> })
                        title="New workspace".to_string()
                        subtitle="Invite teammates after creating".to_string()
                    />
                    <MenuRow
                        leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Action>"⚙"</IconTile> })
                        title="Workspace settings".to_string()
                    />
                </MenuSection>
            </MenuList>
        </PopoverSurface>
    }
}

fn render_row(w: WorkspaceView, is_selected: bool) -> impl IntoView {
    let kind = if is_selected {
        MenuRowKind::Selected
    } else {
        MenuRowKind::Default
    };
    let initials = w.initials;
    let color_style = w
        .color
        .map(|c| format!("background:{c}"))
        .unwrap_or_default();
    let mut badges: Vec<MenuBadgeView> = Vec::new();
    if let Some(plan) = w.plan_badge {
        badges.push(MenuBadgeView {
            label: plan,
            kind: BadgeKind::Plan,
        });
    }
    let member_count_label = leak_str(format_member_count(w.member_count));
    view! {
        <MenuRow
            kind=kind
            leading=ToChildren::to_children(move || view! {
                <span class="icon-tile icon-tile-workspace" style=color_style.clone() aria-hidden="true">{initials}</span>
            })
            title=w.name.to_string()
            subtitle=member_count_label.to_string()
            badges=badges
        />
    }
}

/// Format the member-count subtitle. `1 member` (singular), `N
/// members` (plural).
#[must_use]
pub fn format_member_count(n: u32) -> String {
    if n == 1 {
        "1 member".to_string()
    } else {
        format!("{n} members")
    }
}

/// Leak a `String` to `&'static str` so the leptos view can hold the
/// reference without extra cloning. Storybook stories are short-lived
/// processes (the headless exporter runs once per build) — total leak
/// bytes are negligible.
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_count_singular() {
        assert_eq!(format_member_count(1), "1 member");
    }

    #[test]
    fn member_count_plural() {
        assert_eq!(format_member_count(2), "2 members");
        assert_eq!(format_member_count(14), "14 members");
        assert_eq!(format_member_count(0), "0 members");
    }
}
