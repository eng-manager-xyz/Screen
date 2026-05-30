//! Style inspector — background framing (ED.18 / M-EDIT).
//!
//! The "look" panel: the backdrop the screen sits on (a gradient or flat
//! fill) plus the padding, corner radius, and drop shadow that lift the
//! recording off it — the produced, on-a-nice-desktop framing every modern
//! screen recorder ships. All of it drives one
//! [`BackgroundConfig`](edit::style::BackgroundConfig) on the project,
//! re-derived at render time; the *visible* framing (the drawn padding /
//! rounded canvas / shadow) lands with the render-integration / export pass
//! (ED.20 / ED.21). This chunk is the authoring side, undoable through the
//! shared [`edit::History`].

use edit::EditProject;
use edit::style::{BackgroundConfig, BackgroundSource};
use leptos::prelude::*;

type ProjectSignal = Option<RwSignal<Option<EditProject>>>;
type HistoryStore = Option<StoredValue<Option<edit::History>>>;

/// Background swatch presets, in display order.
#[must_use]
pub fn background_presets() -> Vec<(&'static str, BackgroundSource)> {
    vec![
        ("Aurora", BackgroundSource::default()),
        (
            "Ocean",
            BackgroundSource::Gradient {
                from: [33, 147, 176],
                to: [109, 213, 237],
                angle_deg: 135.0,
            },
        ),
        (
            "Sunset",
            BackgroundSource::Gradient {
                from: [255, 126, 95],
                to: [254, 180, 123],
                angle_deg: 135.0,
            },
        ),
        ("Graphite", BackgroundSource::Color { rgb: [38, 40, 46] }),
        (
            "Snow",
            BackgroundSource::Color {
                rgb: [240, 240, 242],
            },
        ),
    ]
}

/// A CSS `background` value for a [`BackgroundSource`] — used both for the
/// swatch previews and (eventually) the live canvas backdrop.
#[must_use]
pub fn source_css(source: &BackgroundSource) -> String {
    match source {
        BackgroundSource::Gradient {
            from,
            to,
            angle_deg,
        } => format!(
            "linear-gradient({angle_deg}deg, rgb({},{},{}), rgb({},{},{}))",
            from[0], from[1], from[2], to[0], to[1], to[2]
        ),
        BackgroundSource::Color { rgb } => format!("rgb({},{},{})", rgb[0], rgb[1], rgb[2]),
        // Wallpapers need bundled assets (deferred) — show a neutral chip.
        BackgroundSource::Wallpaper { .. } => "#33363d".to_owned(),
    }
}

/// Whether two background sources are the same preset.
#[must_use]
pub fn is_active_source(current: &BackgroundSource, preset: &BackgroundSource) -> bool {
    current == preset
}

/// Parse a numeric field to a `u32`, clamped to `0..=max`. Non-numeric → 0.
#[must_use]
pub fn parse_u32_field(s: &str, max: u32) -> u32 {
    s.trim().parse::<u32>().unwrap_or(0).min(max)
}

fn swatches(project: ProjectSignal, history: HistoryStore) -> AnyView {
    let current = project
        .and_then(|s| s.get().map(|p| p.background.source))
        .unwrap_or_default();
    background_presets()
        .into_iter()
        .map(|(name, source)| {
            let mut class = String::from("style-swatch");
            if is_active_source(&current, &source) {
                class.push_str(" style-swatch--active");
            }
            let css = source_css(&source);
            view! {
                <button
                    class=class
                    title=name
                    style=format!("background:{css}")
                    on:click=move |_| {
                        if let (Some(p), Some(h)) = (project, history) {
                            let mut cfg = p
                                .get_untracked()
                                .map(|pr| pr.background)
                                .unwrap_or_default();
                            cfg.source = source.clone();
                            crate::editor_edits::set_background(p, h, cfg);
                        }
                    }
                ></button>
            }
        })
        .collect_view()
        .into_any()
}

/// One labelled numeric field bound to a `BackgroundConfig` field.
fn number_field(
    project: ProjectSignal,
    history: HistoryStore,
    label: &'static str,
    value: u32,
    max: u32,
    set: fn(&mut BackgroundConfig, u32),
) -> AnyView {
    view! {
        <label class="style-field">
            <span class="style-field-label">{label}</span>
            <input
                class="style-field-input"
                type="number"
                min="0"
                max=max.to_string()
                prop:value=value.to_string()
                on:change=move |ev| {
                    if let (Some(p), Some(h)) = (project, history) {
                        let mut cfg = p.get_untracked().map(|pr| pr.background).unwrap_or_default();
                        set(&mut cfg, parse_u32_field(&event_target_value(&ev), max));
                        crate::editor_edits::set_background(p, h, cfg);
                    }
                }
            />
        </label>
    }
    .into_any()
}

/// The style inspector: backdrop swatches + padding / radius / shadow.
#[component]
pub fn StyleInspector() -> impl IntoView {
    let project = use_context::<RwSignal<Option<EditProject>>>();
    let history = use_context::<StoredValue<Option<edit::History>>>();
    view! {
        <div class="style-inspector">
            <div class="clip-inspector-section">
                <h3 class="clip-inspector-title">"Background"</h3>
                <div class="style-swatches">{move || swatches(project, history)}</div>
            </div>
            <div class="clip-inspector-section">
                <h3 class="clip-inspector-title">"Framing"</h3>
                <div class="style-fields">
                    {move || {
                        let bg = project
                            .and_then(|s| s.get().map(|p| p.background))
                            .unwrap_or_default();
                        view! {
                            {number_field(project, history, "Padding", bg.padding, 400, |c, v| c.padding = v)}
                            {number_field(project, history, "Radius", bg.corner_radius, 200, |c, v| c.corner_radius = v)}
                            {number_field(project, history, "Shadow", bg.shadow, 100, |c, v| c.shadow = v)}
                        }
                    }}
                </div>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_include_default_and_a_color() {
        let p = background_presets();
        assert!(p.len() >= 4);
        assert!(p.iter().any(|(n, _)| *n == "Aurora"));
        assert!(
            p.iter()
                .any(|(_, s)| matches!(s, BackgroundSource::Color { .. }))
        );
    }

    #[test]
    fn source_css_renders_gradient_and_color() {
        let g = source_css(&BackgroundSource::Gradient {
            from: [1, 2, 3],
            to: [4, 5, 6],
            angle_deg: 90.0,
        });
        assert_eq!(g, "linear-gradient(90deg, rgb(1,2,3), rgb(4,5,6))");
        assert_eq!(
            source_css(&BackgroundSource::Color { rgb: [10, 20, 30] }),
            "rgb(10,20,30)"
        );
    }

    #[test]
    fn parse_u32_clamps_and_defaults() {
        assert_eq!(parse_u32_field("64", 400), 64);
        assert_eq!(parse_u32_field("999", 400), 400); // clamp to max
        assert_eq!(parse_u32_field("abc", 400), 0); // non-numeric
        assert_eq!(parse_u32_field("-5", 400), 0); // negative → parse fail → 0
    }

    #[test]
    fn active_source_matches_equal_presets() {
        let a = BackgroundSource::Color { rgb: [1, 1, 1] };
        let b = BackgroundSource::Color { rgb: [1, 1, 1] };
        let c = BackgroundSource::Color { rgb: [2, 2, 2] };
        assert!(is_active_source(&a, &b));
        assert!(!is_active_source(&a, &c));
    }
}
