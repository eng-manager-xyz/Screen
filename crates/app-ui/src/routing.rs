//! Pure-Rust URL-routing helpers (M-TRAY.4 / AUT-253, M-BUBBLE.0 / AUT-273).
//!
//! The browser-facing entry points in [`crate`] pull the query string
//! off `window.location.search()` and hand it to [`parse_surface`]
//! (for the in-`AppShell` surface) or [`parse_mount_point`] (for the
//! window-level mount dispatch — `AppShell` vs `Bubble` vs `DropZone`).
//! Splitting the parse out of the wasm-only path makes it testable
//! on every OS (no `web_sys`, no Tauri runtime).

use ui_storybook::components::shell::AppSection;

/// Which Leptos tree the current page mounts — drives the top-level
/// dispatch in [`crate::run`].
///
/// Distinct from [`AppSection`] (which is for navigation-rail items
/// inside the `AppShell`). A `?mount=bubble` page renders the bubble
/// canvas, not a `NavigationRail` surface; keeping the two enums
/// separate prevents `AppShell`-internal navigation from accidentally
/// reaching the bubble or vice versa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MountPoint {
    /// `?surface=<recorder|library|editor|cursor|prefs>` —
    /// the in-`AppShell` `NavigationRail` surface (M-TRAY.3 / AUT-252).
    AppShell(AppSection),
    /// `?mount=bubble` — the borderless webcam overlay window
    /// (M-BUBBLE.0 / AUT-273).
    Bubble,
    /// No recognised query — falls back to the legacy drop-zone shell
    /// preserved for `trunk serve` browser dev (M-INT.1 flow).
    DropZone,
}

/// Parse a URL query string (with or without the leading `?`) into a
/// [`MountPoint`]. `?surface=…` wins over `?mount=…` so the `AppShell`
/// surface query takes precedence — matters only if both ever appear
/// in the same URL, which they shouldn't.
#[must_use]
pub fn parse_mount_point(query: &str) -> MountPoint {
    if let Some(section) = parse_surface(query) {
        return MountPoint::AppShell(section);
    }
    let stripped = query.strip_prefix('?').unwrap_or(query);
    for pair in stripped.split('&') {
        if let Some((key, value)) = pair.split_once('=')
            && key == "mount"
            && value == "bubble"
        {
            return MountPoint::Bubble;
        }
    }
    MountPoint::DropZone
}

/// Parse a URL query string (with or without the leading `?`) for a
/// `surface=<slug>` parameter. Returns the matching [`AppSection`]
/// or `None` if the param is missing / unrecognised.
///
/// Round-trips with [`surface_slug`]: parsing the output of
/// `surface_slug(x)` returns `Some(x)` for every variant.
#[must_use]
pub fn parse_surface(query: &str) -> Option<AppSection> {
    let stripped = query.strip_prefix('?').unwrap_or(query);
    for pair in stripped.split('&') {
        if let Some((key, value)) = pair.split_once('=')
            && key == "surface"
        {
            return parse_slug(value);
        }
    }
    None
}

/// Map a slug to its [`AppSection`]. Accepts both `recorder` and
/// `record` for the Record variant (the storybook uses `record` in
/// the kebab-case slug; the URL convention is the longer `recorder`).
#[must_use]
pub fn parse_slug(slug: &str) -> Option<AppSection> {
    match slug {
        "recorder" | "record" => Some(AppSection::Record),
        "library" => Some(AppSection::Library),
        "editor" => Some(AppSection::Editor),
        "cursor" => Some(AppSection::Cursor),
        "prefs" => Some(AppSection::Prefs),
        _ => None,
    }
}

/// Convert an [`AppSection`] to its canonical URL slug. Round-trips
/// with [`parse_slug`] / [`parse_surface`].
#[must_use]
pub fn surface_slug(section: AppSection) -> &'static str {
    match section {
        AppSection::Record => "recorder",
        AppSection::Library => "library",
        AppSection::Editor => "editor",
        AppSection::Cursor => "cursor",
        AppSection::Prefs => "prefs",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_every_section() {
        for section in [
            AppSection::Record,
            AppSection::Library,
            AppSection::Editor,
            AppSection::Cursor,
            AppSection::Prefs,
        ] {
            let slug = surface_slug(section);
            let parsed = parse_slug(slug);
            assert_eq!(parsed, Some(section), "slug={slug}");
        }
    }

    #[test]
    fn parse_surface_finds_param_with_leading_question_mark() {
        assert_eq!(parse_surface("?surface=recorder"), Some(AppSection::Record));
    }

    #[test]
    fn parse_surface_finds_param_without_leading_question_mark() {
        assert_eq!(parse_surface("surface=library"), Some(AppSection::Library));
    }

    #[test]
    fn parse_surface_finds_param_among_multiple() {
        assert_eq!(
            parse_surface("?foo=bar&surface=editor&baz=quux"),
            Some(AppSection::Editor)
        );
    }

    #[test]
    fn parse_surface_returns_none_for_unknown_slug() {
        assert_eq!(parse_surface("?surface=mystery"), None);
    }

    #[test]
    fn parse_surface_returns_none_for_missing_param() {
        assert_eq!(parse_surface("?foo=bar"), None);
        assert_eq!(parse_surface(""), None);
    }

    #[test]
    fn parse_surface_accepts_record_alias() {
        // The storybook slug for `Record` is `record`; the URL
        // convention is `recorder`. We accept both.
        assert_eq!(parse_surface("?surface=record"), Some(AppSection::Record));
        assert_eq!(parse_surface("?surface=recorder"), Some(AppSection::Record));
    }

    #[test]
    fn mount_point_dispatches_appshell_for_surface_query() {
        assert_eq!(
            parse_mount_point("?surface=recorder"),
            MountPoint::AppShell(AppSection::Record)
        );
    }

    #[test]
    fn mount_point_dispatches_bubble_for_mount_query() {
        assert_eq!(parse_mount_point("?mount=bubble"), MountPoint::Bubble);
        assert_eq!(parse_mount_point("mount=bubble"), MountPoint::Bubble);
    }

    #[test]
    fn mount_point_falls_back_to_drop_zone_for_unknown_query() {
        assert_eq!(parse_mount_point(""), MountPoint::DropZone);
        assert_eq!(parse_mount_point("?foo=bar"), MountPoint::DropZone);
        assert_eq!(parse_mount_point("?mount=unknown"), MountPoint::DropZone);
    }

    #[test]
    fn mount_point_appshell_wins_when_both_queries_present() {
        // Defensive: shouldn't happen in practice, but if both are
        // set we route to the AppShell surface (the higher-information
        // signal).
        assert_eq!(
            parse_mount_point("?surface=library&mount=bubble"),
            MountPoint::AppShell(AppSection::Library)
        );
    }
}
