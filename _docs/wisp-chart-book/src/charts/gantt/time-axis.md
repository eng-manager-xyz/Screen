# Time axis

The Gantt's horizontal axis maps `Date` to pixel `x`. v1 uses
a **uniform-buckets** approach; ISO-week-correct alignment is
a deliberate deferral.

## Mapping (v1)

For `range = [range.start, range.end)` and a plot area of
`plot_width` pixels:

```text
days_total = (range.end - range.start).num_days() as f32
day_index  = (date - range.start).num_days() as f32
x          = (day_index / days_total) * plot_width
```

This is **uniform per day**. A year with 365 days maps each day
to `plot_width / 365` px; a 366-day leap year maps to
`plot_width / 366` px. Bars across leap-year boundaries are
slightly stretched.

## Row mapping

For row index `i` (top to bottom):

```text
y = header_height + i * theme.row_height + (theme.row_height - theme.bar_height) / 2
```

Bars are centred vertically within their row.

## Grid line detection

The renderer walks `range.start..range.end` once per chart:

- Emit a **week divider** at every Monday (ISO weekday 1).
- Emit a **month divider** at every day 1.

Month dividers paint AFTER week dividers (heavier, on top).

## Why not ISO-week-correct alignment in v1

Per the M-CHART.0 ticket:

> v1 uses uniform 52-bucket division. Documented in
> `time-axis.md`.

Reasons:

1. **Year-boundary handling is the hard part.** ISO-week 53
   doesn't always exist; ISO-week 1 can start in the previous
   calendar year. Getting it right adds branches without
   product value at this scope.
2. **The 2026 fixture doesn't trip any ISO edge cases.** v1's
   demo year is well-behaved; uniform buckets render
   visually-correct dividers.
3. **The data API is unchanged.** When a follow-on chunk ships
   ISO-correct alignment, no `Gantt` / `Bar` field changes —
   only the layout module's `divider_dates` function.

## Future: zoom + scroll

A pannable / zoomable axis is M-CHART.5 (parking lot). The v1
chart is a single fixed-extent composition; interactivity
ships later.
