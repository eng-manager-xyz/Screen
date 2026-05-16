# Calendar / annual heatmap

GitHub-style contribution graph or year-in-review heatmap — one
cell per day, 7 rows × 53 columns, colour intensity reflects
daily value.

The demo plots **weekly excess-mortality from influenza +
pneumonia in NYC across 1918** — the year the Spanish flu
killed more Americans than World War I, World War II, the Korean
War, and the Vietnam War combined. The lighter March / April
band is the "first wave"; the saturated October cluster is the
catastrophic autumn second wave that peaked at ~13 weekly
deaths per 10 k population.

<div style="position: relative; aspect-ratio: 720 / 120; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/calendar-heatmap.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe id="demo-calendar-heatmap" src="../demo/?chart=calendar-heatmap" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: 1918 NYC flu mortality"></iframe>
</div>
<p style="margin: 0.5rem 0 1.25rem;">
  <a class="replay-source-link" href="https://en.wikipedia.org/wiki/1918_flu_pandemic_in_the_United_States" target="_blank" rel="noopener">Source: 1918 flu pandemic in the United States — Wikipedia</a>
</p>

## Public surface

```rust,ignore
use wisp_chart::heatmap::{CalendarHeatmap, CalendarValue, SequentialPalette};
use jiff::civil::date;

let cal = CalendarHeatmap::new(2025, vec![
    CalendarValue::new(date(2025, 1, 15),  5.0),
    CalendarValue::new(date(2025, 6, 1),  12.0),
    /* ... */
])
.palette(SequentialPalette::github());
let g = cal.emit_graphics(&theme, Vec2::new(720.0, 120.0));
```

## Layout

```admonish info
Columns are ISO weeks (1..52, occasionally 53 for leap years).
Rows are weekdays — Monday at row 0, Sunday at row 6. Days with
no entry render at the palette's `0.0` stop. Values from other
years are silently ignored so reusing a multi-year dataset
across multiple heatmaps is safe.
```

## Date math via jiff

```admonish note
`wisp-chart` depends on [jiff](https://docs.rs/jiff) for date /
weekday / ISO-week math. jiff is scoped to this crate's
Cargo.toml so `wisp` itself stays date-free.
```
