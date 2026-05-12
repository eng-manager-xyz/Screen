//! `PopoverSurface` — popover chrome shared across every tray /
//! dropdown / on-screen-options menu (M-UI.3 / AUT-123).

use leptos::prelude::*;

/// Where the popover anchors against its trigger. Drives a CSS class
/// the parent overlay layer can use to position the surface; the
/// component itself doesn't compute coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PopoverPlacement {
    /// Top-left of the trigger.
    TopLeft,
    /// Top-right of the trigger.
    TopRight,
    /// Bottom-left of the trigger. Default for tray menus.
    #[default]
    BottomLeft,
    /// Bottom-right of the trigger.
    BottomRight,
    /// Centered in the viewport — for modal-style popovers.
    Centered,
}

impl PopoverPlacement {
    /// CSS class for the placement.
    #[must_use]
    pub fn css(self) -> &'static str {
        match self {
            PopoverPlacement::TopLeft => "popover-tl",
            PopoverPlacement::TopRight => "popover-tr",
            PopoverPlacement::BottomLeft => "popover-bl",
            PopoverPlacement::BottomRight => "popover-br",
            PopoverPlacement::Centered => "popover-center",
        }
    }
}

#[component]
pub fn PopoverSurface(
    /// Where the popover should anchor — drives a `popover-<placement>`
    /// class but no positioning math.
    #[prop(optional)]
    placement: PopoverPlacement,
    /// Width in CSS pixels. `None` lets content size itself.
    #[prop(optional)]
    width_px: Option<u16>,
    /// Optional title rendered above the content with a small subtitle.
    #[prop(optional, into)]
    title: Option<String>,
    /// Optional description under the title.
    #[prop(optional, into)]
    description: Option<String>,
    /// Body content — typically a `MenuList`.
    children: Children,
    /// Optional bottom footer — primary action button or hint text.
    #[prop(optional)]
    footer: Option<Children>,
) -> impl IntoView {
    let style = width_px.map(|w| format!("width:{w}px")).unwrap_or_default();
    let class = format!("popover-surface {}", placement.css());
    view! {
        <div class=class role="dialog" style=style>
            {title.map(|t| view! {
                <header class="popover-header">
                    <div class="popover-title">{t}</div>
                    {description.map(|d| view! { <div class="popover-description">{d}</div> })}
                </header>
            })}
            <div class="popover-body">{children()}</div>
            {footer.map(|f| view! { <footer class="popover-footer">{f()}</footer> })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_placement_has_unique_class() {
        let classes = [
            PopoverPlacement::TopLeft.css(),
            PopoverPlacement::TopRight.css(),
            PopoverPlacement::BottomLeft.css(),
            PopoverPlacement::BottomRight.css(),
            PopoverPlacement::Centered.css(),
        ];
        let mut sorted = classes.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), classes.len());
    }
}
