//! Story registry — every UI component must have at least one story.
//!
//! Each story is a `(id, render)` pair. The render closure produces an
//! `IntoView` so it can be SSR-rendered to HTML for snapshot tests AND mounted
//! in the browser via Trunk for visual review.

use leptos::prelude::*;

use crate::components::{
    Button, ButtonSize, ButtonVariant, Card, CardBody, CardHeader, DopeSheet, DopeSheetKeyframe,
    DopeSheetTrack, DropZone, DropZoneState, KeyframeKind, TrackKind,
};

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
    /// Render closure — produces SSR HTML synchronously.
    pub render: fn() -> String,
}

/// Every shipped story, in display order.
#[must_use]
pub fn all_stories() -> Vec<Story> {
    vec![
        Story {
            id: "button-variants",
            category: "Primitives",
            title: "Button — variants",
            render: render_button_variants,
        },
        Story {
            id: "button-sizes",
            category: "Primitives",
            title: "Button — sizes",
            render: render_button_sizes,
        },
        Story {
            id: "card-basic",
            category: "Primitives",
            title: "Card — header + body",
            render: render_card_basic,
        },
        Story {
            id: "dope-sheet-basic",
            category: "Editor",
            title: "Dope sheet — multi-track",
            render: render_dope_sheet_basic,
        },
        Story {
            id: "dope-sheet-dense",
            category: "Editor",
            title: "Dope sheet — dense keyframes",
            render: render_dope_sheet_dense,
        },
        Story {
            id: "card-with-dope-sheet",
            category: "Compositions",
            title: "Editor panel — card wrapping dope sheet",
            render: render_editor_panel,
        },
        Story {
            id: "drop-zone-idle",
            category: "App surfaces",
            title: "Drop zone — idle",
            render: render_drop_zone_idle,
        },
        Story {
            id: "drop-zone-active",
            category: "App surfaces",
            title: "Drop zone — active (file dragged)",
            render: render_drop_zone_active,
        },
    ]
}

fn render_drop_zone_idle() -> String {
    render(view! {
        <DropZone state=DropZoneState::Idle hint="⌘O to browse" />
    })
}

fn render_drop_zone_active() -> String {
    render(view! {
        <DropZone state=DropZoneState::Active />
    })
}

fn render<V>(view: V) -> String
where
    V: leptos::IntoView,
{
    // `RenderHtml::to_html` is brought into scope via `leptos::prelude::*`
    // (which re-exports `tachys::prelude::*`). It takes the view by value and
    // walks it into a synchronous HTML string — exactly what we want for an
    // insta snapshot.
    view.into_view().to_html()
}

fn render_button_variants() -> String {
    render(view! {
        <div class="story-row">
            <Button variant=ButtonVariant::Default>"Default"</Button>
            <Button variant=ButtonVariant::Outline>"Outline"</Button>
            <Button variant=ButtonVariant::Ghost>"Ghost"</Button>
            <Button variant=ButtonVariant::Destructive>"Destructive"</Button>
            <Button variant=ButtonVariant::Secondary>"Secondary"</Button>
        </div>
    })
}

fn render_button_sizes() -> String {
    render(view! {
        <div class="story-row">
            <Button size=ButtonSize::Sm>"Small"</Button>
            <Button size=ButtonSize::Md>"Medium"</Button>
            <Button size=ButtonSize::Lg>"Large"</Button>
            <Button disabled=true>"Disabled"</Button>
        </div>
    })
}

fn render_card_basic() -> String {
    render(view! {
        <Card>
            <CardHeader title="Recording 01" subtitle="Captured 2026-05-08 · 1m 24s" />
            <CardBody>
                <p>"Drag the file onto the player or hit "<kbd>"⌘O"</kbd>" to import."</p>
                <div class="story-row">
                    <Button variant=ButtonVariant::Default>"Open editor"</Button>
                    <Button variant=ButtonVariant::Ghost>"Reveal in Finder"</Button>
                </div>
            </CardBody>
        </Card>
    })
}

fn sample_tracks() -> Vec<DopeSheetTrack> {
    vec![
        DopeSheetTrack {
            label: "Video".into(),
            kind: TrackKind::Video,
            keyframes: vec![
                DopeSheetKeyframe {
                    time: 0.0,
                    kind: KeyframeKind::Hold,
                },
                DopeSheetKeyframe {
                    time: 2.5,
                    kind: KeyframeKind::Ease,
                },
                DopeSheetKeyframe {
                    time: 6.0,
                    kind: KeyframeKind::Linear,
                },
            ],
        },
        DopeSheetTrack {
            label: "Cursor".into(),
            kind: TrackKind::Cursor,
            keyframes: vec![
                DopeSheetKeyframe {
                    time: 1.2,
                    kind: KeyframeKind::Linear,
                },
                DopeSheetKeyframe {
                    time: 4.5,
                    kind: KeyframeKind::Linear,
                },
            ],
        },
        DopeSheetTrack {
            label: "Audio".into(),
            kind: TrackKind::Audio,
            keyframes: vec![
                DopeSheetKeyframe {
                    time: 0.0,
                    kind: KeyframeKind::Hold,
                },
                DopeSheetKeyframe {
                    time: 4.0,
                    kind: KeyframeKind::Hold,
                },
                DopeSheetKeyframe {
                    time: 7.5,
                    kind: KeyframeKind::Hold,
                },
            ],
        },
        DopeSheetTrack {
            label: "Caption".into(),
            kind: TrackKind::Caption,
            keyframes: vec![DopeSheetKeyframe {
                time: 3.0,
                kind: KeyframeKind::Marker,
            }],
        },
    ]
}

fn render_dope_sheet_basic() -> String {
    render(view! {
        <DopeSheet
            tracks=sample_tracks()
            duration_seconds=8.0
            playhead_seconds=3.4
        />
    })
}

fn render_dope_sheet_dense() -> String {
    let mut tracks = sample_tracks();
    tracks.push(DopeSheetTrack {
        label: "Zoom".into(),
        kind: TrackKind::Effect,
        keyframes: (0..12)
            .map(|i| DopeSheetKeyframe {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "i ≤ 12 fits in f32 without loss"
                )]
                time: i as f32 * 0.65,
                kind: if i % 3 == 0 {
                    KeyframeKind::Ease
                } else {
                    KeyframeKind::Linear
                },
            })
            .collect(),
    });
    render(view! {
        <DopeSheet
            tracks=tracks
            duration_seconds=8.0
            playhead_seconds=5.1
        />
    })
}

fn render_editor_panel() -> String {
    render(view! {
        <Card>
            <CardHeader title="Timeline" subtitle="4 tracks · 8.0s" />
            <CardBody>
                <DopeSheet
                    tracks=sample_tracks()
                    duration_seconds=8.0
                    playhead_seconds=2.0
                />
            </CardBody>
        </Card>
    })
}
