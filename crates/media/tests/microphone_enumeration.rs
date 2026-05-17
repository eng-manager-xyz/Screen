//! Integration test for `media::microphone::list_microphones`
//! (M-MIC.0 / AUT-277).
//!
//! Runs `gst-device-monitor-1.0 Audio/Source` against the actual host.
//! Skips gracefully when `gst-launch-1.0` is not on `PATH` (which is
//! the same `gstreamer` brew formula that ships `gst-device-monitor`)
//! so machines without `GStreamer` don't fail the gate.

use media::gstreamer::is_available;
use media::list_microphones;

#[test]
fn lists_microphones_or_skips_when_gstreamer_absent() {
    if !is_available() {
        eprintln!("gst-launch-1.0 not on PATH — skipping list_microphones smoke");
        return;
    }
    let mics = list_microphones();

    // The host may legitimately have zero microphones — that's a
    // valid empty-Vec response, not an error. The contract under
    // test is "doesn't panic, doesn't error" and "when non-empty,
    // exactly one entry is the default."
    if mics.is_empty() {
        eprintln!("no microphones attached — empty Vec is the correct response");
        return;
    }

    let default_count = mics.iter().filter(|m| m.is_default).count();
    assert_eq!(
        default_count,
        1,
        "exactly one mic must carry is_default = true; got {default_count} of {} \
         (parser must fall back to first-listed when no explicit default)",
        mics.len()
    );

    for mic in &mics {
        assert!(!mic.id.is_empty(), "mic id must be non-empty: {mic:?}");
        assert!(
            mic.id.starts_with("mic-"),
            "mic id must use mic- prefix to avoid camera-id collisions: {mic:?}"
        );
        assert!(
            !mic.label.is_empty(),
            "mic label must be non-empty: {mic:?}"
        );
        // channels / sample_rate_hz may legitimately be 0 (the
        // "unknown" sentinel for hosts where the gst output omits
        // the value); no assertion there.
    }
}
