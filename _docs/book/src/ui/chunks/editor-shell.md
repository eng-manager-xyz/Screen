# Editor shell

[Linear: AUT-136](https://linear.app/harwood/issue/AUT-136)

Structural shell for the editor screen — macOS-style title bar,
top toolbar (16:9 / Crop / Annotate / Trim + Share / Export), and
three slot regions for the canvas, inspector, and timeline.

<iframe src="../../assets/ui/editor-shell-empty.html" width="980" height="620" frameborder="0"></iframe>

## States

| State | Story |
| --- | --- |
| Empty (no clip) | [`editor-shell-empty`](../../assets/ui/editor-shell-empty.html) |
| Clip loaded | [`editor-shell-clip-loaded`](../../assets/ui/editor-shell-clip-loaded.html) |
| Toolbar states | [`editor-toolbar-states`](../../assets/ui/editor-toolbar-states.html) |
| Export disabled | [`editor-shell-export-disabled`](../../assets/ui/editor-shell-export-disabled.html) |

## API

```rust
use ui_storybook::components::editor::{EditorShell, EditorShellView};
use ui_storybook::fixtures::editor::sample_editor_shell;

view! {
    <EditorShell view=sample_editor_shell(/* has_clip_loaded */ true) />
    // optional children: canvas, inspector, timeline
}
```

```admonish important title="Shell is structural, not opinionated"
`EditorShell` takes `canvas`, `inspector`, and `timeline` as
`Option<Children>` slots. UI-17 (`WispCanvasHost`), UI-18
(`InspectorPanel`), and UI-19 (`TimelineSkeleton`) fill those slots
in subsequent tickets — the shell itself doesn't care what each
slot renders.
```
