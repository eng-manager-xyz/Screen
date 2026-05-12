//! Audio-app fixtures — apps emitting audio that the system-audio
//! picker can target (M-UI.9 / AUT-129).

use crate::components::recorder::{AppIconView, AudioAppView, SystemAudioView};

fn app_icon(id: &'static str, monogram: &'static str, color: &'static str) -> AppIconView {
    AppIconView {
        id,
        monogram,
        color,
    }
}

/// All sample apps in display order.
#[must_use]
pub fn sample_audio_apps() -> Vec<AudioAppView> {
    vec![
        AudioAppView {
            id: "spotify",
            name: "Spotify",
            context: "Discover Weekly · 18 m left",
            selected: true,
            suggested: true,
            live: true,
            level: Some(0.62),
            icon: app_icon("spotify", "S", "#16a34a"),
        },
        AudioAppView {
            id: "chrome-yt",
            name: "Chrome — YouTube",
            context: "Tab 3 · TWIR podcast ep 42",
            selected: true,
            suggested: true,
            live: false,
            level: Some(0.18),
            icon: app_icon("chrome", "C", "#fbbf24"),
        },
        AudioAppView {
            id: "discord",
            name: "Discord",
            context: "#engineering",
            selected: false,
            suggested: true,
            live: false,
            level: Some(0.05),
            icon: app_icon("discord", "D", "#5865f2"),
        },
        AudioAppView {
            id: "zoom",
            name: "Zoom",
            context: "Idle · last call 12m ago",
            selected: false,
            suggested: false,
            live: false,
            level: None,
            icon: app_icon("zoom", "Z", "#0ea5e9"),
        },
        AudioAppView {
            id: "messages",
            name: "Messages",
            context: "Notification sounds",
            selected: false,
            suggested: false,
            live: false,
            level: None,
            icon: app_icon("messages", "M", "#22c55e"),
        },
    ]
}

/// Variant with no apps selected.
#[must_use]
pub fn sample_audio_apps_none_selected() -> Vec<AudioAppView> {
    sample_audio_apps()
        .into_iter()
        .map(|mut a| {
            a.selected = false;
            a
        })
        .collect()
}

/// Variant with every app selected.
#[must_use]
pub fn sample_audio_apps_all_selected() -> Vec<AudioAppView> {
    sample_audio_apps()
        .into_iter()
        .map(|mut a| {
            a.selected = true;
            a
        })
        .collect()
}

/// View-model for the collapsed `SystemAudioRow`.
#[must_use]
pub fn sample_system_audio_view(
    enabled: bool,
    expanded: bool,
    selected_apps: &[AudioAppView],
    total: usize,
) -> SystemAudioView {
    let icon_stack: Vec<AppIconView> = selected_apps.iter().map(|a| a.icon.clone()).collect();
    SystemAudioView {
        enabled,
        expanded,
        selected_count: selected_apps.iter().filter(|a| a.selected).count(),
        total_count: total,
        icon_stack,
    }
}
