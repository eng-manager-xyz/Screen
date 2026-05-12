# Cursor Studio shell + style picker

[Linear: AUT-140](https://linear.app/harwood/issue/AUT-140)

The bottom style strip in Cursor Studio (System / Arrow / Soft /
Dot / Ring / Reticle / Tactile / Hide) plus the structural shell
that wires the preview slot + inspector slot + picker together.

<iframe src="../../assets/ui/cursor-style-picker-arrow-selected.html" width="740" height="160" frameborder="0"></iframe>

## States

| State | Story |
| --- | --- |
| Default — System selected | [`cursor-style-picker-default`](../../assets/ui/cursor-style-picker-default.html) |
| Arrow selected | [`cursor-style-picker-arrow-selected`](../../assets/ui/cursor-style-picker-arrow-selected.html) |
| All disabled | [`cursor-style-picker-disabled`](../../assets/ui/cursor-style-picker-disabled.html) |
| Cursor Studio shell | [`cursor-studio-shell`](../../assets/ui/cursor-studio-shell.html) |

## API

```rust
use ui_storybook::components::cursor::{CursorStudioShell, CursorStyle, CursorStylePicker};
use ui_storybook::fixtures::cursor::{sample_cursor_studio_shell, sample_cursor_style_picker};

view! { <CursorStudioShell view=sample_cursor_studio_shell() /> }
```

```admonish important title="Selected is a single CursorStyle"
The parent passes the selected style and the picker renders the
matching tile in the inverted (white) selected treatment. Individual
tiles can also be disabled (the `Tactile` tile ships disabled by
default until the cursor backend implements it).
```
