# Surface primitives

[Linear: AUT-121](https://linear.app/harwood/issue/AUT-121)

Foundational rendering primitives — `Surface`, `Badge`, `Divider`,
`Kbd`, `IconTile`. Every other UI surface composes from these.

## Surface

Five kinds — pick one per surface. Drives the background, border,
and shadow tokens applied.

<iframe src="../../assets/ui/surface-stack.html" width="100%" height="420" frameborder="0"></iframe>

[Open as live demo →](../../assets/ui/surface-stack.html)

| `SurfaceKind` | Use for |
| --- | --- |
| `Base` | App canvas, full-window background |
| `Elevated` | Panels, cards, tray-popover body |
| `Popover` | Dropdowns, command menus (stronger drop shadow) |
| `Selected` | Highlighted list row inside a menu |
| `Glass` | Translucent overlay over a recording (uses `backdrop-filter`) |

```rust
use ui_storybook::components::{Surface, SurfaceKind};

view! {
    <Surface kind=SurfaceKind::Popover>
        <p>"Tray menu content here"</p>
    </Surface>
}
```

## Badge

<iframe src="../../assets/ui/badge-variants.html" width="100%" height="100" frameborder="0"></iframe>

[Open as live demo →](../../assets/ui/badge-variants.html)

Six kinds, each maps to a recurring badge usage in the recorder:

| `BadgeKind` | Use for |
| --- | --- |
| `Neutral` | Default emphasis ("Beta") |
| `Accent` | Quiet highlight ("New", "Recommended") |
| `Danger` | Error counts / destructive states |
| `Live` | Active recording — pulses |
| `Plan` | Plan / tier label (outlined, small caps) |
| `Count` | Numeric counts inside menu rows |

## Divider

Thin separator. Two orientations:

- `Horizontal` — full width, 1px tall. Default.
- `Vertical` — 1px wide, stretches to parent height. Use inline for
  toolbar separators.

## Kbd

Keyboard-shortcut chip. Pass an ordered slice; each element renders as
a `<kbd>`:

<iframe src="../../assets/ui/kbd-shortcuts.html" width="100%" height="200" frameborder="0"></iframe>

```rust
use ui_storybook::components::Kbd;

view! { <Kbd keys=vec!["⌘", "⇧", "R"] /> }
```

## IconTile

Small square tile for inline icons / monograms. Five kinds:

<iframe src="../../assets/ui/icon-tile-variants.html" width="100%" height="120" frameborder="0"></iframe>

| `IconTileKind` | Use for |
| --- | --- |
| `Workspace` | Workspace monogram (gradient background) |
| `Device` | Device avatar (mic / camera / display) |
| `App` | System app icon (Spotify, Zoom, …) |
| `Action` | Leading glyph on an action menu row |
| `User` | User avatar |

```admonish tip title="Composition over inheritance"
None of these primitives know about each other. A higher-level
component like a device-picker row composes `Surface` +
`IconTile` + `Badge` + `Kbd` together by passing children — the
primitives stay independent and reusable in any product surface.
```
