# Error bars overlay

Show measurement uncertainty alongside a central tendency —
confidence intervals on a bar chart, standard errors on a
scatter, fixed-value error bars on a measurement series. Lifts
a chart from "looks confident" to "is honest about uncertainty".

The demo plots **Robert Millikan's published electron-charge
values from the oil-drop experiment**, 1909 → 1913, with the
uncertainty bands he reported each year. Millikan's 1913 figure
of 4.774 × 10⁻¹⁰ statcoulomb won him the 1923 Nobel and held as
the canonical value until later re-analyses. The visible
year-to-year drift — bars that don't overlap one another's
intervals as you'd expect them to — is the cautionary tale
Richard Feynman cited in *Cargo Cult Science*: error bars don't
include systematic bias.

<div style="position: relative; aspect-ratio: 480 / 320; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/error-bars.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-error-bars" src="../demo/?chart=error-bars" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: Millikan oil drop values 1909-1913"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/Oil_drop_experiment" target="_blank" rel="noopener">Source: Oil drop experiment — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::overlay::{ErrorBars, ErrorPoint, ErrorKind};
use wisp_chart::plot::{self, Mark, Plot, ScaleKind};
use wisp::math::Rect;

// 1. Render the primary chart (Bar, Point, Line — anything cartesian).
let bar = Plot::new(rows)
    .mark(Mark::Bar { value_labels: false })
    .encode(plot::x("quarter", ScaleKind::Band))
    .encode(plot::y("revenue", ScaleKind::Linear));
let bar_g = bar.render(&theme, viewport);
let _ = stage.add_child(root, bar_g);

// 2. Build the error-bars overlay with one entry per primary mark.
//    `x_fraction` is the position along the plot's x extent (0..1).
let bars = ErrorBars::new(
    vec![
        ErrorPoint::symmetric(0.125, 38.0, 5.0),
        ErrorPoint::symmetric(0.375, 52.0, 7.0),
        ErrorPoint::symmetric(0.625, 47.0, 6.0),
        ErrorPoint::symmetric(0.875, 64.0, 9.0),
    ],
    (0.0, 64.0), // Y domain matching the bar chart
);

// 3. Overlay using the SAME plot rect so whiskers land on bar centres.
let plot_rect = Rect::new(60.0, 40.0, viewport.x - 80.0, viewport.y - 80.0);
let overlay = bars.emit_graphics_in_rect(&theme, viewport, plot_rect);
let _ = stage.add_child(root, overlay);
```

## Three error kinds

```admonish info
[`ErrorKind`] documents the three input flavours `ErrorPoint`
helpers convert into the absolute `(lower, upper)` representation
used at render time:

- `Symmetric(half_width)` — standard deviation or `±h` uncertainty.
  Helper: `ErrorPoint::symmetric(x_fraction, mean, half_width)`.
- `Asymmetric { lower, upper }` — skewed distributions, quantile
  intervals. Helper: `ErrorPoint::asymmetric(x_fraction, mean, below, above)`.
- `ConfidenceInterval(half_width)` — caller pre-multiplies the
  standard error by the z-score (`1.96 × SE` for a 95% CI). The
  enum carries the half-width; the caller does the stats.
```

## Plot-rect alignment

```admonish important
The most common slip-up is calling
[`ErrorBars::emit_graphics`] (16-px pad default) when overlaying
on a `Plot::Bar` chart, which uses a 60-px gutter + 40-px header
/ footer. The whiskers land off-centre. **Always use
[`ErrorBars::emit_graphics_in_rect`]** with the underlying
chart's plot rectangle — for a default `Plot` bar chart that's
`Rect::new(60.0, 40.0, viewport.x - 80.0, viewport.y - 80.0)`.
```

## Why this is an overlay, not a mark

```admonish note
v1 ships `ErrorBars` as a self-contained value type rather than
a `Mark::ErrorBar` variant on the [Plot facade](./plot.md). The
overlay use case spans multiple mark families (bar / point /
line / box) and doesn't fit cleanly into the `(X, Y, Color)`
channel model — the per-point lower/upper are extra dimensions.
A future ticket can add `Plot::overlay(ErrorBars)` once enough
real callers settle on a common ergonomic shape.
```
