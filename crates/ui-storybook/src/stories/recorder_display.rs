//! Display-source-card stories (M-UI.7 / AUT-127).

use leptos::prelude::*;

use crate::components::recorder::{DisplayPreviewFrame, DisplaySourceCard};
use crate::fixtures::devices::{
    sample_display_source, sample_display_source_small, sample_display_source_wide,
};

use super::{Story, StoryViewport, render};

/// All display-source-card stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "display-source-built-in-retina",
            category: "Recorder",
            title: "Display source — Built-in Retina (selected)",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 320,
            },
            render: render_built_in,
        },
        Story {
            id: "display-source-selected",
            category: "Recorder",
            title: "Display source — selected outline + open chevron",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 320,
            },
            render: render_selected,
        },
        Story {
            id: "display-source-unavailable",
            category: "Recorder",
            title: "Display source — unavailable (permissions)",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 360,
            },
            render: render_unavailable,
        },
        Story {
            id: "display-preview-wide",
            category: "Recorder",
            title: "Display preview — 21:9 ultrawide",
            viewport: StoryViewport::Fixed {
                width: 420,
                height: 240,
            },
            render: render_preview_wide,
        },
        Story {
            id: "display-preview-small",
            category: "Recorder",
            title: "Display preview — 16:10 single window",
            viewport: StoryViewport::Fixed {
                width: 280,
                height: 220,
            },
            render: render_preview_small,
        },
    ]
}

fn render_built_in() -> String {
    render(view! { <DisplaySourceCard view=sample_display_source(true) /> })
}

fn render_selected() -> String {
    render(view! { <DisplaySourceCard view=sample_display_source(true) open=true /> })
}

fn render_unavailable() -> String {
    render(view! { <DisplaySourceCard view=sample_display_source(false) unavailable=true /> })
}

fn render_preview_wide() -> String {
    let view = sample_display_source_wide();
    render(view! {
        <div style="padding:8px;width:400px">
            <DisplayPreviewFrame view=view.preview />
        </div>
    })
}

fn render_preview_small() -> String {
    let view = sample_display_source_small();
    render(view! {
        <div style="padding:8px;width:260px">
            <DisplayPreviewFrame view=view.preview />
        </div>
    })
}
