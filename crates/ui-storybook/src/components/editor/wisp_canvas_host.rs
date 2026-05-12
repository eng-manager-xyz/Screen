//! `WispCanvasHost` + `EditorDropZoneCanvas` (M-UI.17 / AUT-137).
//!
//! Editor canvas region. The Leptos component renders a host that
//! picks one of three backends:
//!
//! - `CssFallback` — dotted drop-zone over the checkered editor canvas
//!   (SSR-stable, used in the mdBook screenshot).
//! - `WispAsset { asset_path }` — a deterministic Wisp-generated PNG
//!   committed under `_docs/book/src/assets/ui/`. Loaded via `<img>`.
//! - `WispRuntimeUnavailable` — banner explaining that runtime Wisp
//!   embedding in CSR isn't wired yet.
//!
//! The component never touches `wgpu` directly; it just renders the
//! backend the parent picked.

use leptos::prelude::*;

/// Which canvas backend to render.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum CanvasBackendView {
    /// CSS dotted drop-zone over the checkered editor canvas.
    #[default]
    CssFallback,
    /// Pre-rendered Wisp asset (PNG path relative to the chapter).
    WispAsset {
        /// Asset path (e.g. `"../../assets/ui/editor-canvas-wisp.png"`).
        asset_path: &'static str,
    },
    /// Wisp runtime is not available in the current environment.
    WispRuntimeUnavailable,
}

impl CanvasBackendView {
    /// Stable kebab-case slug.
    #[must_use]
    pub fn slug(&self) -> &'static str {
        match self {
            CanvasBackendView::CssFallback => "css-fallback",
            CanvasBackendView::WispAsset { .. } => "wisp-asset",
            CanvasBackendView::WispRuntimeUnavailable => "wisp-runtime-unavailable",
        }
    }
}

/// One drop-zone action card.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropZoneActionView {
    /// Stable id (`"record"`, `"library"`, `"file"`).
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// Description shown beneath the label.
    pub description: &'static str,
    /// Leading glyph.
    pub icon: &'static str,
}

/// One entry in the recent-clip strip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentClipView {
    /// Stable id (file uuid).
    pub id: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Duration label ("1m 24s").
    pub duration_label: &'static str,
    /// CSS gradient string for the thumbnail.
    pub thumbnail_css: &'static str,
}

/// Drop-zone view-model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorDropZoneView {
    /// Backend selection.
    pub backend: CanvasBackendView,
    /// Headline (`"Drop a clip to start editing"`).
    pub headline: &'static str,
    /// Subtext under the headline (often contains a shortcut hint).
    pub subtext: &'static str,
    /// Action cards in display order.
    pub actions: Vec<DropZoneActionView>,
    /// Recent clips strip below the actions. Empty hides the strip.
    pub recent_clips: Vec<RecentClipView>,
    /// `true` to render the active drag overlay (file hover).
    pub drag_active: bool,
}

#[component]
pub fn WispCanvasHost(
    /// Backend selection (CSS fallback / Wisp asset / unavailable).
    backend: CanvasBackendView,
    /// Optional aria-label.
    #[prop(optional, default = "Canvas")]
    aria_label: &'static str,
) -> impl IntoView {
    let class = format!("wisp-canvas-host wisp-canvas-host-{}", backend.slug());
    let body = match backend {
        CanvasBackendView::CssFallback => view! {
            <div class="wisp-canvas-css">
                <span class="wisp-canvas-css-label">"CSS fallback canvas"</span>
            </div>
        }
        .into_any(),
        CanvasBackendView::WispAsset { asset_path } => view! {
            <img class="wisp-canvas-asset" src=asset_path alt="Wisp-generated canvas preview" />
        }
        .into_any(),
        CanvasBackendView::WispRuntimeUnavailable => view! {
            <div class="wisp-canvas-banner">
                <span class="wisp-canvas-banner-glyph">"⚠"</span>
                <span class="wisp-canvas-banner-message">
                    "Wisp runtime not available in this build. Falling back to CSS preview."
                </span>
            </div>
        }
        .into_any(),
    };
    view! {
        <div class=class role="img" aria-label=aria_label>
            {body}
        </div>
    }
}

#[component]
pub fn EditorDropZoneCanvas(view: EditorDropZoneView) -> impl IntoView {
    let EditorDropZoneView {
        backend,
        headline,
        subtext,
        actions,
        recent_clips,
        drag_active,
    } = view;
    let mut class = String::from("editor-drop-zone");
    if drag_active {
        class.push_str(" editor-drop-zone-active");
    }
    let action_cards: Vec<_> = actions
        .into_iter()
        .map(|a| {
            view! {
                <button class="editor-drop-zone-action" data-id=a.id>
                    <span class="editor-drop-zone-action-icon" aria-hidden="true">{a.icon}</span>
                    <span class="editor-drop-zone-action-label">{a.label}</span>
                    <span class="editor-drop-zone-action-desc">{a.description}</span>
                </button>
            }
        })
        .collect();
    let clip_strip = (!recent_clips.is_empty()).then(|| {
        let cards: Vec<_> = recent_clips
            .iter()
            .map(|c| {
                let style = format!("background: {};", c.thumbnail_css);
                view! {
                    <button class="editor-recent-clip" data-id=c.id>
                        <span class="editor-recent-clip-thumb" style=style></span>
                        <span class="editor-recent-clip-meta">
                            <span class="editor-recent-clip-title">{c.title}</span>
                            <span class="editor-recent-clip-duration">{c.duration_label}</span>
                        </span>
                    </button>
                }
            })
            .collect();
        view! {
            <div class="editor-recent-clips" aria-label="Recent clips">
                <span class="editor-recent-clips-heading">"RECENT CLIPS"</span>
                <div class="editor-recent-clips-strip">{cards}</div>
            </div>
        }
    });
    let backend_slug = backend.slug();
    view! {
        <section class=class data-backend=backend_slug role="region" aria-label="Drop a clip">
            <WispCanvasHost backend=backend aria_label="Editor canvas" />
            <div class="editor-drop-zone-content">
                <span class="editor-drop-zone-icon" aria-hidden="true">"⬆"</span>
                <span class="editor-drop-zone-headline">{headline}</span>
                <span class="editor-drop-zone-subtext">{subtext}</span>
                <div class="editor-drop-zone-actions">{action_cards}</div>
                {clip_strip}
            </div>
        </section>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_slugs_kebab_unique() {
        let slugs = [
            CanvasBackendView::CssFallback.slug(),
            CanvasBackendView::WispAsset {
                asset_path: "/x.png",
            }
            .slug(),
            CanvasBackendView::WispRuntimeUnavailable.slug(),
        ];
        let mut sorted = slugs.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), slugs.len());
        for s in slugs {
            assert!(s.chars().all(|c| c.is_ascii_lowercase() || c == '-'));
        }
    }
}
