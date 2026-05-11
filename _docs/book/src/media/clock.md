# Media clock + timestamp model

[Linear: AUT-98](https://linear.app/harwood/issue/AUT-98)

Every audio chunk, video frame, cursor event, and visualization
window in the recorder is stamped against **one** shared timeline.
This module is that timeline's vocabulary.

[api](../api/media/clock/index.html)

## Types

| Type | Role |
|---|---|
| [`MediaTime`](../api/media/clock/struct.MediaTime.html) | A point on the timeline. Internal i64 nanoseconds. |
| [`MediaDuration`](../api/media/clock/struct.MediaDuration.html) | An interval between two `MediaTime`s. Signed (drift can be negative). |
| [`MediaClock`](../api/media/clock/struct.MediaClock.html) | Authoritative timeline source — wall-clock or manual. |
| [`Timestamped<T>`](../api/media/clock/struct.Timestamped.html) | A value plus the `MediaTime` it occurred at. |

## Why nanoseconds (i64)

Internal representation is **`i64` nanoseconds**, signed so a
pre-origin offset is representable.

- `i64::MAX` ns ≈ **292 years** — comfortable headroom for any
  recorder session.
- `f64` seconds drops below 1 µs precision past ~10⁹ s; nanoseconds
  stay exact through arithmetic.
- Integer math for `from_sample` / `to_sample` round-trips exactly
  for any sample rate (44.1 kHz included), thanks to round-half-up
  in `to_sample`. Without rounding, 44.1 kHz drifts -1 sample per
  conversion.

## Sample / frame conversions

```rust
use media::clock::MediaTime;

// 30 fps, frame 90 → 3.0 s.
assert!((MediaTime::from_frame(90, 30.0).as_seconds() - 3.0).abs() < 1e-9);

// 48 kHz, sample 48 000 → 1.0 s exactly (nanos-level).
assert_eq!(MediaTime::from_sample(48_000, 48_000).as_nanos(), 1_000_000_000);

// 44.1 kHz round-trips exactly thanks to round-half-up.
let t = MediaTime::from_sample(1, 44_100);
assert_eq!(t.to_sample(44_100), 1);
```

## Two clock modes

```rust
use media::clock::{MediaClock, MediaDuration, MediaTime};

// Production — anchored to Instant::now().
let live = MediaClock::wall_clock();
let _t = live.now();

// Tests + headless examples — driven by advance_by, byte-exact reproducible.
let mock = MediaClock::manual(MediaTime::ZERO);
mock.advance_by(MediaDuration::from_millis(20));
assert!((mock.now().as_seconds() - 0.020).abs() < 1e-12);
```

`MediaClock::assign(value)` attaches the current timestamp:

```rust
# use media::clock::{MediaClock, MediaTime, Timestamped};
let clock = MediaClock::manual(MediaTime::from_seconds(2.5));
let ts: Timestamped<&str> = clock.assign("hello");
assert_eq!(ts.value, "hello");
```

The synthetic A/V sync harness (M-MEDIA.7) uses
`MediaClock::manual` so the test's drift assertion is deterministic
across hosts. Live capture (M-MEDIA.5/.6) uses
`MediaClock::wall_clock`.

## Arithmetic

```text
MediaTime + MediaDuration  → MediaTime      (forward in time)
MediaTime - MediaDuration  → MediaTime      (backward in time)
MediaTime - MediaTime      → MediaDuration  (interval between)
MediaDuration ± MediaDuration → MediaDuration
```

Saturating arithmetic prevents overflow at the i64 boundary.

`MediaDuration::abs()` is provided specifically for drift reporting —
A/V sync logs typically want the magnitude rather than the signed
delta.
