# Shared fixture library

[Linear: AUT-142](https://linear.app/harwood/issue/AUT-142)

Every storybook component reads its sample data from one of the
per-surface fixture modules under `crates/ui-storybook/src/fixtures/`.
This page is the index of what each fixture provides and why
**stable, deterministic** fixture data matters.

## Why fixtures matter

The component stories double as our SSR snapshot suite. If a story
were to invent its own mock data inline, each story would drift
into its own dialect of "what a recording looks like" and the
snapshot diff between PRs would become noise — every component
change would touch every story body.

Centralizing the canonical samples per surface means:

- Fixtures never use randomness, `Instant::now`, the local
  filesystem, or any OS state.
- Stable IDs (`rec-01`, `ws-northwind`, `space-team`) so snapshot
  diffs are reviewable.
- Fixtures map 1:1 to real DTOs — when the runtime crate lands a
  real `Recording` struct, the fixtures can be replaced (not
  rewritten) by mappers.

## Module index

| Module | Provides |
| --- | --- |
| `fixtures::workspaces` | `WorkspaceView` rows + selected-id helpers |
| `fixtures::devices` | Capture sources, displays, device pickers |
| `fixtures::audio_apps` | System-audio app rows |
| `fixtures::recorder` | Tray-popover composition + on-screen options |
| `fixtures::library` | Recording cards + grid + sidebar |
| `fixtures::editor` | Dope-sheet + editor-shell + drop-zone + inspector + timeline |
| `fixtures::cursor` | Cursor styles + appearance presets |

## Contact sheet

The fixture-gallery story renders one tile per surface so designers
can review the canonical data shape at a glance:

<iframe src="../assets/ui/fixtures-contact-sheet.html" width="1100" height="740" frameborder="0"></iframe>

```admonish important title="Fixtures must be deterministic"
Builders cannot use randomness, the current time, or local-machine
state. If a fixture pulls in non-determinism the snapshot test
flakes on different machines (and `just snapshots-check` cannot
catch byte differences across machines — only missing files).
```

## Adding a new fixture

1. Decide which per-surface module owns it (or create one).
2. Add the builder as `pub fn sample_<thing>() -> Vec<...>`.
3. If the new fixture should appear on the contact sheet, extend
   `contact_sheet::default_ui_fixtures()` and the
   `fixtures-contact-sheet` story.
4. Add a test asserting non-empty + stable IDs.
