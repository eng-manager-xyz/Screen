# Tauri ↔ Leptos integration (M-INT.2)

The chunk that joins the Leptos shell to the Tauri webview, then forwards
OS-level drag-drop events into a Leptos signal.

## Data flow

```text
                            OS drag-drop event
                                    │
                                    ▼
crates/app/src/main.rs
  on_window_event(WindowEvent::DragDrop)
  ─ window.emit("file-dropped", path) ───────────┐
                                                  │  Tauri event channel
                                                  ▼
crates/app-ui/index.html
  <script>
    window.__TAURI__.event.listen("file-dropped", evt =>
      window.dispatchEvent(new CustomEvent("file-dropped", {
        detail: evt.payload
      }))
    )
  </script>                                       │
                                                  │  browser CustomEvent
                                                  ▼
crates/app-ui/src/app.rs
  install_file_drop_listener()
  ─ window.addEventListener("file-dropped", …) ──┐
                                                  │
                                                  ▼
                          set_loaded.set(Some(path))
                                                  │
                                                  ▼
                          <App> swaps drop-zone view → player view
```

Every hop is a one-liner. No `tauri-sys` crate, no JS-side state — the
bridge is one `addEventListener` per direction.

## Tauri side (`crates/app/src/main.rs`)

```rust
.on_window_event(|window, event| {
    if let WindowEvent::DragDrop(DragDropEvent::Drop { paths, .. }) = event
        && let Some(path) = paths.first()
    {
        let payload = path.to_string_lossy().into_owned();
        if let Err(err) = window.emit("file-dropped", payload) {
            eprintln!("failed to emit file-dropped event: {err}");
        }
    }
})
```

Tauri 2's [`WindowEvent::DragDrop`](https://docs.rs/tauri/2/tauri/window/enum.WindowEvent.html)
fires automatically when `tauri.conf.json` has
`"dragDropEnabled": true` (the default for our window).

## JS bridge (`crates/app-ui/index.html`)

```html
<script>
  window.addEventListener("DOMContentLoaded", () => {
    if (window.__TAURI__ && window.__TAURI__.event) {
      window.__TAURI__.event.listen("file-dropped", (event) => {
        window.dispatchEvent(new CustomEvent("file-dropped", {
          detail: event.payload
        }));
      });
    }
  });
</script>
```

Why a CustomEvent instead of calling Leptos directly: it keeps the WASM
bundle dependency-free of Tauri's JS API. Leptos's `web-sys` listener
works against any browser; the bridge degrades to a no-op when
`window.__TAURI__` is absent (e.g. running under `trunk serve` for
component review).

## Leptos side (`crates/app-ui/src/app.rs`)

```rust
fn install_file_drop_listener(set_loaded: WriteSignal<Option<String>>) {
    let Some(window) = web_sys::window() else { return; };
    let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
        if let Ok(ce) = event.dyn_into::<CustomEvent>()
            && let Some(path) = ce.detail().as_string()
        {
            set_loaded.set(Some(path));
        }
    }) as Box<dyn FnMut(_)>);
    let _ = window.add_event_listener_with_callback(
        "file-dropped",
        closure.as_ref().unchecked_ref(),
    );
    closure.forget();   // app-lifetime listener — never removed
}
```

`Closure::forget` is intentional: the listener lives for the whole app
lifetime. Dropping it would silently de-register the handler.

## Tauri config (`tauri.conf.json`)

```json
{
  "build": {
    "frontendDist": "../app-ui/dist",
    "beforeDevCommand": "cd ../app-ui && trunk serve --port 1420",
    "beforeBuildCommand": "cd ../app-ui && trunk build --release",
    "devUrl": "http://localhost:1420"
  }
}
```

`cargo tauri dev` automatically runs `trunk serve` in `crates/app-ui/`
and waits for it on port 1420 before opening the webview.
`cargo tauri build` runs `trunk build --release` first; the resulting
`crates/app-ui/dist/` becomes the bundled webview content.

## How to run

```bash
# Dev — hot-reload Leptos via Trunk + reload the Tauri webview on save.
cd crates/app && cargo tauri dev

# Production — Trunk build + Tauri build, single binary.
cd crates/app && cargo tauri build
```

Drop any file on the running window; the path appears under "Preview
surface · " in the player view.

## Drag-over visual feedback (M-POLISH.1)

Tauri 2 fires four `DragDropEvent` variants — `Enter`, `Over`, `Drop`,
`Leave`. M-INT.2 only handled `Drop`. M-POLISH.1 adds `Enter` and
`Leave` so the drop-zone shows the user "yes, this drop will land here":

```rust
// crates/app/src/main.rs
match drag {
    DragDropEvent::Enter { .. } => emit("file-drag-enter"),
    DragDropEvent::Leave => emit("file-drag-leave"),
    DragDropEvent::Drop { paths, .. } => {
        emit("file-dropped", path);
        emit("file-drag-leave");  // reset visual after drop
    }
    _ => {}  // Over and any future variants
}
```

The JS bridge re-emits as browser CustomEvents (parallel to
`file-dropped` / `player-status`). Leptos installs an `is_dragging`
signal that flips on enter/leave; the existing
[`DropZoneState::Active`](../api/ui_storybook/components/drop_zone/enum.DropZoneState.html)
variant the storybook already shipped (M-UI.1) provides the visual.

Tier-2 e2e (`crates/app-e2e/tests/golden_path.rs`) uses
`__test_drag_enter` and `__test_drag_leave` debug-only commands —
parallel to `__test_drop_file` since WebDriver can't synthesize
OS-level drag events.

## What's NOT here yet

- **Filtering by extension.** Any file path is accepted today;
  M-POLISH.2 will reject non-video paths with an inline error message.
- **Drop-while-loaded reset UI.** Dropping a new file while one is
  already loaded works (the path signal flips), but there's no
  "← Load another" affordance yet — that's M-POLISH.3.

## App-shell visual references

The drop-zone surface (idle) at boot — exactly what the user sees
before any file is dropped:

<iframe src="../assets/ui/drop-zone-idle.html" width="100%" height="280" frameborder="0"></iframe>

[`DropZone` story](../ui/chunks/drop-zone-idle.md) ·
[Editor mock composition](../ui/chunks/editor-mock.md)
