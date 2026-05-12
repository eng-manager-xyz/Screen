//! `CursorStudioShell` + `CursorStylePicker` (M-UI.20 / AUT-140) —
//! the lower style strip in Cursor Studio and the structural shell
//! that holds the preview + inspector + style picker.

use leptos::prelude::*;

/// Cursor style options shown in the bottom strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorStyle {
    /// System default — uses the OS pointer.
    System,
    /// Standard arrow.
    Arrow,
    /// Soft / rounded arrow.
    Soft,
    /// Solid dot.
    Dot,
    /// Hollow ring.
    Ring,
    /// Crosshair / reticle.
    Reticle,
    /// Tactile / 3D-pressed cursor.
    Tactile,
    /// Hide the cursor entirely.
    Hide,
}

impl CursorStyle {
    /// Display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            CursorStyle::System => "System",
            CursorStyle::Arrow => "Arrow",
            CursorStyle::Soft => "Soft",
            CursorStyle::Dot => "Dot",
            CursorStyle::Ring => "Ring",
            CursorStyle::Reticle => "Reticle",
            CursorStyle::Tactile => "Tactile",
            CursorStyle::Hide => "Hide",
        }
    }

    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            CursorStyle::System => "system",
            CursorStyle::Arrow => "arrow",
            CursorStyle::Soft => "soft",
            CursorStyle::Dot => "dot",
            CursorStyle::Ring => "ring",
            CursorStyle::Reticle => "reticle",
            CursorStyle::Tactile => "tactile",
            CursorStyle::Hide => "hide",
        }
    }

    /// Preview glyph drawn inside each tile.
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            CursorStyle::System => "◖",
            CursorStyle::Arrow => "➤",
            CursorStyle::Soft => "❥",
            CursorStyle::Dot => "●",
            CursorStyle::Ring => "○",
            CursorStyle::Reticle => "⊕",
            CursorStyle::Tactile => "◎",
            CursorStyle::Hide => "∅",
        }
    }
}

/// One tile in the picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorStyleTileView {
    /// Which style this tile represents.
    pub style: CursorStyle,
    /// Display label override (falls back to `CursorStyle::label` when None).
    pub label: Option<&'static str>,
    /// `true` for the active tile.
    pub selected: bool,
    /// `true` to dim the tile.
    pub disabled: bool,
}

/// Picker view-model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorStylePickerView {
    /// Tiles in display order.
    pub tiles: Vec<CursorStyleTileView>,
}

#[component]
pub fn CursorStyleTile(view: CursorStyleTileView) -> impl IntoView {
    let style = view.style;
    let mut class = String::from("cursor-style-tile");
    if view.selected {
        class.push_str(" cursor-style-tile-selected");
    }
    if view.disabled {
        class.push_str(" cursor-style-tile-disabled");
    }
    let label = view.label.unwrap_or_else(|| style.label());
    let glyph = style.glyph();
    let slug = style.slug();
    let pressed = view.selected;
    view! {
        <button class=class data-style=slug aria-pressed=pressed disabled=view.disabled>
            <span class="cursor-style-tile-preview" aria-hidden="true">{glyph}</span>
            <span class="cursor-style-tile-label">{label}</span>
        </button>
    }
}

#[component]
pub fn CursorStylePicker(view: CursorStylePickerView) -> impl IntoView {
    let tiles: Vec<_> = view
        .tiles
        .into_iter()
        .map(|t| view! { <CursorStyleTile view=t /> })
        .collect();
    view! {
        <section class="cursor-style-picker" aria-label="Cursor style">
            <span class="cursor-style-heading">"CURSOR STYLE"</span>
            <div class="cursor-style-tiles">{tiles}</div>
        </section>
    }
}

/// Shell view-model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CursorStudioShellView {
    /// Picker shown at the bottom.
    pub picker: CursorStylePickerView,
}

#[component]
pub fn CursorStudioShell(
    /// Shell view-model.
    view: CursorStudioShellView,
    /// Top preview slot.
    #[prop(optional)]
    preview: Option<Children>,
    /// Right inspector slot.
    #[prop(optional)]
    inspector: Option<Children>,
) -> impl IntoView {
    view! {
        <section class="cursor-studio-shell" aria-label="Cursor Studio">
            <div class="cursor-studio-body">
                <div class="cursor-studio-preview">{preview.map(|p| p())}</div>
                <aside class="cursor-studio-inspector">{inspector.map(|i| i())}</aside>
            </div>
            <CursorStylePicker view=view.picker />
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_style_slugs_unique() {
        let styles = [
            CursorStyle::System,
            CursorStyle::Arrow,
            CursorStyle::Soft,
            CursorStyle::Dot,
            CursorStyle::Ring,
            CursorStyle::Reticle,
            CursorStyle::Tactile,
            CursorStyle::Hide,
        ];
        let slugs: Vec<_> = styles.iter().map(|s| s.slug()).collect();
        let mut sorted = slugs.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }

    #[test]
    fn cursor_style_label_round_trips() {
        for s in [CursorStyle::Arrow, CursorStyle::Dot, CursorStyle::Hide] {
            assert!(!s.label().is_empty());
            assert!(!s.glyph().is_empty());
        }
    }
}
