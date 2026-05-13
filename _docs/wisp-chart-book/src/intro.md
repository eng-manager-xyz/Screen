# wisp-chart

> Data structs in, scene-graph subtrees out.

`wisp-chart` is the **opinionated chart composition layer** for
[wisp]({{wisp-link wisp/overview}}). It turns "rows × bars ×
time" (Gantt) — and eventually bar / line / area — into a
`wisp::scene::Node` that drops straight into a `wisp::Stage`.

```admonish important title="Boundary discipline"
`wisp-chart` depends on `wisp`, **never the reverse**. Every
chart-specific dependency (`jiff` for dates, palette types,
contrast utils) lives in `wisp-chart/Cargo.toml` only — `wisp`
stays a Pixi-equivalent primitive renderer with zero awareness
of charts, dates, themes, or palettes.
```

## What this book covers

- **Gantt** — the v1 composition. Hyper-specific first example
  (one concrete year, one concrete team) so the API has zero
  degrees of freedom before we iterate on flexibility.
- **Web demo** — running the same chart in a Chrome
  `<canvas>` via WebGPU. Same crate, same code, different
  surface.

## What this book does not cover

- The wisp renderer itself — see the
  [wisp book]({{wisp-link wisp/overview}}).
- The Screen recorder application — see the
  [Screen book](/Screen/).

## Three books, one site

This is the third mdBook in the Screen monorepo, mounted at
`/Screen/wisp-chart/`. The other two:

| Path | Book | Source |
|---|---|---|
| `/Screen/` | Screen recorder | `_docs/book/` |
| `/Screen/wisp/` | wisp renderer | `_docs/wisp-book/` |
| `/Screen/wisp-chart/` | This book | `_docs/wisp-chart-book/` |

Cross-book links go through the `mdbook-preprocessor-cross`
tags so they survive the deployed path-based routing.
