//! Fixture-gallery story (M-UI.22 / AUT-142). Renders a contact sheet
//! of every major fixture domain so designers can review the canonical
//! data shape at a glance.

use leptos::prelude::*;

use crate::components::cursor::{CursorAppearancePanel, CursorAppearanceView};
use crate::components::library::RecordingCard;
use crate::components::recorder::{CaptureSourceRow, DisplaySourceCard, SystemAudioAppList};
use crate::components::shell::WorkspaceSwitcherMenu;
use crate::fixtures::default_ui_fixtures;

use super::{Story, StoryViewport, render};

/// All fixture-gallery stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        story(
            "fixtures-contact-sheet",
            "Fixtures — contact sheet",
            render_contact_sheet,
        ),
        story("fixtures-recorder", "Fixtures — recorder", render_recorder),
        story("fixtures-library", "Fixtures — library", render_library),
        story("fixtures-cursor", "Fixtures — cursor", render_cursor),
    ]
}

fn story(id: &'static str, title: &'static str, render: fn() -> String) -> Story {
    Story {
        id,
        category: "Fixtures",
        title,
        viewport: StoryViewport::Fixed {
            width: 1080,
            height: 720,
        },
        render,
    }
}

fn render_contact_sheet() -> String {
    let f = default_ui_fixtures();
    let display_cards: Vec<_> = f
        .displays
        .into_iter()
        .map(|d| view! { <DisplaySourceCard view=d /> })
        .collect();
    let cards: Vec<_> = f
        .recordings
        .into_iter()
        .map(|r| view! { <RecordingCard view=r /> })
        .collect();
    let appearances: Vec<_> = f
        .cursor_presets
        .into_iter()
        .map(render_appearance_tile)
        .collect();
    render(view! {
        <section class="contact-sheet">
            <h3 class="contact-sheet-heading">"Workspaces"</h3>
            <WorkspaceSwitcherMenu workspaces=f.workspaces selected_id="ws-northwind".to_string() />

            <h3 class="contact-sheet-heading">"Displays"</h3>
            <div class="contact-sheet-row">{display_cards}</div>

            <h3 class="contact-sheet-heading">"Capture sources"</h3>
            <div class="contact-sheet-row">
                <CaptureSourceRow view=f.devices.camera />
                <CaptureSourceRow view=f.devices.microphone />
            </div>

            <h3 class="contact-sheet-heading">"System audio"</h3>
            <SystemAudioAppList apps=f.audio_apps />

            <h3 class="contact-sheet-heading">"Recordings"</h3>
            <div class="contact-sheet-row contact-sheet-row-grid">{cards}</div>

            <h3 class="contact-sheet-heading">"Cursor presets"</h3>
            <div class="contact-sheet-row">{appearances}</div>
        </section>
    })
}

fn render_appearance_tile(v: CursorAppearanceView) -> AnyView {
    view! {
        <div class="contact-sheet-cursor-tile">
            <CursorAppearancePanel view=v />
        </div>
    }
    .into_any()
}

fn render_recorder() -> String {
    let f = default_ui_fixtures();
    render(view! {
        <section class="contact-sheet">
            <h3 class="contact-sheet-heading">"Capture sources"</h3>
            <CaptureSourceRow view=f.devices.camera />
            <CaptureSourceRow view=f.devices.microphone />
            <h3 class="contact-sheet-heading">"System audio"</h3>
            <SystemAudioAppList apps=f.audio_apps />
        </section>
    })
}

fn render_library() -> String {
    let f = default_ui_fixtures();
    let cards: Vec<_> = f
        .recordings
        .into_iter()
        .map(|r| view! { <RecordingCard view=r /> })
        .collect();
    render(view! {
        <section class="contact-sheet">
            <h3 class="contact-sheet-heading">"Recording cards"</h3>
            <div class="contact-sheet-row contact-sheet-row-grid">{cards}</div>
        </section>
    })
}

fn render_cursor() -> String {
    let f = default_ui_fixtures();
    let tiles: Vec<_> = f
        .cursor_presets
        .into_iter()
        .map(render_appearance_tile)
        .collect();
    render(view! {
        <section class="contact-sheet">
            <h3 class="contact-sheet-heading">"Cursor appearance presets"</h3>
            <div class="contact-sheet-row">{tiles}</div>
        </section>
    })
}
