# Funnel chart

Staged conversion / loss visualisation. Each stage is a
horizontal band; band width reflects remaining count relative to
the widest stage.

<div style="position: relative; aspect-ratio: 400 / 300; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/funnel.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=funnel" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: funnel chart"></iframe>
</div>

## Public surface

```rust,ignore
use wisp_chart::topology::{Funnel, FunnelStage};
use wisp_chart::color::Color;

let c = |hex| Color::from_hex(hex).unwrap();
let f = Funnel::new(vec![
    FunnelStage::new("Visited",   10000.0, c("#0072b2")),
    FunnelStage::new("Signed up",  4000.0, c("#56b4e9")),
    FunnelStage::new("Activated",  1800.0, c("#7faedc")),
    FunnelStage::new("Converted",   600.0, c("#a3c7ea")),
]);
let g = f.emit_graphics(&theme, Vec2::new(400.0, 300.0));
```

```admonish info
Each band is horizontally centred — `width = count / max_count
× plot_width`. The drop-off between adjacent bands shows where
the biggest losses happen, which is usually what the reader
cares about.
```
