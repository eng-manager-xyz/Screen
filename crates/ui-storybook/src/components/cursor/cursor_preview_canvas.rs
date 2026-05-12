//! `CursorPreviewCanvas` + `CursorAppearancePanel` (M-UI.21 / AUT-141).
//!
//! Preview canvas reuses the same backend pattern as the editor's
//! `WispCanvasHost` (CSS fallback, pre-rendered Wisp asset, or
//! runtime-unavailable banner). The appearance panel composes UI-18
//! inspector primitives (`PropertySection` + `PropertyControlView`).

use leptos::prelude::*;

use crate::components::editor::{
    InspectorTab, InspectorTabs, PropertyControlView, PropertyRowView, PropertySection,
    PropertySectionView,
};

/// Which backend should render the preview.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum CursorPreviewBackend {
    /// CSS fallback drawn over the light preview card.
    #[default]
    CssFallback,
    /// Pre-rendered Wisp asset (PNG).
    WispAsset {
        /// Relative path from the chapter to the asset.
        asset_path: &'static str,
    },
    /// Runtime not available in this build.
    RuntimeUnavailable,
}

impl CursorPreviewBackend {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(&self) -> &'static str {
        match self {
            CursorPreviewBackend::CssFallback => "css-fallback",
            CursorPreviewBackend::WispAsset { .. } => "wisp-asset",
            CursorPreviewBackend::RuntimeUnavailable => "runtime-unavailable",
        }
    }
}

/// One named cursor color.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorColor {
    /// Display label ("White", "Yellow").
    pub label: &'static str,
    /// CSS color string.
    pub css: &'static str,
}

/// Click-effect variants displayed in the appearance panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClickEffect {
    /// Outline ring expanding from the click point.
    Ring,
    /// Soft pulse.
    Pulse,
    /// Spotlight cone.
    Spotlight,
    /// No click effect.
    None,
}

impl ClickEffect {
    /// Display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            ClickEffect::Ring => "Ring",
            ClickEffect::Pulse => "Pulse",
            ClickEffect::Spotlight => "Spotlight",
            ClickEffect::None => "None",
        }
    }

    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            ClickEffect::Ring => "ring",
            ClickEffect::Pulse => "pulse",
            ClickEffect::Spotlight => "spotlight",
            ClickEffect::None => "none",
        }
    }
}

/// Coarse cursor-behavior preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CursorBehaviorView {
    /// Default — preserves real motion.
    #[default]
    Natural,
    /// Smoothed motion path.
    Smooth,
    /// Snap to elements under the cursor.
    Snap,
}

impl CursorBehaviorView {
    /// Display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            CursorBehaviorView::Natural => "Natural",
            CursorBehaviorView::Smooth => "Smooth",
            CursorBehaviorView::Snap => "Snap to UI",
        }
    }
}

/// Appearance / motion / behavior view-model.
#[derive(Clone, Debug, PartialEq)]
pub struct CursorAppearanceView {
    /// Cursor size 0..=200 (percent of default).
    pub size_percent: u16,
    /// Currently-selected color.
    pub selected_color: CursorColor,
    /// `true` to enable the halo behind the cursor.
    pub halo_enabled: bool,
    /// Halo strength 0..=100.
    pub halo_strength_percent: u16,
    /// Active click effect.
    pub click_effect: ClickEffect,
    /// Motion smoothing 0..=100.
    pub smoothing_percent: u16,
    /// `true` to render a motion trail.
    pub trail_enabled: bool,
    /// Behavior preset.
    pub behavior: CursorBehaviorView,
}

impl CursorAppearanceView {
    /// Clamp `size_percent` into `0..=200`.
    #[must_use]
    pub fn clamped_size(&self) -> u16 {
        self.size_percent.min(200)
    }
}

#[component]
pub fn CursorPreviewCanvas(
    /// Backend selection.
    backend: CursorPreviewBackend,
    /// Background CSS string for the preview card (so stories can
    /// demo dark / light surfaces).
    #[prop(optional, default = "linear-gradient(135deg,#f8fafc,#e5e7eb)")]
    card_background: &'static str,
) -> impl IntoView {
    let style = format!("background: {card_background};");
    let body = match backend {
        CursorPreviewBackend::CssFallback => view! {
            <span class="cursor-preview-arrow" aria-hidden="true">"➤"</span>
        }
        .into_any(),
        CursorPreviewBackend::WispAsset { asset_path } => view! {
            <img class="cursor-preview-asset" src=asset_path alt="Cursor preview" />
        }
        .into_any(),
        CursorPreviewBackend::RuntimeUnavailable => view! {
            <span class="cursor-preview-banner">"Runtime unavailable — CSS preview"</span>
        }
        .into_any(),
    };
    view! {
        <div class="cursor-preview-canvas" style=style role="img" aria-label="Cursor preview">
            <span class="cursor-preview-ring" aria-hidden="true"></span>
            {body}
        </div>
    }
}

#[component]
pub fn CursorAppearancePanel(view: CursorAppearanceView) -> impl IntoView {
    let appearance = appearance_section(&view);
    let click = click_effect_section(&view);
    let motion = motion_section(&view);
    let behavior = behavior_section(&view);
    view! {
        <aside class="cursor-appearance-panel" aria-label="Cursor appearance">
            <InspectorTabs tabs=vec![InspectorTab::Cursor] active=InspectorTab::Cursor />
            <div class="cursor-appearance-body">
                <PropertySection section=appearance />
                <PropertySection section=click />
                <PropertySection section=motion />
                <PropertySection section=behavior />
            </div>
            <div class="cursor-appearance-footer">
                <button class="btn btn-outline btn-sm">"Reset"</button>
                <button class="btn btn-default btn-sm">"Apply"</button>
            </div>
        </aside>
    }
}

fn appearance_section(v: &CursorAppearanceView) -> PropertySectionView {
    PropertySectionView {
        title: "APPEARANCE",
        rows: vec![
            PropertyRowView {
                label: "Size",
                value: Some(leak(format!("{}%", v.clamped_size()))),
                disabled: false,
                control: PropertyControlView::SliderPercent {
                    percent: pct_from_size(v.clamped_size()),
                },
            },
            PropertyRowView {
                label: "Color",
                value: Some(v.selected_color.label),
                disabled: false,
                control: PropertyControlView::ColorSwatches {
                    swatches: vec!["#fafafa", "#facc15", "#38bdf8", "#a78bfa", "#f97316"],
                    selected: color_index(&v.selected_color),
                },
            },
            PropertyRowView {
                label: "Halo",
                value: None,
                disabled: false,
                control: PropertyControlView::Toggle { on: v.halo_enabled },
            },
            PropertyRowView {
                label: "Halo strength",
                value: Some(leak(format!("{}%", v.halo_strength_percent.min(100)))),
                disabled: !v.halo_enabled,
                control: PropertyControlView::SliderPercent {
                    percent: v.halo_strength_percent.min(100) as u8,
                },
            },
        ],
    }
}

fn click_effect_section(v: &CursorAppearanceView) -> PropertySectionView {
    PropertySectionView {
        title: "CLICK EFFECT",
        rows: vec![PropertyRowView {
            label: "Effect",
            value: Some(v.click_effect.label()),
            disabled: false,
            control: PropertyControlView::SelectPill {
                current_label: v.click_effect.label(),
            },
        }],
    }
}

fn motion_section(v: &CursorAppearanceView) -> PropertySectionView {
    PropertySectionView {
        title: "MOTION",
        rows: vec![
            PropertyRowView {
                label: "Smoothing",
                value: Some(leak(format!("{}%", v.smoothing_percent.min(100)))),
                disabled: false,
                control: PropertyControlView::SliderPercent {
                    percent: v.smoothing_percent.min(100) as u8,
                },
            },
            PropertyRowView {
                label: "Trail",
                value: None,
                disabled: false,
                control: PropertyControlView::Toggle {
                    on: v.trail_enabled,
                },
            },
        ],
    }
}

fn behavior_section(v: &CursorAppearanceView) -> PropertySectionView {
    PropertySectionView {
        title: "BEHAVIOR",
        rows: vec![PropertyRowView {
            label: "Preset",
            value: Some(v.behavior.label()),
            disabled: false,
            control: PropertyControlView::SelectPill {
                current_label: v.behavior.label(),
            },
        }],
    }
}

fn pct_from_size(size: u16) -> u8 {
    let pct = u32::from(size) * 100 / 200;
    u8::try_from(pct.min(100)).expect("clamped value fits in u8")
}

fn color_index(c: &CursorColor) -> usize {
    let palette = ["#fafafa", "#facc15", "#38bdf8", "#a78bfa", "#f97316"];
    palette.iter().position(|p| *p == c.css).unwrap_or(0)
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_effect_slugs_unique() {
        let slugs = [
            ClickEffect::Ring.slug(),
            ClickEffect::Pulse.slug(),
            ClickEffect::Spotlight.slug(),
            ClickEffect::None.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }

    #[test]
    fn cursor_appearance_clamps_size() {
        let v = CursorAppearanceView {
            size_percent: 350,
            selected_color: CursorColor {
                label: "White",
                css: "#fafafa",
            },
            halo_enabled: true,
            halo_strength_percent: 250,
            click_effect: ClickEffect::Ring,
            smoothing_percent: 110,
            trail_enabled: false,
            behavior: CursorBehaviorView::Natural,
        };
        assert_eq!(v.clamped_size(), 200);
    }
}
