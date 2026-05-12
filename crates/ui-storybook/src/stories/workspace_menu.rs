//! Workspace switcher menu stories (M-UI.5 / AUT-125).

use leptos::prelude::*;

use crate::components::shell::WorkspaceSwitcherMenu;
use crate::fixtures::workspaces::{
    sample_workspace_views, sample_workspace_views_long_names, sample_workspace_views_many,
};

use super::{Story, StoryViewport, render};

/// All workspace-switcher stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "workspace-menu-default",
            category: "Menus",
            title: "Workspace switcher — default",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 480,
            },
            render: render_default,
        },
        Story {
            id: "workspace-menu-many",
            category: "Menus",
            title: "Workspace switcher — many workspaces",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 600,
            },
            render: render_many,
        },
        Story {
            id: "workspace-menu-long-names",
            category: "Menus",
            title: "Workspace switcher — long names truncate",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 360,
            },
            render: render_long_names,
        },
        Story {
            id: "workspace-menu-no-selection",
            category: "Menus",
            title: "Workspace switcher — no selection",
            viewport: StoryViewport::Fixed {
                width: 360,
                height: 480,
            },
            render: render_no_selection,
        },
    ]
}

fn render_default() -> String {
    render(view! {
        <WorkspaceSwitcherMenu
            workspaces=sample_workspace_views()
            selected_id="ws-northwind"
        />
    })
}

fn render_many() -> String {
    render(view! {
        <WorkspaceSwitcherMenu
            workspaces=sample_workspace_views_many()
            selected_id="ws-northwind"
        />
    })
}

fn render_long_names() -> String {
    render(view! {
        <WorkspaceSwitcherMenu
            workspaces=sample_workspace_views_long_names()
            selected_id="ws-extremely-long"
        />
    })
}

fn render_no_selection() -> String {
    render(view! {
        <WorkspaceSwitcherMenu
            workspaces=sample_workspace_views()
            selected_id=""
        />
    })
}
