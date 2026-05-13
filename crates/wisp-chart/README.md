# `wisp-chart` — chart compositions for `wisp`

> Data structs in, a scene-graph subtree comes out. v1 ships
> Gantt; bar / line / area follow.

## What it does

`wisp-chart` is the opinionated composition layer that turns
"rows × bars × time" into a `wisp::scene::Node` the renderer
draws. It depends on `wisp` (never the reverse) and pins all
chart-specific dependencies (`jiff` for dates, palette types,
contrast utils) to this crate's `Cargo.toml`.

## Where it fits

```mermaid
flowchart LR
    Data[Gantt data struct]:::chart --> Render[Gantt::render]:::chart
    Render --> Node[wisp::scene::Node]:::wisp
    Node --> Stage[wisp::Stage]:::wisp
    Stage --> GPU[wgpu device]:::shell

    classDef chart fill:#312e81,stroke:#6366f1,color:#e0e7ff
    classDef wisp fill:#7c2d12,stroke:#ea580c,color:#fed7aa
    classDef shell fill:#1e293b,stroke:#475569,color:#e2e8f0
```

## Quickstart

```rust,ignore
use wisp_chart::{Bar, DateRange, Gantt, Row, Theme};
use jiff::civil::date;

let chart = Gantt {
    range: DateRange::year(2026),
    rows: vec![Row::new("vec", "M-VEC")],
    bars: vec![Bar::new(
        "vec",
        date(2026, 2, 1)..date(2026, 3, 15),
        "Matt",
    )],
    people: Default::default(),
};

let node = chart.render(&Theme::light());
stage.add(node);
```

> [!IMPORTANT]
> **`wisp-chart` adds chart-specific dependencies to its own
> Cargo.toml only.** Never add `jiff`, palette types, or chart
> colour primitives to `wisp`'s `Cargo.toml` — that breaks the
> "wisp stays a Pixi-equivalent primitive renderer" boundary.

## Where it runs

- **Native** — same path as everything else in `wisp`.
- **WebGPU / wasm** — `wisp-chart` is `wasm32-unknown-unknown`-
  clean (no `winit`, no `pollster` in its tree). A consumer
  crate constructs a `wgpu::Surface` from `HtmlCanvasElement`,
  asks `wisp-chart` for a `SceneNode`, and `wisp`'s renderer
  module draws it.
  See `crates/wisp-chart-web/` for the reference demo.

## Public API at a glance

| Item | Notes |
|---|---|
| `Gantt`, `Row`, `Bar`, `DateRange`, `PersonMap`, `Person` | Data-struct API. v1. |
| `Theme`, `Theme::light()` | Light theme + Wong palette default. |
| `OwnerPalette::Wong` | 8-colour colourblind-friendly palette. Hash-by-name auto-assignment. |
| `Color`, `contrast_text_color` | Chart-scoped colour + WCAG-2.x luminance. |
| `Gantt::render(&Theme) -> SceneNode` | (lands in M-CHART.0 render chunk) |

## Runbook

- **Build:** `cargo build -p wisp-chart`
- **Test:** `cargo nextest run -p wisp-chart`
- **WASM build:** `cargo check --target wasm32-unknown-unknown -p wisp-chart`
- **Live web demo:** `just dev-wisp-chart-demo` (Trunk serves
  `crates/wisp-chart-web/` at `http://127.0.0.1:8080`)

## Deep dive

- [wisp-chart book](https://eng-manager-xyz.github.io/Screen/wisp-chart/) — chapters under `_docs/wisp-chart-book/`
- [`_docs/adr/M-CHART-date-lib.md`](../../_docs/adr/M-CHART-date-lib.md) — why `jiff`
- [Linear M-CHART.0 (AUT-180)](https://linear.app/harwood/issue/AUT-180) — the originating ticket
- Sibling `wisp` README: [`../wisp/README.md`](../wisp/README.md)
- WebGPU consumer demo: [`../wisp-chart-web/README.md`](../wisp-chart-web/README.md)

## License

MIT.
