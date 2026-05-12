//! `OnScreenOptionsPopover` — tray-popover section for desktop
//! cleanup / keypress overlay / sensitive-info blur (M-UI.10 /
//! AUT-130). Composes UI-03 `PopoverSurface` + UI-04 `ToggleSwitch`.

use leptos::prelude::*;

use crate::components::menus::{MenuFooter, PopoverPlacement, PopoverSurface};
use crate::components::primitives::ToggleSwitch;

/// Which built-in on-screen option this row represents. Used as the
/// id in `OnScreenOptionView` so callers can match on a stable enum
/// instead of a string id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OnScreenOptionKind {
    /// Hide desktop icons + dock during recording.
    CleanDesktop,
    /// Render keypress badges over the recording.
    ShowKeys,
    /// Auto-blur regions tagged as sensitive (e.g. password fields).
    BlurSensitiveInfo,
}

impl OnScreenOptionKind {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            OnScreenOptionKind::CleanDesktop => "clean-desktop",
            OnScreenOptionKind::ShowKeys => "show-keys",
            OnScreenOptionKind::BlurSensitiveInfo => "blur-sensitive-info",
        }
    }
}

/// View-model for one on-screen-option row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnScreenOptionView {
    /// Which option this row drives.
    pub id: OnScreenOptionKind,
    /// Display title.
    pub title: &'static str,
    /// Multi-line description shown beneath the title.
    pub description: &'static str,
    /// `true` when the toggle is on.
    pub enabled: bool,
    /// `true` to render the row dimmed and non-interactive (e.g.
    /// auto-blur pending feature work).
    pub disabled: bool,
}

#[component]
pub fn OnScreenOptionsPopover(
    /// Options in display order. Order is stable — the parent decides
    /// which option appears first.
    options: Vec<OnScreenOptionView>,
    /// Footer label ("Applies to this recording" / "Applies to all
    /// recordings"). Defaults to the most common case.
    #[prop(optional, default = "Applies to this recording")]
    applies_label: &'static str,
) -> impl IntoView {
    let rows: Vec<_> = options.into_iter().map(render_row).collect();
    view! {
        <PopoverSurface
            placement=PopoverPlacement::BottomLeft
            width_px=380_u16
            title="On-screen".to_string()
            description="Choose what shows during recording.".to_string()
            footer=ToChildren::to_children(move || view! {
                <MenuFooter>
                    <span style="font-size:11px;color:var(--text-tertiary)">{applies_label}</span>
                    <button class="btn btn-default btn-sm">"Done"</button>
                </MenuFooter>
            })
        >
            <ul class="on-screen-options" role="group" aria-label="On-screen options">
                {rows}
            </ul>
        </PopoverSurface>
    }
}

fn render_row(opt: OnScreenOptionView) -> impl IntoView {
    let mut class = String::from("on-screen-option-row");
    if opt.disabled {
        class.push_str(" on-screen-option-row-disabled");
    }
    let toggle_label = format!("Enable {}", opt.title.to_ascii_lowercase());
    view! {
        <li class=class data-option=opt.id.slug()>
            <span class="on-screen-option-toggle">
                <ToggleSwitch checked=opt.enabled disabled=opt.disabled label=toggle_label />
            </span>
            <span class="on-screen-option-text">
                <span class="on-screen-option-title">{opt.title}</span>
                <span class="on-screen-option-description">{opt.description}</span>
            </span>
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_kind_has_unique_slug() {
        let slugs = [
            OnScreenOptionKind::CleanDesktop.slug(),
            OnScreenOptionKind::ShowKeys.slug(),
            OnScreenOptionKind::BlurSensitiveInfo.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }

    #[test]
    fn slugs_are_kebab_case() {
        for k in [
            OnScreenOptionKind::CleanDesktop,
            OnScreenOptionKind::ShowKeys,
            OnScreenOptionKind::BlurSensitiveInfo,
        ] {
            let s = k.slug();
            assert!(!s.is_empty());
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }
}
