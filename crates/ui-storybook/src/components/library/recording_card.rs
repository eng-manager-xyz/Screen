//! `RecordingCard` + `LibraryGrid` (M-UI.15 / AUT-135). Card renders
//! one library tile (thumbnail + footer + processing overlay); grid
//! arranges cards under a `LibraryToolbar` (filter pills + sort).
//!
//! No real video — `ThumbnailView` is a CSS-gradient mock so SSR + the
//! mdBook iframe both render deterministically.

use leptos::prelude::*;

/// Card lifecycle state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecordingCardState {
    /// Encoded + playable.
    Ready,
    /// Encoder running. `percent` is `0..=100`.
    Processing {
        /// Completion percent.
        percent: u8,
    },
    /// Encode aborted — render the error overlay.
    Failed,
}

impl RecordingCardState {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            RecordingCardState::Ready => "ready",
            RecordingCardState::Processing { .. } => "processing",
            RecordingCardState::Failed => "failed",
        }
    }
}

/// CSS-gradient mock thumbnail. The grid renders a colored block,
/// not a real video frame, so SSR snapshots stay deterministic.
#[derive(Clone, Debug, PartialEq)]
pub struct ThumbnailView {
    /// Inline CSS `background:` value (gradient or solid). Empty
    /// string renders the missing-thumbnail placeholder.
    pub css_background: String,
}

/// Engagement metrics shown in the card footer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordingMetricsView {
    /// Total views.
    pub views: u32,
    /// Comment count.
    pub comments: u32,
    /// Reaction count.
    pub reactions: u32,
}

/// View-model for one card.
#[derive(Clone, Debug, PartialEq)]
pub struct RecordingCardView {
    /// Stable id (file uuid in production).
    pub id: String,
    /// Display title.
    pub title: String,
    /// Pre-formatted captured-at label ("Captured 2026-05-09").
    pub subtitle: String,
    /// Pre-formatted duration ("1m 24s").
    pub duration_label: String,
    /// Optional category pill text ("Tutorial").
    pub category: Option<String>,
    /// Metric counts.
    pub metrics: RecordingMetricsView,
    /// Thumbnail (mock gradient).
    pub thumbnail: ThumbnailView,
    /// Lifecycle state.
    pub state: RecordingCardState,
    /// `true` when this card is the selected one.
    pub selected: bool,
}

/// Toolbar layout mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LibraryLayoutMode {
    /// Card grid (default).
    #[default]
    Grid,
    /// List rows.
    List,
}

impl LibraryLayoutMode {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            LibraryLayoutMode::Grid => "grid",
            LibraryLayoutMode::List => "list",
        }
    }
}

/// Filter chip view-model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordingFilterView {
    /// Stable id (`"all"`, `"tutorial"`, `"demo"`).
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// `true` for the active chip.
    pub active: bool,
}

/// View-model for the toolbar above the grid.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryToolbarView {
    /// Filter chips in display order.
    pub filters: Vec<RecordingFilterView>,
    /// Pre-formatted sort label ("Most recent", "Most viewed").
    pub sort_label: &'static str,
    /// Active layout.
    pub layout: LibraryLayoutMode,
}

/// View-model for the whole grid.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryGridView {
    /// Toolbar above the grid.
    pub toolbar: LibraryToolbarView,
    /// Cards in display order.
    pub recordings: Vec<RecordingCardView>,
    /// Empty-state message shown when `recordings` is empty.
    pub empty_message: &'static str,
}

#[component]
pub fn RecordingCard(view: RecordingCardView) -> impl IntoView {
    let mut class = format!("recording-card recording-card-{}", view.state.slug());
    if view.selected {
        class.push_str(" recording-card-selected");
    }
    let thumb_style = if view.thumbnail.css_background.is_empty() {
        "background: var(--surface-elevated);".to_owned()
    } else {
        format!("background: {};", view.thumbnail.css_background)
    };
    let category_pill = view.category.as_ref().map(|c| {
        let text = c.clone();
        view! { <span class="recording-card-category">{text}</span> }
    });
    let metrics_row = view! {
        <span class="recording-card-metrics" aria-label="Engagement">
            <span class="recording-card-metric">"▶ " {view.metrics.views}</span>
            <span class="recording-card-metric">"💬 " {view.metrics.comments}</span>
            <span class="recording-card-metric">"❤ " {view.metrics.reactions}</span>
        </span>
    };
    let overlay = render_state_overlay(view.state);
    view! {
        <article class=class data-id=view.id role="group" aria-label="Recording">
            <div class="recording-card-thumb" style=thumb_style>
                {overlay}
                <span class="recording-card-duration">{view.duration_label}</span>
                {category_pill}
            </div>
            <div class="recording-card-body">
                <span class="recording-card-title">{view.title}</span>
                <span class="recording-card-subtitle">{view.subtitle}</span>
                {metrics_row}
            </div>
        </article>
    }
}

fn render_state_overlay(state: RecordingCardState) -> Option<impl IntoView> {
    match state {
        RecordingCardState::Ready => None,
        RecordingCardState::Processing { percent } => {
            let p = percent.min(100);
            let fill = format!("width: {p}%;");
            Some(
                view! {
                    <span class="recording-card-overlay recording-card-overlay-processing">
                        <span class="recording-card-overlay-label">"Processing " {p} "%"</span>
                        <span class="recording-card-overlay-bar">
                            <span class="recording-card-overlay-fill" style=fill></span>
                        </span>
                    </span>
                }
                .into_any(),
            )
        }
        RecordingCardState::Failed => Some(
            view! {
                <span class="recording-card-overlay recording-card-overlay-failed">
                    <span class="recording-card-overlay-label">"Encode failed"</span>
                </span>
            }
            .into_any(),
        ),
    }
}

#[component]
pub fn LibraryToolbar(view: LibraryToolbarView) -> impl IntoView {
    let chips: Vec<_> = view
        .filters
        .iter()
        .map(|f| {
            let mut class = String::from("library-filter");
            if f.active {
                class.push_str(" library-filter-active");
            }
            let id = f.id;
            let label = f.label;
            let pressed = f.active;
            view! {
                <button class=class data-filter=id aria-pressed=pressed>{label}</button>
            }
        })
        .collect();
    let layout = view.layout;
    view! {
        <div class="library-toolbar" data-layout=layout.slug()>
            <div class="library-filters" role="toolbar" aria-label="Filters">{chips}</div>
            <div class="library-toolbar-right">
                <button class="library-sort">{view.sort_label} " ▾"</button>
                <span class="library-layout-toggle" role="group" aria-label="Layout">
                    <button
                        class=if matches!(layout, LibraryLayoutMode::Grid) { "library-layout-btn library-layout-btn-active" } else { "library-layout-btn" }
                        data-layout="grid"
                        aria-pressed=matches!(layout, LibraryLayoutMode::Grid)
                    >"▦"</button>
                    <button
                        class=if matches!(layout, LibraryLayoutMode::List) { "library-layout-btn library-layout-btn-active" } else { "library-layout-btn" }
                        data-layout="list"
                        aria-pressed=matches!(layout, LibraryLayoutMode::List)
                    >"≡"</button>
                </span>
            </div>
        </div>
    }
}

#[component]
pub fn LibraryGrid(view: LibraryGridView) -> impl IntoView {
    let LibraryGridView {
        toolbar,
        recordings,
        empty_message,
    } = view;
    let layout = toolbar.layout;
    let cards: Vec<_> = recordings
        .into_iter()
        .map(|r| view! { <RecordingCard view=r /> })
        .collect();
    let body = if cards.is_empty() {
        view! {
            <div class="library-empty" role="status">
                <span class="library-empty-icon" aria-hidden="true">"⊘"</span>
                <span class="library-empty-message">{empty_message}</span>
            </div>
        }
        .into_any()
    } else {
        let class = format!("library-grid library-grid-{}", layout.slug());
        view! { <div class=class>{cards}</div> }.into_any()
    };
    view! {
        <section class="library-grid-host" aria-label="Recordings">
            <LibraryToolbar view=toolbar />
            {body}
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_slugs_unique() {
        let slugs = [
            RecordingCardState::Ready.slug(),
            RecordingCardState::Processing { percent: 33 }.slug(),
            RecordingCardState::Failed.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
    }

    #[test]
    fn layout_mode_slugs_kebab() {
        for s in [
            LibraryLayoutMode::Grid.slug(),
            LibraryLayoutMode::List.slug(),
        ] {
            assert!(s.chars().all(|c| c.is_ascii_lowercase()));
        }
    }
}
