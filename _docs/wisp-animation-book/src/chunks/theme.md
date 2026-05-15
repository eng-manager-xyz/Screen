# AnimTheme

`AnimTheme` is the host app's motion identity — default
duration, default ease, default stagger gap. Pass it where you'd
hard-code numbers; every primitive has a `from_theme` /
`theme.tween(...)` constructor that consumes it.

Two presets ship today:

| Preset | Duration | Ease | Stagger |
|---|---|---|---|
| `AnimTheme::snappy()` | 250 ms | OutCubic | 30 ms |
| `AnimTheme::smooth()` | 450 ms | OutExpo | 60 ms |

`AnimTheme::default()` returns `smooth()` — calmer is the safer
default for charts.

```rust,ignore
use wisp_animation::AnimTheme;

let theme = AnimTheme::snappy();

// Tween / Stagger inherit theme values:
let tween = theme.tween(0.0_f32, 1.0);   // 250 ms, OutCubic
let stagger = theme.stagger();           // 30 ms gap

// Per-tween overrides win:
let custom = theme
    .tween(0.0_f32, 1.0)
    .duration(std::time::Duration::from_millis(500))
    .ease(wisp_animation::Ease::OutBack);
```

```admonish important title="Value, not global"
`AnimTheme` is a plain struct — not a `static`, not in
thread-local storage. Hosts plumb it explicitly. That keeps the
"same animation, same inputs, same output" property of the
whole crate. Two parallel host apps can run completely
different theme presets without ever colliding.
```

## Test invariants

- `snappy()` returns 250 ms duration + OutCubic.
- `smooth()` returns 450 ms duration + OutExpo.
- `theme.tween(a, b)` produces a tween with the theme's duration
  and ease.
- `theme.stagger().offset_for(i, n)` uses the theme's gap.
- `AnimTheme::default() == AnimTheme::smooth()`.

Full source: [`crates/wisp-animation/src/theme.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/theme.rs).
