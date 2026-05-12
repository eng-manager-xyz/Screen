# UI — components

Index of every shipped component, grouped by the same product-surface
subfolders as `crates/ui-storybook/src/components/`. New components
must land in the matching subgroup AND have at least one story registered
in `stories::all_stories()`.

[Linear: AUT-120](https://linear.app/harwood/issue/AUT-120)

```admonish info title="Subgroup map"
| Subgroup | Subfolder | Tickets |
| --- | --- | --- |
| Primitives | `components/primitives/` | UI-01 / UI-04 |
| Shell | `components/shell/` | UI-02 |
| Menus | `components/menus/` | UI-03 / UI-05 / UI-10 |
| Recorder | `components/recorder/` | UI-06..13 |
| Library | `components/library/` | UI-14 / UI-15 |
| Editor | `components/editor/` | UI-16..19 (+ existing dope sheet, player) |
| Cursor | `components/cursor/` | UI-20 / UI-21 |
```

## Primitives

### Button

Variants: `Default`, `Outline`, `Ghost`, `Destructive`, `Secondary`.
Sizes: `Sm`, `Md`, `Lg`. Disabled state.

<iframe src="../assets/ui/button-variants.html" width="100%" height="120" frameborder="0"></iframe>

[Open as live demo →](../assets/ui/button-variants.html)

### Button sizes

<iframe src="../assets/ui/button-sizes.html" width="100%" height="120" frameborder="0"></iframe>

[Open as live demo →](../assets/ui/button-sizes.html)

### Card

`Card`, `CardHeader { title, subtitle }`, `CardBody`. Composable surface
container — used everywhere the editor groups controls.

<iframe src="../assets/ui/card-basic.html" width="100%" height="240" frameborder="0"></iframe>

[Open as live demo →](../assets/ui/card-basic.html)

## Shell

### Drop zone

States: `Idle`, `Active`. Used as the editor's empty state and as the
"drag your recording here" surface.

<iframe src="../assets/ui/drop-zone-idle.html" width="100%" height="220" frameborder="0"></iframe>

### Status bar

Kinds: `Ready`, `Busy`, `Error`. Bottom-of-window strip — FPS, encoder,
file size, transient detail line.

<iframe src="../assets/ui/status-bar-ready.html" width="100%" height="60" frameborder="0"></iframe>

## Recorder

### Recording toolbar

States: `Idle`, `Recording`, `Paused`. The legacy single-row toolbar; the
new compositions (UI-11 footer, UI-12 tray popover) live alongside it.

<iframe src="../assets/ui/recording-toolbar-idle.html" width="100%" height="80" frameborder="0"></iframe>

## Editor + Player

- [Dope sheet](./dope-sheet.md) — full chapter with multi-track + dense
  variants.
- `PlayerControls` — three positions (paused at start / playing mid-clip /
  near end of clip).
- `editor-mock` composition.

<iframe src="../assets/ui/player-controls-playing.html" width="100%" height="80" frameborder="0"></iframe>

## Menus / Library / Cursor

Empty in UI-00 — the follow-up tickets land components into these
subgroups:

- **Menus:** UI-03 (`MenuShell`, `MenuItem`, popover anchors),
  UI-05 (`WorkspaceSwitcherMenu`), UI-10 (`OnScreenOptionsPopover`).
- **Library:** UI-14 (`LibrarySidebar` + storage meter),
  UI-15 (`RecordingCard` + `LibraryGrid`).
- **Cursor:** UI-20 (`CursorStudioShell` + style picker),
  UI-21 (`CursorPreviewCanvas`).
