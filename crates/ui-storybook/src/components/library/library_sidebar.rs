//! `LibrarySidebar` + storage meter (M-UI.14 / AUT-134) — left rail of
//! the library screen. Nav items + sections (`SPACES`, `TAGS`) + a
//! bottom storage quota meter.
//!
//! Pure presentational composition over UI-01 / UI-04 primitives. The
//! selected nav id + counts + storage values flow in as props.

use leptos::prelude::*;

/// One row in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryNavItemView {
    /// Stable id ("new", "all", "starred", "shared", "inbox", or a
    /// space/tag id).
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// Leading glyph.
    pub icon: &'static str,
    /// Optional N-recordings count shown muted on the right.
    pub count: Option<u32>,
    /// Optional unread/notification badge ("3" on Inbox).
    pub badge: Option<u32>,
    /// `true` when this row is the active selection.
    pub selected: bool,
    /// `true` to render dimmed and non-interactive.
    pub disabled: bool,
}

/// One section in the sidebar (`SPACES`, `TAGS`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibrarySectionView {
    /// Uppercase section header (`"SPACES"`).
    pub heading: &'static str,
    /// Rows in display order.
    pub items: Vec<LibraryNavItemView>,
}

/// Storage quota meter shown at the bottom of the sidebar.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageMeterView {
    /// Pre-formatted used label ("12.4 GB").
    pub used_bytes_label: String,
    /// Pre-formatted quota label ("50 GB").
    pub quota_label: String,
    /// `[0, 1]` ratio. The renderer clamps and converts to percent.
    pub percent_used: f32,
    /// Optional plan name ("Free", "Pro").
    pub plan_label: Option<&'static str>,
}

/// Composite sidebar view-model.
#[derive(Debug, Clone, PartialEq)]
pub struct LibrarySidebarView {
    /// Top section (New / All / Starred / Shared / Inbox). Rendered
    /// without a section heading.
    pub primary: Vec<LibraryNavItemView>,
    /// Additional sections in display order (`SPACES`, `TAGS`).
    pub sections: Vec<LibrarySectionView>,
    /// Bottom storage meter.
    pub storage: StorageMeterView,
}

/// Clamp a `0.0..=1.0` fraction to a `0..=100` integer percent.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "percent is clamped to [0, 100] before cast"
)]
pub fn storage_percent(fraction: f32) -> u8 {
    (fraction.clamp(0.0, 1.0) * 100.0).round() as u8
}

#[component]
pub fn LibrarySidebar(view: LibrarySidebarView) -> impl IntoView {
    let primary_rows: Vec<_> = view.primary.into_iter().map(render_nav_row).collect();
    let sections: Vec<_> = view
        .sections
        .into_iter()
        .map(|s| {
            let rows: Vec<_> = s.items.into_iter().map(render_nav_row).collect();
            view! {
                <section class="library-section">
                    <span class="library-section-heading">{s.heading}</span>
                    <ul class="library-section-rows" role="group" aria-label=s.heading>{rows}</ul>
                </section>
            }
        })
        .collect();
    view! {
        <aside class="library-sidebar" role="navigation" aria-label="Library">
            <ul class="library-primary" role="list">{primary_rows}</ul>
            {sections}
            <StorageMeter view=view.storage />
        </aside>
    }
}

/// Bottom storage meter (also used standalone in `StorageMeter`
/// stories so the slider is reviewable without the rest of the
/// sidebar).
#[component]
pub fn StorageMeter(view: StorageMeterView) -> impl IntoView {
    let pct = storage_percent(view.percent_used);
    let fill_style = format!("width: {pct}%;");
    let warn_class = if pct >= 85 { " storage-meter-warn" } else { "" };
    let class = format!("storage-meter{warn_class}");
    view! {
        <div class=class>
            <div class="storage-meter-bar" role="progressbar" aria-valuenow=pct aria-valuemin="0" aria-valuemax="100">
                <span class="storage-meter-fill" style=fill_style></span>
            </div>
            <div class="storage-meter-labels">
                <span class="storage-meter-used">{view.used_bytes_label} " / " {view.quota_label}</span>
                {view.plan_label.map(|p| view! {
                    <span class="storage-meter-plan">{p}</span>
                })}
            </div>
        </div>
    }
}

fn render_nav_row(item: LibraryNavItemView) -> impl IntoView {
    let mut class = String::from("library-nav-row");
    if item.selected {
        class.push_str(" library-nav-row-selected");
    }
    if item.disabled {
        class.push_str(" library-nav-row-disabled");
    }
    view! {
        <li class=class data-id=item.id aria-current=item.selected.then_some("page")>
            <span class="library-nav-icon" aria-hidden="true">{item.icon}</span>
            <span class="library-nav-label">{item.label}</span>
            {item.badge.map(|b| view! {
                <span class="library-nav-badge" aria-label=format!("{b} unread")>{b}</span>
            })}
            {item.count.map(|c| view! {
                <span class="library-nav-count">{c}</span>
            })}
        </li>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_percent_clamps() {
        assert_eq!(storage_percent(-0.5), 0);
        assert_eq!(storage_percent(0.0), 0);
        assert_eq!(storage_percent(0.5), 50);
        assert_eq!(storage_percent(0.85), 85);
        assert_eq!(storage_percent(1.0), 100);
        assert_eq!(storage_percent(2.0), 100);
    }
}
