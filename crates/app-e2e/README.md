# `app-e2e` — Tier-2 WebDriver e2e tests for `screen-app`

Spawns `tauri-driver` plus the `screen-app` binary and drives the real
webview with `fantoccini`. Covers the golden-path user flow (drag-drop
→ playback) end-to-end against a production-like build.

**Linux-only.** macOS is skipped because `tauri-driver`'s WKWebView
support is incomplete upstream. The Tier-1 IPC harness in
`crates/app/tests/commands.rs` runs cross-platform and is included in
`just gate`.

## Run locally

### Linux

```bash
# from: anywhere — one-time setup.
cargo install --locked tauri-driver
sudo apt-get install -y webkit2gtk-driver xvfb

# from: repo root — run the suite under a virtual display.
just e2e
# Equivalent (also from: repo root, so the workspace target/ is reused):
xvfb-run --auto-servernum cargo nextest run -p app-e2e
```

`just e2e` builds `screen-app`, spawns `tauri-driver` on port 4444,
points fantoccini at it, and runs the tests. Drop kills both processes.

### macOS

```bash
# from: repo root
just e2e   # prints a clear skip message and exits 0
```

Use Linux CI for the gate; mac uses manual smoke tests before tagging.

## Test locally

The same `just e2e` recipe is the test entry point. To target a single
test:

```bash
# from: repo root (Linux only)
xvfb-run --auto-servernum cargo nextest run -p app-e2e golden_path
```

## Why it's excluded from `just gate`

`just gate`'s `test` step uses `--exclude app-e2e` because the harness
needs `tauri-driver` + `webkit2gtk-driver` + `xvfb`, which aren't on
default dev hosts. Running e2e is opt-in via `just e2e` (or the CI
matrix's Linux job).
