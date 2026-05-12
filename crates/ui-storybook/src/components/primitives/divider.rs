//! `Divider` — thin separator line (M-UI.1 / AUT-121).
//!
//! Used to separate groups inside menus, sidebars, and panels. Vertical
//! variant for inline toolbar separators.

use leptos::prelude::*;

/// Divider orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DividerOrientation {
    /// Spans horizontally; takes the full width of its container.
    #[default]
    Horizontal,
    /// Spans vertically; takes the full height of its container.
    /// 1px wide. Use inside flex rows for inline separators.
    Vertical,
}

impl DividerOrientation {
    /// CSS class for the orientation.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            DividerOrientation::Horizontal => "divider-h",
            DividerOrientation::Vertical => "divider-v",
        }
    }
}

#[component]
pub fn Divider(
    #[prop(optional)] orientation: DividerOrientation,
    #[prop(optional, into)] extra_class: String,
) -> impl IntoView {
    let class = format!(
        "divider {}{}{}",
        orientation.css(),
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! { <div class=class role="separator"></div> }
}
