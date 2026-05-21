//! `DisplaySourceCard` + `DisplayPreviewFrame` — selected screen card
//! shown in the tray record popover after capture-mode selection
//! (M-UI.7 / AUT-127).
//!
//! Pure presentation. SSR-stable: the preview is rendered as
//! CSS-positioned divs (the "deterministic non-Wisp fallback" called
//! out in the spec). A Wisp-backed PNG variant can land later via the
//! existing `wisp-export-stories` harness.

use leptos::prelude::*;

/// View-model for the card.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplaySourceView {
    /// Stable id.
    pub id: String,
    /// Display name ("Built-in Retina display").
    pub name: String,
    /// Compact size label ("14\"").
    pub size_label: String,
    /// Resolution label ("3024 × 1964").
    pub dimensions_label: String,
    /// `true` to render the favourited star glyph.
    pub is_favorite: bool,
    /// `true` to outline the card in the action-record color.
    pub is_selected: bool,
    /// Preview content shown inside the card.
    pub preview: DisplayPreviewView,
}

/// Visual preview content for the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplayPreviewView {
    /// Aspect ratio numerator + denominator (e.g. `(16, 10)` for the
    /// Retina display). Drives the `aspect-ratio:` CSS property.
    pub aspect_ratio: (u16, u16),
    /// Optional overlay label drawn in the top-right corner ("Built-in
    /// 14\"").
    pub overlay_label: Option<String>,
    /// Mock window chips drawn inside the preview to suggest "this is
    /// what would be captured".
    pub mock_windows: Vec<PreviewWindowChip>,
}

/// One mock window chip — drawn as a colored rounded rect at a
/// percentage offset / size inside the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewWindowChip {
    /// Display label (truncates).
    pub label: String,
    /// CSS background color string.
    pub color: String,
    /// Left offset in `[0, 100]` percent.
    pub left_pct: u8,
    /// Top offset in `[0, 100]` percent.
    pub top_pct: u8,
    /// Width in `[0, 100]` percent.
    pub width_pct: u8,
    /// Height in `[0, 100]` percent.
    pub height_pct: u8,
}

#[component]
pub fn DisplayPreviewFrame(view: DisplayPreviewView) -> impl IntoView {
    let (num, den) = view.aspect_ratio;
    let aspect_style = format!("aspect-ratio: {num} / {den}");
    view! {
        <div class="display-preview" style=aspect_style>
            <div class="display-preview-titlebar">
                <span class="display-preview-tlbtn display-preview-tlbtn-red"></span>
                <span class="display-preview-tlbtn display-preview-tlbtn-amber"></span>
                <span class="display-preview-tlbtn display-preview-tlbtn-green"></span>
            </div>
            <div class="display-preview-body">
                {view.mock_windows.into_iter().map(|chip| {
                    let style = format!(
                        "left:{}%;top:{}%;width:{}%;height:{}%;background:{};",
                        chip.left_pct, chip.top_pct, chip.width_pct, chip.height_pct, chip.color,
                    );
                    view! {
                        <span class="display-preview-chip" style=style>
                            <span class="display-preview-chip-label">{chip.label}</span>
                        </span>
                    }
                }).collect_view()}
                {view.overlay_label.as_ref().map(|l| {
                    let label = l.clone();
                    view! {
                        <span class="display-preview-overlay">{label}</span>
                    }
                })}
            </div>
        </div>
    }
}

#[component]
pub fn DisplaySourceCard(
    view: DisplaySourceView,
    /// `true` when the parent's source-picker popover is open.
    #[prop(optional)]
    open: bool,
    /// `true` to render the card disabled (e.g. permissions blocked).
    #[prop(optional)]
    unavailable: bool,
) -> impl IntoView {
    let mut card_class = String::from("display-source-card");
    if view.is_selected {
        card_class.push_str(" display-source-card-selected");
    }
    if unavailable {
        card_class.push_str(" display-source-card-unavailable");
    }
    let mut chevron_class = String::from("display-source-chevron");
    if open {
        chevron_class.push_str(" display-source-chevron-open");
    }
    let name = view.name;
    let size = view.size_label;
    let dims = view.dimensions_label;
    let favorite = view.is_favorite;
    let preview = view.preview.clone();
    view! {
        <div class=card_class>
            <header class="display-source-header">
                <div class="display-source-label">
                    <span class="display-source-name">{name}</span>
                    <span class="display-source-size">{size}</span>
                    {favorite.then(|| view! { <span class="display-source-star" aria-label="Favourite" title="Favourite">"★"</span> })}
                </div>
                <div class="display-source-meta">
                    <span class="display-source-dims">{dims}</span>
                    <span class=chevron_class aria-hidden="true">"▾"</span>
                </div>
            </header>
            <DisplayPreviewFrame view=preview />
            {unavailable.then(|| view! {
                <div class="display-source-unavailable-banner">
                    "Screen recording permission required"
                </div>
            })}
        </div>
    }
}

/// CSS `aspect-ratio` property value for a `(num, den)` pair, with
/// `1 / 1` fallback for zero denominator.
#[must_use]
pub fn aspect_ratio_css(num: u16, den: u16) -> String {
    if den == 0 {
        "1 / 1".to_string()
    } else {
        format!("{num} / {den}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_css_basic() {
        assert_eq!(aspect_ratio_css(16, 9), "16 / 9");
        assert_eq!(aspect_ratio_css(3024, 1964), "3024 / 1964");
    }

    #[test]
    fn aspect_ratio_css_zero_denominator_falls_back() {
        assert_eq!(aspect_ratio_css(16, 0), "1 / 1");
    }
}
