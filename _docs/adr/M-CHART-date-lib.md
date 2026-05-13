# ADR — date library for `wisp-chart` (M-CHART.0 / AUT-180)

**Status:** Accepted, 2026-05-13
**Scope:** `crates/wisp-chart/` only. The chosen dep does NOT enter `wisp`.

## Context

`wisp-chart` needs a date type for `DateRange { start, end }` plus a
small operation set (Y-M-D construction, day arithmetic, weekday
detection, month boundaries). The crate compiles for native AND
`wasm32-unknown-unknown` so the chart can render in a Chrome
canvas via WebGPU.

Candidates surveyed:

| Crate | Pros | Cons |
|---|---|---|
| `chrono` (0.4) | Ubiquitous, mature, broad ecosystem | Pulls system-time bindings on most targets; long-standing soundness issues around `Local`; wasm support requires explicit feature flags |
| `time` (0.3) | Lean, `no_std`-friendly, wasm-clean | Macro-heavy for date literals; smaller op set than chrono; community is fragmented |
| **`jiff` (0.2)** | Modern (Rust 2024-era), zoned and civil types are separate, wasm-clean, audited dependency tree, designed by BurntSushi | Younger crate; smaller community than chrono |
| `icu_calendar` | Correct for non-Gregorian calendars | Overkill — we only need Gregorian dates |

## Decision

**Use `jiff`** (`= "0.2"`, `default-features = false`, `features = ["std"]`).

### Rationale

- **Scope fit.** `wisp-chart` needs `Date` for Y-M-D arithmetic +
  ISO-week detection. `jiff::civil::Date` is exactly this — no
  timezone baggage, no leap-second machinery.
- **WASM-clean.** `jiff` compiles cleanly to
  `wasm32-unknown-unknown` with the `std` feature; no JS interop
  required, no system-time bindings forced on us. Verified during
  the M-CHART.0 demo bring-up.
- **Boundary discipline.** `jiff` is pinned in
  `crates/wisp-chart/Cargo.toml` **only** — not added to the
  workspace `[workspace.dependencies]`, not added to `wisp`'s
  `Cargo.toml`. Per AUT-180: *"wisp continues to know nothing
  about dates, themes, charts, or palettes — it stays a
  Pixi-equivalent primitive renderer."* If `wisp-chart` is later
  removed from the workspace, `jiff` leaves with it.
- **Surface area control.** v1 imports only `jiff::civil::Date`
  and the `date!` constructor. The bigger zoned-time / parsing /
  formatting surface stays unused until a later chart type
  (e.g. timeline annotations) demands it.

## Consequences

- New transitive deps (handful) land in `Cargo.lock`. Verified
  against `cargo deny` policy (no GPL, no banned crates).
- ISO-week-correct alignment is **not** implemented in v1 — the
  Gantt's time axis uses a uniform 52-bucket division. Documented
  in `_docs/wisp-chart-book/src/charts/gantt/time-axis.md`. A
  future chunk swaps the bucketing to ISO-week-aware once we
  have a real year-boundary test case.
- Future ingest formats (CSV, Linear export) can lean on
  `jiff::fmt::strtime` without bumping a major.

## Alternatives explicitly rejected

- **`chrono`** — too large a surface for our needs; brings
  timezone machinery we don't use; wasm + system-time mode
  selection is fiddlier than `jiff`'s.
- **Roll our own `Date(i32, u8, u8)`** — would need to re-derive
  leap-year + day-of-week logic; not worth the maintenance
  burden for a chart crate.

## Anti-pattern guardrails

- ❌ Adding `jiff` to `[workspace.dependencies]` — would invite
  `wisp` itself to depend on it later, breaking the boundary.
- ❌ Re-exporting `jiff::civil::Date` from `wisp-chart::prelude`
  if consumers shouldn't need it — keep the date type behind
  `DateRange` constructors (`DateRange::year`,
  `DateRange::from_range`) as much as possible.

## Revisit triggers

- A non-Gregorian chart use case lands → swap in `icu_calendar`
  or add a feature flag.
- `jiff` stalls / loses maintenance → switch to `time 0.3` with
  an additive shim; v1's `DateRange` surface is small enough that
  the swap is mechanical.
