//! `SystemAudioRow` + `SystemAudioAppList` (M-UI.9 / AUT-129) — the
//! tray popover's system-audio section. Collapsed = one row showing
//! the currently-selected count + overlapping app icons + toggle.
//! Expanded = a filter row (All / None / Suggested) + per-app rows
//! with selection checkboxes + live meters.
//!
//! Stateless: selection set is a prop. Callbacks would land in
//! `app-ui` per the presentational contract.

use leptos::prelude::*;

use crate::components::primitives::{Badge, BadgeKind, Meter, ToggleSwitch};

/// View-model for one app icon — used both in the collapsed stack
/// and inside expanded `AudioAppRow`s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppIconView {
    /// Stable id.
    pub id: &'static str,
    /// 1- or 2-letter monogram drawn inside the tile.
    pub monogram: &'static str,
    /// CSS background color string.
    pub color: &'static str,
}

/// View-model for the collapsed `SystemAudioRow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemAudioView {
    /// `true` when system audio capture is on.
    pub enabled: bool,
    /// `true` when the expanded picker is open below this row.
    pub expanded: bool,
    /// How many apps are currently selected.
    pub selected_count: usize,
    /// How many apps are available in total.
    pub total_count: usize,
    /// Apps to render in the overlapping leading-icon stack. The list
    /// can be longer than [`ICON_STACK_MAX`] — the row truncates and
    /// shows an overflow pill.
    pub icon_stack: Vec<AppIconView>,
}

/// Filter chip kind for the expanded list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFilter {
    /// Selects every app.
    All,
    /// Clears selection.
    None,
    /// Selects the system-suggested subset (apps emitting audio now).
    Suggested,
}

impl AudioFilter {
    /// Display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AudioFilter::All => "All",
            AudioFilter::None => "None",
            AudioFilter::Suggested => "Suggested",
        }
    }

    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            AudioFilter::All => "all",
            AudioFilter::None => "none",
            AudioFilter::Suggested => "suggested",
        }
    }
}

/// One app row inside the expanded list.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioAppView {
    /// Stable id (matches the underlying app handle).
    pub id: &'static str,
    /// Display name (`"Spotify"`, `"Chrome"`).
    pub name: &'static str,
    /// Secondary context (`"Discovery Weekly · 18 m left"`, `"YouTube"`).
    pub context: &'static str,
    /// `true` when this app is in the active selection set.
    pub selected: bool,
    /// `true` to render the "Suggested" badge.
    pub suggested: bool,
    /// `true` to render the pulsing red LIVE dot + label.
    pub live: bool,
    /// Optional normalized audio level `[0, 1]` for the per-app meter.
    pub level: Option<f32>,
    /// Leading icon.
    pub icon: AppIconView,
}

/// Collapsed system-audio row.
#[component]
pub fn SystemAudioRow(view: SystemAudioView) -> impl IntoView {
    let chevron_class = if view.expanded {
        "system-audio-chevron system-audio-chevron-open"
    } else {
        "system-audio-chevron"
    };
    let count_label = format_selection_count(view.selected_count, view.total_count);
    let stack = build_icon_stack(view.icon_stack);
    view! {
        <div class="system-audio-row">
            <span class="system-audio-leading">{stack}</span>
            <span class="system-audio-text">
                <span class="system-audio-title">"System audio"</span>
                <span class="system-audio-subtitle">{count_label}</span>
            </span>
            <span class="system-audio-toggle">
                <ToggleSwitch checked=view.enabled label="Enable system audio".to_string() />
            </span>
            <button class=chevron_class aria-label="Expand system audio app list" aria-expanded=view.expanded>
                "▾"
            </button>
        </div>
    }
}

const ICON_STACK_MAX_VISIBLE: usize = 3;

/// Stack at most [`ICON_STACK_MAX_VISIBLE`] icons + an overflow pill
/// for the rest.
fn build_icon_stack(icons: Vec<AppIconView>) -> AnyView {
    let visible: Vec<_> = icons.iter().take(ICON_STACK_MAX_VISIBLE).cloned().collect();
    let overflow = icons.len().saturating_sub(visible.len());
    let visible_views = visible
        .into_iter()
        .map(|icon| {
            let style = format!("background:{}", icon.color);
            let monogram = icon.monogram;
            view! {
                <span class="system-audio-icon" style=style aria-hidden="true">{monogram}</span>
            }
        })
        .collect_view();
    view! {
        <span class="system-audio-icon-stack">
            {visible_views}
            {(overflow > 0).then(|| {
                let label = format!("+{overflow}");
                view! { <span class="system-audio-icon-overflow" aria-label=format!("{overflow} more")>{label}</span> }
            })}
        </span>
    }
    .into_any()
}

/// Expanded list — filter row + per-app rows.
#[component]
pub fn SystemAudioAppList(
    /// All apps available for selection, in display order.
    apps: Vec<AudioAppView>,
    /// Currently-active filter chip. Cosmetic only; the parent decides
    /// which apps are selected (this prop is purely visual highlight).
    #[prop(optional, default = AudioFilter::All)]
    active_filter: AudioFilter,
) -> impl IntoView {
    let app_rows: Vec<_> = apps.into_iter().map(render_app_row).collect();
    let chips: Vec<_> = [AudioFilter::All, AudioFilter::None, AudioFilter::Suggested]
        .iter()
        .map(|f| {
            let mut class = String::from("system-audio-filter");
            if *f == active_filter {
                class.push_str(" system-audio-filter-active");
            }
            let slug = f.slug();
            let label = f.label();
            view! {
                <button class=class data-filter=slug aria-pressed=*f == active_filter>{label}</button>
            }
        })
        .collect();
    view! {
        <div class="system-audio-applist">
            <div class="system-audio-filters" role="toolbar" aria-label="Filter audio apps">{chips}</div>
            <ul class="system-audio-rows" role="listbox" aria-label="Audio apps">{app_rows}</ul>
        </div>
    }
}

fn render_app_row(app: AudioAppView) -> impl IntoView {
    let mut class = String::from("audio-app-row");
    if app.selected {
        class.push_str(" audio-app-row-selected");
    }
    if app.live {
        class.push_str(" audio-app-row-live");
    }
    let icon_style = format!("background:{}", app.icon.color);
    let monogram = app.icon.monogram;
    let level_view = app.level.map(|l| view! { <Meter level=l bar_count=8 /> });
    view! {
        <li class=class role="option" aria-selected=app.selected>
            <span class=if app.selected { "audio-app-check audio-app-check-on" } else { "audio-app-check" } aria-hidden="true">
                {if app.selected { "✓" } else { "" }}
            </span>
            <span class="audio-app-icon" style=icon_style aria-hidden="true">{monogram}</span>
            <span class="audio-app-text">
                <span class="audio-app-name">{app.name}</span>
                <span class="audio-app-context">{app.context}</span>
            </span>
            {app.suggested.then(|| view! { <Badge kind=BadgeKind::Accent>"Suggested"</Badge> })}
            {app.live.then(|| view! {
                <span class="audio-app-live" aria-label="Live audio">
                    <span class="audio-app-live-dot" aria-hidden="true"></span>
                    "LIVE"
                </span>
            })}
            {level_view.map(|m| view! { <span class="audio-app-meter">{m}</span> })}
        </li>
    }
}

/// Format `"N of M apps"` (plural-aware).
#[must_use]
pub fn format_selection_count(selected: usize, total: usize) -> String {
    let apps = if total == 1 { "app" } else { "apps" };
    format!("{selected} of {total} {apps}")
}

/// Maximum visible icons before the stack collapses to an overflow
/// pill. Module-level so tests + downstream consumers can read it
/// without a Leptos-component receiver (components are functions, not
/// types — `impl Component {}` won't compile).
pub const ICON_STACK_MAX: usize = ICON_STACK_MAX_VISIBLE;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_count_pluralises() {
        assert_eq!(format_selection_count(0, 7), "0 of 7 apps");
        assert_eq!(format_selection_count(4, 7), "4 of 7 apps");
        assert_eq!(format_selection_count(7, 7), "7 of 7 apps");
        assert_eq!(format_selection_count(1, 1), "1 of 1 app");
        assert_eq!(format_selection_count(0, 1), "0 of 1 app");
    }

    #[test]
    fn icon_stack_max_is_three() {
        assert_eq!(ICON_STACK_MAX, 3);
    }

    #[test]
    fn audio_filter_slugs_unique() {
        let slugs = [
            AudioFilter::All.slug(),
            AudioFilter::None.slug(),
            AudioFilter::Suggested.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }
}
