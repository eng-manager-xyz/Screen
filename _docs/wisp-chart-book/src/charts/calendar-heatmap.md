# Calendar / annual heatmap

GitHub-style contribution graph or year-in-review heatmap — one
cell per day, 7 rows × 53 columns, colour intensity reflects
daily value.

<div style="position: relative; aspect-ratio: 720 / 120; max-width: 100%; margin: 1rem 0; background: url('../assets/wisp-chart-web/calendar-heatmap.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="../demo/?chart=calendar-heatmap" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: calendar heatmap"></iframe>
</div>

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
