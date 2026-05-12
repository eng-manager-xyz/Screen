//! `InspectorPanel` + property primitives (M-UI.18 / AUT-138).
//!
//! Right-pane inspector used by both the editor (UI-16/17) and the
//! cursor studio (UI-20/21). Composes a tab strip + zero-or-more
//! `PropertySection` rows. Tabs and active state are controlled
//! props; the component owns no state.

use leptos::prelude::*;

/// Which inspector tab is active.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InspectorTab {
    /// Visual / typographic style.
    #[default]
    Style,
    /// Cursor appearance and motion.
    Cursor,
    /// Audio tracks + ducking.
    Audio,
    /// Generated captions.
    Captions,
    /// AI tools (B-roll suggestions, etc.).
    Ai,
}

impl InspectorTab {
    /// Display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            InspectorTab::Style => "Style",
            InspectorTab::Cursor => "Cursor",
            InspectorTab::Audio => "Audio",
            InspectorTab::Captions => "Captions",
            InspectorTab::Ai => "AI",
        }
    }

    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            InspectorTab::Style => "style",
            InspectorTab::Cursor => "cursor",
            InspectorTab::Audio => "audio",
            InspectorTab::Captions => "captions",
            InspectorTab::Ai => "ai",
        }
    }
}

/// One property row inside a section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertyRowView {
    /// Display label.
    pub label: &'static str,
    /// Optional pre-formatted value text shown right-aligned.
    pub value: Option<&'static str>,
    /// `true` to dim and disable the row.
    pub disabled: bool,
    /// Which built-in control to render. Custom controls land via
    /// composition in the parent.
    pub control: PropertyControlView,
}

/// Built-in property-row controls.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PropertyControlView {
    /// Read-only — just the value.
    ValueOnly,
    /// Slider with a `0..=100` percent. Renders the value column too.
    SliderPercent {
        /// Current value.
        percent: u8,
    },
    /// Toggle switch.
    Toggle {
        /// `true` when on.
        on: bool,
    },
    /// Inline color swatch group. Each `&'static str` is a CSS color.
    ColorSwatches {
        /// Color strings in order.
        swatches: Vec<&'static str>,
        /// Index of the selected swatch.
        selected: usize,
    },
    /// Pill-style select. Renders the value as a button with chevron.
    SelectPill {
        /// Pre-formatted current label.
        current_label: &'static str,
    },
}

/// One section with a heading and rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PropertySectionView {
    /// Uppercase section title ("APPEARANCE", "MOTION").
    pub title: &'static str,
    /// Rows in display order.
    pub rows: Vec<PropertyRowView>,
}

/// Inspector view-model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectorPanelView {
    /// Tabs in display order.
    pub tabs: Vec<InspectorTab>,
    /// Active tab.
    pub active: InspectorTab,
    /// Sections rendered under the active tab.
    pub sections: Vec<PropertySectionView>,
}

#[component]
pub fn InspectorTabs(
    /// Available tabs.
    tabs: Vec<InspectorTab>,
    /// Active tab.
    active: InspectorTab,
) -> impl IntoView {
    let chips: Vec<_> = tabs
        .into_iter()
        .map(|t| {
            let mut class = String::from("inspector-tab");
            if t == active {
                class.push_str(" inspector-tab-active");
            }
            let slug = t.slug();
            let label = t.label();
            let pressed = t == active;
            view! {
                <button class=class data-tab=slug aria-pressed=pressed>{label}</button>
            }
        })
        .collect();
    view! {
        <div class="inspector-tabs" role="tablist" aria-label="Inspector tabs">{chips}</div>
    }
}

#[component]
pub fn PropertySection(section: PropertySectionView) -> impl IntoView {
    let rows: Vec<_> = section.rows.into_iter().map(render_row).collect();
    view! {
        <section class="property-section">
            <span class="property-section-heading">{section.title}</span>
            <ul class="property-section-rows" role="list">{rows}</ul>
        </section>
    }
}

#[component]
pub fn InspectorPanel(view: InspectorPanelView) -> impl IntoView {
    let sections: Vec<_> = view
        .sections
        .into_iter()
        .map(|s| view! { <PropertySection section=s /> })
        .collect();
    view! {
        <aside class="inspector-panel" data-tab=view.active.slug() aria-label="Inspector">
            <InspectorTabs tabs=view.tabs active=view.active />
            <div class="inspector-body">{sections}</div>
        </aside>
    }
}

fn render_row(row: PropertyRowView) -> impl IntoView {
    let mut class = String::from("property-row");
    if row.disabled {
        class.push_str(" property-row-disabled");
    }
    let control = render_control(row.control);
    view! {
        <li class=class>
            <span class="property-row-label">{row.label}</span>
            <span class="property-row-control">{control}</span>
            {row.value.map(|v| view! { <span class="property-row-value">{v}</span> })}
        </li>
    }
}

fn render_control(control: PropertyControlView) -> AnyView {
    match control {
        PropertyControlView::ValueOnly => view! { <span class="property-control-value"></span> }.into_any(),
        PropertyControlView::SliderPercent { percent } => {
            let p = percent.min(100);
            let style = format!("width: {p}%;");
            view! {
                <span class="property-slider" role="slider" aria-valuemin="0" aria-valuemax="100" aria-valuenow=p>
                    <span class="property-slider-fill" style=style></span>
                </span>
            }
            .into_any()
        }
        PropertyControlView::Toggle { on } => view! {
            <span class=if on { "property-toggle property-toggle-on" } else { "property-toggle" } aria-pressed=on>
                <span class="property-toggle-knob"></span>
            </span>
        }
        .into_any(),
        PropertyControlView::ColorSwatches { swatches, selected } => {
            let chips: Vec<_> = swatches
                .iter()
                .enumerate()
                .map(|(i, color)| {
                    let style = format!("background: {color};");
                    let is_sel = i == selected;
                    let class = if is_sel {
                        "property-swatch property-swatch-selected"
                    } else {
                        "property-swatch"
                    };
                    view! {
                        <button class=class style=style aria-pressed=is_sel></button>
                    }
                })
                .collect();
            view! { <span class="property-swatches">{chips}</span> }.into_any()
        }
        PropertyControlView::SelectPill { current_label } => view! {
            <button class="property-pill">{current_label} " ▾"</button>
        }
        .into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_slugs_unique_kebab() {
        let slugs = [
            InspectorTab::Style.slug(),
            InspectorTab::Cursor.slug(),
            InspectorTab::Audio.slug(),
            InspectorTab::Captions.slug(),
            InspectorTab::Ai.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }
}
