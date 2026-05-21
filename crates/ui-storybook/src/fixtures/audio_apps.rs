//! Audio-app fixtures — apps emitting audio that the system-audio
//! picker can target (M-UI.9 / AUT-129).

use crate::components::recorder::{AppIconView, AudioAppView, SystemAudioView};

fn app_icon(id: &str, monogram: &str, color: &str) -> AppIconView {
    AppIconView {
        id: id.to_owned(),
        monogram: monogram.to_owned(),
        color: color.to_owned(),
    }
}

/// All sample apps in display order.
#[must_use]
pub fn sample_audio_apps() -> Vec<AudioAppView> {
    vec![
        AudioAppView {
            id: "spotify".to_owned(),
            name: "Spotify".to_owned(),
            context: "Discover Weekly · 18 m left".to_owned(),
            selected: true,
            suggested: true,
            live: true,
            level: Some(0.62),
            icon: app_icon("spotify", "S", "#16a34a"),
        },
        AudioAppView {
            id: "chrome-yt".to_owned(),
            name: "Chrome — YouTube".to_owned(),
            context: "Tab 3 · TWIR podcast ep 42".to_owned(),
            selected: true,
            suggested: true,
            live: false,
            level: Some(0.18),
            icon: app_icon("chrome", "C", "#fbbf24"),
        },
        AudioAppView {
            id: "discord".to_owned(),
            name: "Discord".to_owned(),
            context: "#engineering".to_owned(),
            selected: false,
            suggested: true,
            live: false,
            level: Some(0.05),
            icon: app_icon("discord", "D", "#5865f2"),
        },
        AudioAppView {
            id: "zoom".to_owned(),
            name: "Zoom".to_owned(),
            context: "Idle · last call 12m ago".to_owned(),
            selected: false,
            suggested: false,
            live: false,
            level: None,
            icon: app_icon("zoom", "Z", "#0ea5e9"),
        },
        AudioAppView {
            id: "messages".to_owned(),
            name: "Messages".to_owned(),
            context: "Notification sounds".to_owned(),
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
