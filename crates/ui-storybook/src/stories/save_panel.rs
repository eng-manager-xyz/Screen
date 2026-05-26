//! Save-panel stories (M-SAVE.GATE) — the post-record Save panel in
//! its three visible states (choosing / exporting / saved).

use leptos::prelude::*;

use crate::components::recorder::SavePanel;
use crate::fixtures::recorder::{
    sample_save_panel_choosing, sample_save_panel_exporting, sample_save_panel_saved,
};

use super::{Story, StoryViewport, render};

/// All Save-panel stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "save-panel-choosing",
            category: "Recorder",
            title: "Save panel — choosing (folder + format)",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 220,
            },
            render: render_choosing,
        },
        Story {
            id: "save-panel-exporting",
            category: "Recorder",
            title: "Save panel — exporting (controls busy)",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 220,
            },
            render: render_exporting,
        },
        Story {
            id: "save-panel-saved",
            category: "Recorder",
            title: "Save panel — saved (reveal / done)",
            viewport: StoryViewport::Fixed {
                width: 460,
                height: 160,
            },
            render: render_saved,
        },
    ]
}

fn render_choosing() -> String {
    render(view! {
        <SavePanel view=sample_save_panel_choosing() />
    })
}

fn render_exporting() -> String {
    render(view! {
        <SavePanel view=sample_save_panel_exporting() />
    })
}

fn render_saved() -> String {
    render(view! {
        <SavePanel view=sample_save_panel_saved() />
    })
}
