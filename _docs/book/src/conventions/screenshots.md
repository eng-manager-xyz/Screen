# Story / screenshot pipeline

Every renderable chunk regenerates its asset under
`_docs/book/src/assets/<crate>/<id>.{png,html}`. The mdBook chapter for that
chunk embeds the asset; without it, the page renders empty — that's the gate.

## Producing assets

```bash
just snapshots         # all crates
just snapshots-wisp    # wgpu PNGs only
just snapshots-ui      # Leptos SSR HTML only
```

Both exporters are real binaries inside their respective storybooks, so they
run on CI without any GUI.

### `wisp-storybook` exporter (PNG)

For each `Story` in `wisp_storybook::stories::all_stories()`:

1. Build a fresh `Application` and `Renderer` (Rgba8Unorm, 256×256).
2. Call `story.build(app, &mut stage)`; if `tick` is set, call `tick(stage, 0.0)`.
3. Render to a `RenderTexture` and read pixels back.
4. Save to `_docs/book/src/assets/wisp/<id>.png`.

### `ui-storybook` exporter (HTML)

For each `Story` in `ui_storybook::stories::all_stories()`:

1. Call `story.render()` to get the SSR HTML body.
2. Wrap it in a complete `<html>` document with `style.css` inlined.
3. Save to `_docs/book/src/assets/ui/<id>.html`.

A future upgrade swaps the HTML output for a PNG via `headless_chrome`.

## Embedding in mdBook

Standard markdown:

```markdown
![](assets/wisp/filter-blur.png)
```

Or for a UI demo with iframe:

```markdown
<iframe src="../assets/ui/dope-sheet-basic.html" width="100%" height="280"></iframe>
```

## Convention checklist (per chunk)

When closing any visible chunk:

- [ ] Story exists and snapshot test is green.
- [ ] `just snapshots` regenerated the asset.
- [ ] mdBook chapter for the chunk references the asset.
- [ ] Asset committed (`_docs/book/src/assets/...` is part of the commit).
