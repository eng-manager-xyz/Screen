# Inspector panel

[Linear: AUT-138](https://linear.app/harwood/issue/AUT-138)

Right-pane inspector used by both the editor and the cursor studio.
Tab strip (Style / Cursor / Audio / Captions / AI) + zero-or-more
`PropertySection` blocks rendering rows with slider / toggle / color
swatch / pill controls.

<iframe src="../../assets/ui/inspector-style-tab.html" width="340" height="500" frameborder="0"></iframe>

## States

| State | Story |
| --- | --- |
| Style tab | [`inspector-style-tab`](../../assets/ui/inspector-style-tab.html) |
| Cursor tab | [`inspector-cursor-tab`](../../assets/ui/inspector-cursor-tab.html) |
| Disabled rows | [`inspector-disabled-section`](../../assets/ui/inspector-disabled-section.html) |
| Slider row | [`property-row-slider`](../../assets/ui/property-row-slider.html) |
| Toggle row | [`property-row-toggle`](../../assets/ui/property-row-toggle.html) |
| Color swatch row | [`property-row-color-swatches`](../../assets/ui/property-row-color-swatches.html) |

## API

```rust
use ui_storybook::components::editor::{InspectorPanel, InspectorTab};
use ui_storybook::fixtures::editor::sample_inspector_style_tab;

view! { <InspectorPanel view=sample_inspector_style_tab() /> }
```

```admonish important title="Controls are an enum, not a slot"
`PropertyControlView` is an enum (`ValueOnly`, `SliderPercent`,
`Toggle`, `ColorSwatches`, `SelectPill`) so all five common controls
render through the same row layout without dragging a `Children`
slot through the macro. If a future panel needs something custom,
add a new variant — the parent never has to ship its own row markup.
```
