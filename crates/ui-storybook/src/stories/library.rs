//! Library stories — UI-14 (`LibrarySidebar`) + UI-15
//! (`RecordingCard` + `LibraryGrid`).

use leptos::prelude::*;

use crate::components::library::LibrarySidebar;
use crate::fixtures::library::{
    sample_library_sidebar, sample_library_sidebar_high_storage,
    sample_library_sidebar_inbox_active,
};

use super::{Story, StoryViewport, render};

const RAIL_W: u16 = 260;
const RAIL_H: u16 = 620;

/// All library-surface stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        story(
            "library-sidebar-default",
            "Library sidebar — default",
            render_default,
        ),
        story(
            "library-sidebar-inbox-active",
            "Library sidebar — inbox active",
            render_inbox,
        ),
        story(
            "library-sidebar-high-storage",
            "Library sidebar — 95% storage",
            render_high_storage,
        ),
        story(
            "library-sidebar-empty-spaces",
            "Library sidebar — no spaces",
            render_empty_spaces,
        ),
        story(
            "library-sidebar-long-labels",
            "Library sidebar — overflow labels",
            render_long_labels,
        ),
    ]
}

fn story(id: &'static str, title: &'static str, render: fn() -> String) -> Story {
    Story {
        id,
        category: "Library",
        title,
        viewport: StoryViewport::Fixed {
            width: RAIL_W,
            height: RAIL_H,
        },
        render,
    }
}

fn render_default() -> String {
    render(view! { <LibrarySidebar view=sample_library_sidebar(3) /> })
}

fn render_inbox() -> String {
    render(view! { <LibrarySidebar view=sample_library_sidebar_inbox_active() /> })
}

fn render_high_storage() -> String {
    render(view! { <LibrarySidebar view=sample_library_sidebar_high_storage() /> })
}

fn render_empty_spaces() -> String {
    let mut v = sample_library_sidebar(0);
    v.sections.retain(|s| s.heading != "SPACES");
    render(view! { <LibrarySidebar view=v /> })
}

fn render_long_labels() -> String {
    let mut v = sample_library_sidebar(99);
    if let Some(item) = v.primary.iter_mut().find(|i| i.id == "shared") {
        item.label = "Shared with me — Northwind Studio quarterly review";
    }
    render(view! { <LibrarySidebar view=v /> })
}
