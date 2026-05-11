# GStreamer audio capture — M-MEDIA.5 / AUT-101

Spawns `gst-launch-1.0` with a pipeline that emits normalized `F32LE`
raw audio on stdout, then chunks the byte stream into
[`AudioChunk`](../api/media/audio/struct.AudioChunk.html)s with
contiguous PTS.

[api](../api/media/gstreamer_audio/struct.GstreamerAudioCapture.html)

## Two source modes

```text
GstreamerAudioCapture::test_source(format, freq_hz)
  audiotestsrc wave=sine freq=F
    ! audioconvert
    ! audioresample
    ! audio/x-raw,format=F32LE,rate=R,channels=C,layout=interleaved
    ! fdsink fd=1

GstreamerAudioCapture::from_file(path, format)
  filesrc location=PATH
    ! decodebin
    ! audioconvert
    ! audioresample
    ! audio/x-raw,format=F32LE,rate=R,channels=C,layout=interleaved
    ! fdsink fd=1
```

`test_source` is the AUT-101 deliverable — deterministic, no
external file required. `from_file` is the companion fixture path
that decodes a real audio file (the bundled
`crates/media/tests/fixtures/sample-audio.mp3`, a deterministic 35-s
440 Hz sine).

Real fixtures matter for downstream tests: M-MEDIA.8 (audio
histogram) needs to assert numeric correctness on real-world signal,
not just mock data. The fixture's pure 440 Hz sine has a known RMS
(amplitude / √2) so the assertions stay tight.

## Lifecycle

`GstreamerAudioCapture` owns the child process. Drop kills the child
and waits — without this, `gst-launch-1.0` keeps decoding into a
dropped pipe and burns CPU. Matches the
[`decode::GstreamerPipeStream`](../api/decode/gstreamer_pipe/struct.GstreamerPipeStream.html)
pattern.

```rust,no_run
use media::audio::AudioFormat;
use media::gstreamer_audio::GstreamerAudioCapture;

let fmt = AudioFormat::mono_f32(48_000);
let mut cap = GstreamerAudioCapture::test_source(fmt, 440.0)?;

// Read 100 ms chunks for 1 second.
for _ in 0..10 {
    let chunk = cap.next_chunk(4_800)?;
    println!(
        "pts={:.3}s rms={:.3} peak={:.3}",
        chunk.pts().as_seconds(),
        chunk.rms(),
        chunk.peak(),
    );
}
# Ok::<(), media::gstreamer_audio::Error>(())
```

## Format support

Only [`SampleFormat::F32`](../api/media/audio/enum.SampleFormat.html#variant.F32)
is supported at construction — the pipeline caps it to `F32LE`
explicitly and converting integer formats at this seam would push
sample-format complexity into the public API for no payoff.
[`crate::audio::SampleFormat::I16`] / `U8` exist for future capture
backends that need to declare an upstream layout, but the public
`from_file` / `test_source` constructors require `F32`. A non-F32
format returns [`Error::UnsupportedFormat`](../api/media/gstreamer_audio/enum.Error.html#variant.UnsupportedFormat).

## Integration tests

4 tests in `crates/media/tests/gstreamer_audio_integration.rs`,
each skip-guarded via
[`media::gstreamer::is_available`](../api/media/gstreamer/fn.is_available.html):

| Test | Asserts |
|---|---|
| `test_source_emits_chunks_with_expected_format_and_pts` | 3 × 100 ms chunks, contiguous PTS (0.0, 0.1, 0.2 s), correct frame count. |
| `test_source_sine_440hz_has_rms_near_amp_over_sqrt_2` | 1 s of audiotestsrc at default volume 0.8 → RMS ≈ 0.566 ± 0.02. |
| `test_source_stereo_interleaves_correctly` | Stereo chunks have L ≈ R per frame (audiotestsrc emits the same waveform on every channel). |
| `from_file_decodes_real_mp3_fixture` | The bundled MP3 decodes to 44.1 kHz stereo with RMS in `(0.4, 0.95)` — pure sine after MP3 round-trip. |

## Manual regression

```bash
just gate                           # runs the integration tests via nextest
                                    # (skips silently if GStreamer absent)
cargo run -p media --example gst_audio_dump  # planned M-MEDIA.11 follow-up
```

## Done when

- [x] `GstreamerAudioCapture::test_source` produces valid `AudioChunk`s.
- [x] `GstreamerAudioCapture::from_file` decodes the bundled fixture.
- [x] Chunks include rate, channels, duration, PTS.
- [x] Integration test skips gracefully when GStreamer is unavailable.
- [x] `Drop` kills the child + waits.
- [x] mdBook chapter (this page).
- [x] `just gate` green.
