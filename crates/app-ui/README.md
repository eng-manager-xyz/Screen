# `app-ui` — Leptos CSR shell

The HTML/UI layer that Tauri serves into its webview. Composes
components from `ui-storybook` into the recorder surface (toolbar,
drop-zone / player view, status bar). Trunk builds the wasm bundle into
`crates/app-ui/dist/`, which `tauri.conf.json` points at as
`frontendDist`.

`crate-type = ["cdylib", "rlib"]` — `cdylib` for Trunk's wasm-bindgen
step, `rlib` so `cargo check --workspace` (native) still type-checks.

## Run locally

### Standalone in a browser (no Tauri)

```bash
# from: repo root
just app-ui

# Or directly — from: crates/app-ui/ (Trunk reads index.html + Trunk.toml
# from the cwd).
cd crates/app-ui && trunk serve --open
```

Opens `localhost:8080` with hot reload. Useful for iterating on
components — but Tauri-only features (file drop, player IPC) are no-ops
here. The drop-zone has a click-to-load demo affordance so the view
swap is still exercisable.

### Inside the Tauri shell

```bash
# from: crates/app/ (cargo tauri dev resolves tauri.conf.json from cwd).
cd crates/app
cargo tauri dev
```

`cargo tauri dev` runs `trunk serve --port 1420` against this crate as
its `beforeDevCommand` and points the webview at it.

### Production build

```bash
# from: repo root
just app-ui-build

# Or directly — from: crates/app-ui/.
cd crates/app-ui && trunk build --release
```

Output lands in `crates/app-ui/dist/` — the path `tauri.conf.json`'s
`frontendDist` reads.

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)

# Native type-check + unit tests (rlib half of the crate).
cargo nextest run -p app-ui
cargo test -p app-ui --doc
```

The component-level snapshot coverage lives in `ui-storybook` (this
crate just composes).
