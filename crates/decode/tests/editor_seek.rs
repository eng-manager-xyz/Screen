//! Integration test for [`EditorVideoStream`] (ED.3) — random-access seek.
//!
//! Reads back the committed `tests/fixtures/sample.mp4` (480×270 @ 30 fps,
//! 8 frames). All byte-level assertions compare frames decoded by **one**
//! stream (the cache returns decoded bytes verbatim, which is fully
//! deterministic) — we deliberately do NOT byte-compare two independent
//! `gst` decode processes, which is not reliably reproducible under load
//! (CLAUDE.md: prefer tolerant / within-run comparisons for decoded
//! media). The re-decode (re-spawn) path is verified by frame index +
//! dimensions instead. Skips gracefully when the `GStreamer` CLI tools
//! aren't on `PATH`.

use std::path::Path;
use std::time::Duration;

use decode::EditorVideoStream;
use media::gstreamer::is_available as gstreamer_available;

const FIXTURE: &str = "tests/fixtures/sample.mp4";

/// Forward-decode the whole clip through the seekable API, returning each
/// frame's bytes by index. Stops at the clamped end (where `frame_index`
/// no longer advances). All frames come from one decode pass, so byte
/// comparisons against this reference are deterministic.
fn decode_all(stream: &mut EditorVideoStream) -> Vec<Vec<u8>> {
    let mut frames = Vec::new();
    let mut i = 0u64;
    while let Some(frame) = stream.frame(i) {
        // Past the end, the clamp returns the last frame with a stale index.
        if frame.frame_index != i {
            break;
        }
        frames.push(frame.bgra);
        i += 1;
        if i > 10_000 {
            break; // safety net — the fixture is tiny
        }
    }
    frames
}

#[test]
fn cached_seek_returns_decoded_bytes_verbatim() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping cached_seek_returns_decoded_bytes_verbatim");
        return;
    }
    let mut stream = EditorVideoStream::open(Path::new(FIXTURE)).expect("open");
    let reference = decode_all(&mut stream);
    assert!(
        reference.len() >= 4,
        "fixture should decode several frames, got {}",
        reference.len()
    );
    let spawns = stream.spawn_count();
    let n = reference.len();

    // Seek backward, out of order — every frame is cached, so each seek
    // returns the exact bytes that were decoded at that index, and no
    // re-spawn happens.
    for &i in &[n - 1, 0, n / 2, 1] {
        let idx = u64::try_from(i).unwrap();
        let frame = stream.frame(idx).expect("cached frame");
        assert_eq!(frame.frame_index, idx);
        assert_eq!(
            &frame.bgra, &reference[i],
            "cached seek to {i} must return the decoded bytes verbatim"
        );
    }
    assert_eq!(
        stream.spawn_count(),
        spawns,
        "cached backward seeks must not re-spawn the pipe"
    );
}

#[test]
fn backward_seek_redecode_lands_on_correct_index() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping backward_seek_redecode_lands_on_correct_index");
        return;
    }
    // A 1-frame cache forces the backward seek to re-decode rather than
    // serve from cache.
    let mut stream = EditorVideoStream::open_with_cache(Path::new(FIXTURE), 1).expect("open");
    let bytes_per_frame = (stream.width() as usize) * (stream.height() as usize) * 4;

    let f5 = stream.frame(5).expect("decode forward to 5");
    assert_eq!(f5.frame_index, 5);
    assert_eq!(f5.bgra.len(), bytes_per_frame);

    let spawns = stream.spawn_count();
    // Frame 1 was evicted from the 1-slot cache and is before the pipe's
    // position, so this re-spawns and re-decodes from 0.
    let f1 = stream.frame(1).expect("re-decode to 1");
    assert_eq!(f1.frame_index, 1, "re-decode lands on the requested index");
    assert_eq!(f1.bgra.len(), bytes_per_frame);
    assert!(
        stream.spawn_count() > spawns,
        "a backward seek past the cache re-spawns the pipe"
    );
}

#[test]
fn cache_hit_avoids_respawn() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping cache_hit_avoids_respawn");
        return;
    }
    let mut stream = EditorVideoStream::open(Path::new(FIXTURE)).expect("open");
    let _ = stream.frame(3).expect("frame 3");
    let spawns = stream.spawn_count();
    assert!(
        spawns >= 1,
        "first decode should have spawned the pipe once"
    );
    // Re-request frame 3 (exact) and frame 2 (earlier, but cached) — both
    // are served from cache, so no re-spawn happens.
    let _ = stream.frame(3).expect("frame 3 again");
    let _ = stream.frame(2).expect("frame 2 cached");
    assert_eq!(
        stream.spawn_count(),
        spawns,
        "cached frames must not trigger a re-spawn"
    );
}

#[test]
fn out_of_range_seek_clamps_to_last() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping out_of_range_seek_clamps_to_last");
        return;
    }
    let mut stream = EditorVideoStream::open(Path::new(FIXTURE)).expect("open");
    let reference = decode_all(&mut stream);
    let last_idx = reference.len() - 1;
    // Seek way past the end — clamps to the last frame, served from cache.
    let frame = stream.frame(99_999).expect("clamped frame");
    assert_eq!(frame.frame_index, u64::try_from(last_idx).unwrap());
    assert_eq!(&frame.bgra, &reference[last_idx]);
}

#[test]
fn seek_to_time_maps_to_frame() {
    if !gstreamer_available() {
        eprintln!("gstreamer not on PATH — skipping seek_to_time_maps_to_frame");
        return;
    }
    let mut stream = EditorVideoStream::open(Path::new(FIXTURE)).expect("open");
    // 100 ms at 30 fps → frame 3.
    let frame = stream
        .seek_to_time(Duration::from_millis(100))
        .expect("frame at 100ms");
    assert_eq!(frame.frame_index, 3);
}
