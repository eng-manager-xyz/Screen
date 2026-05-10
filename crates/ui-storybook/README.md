# `ui-storybook` — Leptos component library + visual gallery

Parallels `wisp-storybook` for the HTML/CSS side. Houses the reusable
Leptos components consumed by `app-ui` (Button, Card, DopeSheet,
DropZone, PlayerControls, RecordingToolbar, StatusBar) plus an
isolated-view gallery and SSR snapshot tests.

Two cargo features control compile mode:

- `ssr` (default) — native target. `cargo check` / `cargo test` /
  snapshot tests. Used by everything except the dev server.
- `csr` — wasm target. Trunk-built browser gallery.

## Run the browser gallery (CSR)

```bash
# from: repo root
just ui-storybook

# Or directly — from: crates/ui-storybook/ (Trunk reads index.html + Trunk.toml
# from the cwd, so this command must run from the crate's own directory).
cd crates/ui-storybook && trunk serve --no-default-features --features csr --open
```

Opens `localhost:8080` with the interactive gallery.

## Export story HTML (headless, SSR)

Used by `just snapshots` to produce the standalone HTML files embedded
in the mdBook chapters. One file per story under
`_docs/book/src/assets/ui/<id>.html` (path resolved relative to the
workspace root, so cwd matters).

```bash
# from: repo root
cargo run -p ui-storybook --bin ui-export-stories
# or, for both wisp + ui storybooks:
just snapshots
```

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)

# SSR snapshot suite + unit tests.
cargo nextest run -p ui-storybook

# Doctests.
cargo test -p ui-storybook --doc
```

`tests/snapshots.rs` renders each story to HTML via Leptos's
`to_html()` and `insta`-snapshots the output. First-run snapshots write
`*.snap.new` and fail; accept with `cargo insta accept`.
