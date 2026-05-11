//! `Kbd` — monospaced keyboard-shortcut chip (M-UI.1 / AUT-121).
//!
//! Wraps each key glyph in a `<kbd>` element. The component takes a
//! `keys` prop — a slice of `&'static str` — so the caller composes
//! `["⌘", "⇧", "R"]` and the renderer emits `<kbd>⌘</kbd><kbd>⇧</kbd>
//! <kbd>R</kbd>` with a thin separator glyph between each key.

use leptos::prelude::*;

#[component]
pub fn Kbd(
    /// Keys to render as separate `<kbd>` chips, in display order.
    /// Pass `&["⌘", "R"]` for ⌘R.
    keys: Vec<&'static str>,
    #[prop(optional, into)] extra_class: String,
) -> impl IntoView {
    let class = format!(
        "kbd-row{}{}",
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! {
        <span class=class>
            {keys.into_iter()
                .map(|k| view! { <kbd class="kbd-key">{k}</kbd> })
                .collect_view()}
        </span>
    }
}
