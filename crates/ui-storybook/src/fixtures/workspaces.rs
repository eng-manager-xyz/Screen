//! Workspace fixtures — list of workspaces the workspace switcher menu
//! displays. Used by UI-05 (`WorkspaceSwitcherMenu`).

use crate::components::shell::WorkspaceView;

/// One workspace the user could switch to. Stays here as the small
/// pre-UI-05 shape; `WorkspaceView` (used by `WorkspaceSwitcherMenu`)
/// is the richer view-model with plan badge + member count + color.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceFixture {
    /// Stable id.
    pub id: &'static str,
    /// Human label.
    pub label: &'static str,
    /// Two-letter avatar / monogram shown when the workspace has no icon.
    pub monogram: &'static str,
    /// `true` when this workspace is the currently-active one.
    pub active: bool,
}

/// `WorkspaceView`s used by UI-05's `WorkspaceSwitcherMenu` stories.
#[must_use]
pub fn sample_workspace_views() -> Vec<WorkspaceView> {
    vec![
        WorkspaceView {
            id: "ws-northwind",
            initials: "NS",
            name: "Northwind Studio",
            plan_badge: Some("Pro"),
            member_count: 14,
            color: Some("#ef4444"),
        },
        WorkspaceView {
            id: "ws-personal",
            initials: "PE",
            name: "Personal",
            plan_badge: Some("Free"),
            member_count: 1,
            color: Some("#0ea5e9"),
        },
        WorkspaceView {
            id: "ws-acme",
            initials: "AC",
            name: "Acme Inc.",
            plan_badge: Some("Team"),
            member_count: 4,
            color: Some("#a78bfa"),
        },
    ]
}

/// Workspace list with many entries (for the `many` story variant).
#[must_use]
pub fn sample_workspace_views_many() -> Vec<WorkspaceView> {
    let mut out = sample_workspace_views();
    out.extend([
        WorkspaceView {
            id: "ws-client-demos",
            initials: "CD",
            name: "Client demos",
            plan_badge: Some("Team"),
            member_count: 7,
            color: Some("#10b981"),
        },
        WorkspaceView {
            id: "ws-onboarding",
            initials: "ON",
            name: "Onboarding",
            plan_badge: Some("Free"),
            member_count: 1,
            color: Some("#f59e0b"),
        },
        WorkspaceView {
            id: "ws-acquired",
            initials: "AQ",
            name: "Acquired Brands",
            plan_badge: Some("Team"),
            member_count: 21,
            color: Some("#ec4899"),
        },
    ]);
    out
}

/// Workspace with a very long name for the long-names truncation story.
#[must_use]
pub fn sample_workspace_views_long_names() -> Vec<WorkspaceView> {
    vec![
        WorkspaceView {
            id: "ws-extremely-long",
            initials: "VL",
            name: "A very long workspace name that should truncate cleanly in the row",
            plan_badge: Some("Team"),
            member_count: 12,
            color: Some("#ef4444"),
        },
        WorkspaceView {
            id: "ws-also-long",
            initials: "AL",
            name: "Another long workspace name with more words",
            plan_badge: Some("Pro"),
            member_count: 2,
            color: Some("#a78bfa"),
        },
    ]
}

/// Sample workspace list for the switcher.
#[must_use]
pub fn sample_workspaces() -> Vec<WorkspaceFixture> {
    vec![
        WorkspaceFixture {
            id: "ws-personal",
            label: "Personal",
            monogram: "PE",
            active: true,
        },
        WorkspaceFixture {
            id: "ws-acme",
            label: "Acme Inc.",
            monogram: "AC",
            active: false,
        },
        WorkspaceFixture {
            id: "ws-client",
            label: "Client demos",
            monogram: "CD",
            active: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_has_exactly_one_active() {
        let actives = sample_workspaces().into_iter().filter(|w| w.active).count();
        assert_eq!(actives, 1, "exactly one workspace must be active");
    }
}
