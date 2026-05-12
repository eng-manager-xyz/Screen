//! Library stories — UI-14 (`LibrarySidebar`) + UI-15
//! (`RecordingCard` + `LibraryGrid`).

use leptos::prelude::*;

use crate::components::library::{LibraryGrid, LibrarySidebar, RecordingCard, RecordingCardState};
use crate::fixtures::library::{
    sample_library_grid, sample_library_grid_empty, sample_library_grid_list_mode,
    sample_library_sidebar, sample_library_sidebar_high_storage,
    sample_library_sidebar_inbox_active, sample_recording_cards,
};

use super::{Story, StoryViewport, render};

const RAIL_W: u16 = 260;
const RAIL_H: u16 = 620;
const GRID_W: u16 = 880;
const GRID_H: u16 = 520;
const CARD_W: u16 = 360;
const CARD_H: u16 = 280;

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
        card_story(
            "recording-card-ready",
            "Recording card — ready",
            render_card_ready,
        ),
        card_story(
            "recording-card-processing",
            "Recording card — processing",
            render_card_processing,
        ),
        card_story(
            "recording-card-empty-thumbnail",
            "Recording card — missing thumbnail",
            render_card_empty_thumb,
        ),
        grid_story(
            "library-grid-default",
            "Library grid — default",
            render_grid_default,
        ),
        grid_story(
            "library-grid-empty",
            "Library grid — empty state",
            render_grid_empty,
        ),
        grid_story(
            "library-grid-list-mode",
            "Library grid — list mode",
            render_grid_list,
        ),
    ]
}

fn card_story(id: &'static str, title: &'static str, render: fn() -> String) -> Story {
    Story {
        id,
        category: "Library",
        title,
        viewport: StoryViewport::Fixed {
            width: CARD_W,
            height: CARD_H,
        },
        render,
    }
}

fn grid_story(id: &'static str, title: &'static str, render: fn() -> String) -> Story {
    Story {
        id,
        category: "Library",
        title,
        viewport: StoryViewport::Fixed {
            width: GRID_W,
            height: GRID_H,
        },
        render,
    }
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

fn render_card_ready() -> String {
    let cards = sample_recording_cards();
    let card = cards.into_iter().next().expect("at least one card");
    render(view! { <RecordingCard view=card /> })
}

fn render_card_processing() -> String {
    let cards = sample_recording_cards();
    let mut card = cards.into_iter().nth(1).expect("processing fixture");
    card.state = RecordingCardState::Processing { percent: 42 };
    render(view! { <RecordingCard view=card /> })
}

fn render_card_empty_thumb() -> String {
    let cards = sample_recording_cards();
    let card = cards.into_iter().last().expect("empty-thumb fixture");
    render(view! { <RecordingCard view=card /> })
}

fn render_grid_default() -> String {
    render(view! { <LibraryGrid view=sample_library_grid() /> })
}

fn render_grid_empty() -> String {
    render(view! { <LibraryGrid view=sample_library_grid_empty() /> })
}

fn render_grid_list() -> String {
    render(view! { <LibraryGrid view=sample_library_grid_list_mode() /> })
}
