//! `Badge` — small inline label with semantic variants (M-UI.1 /
//! AUT-121).
//!
//! Six kinds covering the recurring badge usages across the product:
//! neutral counts, accent emphasis, danger states, the live-recording
//! pulse, plan / pricing labels, and small numeric counts.

use leptos::prelude::*;

/// Semantic badge kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeKind {
    /// Default — muted background, default text. Used for neutral
    /// metadata ("Free", "Beta").
    #[default]
    Neutral,
    /// Accent — calls attention without alarming. Used for highlights
    /// ("New", "Recommended").
    Accent,
    /// Danger — red. Used for destructive states or error counts.
    Danger,
    /// Live — pulsing red. Used during active recording / capture.
    Live,
    /// Plan — outlined neutral. Used for plan / tier labels.
    Plan,
    /// Count — small monospace numeric badge. Used for unread / queue
    /// counts inside menu items.
    Count,
}

impl BadgeKind {
    /// CSS class that controls the badge look.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            BadgeKind::Neutral => "badge-neutral",
            BadgeKind::Accent => "badge-accent",
            BadgeKind::Danger => "badge-danger",
            BadgeKind::Live => "badge-live",
            BadgeKind::Plan => "badge-plan",
            BadgeKind::Count => "badge-count",
        }
    }
}

#[component]
pub fn Badge(
    #[prop(optional)] kind: BadgeKind,
    #[prop(optional, into)] extra_class: String,
    children: Children,
) -> impl IntoView {
    let class = format!(
        "badge {}{}{}",
        kind.css(),
        if extra_class.is_empty() { "" } else { " " },
        extra_class,
    );
    view! { <span class=class>{children()}</span> }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_unique_class() {
        let classes = [
            BadgeKind::Neutral.css(),
            BadgeKind::Accent.css(),
            BadgeKind::Danger.css(),
            BadgeKind::Live.css(),
            BadgeKind::Plan.css(),
            BadgeKind::Count.css(),
        ];
        let mut sorted = classes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), classes.len());
    }
}
