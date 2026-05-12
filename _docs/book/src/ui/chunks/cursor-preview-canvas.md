# Cursor preview canvas + appearance controls

[Linear: AUT-141](https://linear.app/harwood/issue/AUT-141)

`CursorPreviewCanvas` reuses the same backend pattern as the editor
canvas (CSS fallback / Wisp asset / runtime unavailable) so a pixel-
accurate Wisp preview can drop in later without changing the
component contract. `CursorAppearancePanel` composes UI-18 inspector
primitives into `APPEARANCE` / `CLICK EFFECT` / `MOTION` / `BEHAVIOR`
sections with a footer (Reset / Apply).

<iframe src="../../assets/ui/cursor-appearance-panel-default.html" width="340" height="500" frameborder="0"></iframe>

## Preview states

| State | Story |
| --- | --- |
| Arrow with ring (light bg) | [`cursor-preview-arrow-ring`](../../assets/ui/cursor-preview-arrow-ring.html) |
| Dark background | [`cursor-preview-dark-bg`](../../assets/ui/cursor-preview-dark-bg.html) |

## Appearance panel states

| State | Story |
| --- | --- |
| Default | [`cursor-appearance-panel-default`](../../assets/ui/cursor-appearance-panel-default.html) |
| Pulse click effect | [`cursor-appearance-panel-pulse`](../../assets/ui/cursor-appearance-panel-pulse.html) |
| Spotlight click effect | [`cursor-appearance-panel-spotlight`](../../assets/ui/cursor-appearance-panel-spotlight.html) |
| Trail on (high smoothing) | [`cursor-appearance-panel-trail-on`](../../assets/ui/cursor-appearance-panel-trail-on.html) |

## API

```rust
use ui_storybook::components::cursor::{
    CursorPreviewCanvas, CursorPreviewBackend, CursorAppearancePanel,
};
use ui_storybook::fixtures::cursor::sample_cursor_appearance;

view! {
    <CursorPreviewCanvas backend=CursorPreviewBackend::CssFallback />
    <CursorAppearancePanel view=sample_cursor_appearance() />
}
```

```admonish important title="Halo strength row dims when halo is off"
The appearance panel ships with `disabled: true` on the halo
strength slider when `halo_enabled == false` — a small touch that
prevents the parent from worrying about how to disable individual
controls inline.
```
