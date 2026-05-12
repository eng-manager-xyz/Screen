# Dev loop — local

[Linear: AUT-148](https://linear.app/harwood/issue/AUT-148) (`just dev`),
[AUT-145](https://linear.app/harwood/issue/AUT-145) (`dev-server` crate),
[AUT-146](https://linear.app/harwood/issue/AUT-146) (file watcher),
[AUT-147](https://linear.app/harwood/issue/AUT-147) (storybook index),
[AUT-152](https://linear.app/harwood/issue/AUT-152) (live reload),
[AUT-151](https://linear.app/harwood/issue/AUT-151) (search filter).

`just dev` boots the `dev-server` crate against the existing
storybook artifacts under `_docs/book/src/assets/ui/`, watches
`crates/ui-storybook/src/**` + `assets/style.css`, and live-reloads
the browser when anything changes. One command, no flags to
remember.

```bash
just dev
# → Serving _docs/book/src/assets/ui at http://127.0.0.1:3000/
```

Visit `http://127.0.0.1:3000/` → land on the cockpit index. Sidebar
lists every story grouped by category. Click a row, the iframe
loads it; refresh keeps the same story open (URL hash routing).
Press `/` to focus the filter box; Esc clears.

## What gets watched

| Path | Effect |
| --- | --- |
| `crates/ui-storybook/src/**/*.rs` | Re-runs `cargo run -p ui-storybook --bin ui-export-stories`, then broadcasts reload (3–8 s warm). |
| `crates/ui-storybook/assets/style.css` | **CSS fast path** — copies the file straight into the served directory and broadcasts reload (<500 ms, no cargo build). |
| Anything else under the watched directories | Same as the first row — full rebuild. |

The watcher coalesces rapid-fire saves (10 saves in a debounce
window → exactly one rebuild). Compile errors are logged but **do
not trigger a reload**, so the browser stays on the last-good state.

## Phone / remote dev

See [remote dev](./remote-dev.md) for the `just dev-remote` flow
that puts this loop behind a Tailscale Serve URL.

## Linker speedup (opt-in)

Symlink or copy [`.cargo/config.toml.example`](../../../../.cargo/config.toml.example)
to `.cargo/config.toml` (workspace root) to wire `mold` (Linux)
or `lld` (macOS) into the link step. Knocks ~30–50 % off warm
incremental rebuilds. Prerequisite: `brew install lld` (macOS) or
`apt install mold` (Linux). The file is gitignored — every dev opts
in independently.

```admonish note title="The build hot path is `ui-storybook`"
The watched crate is `ui-storybook`. The rebuild step is
`cargo run -p ui-storybook --bin ui-export-stories`. Most of that
3–8 s window is cargo deciding which artifacts to re-link. The
linker config above attacks that directly. DEV-07 (persistent worker)
will attack it differently — keeping the renderer warm and skipping
the link entirely on each iteration. Both stack.
```
