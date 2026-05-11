//! Workspace fixtures — list of workspaces the workspace switcher menu
//! displays. Filled in across UI-05 (`WorkspaceSwitcherMenu`).

/// One workspace the user could switch to.
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
