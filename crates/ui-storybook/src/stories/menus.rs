//! Menu / popover stories (M-UI.3 / AUT-123). Concrete tray menus
//! (workspace switcher, device pickers, on-screen options, tray
//! record popover) compose these primitives in UI-05 / UI-08 / UI-10
//! / UI-12.

use leptos::prelude::*;

use crate::components::menus::{
    MenuBadgeView, MenuFooter, MenuList, MenuRow, MenuRowKind, MenuSection, PopoverPlacement,
    PopoverSurface,
};
use crate::components::primitives::{
    BadgeKind, Button, ButtonVariant, IconTile, IconTileKind, Kbd,
};

use super::{Story, StoryViewport, render};

/// All menu-surface stories, in display order.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        Story {
            id: "popover-basic",
            category: "Menus",
            title: "Popover — header + menu list",
            viewport: StoryViewport::Fixed {
                width: 320,
                height: 280,
            },
            render: render_popover_basic,
        },
        Story {
            id: "popover-with-footer",
            category: "Menus",
            title: "Popover — with footer button",
            viewport: StoryViewport::Fixed {
                width: 320,
                height: 320,
            },
            render: render_popover_with_footer,
        },
        Story {
            id: "menu-row-selected",
            category: "Menus",
            title: "Menu row — selected",
            viewport: StoryViewport::Fixed {
                width: 280,
                height: 60,
            },
            render: render_menu_row_selected,
        },
        Story {
            id: "menu-row-action",
            category: "Menus",
            title: "Menu row — action",
            viewport: StoryViewport::Fixed {
                width: 280,
                height: 60,
            },
            render: render_menu_row_action,
        },
        Story {
            id: "menu-row-device",
            category: "Menus",
            title: "Menu row — device with subtitle + kbd",
            viewport: StoryViewport::Fixed {
                width: 320,
                height: 80,
            },
            render: render_menu_row_device,
        },
        Story {
            id: "menu-long-labels",
            category: "Menus",
            title: "Menu — long labels truncate cleanly",
            viewport: StoryViewport::Fixed {
                width: 280,
                height: 200,
            },
            render: render_menu_long_labels,
        },
    ]
}

fn render_popover_basic() -> String {
    render(view! {
        <PopoverSurface
            placement=PopoverPlacement::BottomLeft
            width_px=280_u16
            title="Workspaces".to_string()
            description="Switch between your personal and team workspaces.".to_string()
        >
            <MenuList label="Workspaces">
                <MenuSection heading="Personal".to_string()>
                    <MenuRow
                        kind=MenuRowKind::Selected
                        leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Workspace>"PE"</IconTile> })
                        title="Personal".to_string()
                    />
                </MenuSection>
                <MenuSection heading="Teams".to_string()>
                    <MenuRow
                        leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Workspace>"AC"</IconTile> })
                        title="Acme Inc.".to_string()
                        subtitle="3 members".to_string()
                    />
                    <MenuRow
                        leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Workspace>"CD"</IconTile> })
                        title="Client demos".to_string()
                    />
                </MenuSection>
            </MenuList>
        </PopoverSurface>
    })
}

fn render_popover_with_footer() -> String {
    render(view! {
        <PopoverSurface
            placement=PopoverPlacement::BottomLeft
            width_px=300_u16
            title="Choose camera".to_string()
            footer=ToChildren::to_children(|| view! {
                <MenuFooter>
                    <span>"Tip: press "<Kbd keys=vec!["C"] />" to switch"</span>
                    <Button variant=ButtonVariant::Default>"Add device"</Button>
                </MenuFooter>
            })
        >
            <MenuList label="Cameras">
                <MenuRow
                    kind=MenuRowKind::Selected
                    leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Device>"📷"</IconTile> })
                    title="FaceTime HD camera".to_string()
                    subtitle="MacBook · 1080p".to_string()
                />
                <MenuRow
                    leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Device>"📷"</IconTile> })
                    title="iPhone 15 Pro".to_string()
                    subtitle="Continuity Camera · 4K".to_string()
                    badges=vec![MenuBadgeView { label: "New", kind: BadgeKind::Accent }]
                />
                <MenuRow
                    kind=MenuRowKind::Disabled
                    leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Device>"📷"</IconTile> })
                    title="OBS Virtual Camera".to_string()
                    subtitle="Not connected".to_string()
                />
            </MenuList>
        </PopoverSurface>
    })
}

fn render_menu_row_selected() -> String {
    render(view! {
        <ul class="menu-list" role="menu" aria-label="example" style="width:260px;padding:6px;background:var(--surface-popover);border:1px solid var(--line-subtle);border-radius:8px;">
            <MenuRow
                kind=MenuRowKind::Selected
                leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Action>"⏵"</IconTile> })
                title="Open editor".to_string()
                trailing=ToChildren::to_children(|| view! { <Kbd keys=vec!["⌘", "O"] /> })
            />
        </ul>
    })
}

fn render_menu_row_action() -> String {
    render(view! {
        <ul class="menu-list" role="menu" aria-label="example" style="width:260px;padding:6px;background:var(--surface-popover);border:1px solid var(--line-subtle);border-radius:8px;">
            <MenuRow
                kind=MenuRowKind::Action
                leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Action>"+"</IconTile> })
                title="Add workspace".to_string()
            />
            <MenuRow
                kind=MenuRowKind::Danger
                leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Action>"−"</IconTile> })
                title="Remove workspace".to_string()
            />
        </ul>
    })
}

fn render_menu_row_device() -> String {
    render(view! {
        <ul class="menu-list" role="menu" aria-label="example" style="width:300px;padding:6px;background:var(--surface-popover);border:1px solid var(--line-subtle);border-radius:8px;">
            <MenuRow
                leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Device>"🎙"</IconTile> })
                title="AirPods Pro".to_string()
                subtitle="Bluetooth · 86% battery".to_string()
                badges=vec![MenuBadgeView { label: "Live", kind: BadgeKind::Live }]
                trailing=ToChildren::to_children(|| view! { <Kbd keys=vec!["⌘", "M"] /> })
            />
        </ul>
    })
}

fn render_menu_long_labels() -> String {
    render(view! {
        <PopoverSurface placement=PopoverPlacement::BottomLeft width_px=260_u16>
            <MenuList label="Long labels">
                <MenuRow
                    leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::Device>"🎙"</IconTile> })
                    title="A very long microphone label that overflows the row width".to_string()
                    subtitle="Subtitle that is also extremely long and needs to truncate cleanly".to_string()
                />
                <MenuRow
                    leading=ToChildren::to_children(|| view! { <IconTile kind=IconTileKind::App>"S"</IconTile> })
                    title="App name with lots of words to test truncation".to_string()
                />
            </MenuList>
        </PopoverSurface>
    })
}
