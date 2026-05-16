# Box plot

Five-number summary of a distribution per category — min / Q1 /
median / Q3 / max. Side-by-side boxes compare distributions
across categories.

The demo plots **Boston Marathon men's winning times by decade**
(1900s through 1960s), in minutes. The marathon began in 1897
as the second-oldest marathon worldwide; each box summarises
the per-decade spread of the *winning* time. The progression
from a 158-min median in the 1900s to 138 min in the 1960s
tracks training science + course modernisation; the wide 1920s
box is the pre-pace-strategy era where race tactics were still
being invented. Source: Wikipedia "List of Boston Marathon
winners".

<div style="position: relative; aspect-ratio: 400 / 240; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/boxplot.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-boxplot" src="../demo/?chart=boxplot" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: Boston Marathon winning times by decade"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/List_of_Boston_Marathon_winners" target="_blank" rel="noopener">Source: List of Boston Marathon winners — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::distributions::{BoxPlot, Box};
use wisp_chart::color::Color;

// From precomputed quartiles:
let bp = BoxPlot::new(vec![
    Box::from_summary("A", 10.0, 20.0, 30.0, 45.0, 60.0, Color::from_hex("#0072b2").unwrap()),
    /* ... */
]);

// Or from raw samples:
let samples: Vec<f32> = /* … */;
let bx = Box::from_samples("ints", &samples, Color::from_hex("#d55e00").unwrap()).unwrap();
```

```admonish info
`from_samples` uses the inclusive-method percentile lookup —
`samples[((n-1) × p) as usize]` after sorting. Good for a quick
exploration; for publication-grade quartiles compute them
upstream (e.g. with `statrs`) and call `from_summary`.
```

## Per-box primitives

Each box emits 6 primitives: 1 filled rect (Q1→Q3), 1 median
line, 2 whiskers (min→Q1 + Q3→max), 2 whisker caps. A 4-box
plot is 24 primitives total.
