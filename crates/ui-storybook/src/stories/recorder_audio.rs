//! System-audio picker stories (M-UI.9 / AUT-129).

use leptos::prelude::*;

use crate::components::recorder::{AudioFilter, SystemAudioAppList, SystemAudioRow};
use crate::fixtures::audio_apps::{
    sample_audio_apps, sample_audio_apps_all_selected, sample_audio_apps_none_selected,
    sample_system_audio_view,
};

use super::{Story, StoryViewport, render};

/// All system-audio stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "system-audio-collapsed",
            category: "Recorder",
            title: "System audio — collapsed row",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 80,
            },
            render: render_collapsed,
        },
        Story {
            id: "system-audio-expanded",
            category: "Recorder",
            title: "System audio — expanded app list",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 360,
            },
            render: render_expanded,
        },
        Story {
            id: "system-audio-none-selected",
            category: "Recorder",
            title: "System audio — no apps selected",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 360,
            },
            render: render_none_selected,
        },
        Story {
            id: "system-audio-all-selected",
            category: "Recorder",
            title: "System audio — every app selected",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 360,
            },
            render: render_all_selected,
        },
        Story {
            id: "audio-app-row-live",
            category: "Recorder",
            title: "Audio app row — live with meter + Suggested",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 60,
            },
            render: render_live_row,
        },
        Story {
            id: "audio-app-row-muted",
            category: "Recorder",
            title: "Audio app row — idle / unselected",
            viewport: StoryViewport::Fixed {
                width: 400,
                height: 60,
            },
            render: render_muted_row,
        },
    ]
}

fn render_collapsed() -> String {
    let apps = sample_audio_apps();
    let total = apps.len();
    let selected_subset: Vec<_> = apps.iter().filter(|a| a.selected).cloned().collect();
    let view = sample_system_audio_view(true, false, &selected_subset, total);
    render(view! {
        <div style="padding:12px">
            <SystemAudioRow view=view />
        </div>
    })
}

fn render_expanded() -> String {
    render(view! {
        <div style="padding:12px">
            <SystemAudioAppList apps=sample_audio_apps() active_filter=AudioFilter::Suggested />
        </div>
    })
}

fn render_none_selected() -> String {
    render(view! {
        <div style="padding:12px">
            <SystemAudioAppList apps=sample_audio_apps_none_selected() active_filter=AudioFilter::None />
        </div>
    })
}

fn render_all_selected() -> String {
    render(view! {
        <div style="padding:12px">
            <SystemAudioAppList apps=sample_audio_apps_all_selected() active_filter=AudioFilter::All />
        </div>
    })
}

fn render_live_row() -> String {
    let apps = sample_audio_apps();
    let single = vec![apps[0].clone()];
    render(view! {
        <div style="padding:12px">
            <SystemAudioAppList apps=single active_filter=AudioFilter::Suggested />
        </div>
    })
}

fn render_muted_row() -> String {
    let apps = sample_audio_apps();
    let single = vec![apps[3].clone()];
    render(view! {
        <div style="padding:12px">
            <SystemAudioAppList apps=single active_filter=AudioFilter::All />
        </div>
    })
}
