//! Primitive stories — `Button`, `Card`, `Surface`, `Badge`,
//! `Divider`, `Kbd`, `IconTile`. UI-01 expanded this list with the
//! design-token + base surface primitives.

use leptos::prelude::*;

use crate::components::primitives::{
    Badge, BadgeKind, Button, ButtonSize, ButtonVariant, Card, CardBody, CardHeader, Divider,
    DividerOrientation, IconTile, IconTileKind, Kbd, Surface, SurfaceKind,
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
        Story {
            id: "tokens-dark-zinc",
            category: "Primitives",
            title: "Tokens — semantic surface + text",
            viewport: StoryViewport::Auto,
            render: render_tokens_dark_zinc,
        },
        Story {
            id: "surface-stack",
            category: "Primitives",
            title: "Surface — five kinds stacked",
            viewport: StoryViewport::Auto,
            render: render_surface_stack,
        },
        Story {
            id: "badge-variants",
            category: "Primitives",
            title: "Badge — six kinds",
            viewport: StoryViewport::Auto,
            render: render_badge_variants,
        },
        Story {
            id: "kbd-shortcuts",
            category: "Primitives",
            title: "Kbd — keyboard shortcut chips",
            viewport: StoryViewport::Auto,
            render: render_kbd_shortcuts,
        },
        Story {
            id: "icon-tile-variants",
            category: "Primitives",
            title: "IconTile — five kinds",
            viewport: StoryViewport::Auto,
            render: render_icon_tile_variants,
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

fn render_tokens_dark_zinc() -> String {
    // Inline swatch styles use `style=` because the tokens are the
    // demo content — the only place raw colors are allowed outside
    // `style.css`.
    render(view! {
        <div class="token-grid">
            <div class="token-name">"--surface-base"</div>
            <div class="token-swatch" style="background:var(--surface-base)"></div>
            <div class="token-value">"App background"</div>

            <div class="token-name">"--surface-elevated"</div>
            <div class="token-swatch" style="background:var(--surface-elevated)"></div>
            <div class="token-value">"Panels + cards"</div>

            <div class="token-name">"--surface-popover"</div>
            <div class="token-swatch" style="background:var(--surface-popover)"></div>
            <div class="token-value">"Menus + popovers"</div>

            <div class="token-name">"--surface-selected"</div>
            <div class="token-swatch" style="background:var(--surface-selected)"></div>
            <div class="token-value">"Selected row inside a menu"</div>

            <div class="token-name">"--text-primary"</div>
            <div class="token-swatch" style="background:var(--text-primary)"></div>
            <div class="token-value">"Default text colour"</div>

            <div class="token-name">"--text-secondary"</div>
            <div class="token-swatch" style="background:var(--text-secondary)"></div>
            <div class="token-value">"Muted labels"</div>

            <div class="token-name">"--text-tertiary"</div>
            <div class="token-swatch" style="background:var(--text-tertiary)"></div>
            <div class="token-value">"Footnotes + axis labels"</div>

            <div class="token-name">"--action-record"</div>
            <div class="token-swatch" style="background:var(--action-record)"></div>
            <div class="token-value">"Record / destructive action"</div>
        </div>
    })
}

fn render_surface_stack() -> String {
    render(view! {
        <div class="surface-stack">
            <Surface kind=SurfaceKind::Base>
                <div class="surface-demo">"surface-base — the app canvas"</div>
            </Surface>
            <Surface kind=SurfaceKind::Elevated>
                <div class="surface-demo">"surface-elevated — panels + cards"</div>
            </Surface>
            <Surface kind=SurfaceKind::Popover>
                <div class="surface-demo">"surface-popover — menus + tray popovers"</div>
            </Surface>
            <Surface kind=SurfaceKind::Selected>
                <div class="surface-demo">"surface-selected — highlighted row inside a menu"</div>
            </Surface>
            <Surface kind=SurfaceKind::Glass>
                <div class="surface-demo">"surface-glass — overlay over recording"</div>
            </Surface>
        </div>
    })
}

fn render_badge_variants() -> String {
    render(view! {
        <div class="badge-row">
            <Badge kind=BadgeKind::Neutral>"Beta"</Badge>
            <Badge kind=BadgeKind::Accent>"New"</Badge>
            <Badge kind=BadgeKind::Danger>"Failed"</Badge>
            <Badge kind=BadgeKind::Live>"Live"</Badge>
            <Badge kind=BadgeKind::Plan>"Pro"</Badge>
            <Badge kind=BadgeKind::Count>"12"</Badge>
            <Divider orientation=DividerOrientation::Vertical />
            <span style="font-size:11px;color:var(--text-tertiary)">"divider shown inline (vertical)"</span>
        </div>
    })
}

fn render_kbd_shortcuts() -> String {
    render(view! {
        <div class="story-row" style="padding:14px;flex-direction:column;align-items:flex-start;gap:10px">
            <div class="story-row">
                <span style="font-size:12px;color:var(--text-secondary)">"Start recording"</span>
                <Kbd keys=vec!["⌘", "⇧", "R"] />
            </div>
            <div class="story-row">
                <span style="font-size:12px;color:var(--text-secondary)">"Stop recording"</span>
                <Kbd keys=vec!["⌘", "⇧", "S"] />
            </div>
            <div class="story-row">
                <span style="font-size:12px;color:var(--text-secondary)">"Pause / resume"</span>
                <Kbd keys=vec!["Space"] />
            </div>
            <div class="story-row">
                <span style="font-size:12px;color:var(--text-secondary)">"Open editor"</span>
                <Kbd keys=vec!["⌘", "O"] />
            </div>
        </div>
    })
}

fn render_icon_tile_variants() -> String {
    render(view! {
        <div class="story-row" style="padding:14px;gap:14px">
            <div class="story-row" style="flex-direction:column;gap:4px">
                <IconTile kind=IconTileKind::Workspace>"PE"</IconTile>
                <span style="font-size:11px;color:var(--text-tertiary)">"Workspace"</span>
            </div>
            <div class="story-row" style="flex-direction:column;gap:4px">
                <IconTile kind=IconTileKind::Device>"🎙"</IconTile>
                <span style="font-size:11px;color:var(--text-tertiary)">"Device"</span>
            </div>
            <div class="story-row" style="flex-direction:column;gap:4px">
                <IconTile kind=IconTileKind::App>"S"</IconTile>
                <span style="font-size:11px;color:var(--text-tertiary)">"App"</span>
            </div>
            <div class="story-row" style="flex-direction:column;gap:4px">
                <IconTile kind=IconTileKind::Action>"+"</IconTile>
                <span style="font-size:11px;color:var(--text-tertiary)">"Action"</span>
            </div>
            <div class="story-row" style="flex-direction:column;gap:4px">
                <IconTile kind=IconTileKind::User>"M"</IconTile>
                <span style="font-size:11px;color:var(--text-tertiary)">"User"</span>
            </div>
        </div>
    })
}
