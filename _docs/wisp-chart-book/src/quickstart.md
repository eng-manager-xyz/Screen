# Quickstart

Add `wisp-chart` to a workspace crate that already depends on
`wisp`. The chart-specific dep `jiff` rides in via `wisp-chart`
— don't add it to your own `Cargo.toml` unless you need
`jiff` directly.

```toml
[dependencies]
wisp.workspace = true
wisp-chart = { path = "../wisp-chart" }
```

Build the smallest possible Gantt:

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

```admonish note title="`Gantt::render` lands in M-CHART.0 chunk 3"
The foundation commit (chunk 1) ships the data + theme + palette
modules. The render pass — `Gantt::render(&Theme) -> SceneNode`
— follows in [chunk 3](./charts/gantt/overview.md). Until that
chunk lands, the snippet above is illustrative; build with
`cargo check -p wisp-chart` to confirm the data API.
```

## Run it in a browser

The same crate compiles for `wasm32-unknown-unknown` and
renders into a `<canvas>` via WebGPU. See
[Run wisp-chart in Chrome via WebGPU](./web-demo.md).
