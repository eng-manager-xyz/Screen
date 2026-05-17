//! Screen-capture smoke (M-SCK.0 / AUT-267).
//!
//! Opens a `ScreenCaptureStream` on the primary display at default
//! 1920×1080 / 30 fps for 2 seconds, then reports the cumulative
//! frame count. Acceptance criterion: frame counter should be
//! ≥ 30 after a 1-second observation window once permission is
//! granted.
//!
//! Triggers the macOS Screen Recording TCC prompt on first run.
//! After grant the user must **relaunch the app** for the new TCC
//! entry to take effect — same well-known platform quirk as the
//! rest of SCK.
//!
//! ```bash
//! cargo run -p media --example screen_capture_smoke
//! ```

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "screen_capture_smoke is macOS-only — ScreenCaptureKit is not \
         available on this platform. Skipping."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    use media::sck_video::{ScreenCaptureConfig, ScreenCaptureStream};
    use std::time::{Duration, Instant};

    println!("screen_capture_smoke: opening SCK screen capture (1920×1080 @ 30 fps)");
    let started = Instant::now();
    let stream = match ScreenCaptureStream::new(ScreenCaptureConfig::default()) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!();
            eprintln!("most likely cause: macOS Screen Recording permission is not granted.");
            eprintln!("grant it under System Settings → Privacy & Security → Screen Recording,");
            eprintln!("then QUIT AND RELAUNCH the binary (macOS requires a relaunch for the");
            eprintln!("new TCC entry to take effect — well-known platform quirk).");
            std::process::exit(1);
        }
    };
    println!("stream up after {:?}", started.elapsed());

    let counters = stream.counters();
    let observe_secs = 2;
    println!("observing for {observe_secs}s...");
    let observation_start = Instant::now();
    let baseline = counters.frames_received();
    std::thread::sleep(Duration::from_secs(observe_secs));
    let after = counters.frames_received();
    let delta = after.saturating_sub(baseline);
    let elapsed = observation_start.elapsed().as_secs_f64();
    #[allow(
        clippy::cast_precision_loss,
        reason = "frame counts well below 2^53; precision is fine for a human-readable summary"
    )]
    let fps = delta as f64 / elapsed;
    println!();
    println!(
        "summary: {after} frames cumulative; {delta} frames in {observe_secs}s ≈ {fps:.1} fps"
    );
    if delta == 0 {
        eprintln!("WARNING: no frames received. permission may be denied — see error above");
    } else {
        println!("(SCK screen-capture path verified — frames are arriving in Rust)");
    }
    drop(stream);
}
