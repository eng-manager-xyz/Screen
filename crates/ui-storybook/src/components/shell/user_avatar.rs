//! `UserAvatar` — bottom-of-rail user marker (M-UI.2 / AUT-122).
//!
//! Square tile with rounded corners. Shows either an initial monogram
//! or a `src` (URL/path) when available. The component does not load
//! the image — production wiring in `app-ui` sets `src` to the
//! resolved avatar path or leaves it `None` to fall back to monogram.

use leptos::prelude::*;

/// View-model for the user avatar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserAvatarView {
    /// 1- or 2-letter monogram used when `src` is `None`.
    pub monogram: &'static str,
    /// Optional resolved avatar URL/path.
    pub src: Option<&'static str>,
    /// Full display name, used for the accessible title.
    pub name: &'static str,
}

#[component]
pub fn UserAvatar(view: UserAvatarView) -> impl IntoView {
    let src = view.src;
    let monogram = view.monogram;
    view! {
        <button class="user-avatar" title=view.name aria-label=view.name>
            {match src {
                Some(s) => view! { <img class="user-avatar-img" src=s alt=view.name /> }.into_any(),
                None => view! { <span class="user-avatar-monogram">{monogram}</span> }.into_any(),
            }}
        </button>
    }
}
