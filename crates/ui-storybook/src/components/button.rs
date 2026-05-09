//! Button — rust-ui-style with a small variant matrix.
//!
//! Variants follow the shadcn / rust-ui convention: a primary `Default`, a
//! transparent-edged `Outline`, a borderless `Ghost`, a destructive red, and a
//! quiet `Secondary`. Sizes are `Sm`, `Md` (default), `Lg`.

use leptos::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    #[default]
    Default,
    Outline,
    Ghost,
    Destructive,
    Secondary,
}

impl ButtonVariant {
    fn css(self) -> &'static str {
        match self {
            ButtonVariant::Default => "btn-default",
            ButtonVariant::Outline => "btn-outline",
            ButtonVariant::Ghost => "btn-ghost",
            ButtonVariant::Destructive => "btn-destructive",
            ButtonVariant::Secondary => "btn-secondary",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl ButtonSize {
    fn css(self) -> &'static str {
        match self {
            ButtonSize::Sm => "btn-sm",
            ButtonSize::Md => "btn-md",
            ButtonSize::Lg => "btn-lg",
        }
    }
}

#[component]
pub fn Button(
    #[prop(optional)] variant: ButtonVariant,
    #[prop(optional)] size: ButtonSize,
    #[prop(optional, into)] extra_class: String,
    #[prop(optional)] disabled: bool,
    children: Children,
) -> impl IntoView {
    let class = format!(
        "btn {} {}{}{}",
        variant.css(),
        size.css(),
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! {
        <button class=class disabled=disabled>
            {children()}
        </button>
    }
}
