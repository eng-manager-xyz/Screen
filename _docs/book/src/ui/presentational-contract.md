# Presentational contract

[Linear: AUT-120](https://linear.app/harwood/issue/AUT-120)

Every Leptos component in `crates/ui-storybook` is **presentational** —
it renders its props and emits callbacks, full stop. Application state,
side effects, and runtime concerns live one layer up in
`crates/app-ui`. This file is the explicit ruleset that keeps the
boundary honest.

## The rules

```admonish important title="The five rules"
1. **Inputs flow top-down through plain props and fixture structs.** No
   global signals, no thread-locals, no module-private caches.
2. **Callbacks for output, never observation.** Components may expose
   `on_click` / `on_select` / `on_toggle` / `on_open_change` props, but
   they MUST NOT subscribe to or own application state.
3. **No Tauri, no media, no I/O.** No `invoke` calls, no
   `media_capture::*`, no `localStorage`, no timers, no global services.
4. **No `signal`, `RwSignal`, or `Effect` inside a component.** The only
   exception is a story-only wrapper that creates a signal for
   demonstration. App wiring lives in `crates/app-ui`.
5. **Visual state is an explicit prop.** Use `selected`, `active`,
   `open`, `disabled`, `loading`, `expanded`, `recording_state`,
   `drag_state`, `permission_state` — never an internal `is_open` bool
   the parent can't read.
```

## Why these rules exist

```admonish note title="Stories drive snapshots, snapshots drive trust"
Every story is a deterministic SSR-to-HTML render. If a component
reads from a global signal or a Tauri command, two things break: the
SSR render either panics (no Tauri runtime in `cargo test`) or produces
non-deterministic HTML that churns the snapshot. Both kill the gate.
```

The contract also means each component slots into a different host
unchanged: the same `DropZone` works in the Tauri app, in the
storybook, in a hypothetical web preview, and in a future test
harness. Internal state would tie the component to a specific host's
lifecycle.

## Wisp / canvas components

Components that need a `<canvas>` (the editor preview, the cursor
preview canvas, the display source thumbnail) follow a two-path rule:

- A **feature-gated Wisp-backed story / export path** under
  `#[cfg(feature = "csr")]` that mounts wisp into the canvas.
- A **deterministic non-Wisp fallback** for SSR + mdBook — a static
  PNG sprite or a CSS-only placeholder. The story renders the fallback
  by default; tests only see deterministic HTML.

This is how UI-07 (`DisplaySourceCard`), UI-17 (`WispCanvasHost`), and
UI-21 (`CursorPreviewCanvas`) all stay in-bounds.

## Enforcement

```admonish warning title="UI-23 is the grep guardrail"
[UI-23 / AUT-143](https://linear.app/harwood/issue/AUT-143) lands a
guardrail test that greps `crates/ui-storybook/src/components/` for
`tauri::`, `wasm_bindgen::`, `invoke`, `RwSignal::new`, etc. and
fails the build if any appear outside an opt-in `#[cfg(feature =
"csr")]` story-only wrapper. Read the rules here first; the grep is
just the backstop.
```

## Composition

Components compose by passing data + callbacks through plain props.
A higher-level surface (e.g. `TrayRecordPopover` from UI-12) is built
by stacking lower-level primitives (`CaptureModeTabs` /
`DisplaySourceCard` / `CaptureSourceRow` / `SystemAudioPickerList` /
`OnScreenOptionsPopover` / `RecordingControlsFooter`) — none of which
import any of the others. The popover's parent in `app-ui` owns the
state machine and threads selections back via callback props.

## Empty subgroups

Some subgroups (`menus`, `library`, `cursor`) start empty — the
follow-up tickets fill them in. The empty `pub mod` declarations in
`components/mod.rs` keep the structure visible so authors know where
new components belong instead of inventing parallel locations.
