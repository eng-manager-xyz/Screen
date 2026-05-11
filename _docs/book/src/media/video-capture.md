# GStreamer video capture — M-MEDIA.6 / AUT-102

Spawns `gst-launch-1.0` with a `videotestsrc`-fed pipeline that emits
raw BGRA frames on stdout, then chunks the byte stream into
[`VideoFrame`](../api/media/video/struct.VideoFrame.html)s — the same
type [`decode::VideoStream`](../api/decode/trait.VideoStream.html)
returns.

[api](../api/media/gstreamer_video/struct.GstreamerVideoCapture.html)

## Pipeline

```text
videotestsrc is-live=false
  ! videoconvert
  ! video/x-raw,format=BGRA,width=W,height=H,framerate=F/1
  ! fdsink fd=1
```

`videotestsrc`'s default pattern is SMPTE colorbars with a tiny
animated ball — successive frames carry different bytes, which makes
visual smoke checks cheap.

AUT-102 only covers the `videotestsrc` path. M-MEDIA.16 (live
webcam) will add an `autovideosrc` variant; M-MEDIA.17 (playback
harness) will add a `filesrc ! decodebin` variant that decodes the
existing `crates/decode/tests/fixtures/sample.mp4`.

## Quick start

```rust,no_run
use media::gstreamer_video::GstreamerVideoCapture;

let mut cap = GstreamerVideoCapture::test_source(640, 360, 30)?;
for _ in 0..30 {
    let frame = cap.next_frame()?;
    println!(
        "frame {} pts={:.4}s {}×{} bgra-bytes={}",
        frame.frame_index,
        frame.pts_seconds,
        frame.width,
        frame.height,
        frame.bgra.len(),
    );
}
# Ok::<(), media::gstreamer_video::Error>(())
```

## Lifecycle

Drop kills the child + waits. Same pattern as
[`gstreamer_audio`](audio-capture.html) and
[`decode::GstreamerPipeStream`](../api/decode/gstreamer_pipe/struct.GstreamerPipeStream.html)
— a dropped pipe without an explicit kill keeps the gst-launch
process decoding into the void.

## PTS

PTS is computed from `frame_index / framerate` via
[`MediaTime::from_frame`](../api/media/clock/struct.MediaTime.html#method.from_frame).
At 30 fps, frame 90's PTS is exactly 3.0 s — no rounding drift
across long captures (the rounding in `MediaTime`'s sample/frame
helpers ensures the integer round-trip stays tight).

## Integration tests

3 tests in `crates/media/tests/gstreamer_video_integration.rs`,
all skip-guarded via
[`media::gstreamer::is_available`](../api/media/gstreamer/fn.is_available.html):

| Test | Asserts |
|---|---|
| `emits_frames_with_expected_dimensions_and_pts` | 5 frames, correct dimensions, byte length, frame index, contiguous PTS. |
| `frames_have_distinct_content_smpte_colorbars` | Frame 0 vs frame 16 differ in many bytes (animated SMPTE ball moves). |
| `dimensions_and_framerate_round_trip_through_capture` | `dimensions()` + `framerate()` accessors match construction args. |

## Done when

- [x] Emits BGRA `VideoFrame`s with correct dimensions.
- [x] Frame PTS increments at the expected frame rate.
- [x] Frame dimensions are stable across the capture session.
- [x] Integration test skips if GStreamer is unavailable.
- [x] `Drop` kills the child + waits.
- [x] mdBook chapter (this page).
- [x] `just gate` green.
