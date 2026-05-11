//! Primitive stories — Button, Card. UI-01 / UI-04 expand this list.

use leptos::prelude::*;

use crate::components::primitives::{
    Button, ButtonSize, ButtonVariant, Card, CardBody, CardHeader,
};

use super::{Story, StoryViewport, render};

/// All primitive-surface stories, in display order.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "button-variants",
            category: "Primitives",
            title: "Button — variants",
            viewport: StoryViewport::Auto,
            render: render_button_variants,
        },
        Story {
            id: "button-sizes",
            category: "Primitives",
            title: "Button — sizes",
            viewport: StoryViewport::Auto,
            render: render_button_sizes,
        },
        Story {
            id: "card-basic",
            category: "Primitives",
            title: "Card — header + body",
            viewport: StoryViewport::Auto,
            render: render_card_basic,
        },
    ]
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
