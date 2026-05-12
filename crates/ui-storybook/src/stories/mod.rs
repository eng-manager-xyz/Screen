//! Story registry — every UI component must have at least one story.
//!
//! Each story is a `(id, render)` pair. The render closure produces an
//! `IntoView` so it can be SSR-rendered to HTML for snapshot tests AND mounted
//! in the browser via Trunk for visual review.
//!
//! Stories are split across per-surface sub-modules
//! ([`primitives`] / [`shell`] / [`recorder`] / [`editor`] /
//! [`menus`] / [`library`] / [`cursor`]) but every shipped story is
//! re-collected into [`all_stories`] in the order they should appear in
//! the gallery sidebar. The flat registry is what
//! `ui_storybook::tests::snapshots` and the `ui-export-stories` binary
//! both consume — they don't need to know about the per-surface split.

pub mod controls;
pub mod cursor;
pub mod editor;
pub mod library;
pub mod menus;
pub mod primitives;
pub mod recorder;
pub mod recorder_audio;
pub mod recorder_devices;
pub mod recorder_display;
pub mod shell;
pub mod workspace_menu;

/// Viewport hint for a story. Drives how the storybook exporter sizes
/// the surrounding chrome and how mdBook's `<iframe>` height is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StoryViewport {
    /// Let the content size itself. The exporter picks a sensible default
    /// height per category.
    #[default]
    Auto,
    /// Fixed-pixel viewport — used by popovers, tray surfaces, and any
    /// component whose layout depends on the surrounding window size.
    Fixed {
        /// Viewport width in CSS pixels.
        width: u16,
        /// Viewport height in CSS pixels.
        height: u16,
    },
    /// Tall, narrow viewport — used by inspector panels, sidebars, and
    /// other vertically-scrolling surfaces. Height is determined by the
    /// content; width is fixed.
    TallPanel {
        /// Viewport width in CSS pixels.
        width: u16,
    },
}

/// One UI gallery story — a closure that renders an `IntoView` to its SSR
/// HTML string, plus identifying metadata.
pub struct Story {
    /// Stable kebab-case identifier — also the asset filename
    /// (`_docs/book/src/assets/ui/<id>.html`).
    pub id: &'static str,
    /// Logical bucket in the gallery (e.g. `"Primitives"`, `"Editor"`).
    pub category: &'static str,
    /// Display title shown in the gallery sidebar.
    pub title: &'static str,
    /// Viewport hint for the exporter / mdBook.
    pub viewport: StoryViewport,
    /// Render closure — produces SSR HTML synchronously.
    pub render: fn() -> String,
}

/// Render any Leptos view to a plain HTML string. Used by every story
/// renderer in the per-surface modules.
///
/// `RenderHtml::to_html` is brought into scope via `leptos::prelude::*`
/// (which re-exports `tachys::prelude::*`). It takes the view by value
/// and walks it into a synchronous HTML string — exactly what we want
/// for an insta snapshot.
pub(crate) fn render<V>(view: V) -> String
where
    V: leptos::IntoView,
{
    use leptos::prelude::*;
    view.into_view().to_html()
}

/// Every shipped story, in display order. Aggregates the per-surface
/// `stories()` lists so the snapshot test + asset exporter see a single
/// flat registry.
#[must_use]
pub fn all_stories() -> Vec<Story> {
    let mut out = Vec::new();
    out.extend(primitives::stories());
    out.extend(controls::stories());
    out.extend(editor::stories());
    out.extend(shell::stories());
    out.extend(recorder::stories());
    out.extend(recorder_display::stories());
    out.extend(recorder_devices::stories());
    out.extend(recorder_audio::stories());
    // Menus is populated by UI-03 / UI-05; library + cursor land later
    // (UI-14 / UI-15 / UI-20 / UI-21).
    out.extend(menus::stories());
    out.extend(workspace_menu::stories());
    out.extend(library::stories());
    out.extend(cursor::stories());
    out
}
