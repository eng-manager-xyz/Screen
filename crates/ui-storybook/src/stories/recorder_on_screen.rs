//! On-screen-options-popover stories (M-UI.10 / AUT-130).

use leptos::prelude::*;

use crate::components::recorder::OnScreenOptionsPopover;
use crate::fixtures::recorder::{
    sample_on_screen_options, sample_on_screen_options_all_on, sample_on_screen_options_long_copy,
};

use super::{Story, StoryViewport, render};

/// All on-screen-options stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "on-screen-options-default",
            category: "Recorder",
            title: "On-screen options — default (some on)",
            viewport: StoryViewport::Fixed {
                width: 420,
                height: 360,
            },
            render: render_default,
        },
        Story {
            id: "on-screen-options-all-on",
            category: "Recorder",
            title: "On-screen options — all toggles on",
            viewport: StoryViewport::Fixed {
                width: 420,
                height: 360,
            },
            render: render_all_on,
        },
        Story {
            id: "on-screen-options-sensitive-disabled",
            category: "Recorder",
            title: "On-screen options — Blur sensitive disabled",
            viewport: StoryViewport::Fixed {
                width: 420,
                height: 360,
            },
            render: render_sensitive_disabled,
        },
        Story {
            id: "on-screen-options-long-copy",
            category: "Recorder",
            title: "On-screen options — long copy wraps",
            viewport: StoryViewport::Fixed {
                width: 420,
                height: 380,
            },
            render: render_long_copy,
        },
    ]
}

fn render_default() -> String {
    render(view! {
        <OnScreenOptionsPopover options=sample_on_screen_options(false) />
    })
}

fn render_all_on() -> String {
    render(view! {
        <OnScreenOptionsPopover options=sample_on_screen_options_all_on() />
    })
}

fn render_sensitive_disabled() -> String {
    render(view! {
        <OnScreenOptionsPopover options=sample_on_screen_options(true) />
    })
}

fn render_long_copy() -> String {
    render(view! {
        <OnScreenOptionsPopover options=sample_on_screen_options_long_copy() />
    })
}
