# Editor surface + Record→Edit handoff — ED.5

`?surface=editor` routed to a placeholder (`<h1>Editor</h1>`). ED.5
activates it: the surface now renders the real
[`EditorShell`](../../api/ui_storybook/components/editor/index.html) chrome
— title bar, toolbar, and body layout — driven by the loaded
[`EditProject`](../../api/edit/project/struct.EditProject.html), and wires
the handoff that loads a finished recording into it.

## The handoff

```mermaid
sequenceDiagram
  participant U as User
  participant JS as index.html bridge
  participant Cmd as open_in_editor (Tauri)
  participant UI as EditorSurface (Leptos)
  U->>JS: __screenOpenInEditor(path)
  JS->>Cmd: invoke("open_in_editor", { path })
  Cmd->>Cmd: probe metadata → EditProject::from_recording
  Cmd-->>JS: EditProject (serialized)
  JS->>UI: dispatch "editor-project" CustomEvent
  UI->>UI: deserialize → RwSignal&lt;Option&lt;EditProject&gt;&gt;
  UI->>U: jump to the editor, render EditorShell populated
```

`open_in_editor` (a thin Tauri command) probes the recording with
`gst-discoverer-1.0` and returns a default, untouched `EditProject` — one
full-length real-time segment. The webview bridge re-emits it as an
`editor-project` event; a Leptos listener deserializes it into a context
signal that the surface reads.

```admonish important title="No mirror type — app-ui depends on `edit`"
`app-ui` can't depend on `screen-app` (Tauri-native), but it *can* depend
on the pure `edit` crate (serde-only, wasm-clean). So the command's
payload deserializes **straight into `edit::EditProject`** — no
hand-maintained IPC mirror struct to drift out of sync.
```

The surface maps the project onto the shell view-model (title = file name,
subtitle = `1920×1080 · 30 fps · m:ss`, toolbar enabled); with no clip it
shows the "No clip loaded" empty state. The canvas, timeline, and
inspector slots are filled by the chunks that follow — preview (ED.6),
transport (ED.7), timeline (ED.8), inspector (ED.18).

```admonish note title="Handoff trigger"
ED.5 lands the complete receiving mechanism (command → event → signal →
surface, with an auto-jump to the editor when a project loads). The
user-facing "Open in Editor" button on the recorder's save panel rides
in with the recordings Library (ED.24), which is where browsing and
re-opening past recordings lives.
```
