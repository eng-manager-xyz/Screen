# TypeWriter

`TypeWriter` reveals characters of a fixed-length string one at
a time. Output is `usize` — the visible character count. Caller
truncates the string (or, future-state, calls
`wisp::Text::set_visible_glyphs(count)`) each frame.

<div style="position: relative; aspect-ratio: 1 / 1; max-width: 360px; margin: 1rem 0; background: url('../assets/wisp-animation/typewriter-hero.png') center/contain no-repeat #fafafa; border: 1px solid #e5e5e5;">
  <iframe src="https://eng-manager-xyz.github.io/Screen/wisp-chart/demo/?chart=polar&amp;animate=type-in" style="position: absolute; inset: 0; width: 100%; height: 100%; border: 0;" loading="lazy" title="Live WebGPU demo: polar plot staircased to scale via TypeWriter"></iframe>
</div>

The demo maps the `usize` output (0..=10) to chart scale —
the chart visually "types in" through 10 discrete steps over
1.5 seconds. Same primitive, applied to scale rather than glyph
count.

## API surface

```rust,ignore
use std::time::Duration;
use wisp_animation::{Animation, TypeWriter};

let tw = TypeWriter::new(20, Duration::from_secs(1));
let visible: usize = tw.sample(Duration::from_millis(500));
// visible == 10 (halfway).

// Rate-based constructor:
let fast = TypeWriter::at_rate(40, 60.0); // 60 chars / sec
```

## Test invariants

- `sample(Duration::ZERO)` is 0.
- `sample(duration)` is `total_chars`.
- Midpoint sample is `total_chars / 2` (rounded).
- Past the end clamps to `total_chars`.
- `at_rate` derives duration as `total_chars / rate`.

Full source: [`crates/wisp-animation/src/typewriter.rs`](https://github.com/eng-manager-xyz/Screen/blob/main/crates/wisp-animation/src/typewriter.rs).
