//! `IconButton` — square icon-only button (M-UI.4 / AUT-124).

use leptos::prelude::*;

/// Visual variant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IconButtonVariant {
    /// Quiet — transparent background, text-secondary glyph. Default.
    #[default]
    Ghost,
    /// Filled neutral chip.
    Filled,
    /// Filled in the danger color.
    Danger,
}

impl IconButtonVariant {
    /// CSS class for the variant.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            IconButtonVariant::Ghost => "icon-btn-ghost",
            IconButtonVariant::Filled => "icon-btn-filled",
            IconButtonVariant::Danger => "icon-btn-danger",
        }
    }
}

#[component]
pub fn IconButton(
    /// Single glyph rendered as the button label.
    #[prop(into)]
    glyph: String,
    /// Accessible label — required for screen readers since the visible
    /// content is just a glyph.
    #[prop(into)]
    label: String,
    #[prop(optional)] variant: IconButtonVariant,
    #[prop(optional)] disabled: bool,
    #[prop(optional)] pressed: bool,
) -> impl IntoView {
    let mut class = format!("icon-btn {}", variant.css());
    if pressed {
        class.push_str(" icon-btn-pressed");
    }
    view! {
        <button class=class disabled=disabled aria-label=label aria-pressed=pressed>
            <span aria-hidden="true">{glyph}</span>
        </button>
    }
}
