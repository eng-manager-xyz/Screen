# `screen` — engineering site

This is the offline engineering site for the `screen` recorder. It bundles:

- **Prose architecture** — the theatre metaphor, milestones, conventions.
- **Per-feature stories** — every renderable chunk has a screenshot under
  `assets/<crate>/<id>.png`, embedded inline.
- **Per-component UI demos** — every Leptos component has a snapshot of its
  SSR HTML alongside the reference live render.
- **Full API reference** — `cargo doc` output mounted at `/api/`.

## How to read this

- New here? Start with the [theatre metaphor](./orientation/metaphor.md) — that's
  the navigation language for the entire codebase.
- Looking for a specific feature? Each milestone chapter ([M0](./milestones/m0.md),
  [M1](./milestones/m1.md), …) lists every chunk with its screenshot and
  link into the API ref.
- Touching code? Read the [Workflow](./conventions/workflow.md) and the
  [Documentation gate](./conventions/docs.md) before you start — they're the
  rules that protect the build.

## How to regenerate this site

```bash
just site
# Opens target/book/index.html
```

`just site` runs three things in sequence:
1. `mdbook build _docs/book` — the prose chapters → `target/book/`
2. `cargo doc --workspace --no-deps` — the API reference → `target/book/api/`
3. (Optional, on demand) `just snapshots` — regenerates per-feature assets
   under `_docs/book/src/assets/<crate>/<id>.{png,html}`.

## How screenshots work

Every visible feature ships with a story (see [conventions](./conventions/screenshots.md)).
Stories are headlessly rendered to `_docs/book/src/assets/`:

- **wisp** stories → 256×256 PNGs via `wisp-storybook`'s headless exporter
  (uses the same `Renderer::render_stage` + `RenderTexture::read_pixels` path
  the integration tests use).
- **ui-storybook** stories → standalone HTML files (SSR + inlined CSS) so they
  can be opened in a browser tab as live demos. Future work upgrades this to
  PNGs via `headless_chrome`.
- **Recorder app** screenshots are committed manually for now.

The result: `mdbook build` is purely declarative — every asset already exists
on disk. CI doesn't need a GPU or a browser to publish the site.
