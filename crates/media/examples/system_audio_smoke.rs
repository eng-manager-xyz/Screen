//! System-audio capture smoke (M-AUDIO-SYS.0 / AUT-280).
//!
//! Captures one second of audio off the macOS speakers via
//! `ScreenCaptureKit`, prints peak + RMS so the user can hear-verify
//! by playing a `YouTube` video in another window during the capture.
//!
//! Acceptance criterion for M-AUDIO-SYS.0: this example prints
//! non-zero RMS when something is playing through the speakers
//! (silence is OK if nothing is playing — RMS ≈ 0 in that case).
//!
//! Triggers the macOS Screen Recording permission prompt on first
//! run (covered by `NSScreenCaptureUsageDescription` in
//! `crates/app/Info.plist`). After grant, the user must relaunch
//! the binary — same well-known macOS quirk as the screen-video
//! path.
//!
//! ```bash
//! cargo run -p media --example system_audio_smoke
//! ```

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "system_audio_smoke is macOS-only — ScreenCaptureKit's audio API \
         is not available on this platform. Skipping."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    use media::sck_audio::{SystemAudioConfig, SystemAudioStream};
    use std::time::Instant;

    println!("system_audio_smoke: requesting SCK system-audio capture");
    println!("  (tip: play a YouTube tab or any audio source in another window before running)");
    let started = Instant::now();
    let mut stream = match SystemAudioStream::new(SystemAudioConfig::default()) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!();
            eprintln!("most likely cause: macOS Screen Recording permission is not granted.");
            eprintln!("grant it under System Settings → Privacy & Security → Screen Recording,");
            eprintln!("then quit and re-launch this binary (macOS requires a relaunch for the");
            eprintln!("new TCC entry to take effect — well-known platform quirk).");
            std::process::exit(1);
        }
    };
    let cfg = stream.config();
    println!(
        "stream up after {:?}; sample_rate={} Hz channels={}",
        started.elapsed(),
        cfg.sample_rate_hz,
        cfg.channels,
    );

    // Pull 1 second of audio in 100 ms chunks. Printing the chunk's
    // peak + RMS gives the user a per-100 ms loudness trace.
    let chunk_frames = u64::from(cfg.sample_rate_hz) / 10;
    let mut total_chunks = 0_u32;
    let mut total_peak = 0.0_f32;
    let mut total_rms_sq = 0.0_f64;
    for chunk_idx in 0..10 {
        match stream.next_chunk(chunk_frames) {
            Ok(chunk) => {
                let peak = chunk.peak();
                let rms = chunk.rms();
                println!(
                    "chunk {chunk_idx}: pts={:>5.2}s frames={} peak={peak:.4} rms={rms:.4}",
                    chunk.pts().as_seconds(),
                    chunk.frame_count(),
                );
                total_peak = total_peak.max(peak);
                total_rms_sq += f64::from(rms).powi(2);
                total_chunks += 1;
            }
            Err(err) => {
                eprintln!("chunk {chunk_idx} failed: {err}");
                break;
            }
        }
    }

    if total_chunks == 0 {
        eprintln!("no chunks captured");
        std::process::exit(1);
    }
    #[allow(
        clippy::cast_possible_truncation,
        reason = "10-chunk sum then sqrt fits in f32; precision is fine for a human-readable summary"
    )]
    let avg_rms = (total_rms_sq / f64::from(total_chunks)).sqrt() as f32;
    println!();
    println!("summary: {total_chunks} chunks; peak={total_peak:.4} avg_rms={avg_rms:.4}");
    if avg_rms < 1e-4 {
        println!("(silence — try playing audio in another window before re-running)");
    } else {
        println!("(audio was captured; non-zero RMS confirms the SCK system-audio path)");
    }
}
