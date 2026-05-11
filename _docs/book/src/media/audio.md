# Audio data model — M-MEDIA.3 / AUT-99

Every audio buffer in the recorder flows through one type:
[`AudioChunk`](../api/media/audio/struct.AudioChunk.html) — a
timestamped slice of normalized `f32` samples with its
[`AudioFormat`](../api/media/audio/struct.AudioFormat.html). The
GStreamer capture path (M-MEDIA.5), the deterministic mock sources
(M-MEDIA.4), the histogram quantizer (M-MEDIA.8), and the live
microphone path (M-MEDIA.15) all produce / consume this type.

[api](../api/media/audio/index.html)

## Why normalized `f32`

- It's what every downstream visualization wants — `AudioHistogram`'s
  RMS / peak math runs cleaner on floats than on integers.
- It's what GStreamer's `audioconvert ! audio/x-raw,format=F32LE`
  produces natively — capture pipelines don't have to re-quantize.
- Future device-capture backends (`cpal`, `coreaudio-rs`) emit `f32`
  as their preferred shape too — no buffer layout churn at the seam.

The [`SampleFormat`](../api/media/audio/enum.SampleFormat.html) enum
exists so capture-side code can declare its **input** layout
(F32 / I16 / U8) before normalization. Internally,
`AudioChunk::samples` is always `&[f32]`.

## Interleave order — planar-per-frame

Stereo: `[L₀, R₀, L₁, R₁, …]`. Mono: `[s₀, s₁, …]`. Matches GStreamer
raw-audio, `cpal`, `coreaudio-rs`. No re-layout needed at the
capture seam.

## Validation

[`AudioChunk::new`] rejects:

- `samples.len() % channels != 0` — each frame must carry exactly one
  sample per channel.
- `channels == 0`.
- `sample_rate == 0`.

These are the three "is this a meaningful chunk?" checks. The
remaining shape questions (clipping, NaN, DC offset) are visualization
concerns, not data-model concerns.

## Derived metrics

`AudioChunk::peak()` and `AudioChunk::rms()` are pre-computed
shortcuts used by M-MEDIA.8 (histogram quantization) and capture-side
regression checks. They run in `O(n)` over the buffer; cache the
result if you need it more than once per chunk.

## Quick start

```rust
use media::audio::{AudioChunk, AudioFormat};
use media::clock::MediaTime;

let fmt = AudioFormat::mono_f32(48_000);
let samples = vec![0.0_f32; 48_000]; // 1.0 s of silence at 48 kHz.
let chunk = AudioChunk::new(fmt, samples, MediaTime::ZERO).expect("valid");
assert_eq!(chunk.frame_count(), 48_000);
assert!((chunk.duration().as_seconds() - 1.0).abs() < 1e-9);
```

## Done when

- [x] `AudioFormat` + `AudioChunk` exist with normalized `f32` samples.
- [x] Validation rejects unaligned buffers / zero channels / zero rate.
- [x] Tests cover mono chunks.
- [x] Tests cover stereo chunks.
- [x] Tests cover chunk duration from sample count.
- [x] Tests cover invalid chunk shape.
- [x] Convenience `peak()` + `rms()` for histogram consumers.
- [x] mdBook chapter (this page).
- [x] `just gate` green.
