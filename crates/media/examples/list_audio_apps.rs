//! List every running app SCK can see (M-AUDIO-SYS.1 / AUT-281).
//!
//! Triggers the macOS Screen Recording permission prompt on first
//! run. Granting it once covers every `ScreenCaptureKit` path
//! (system audio, per-app audio, screen video).
//!
//! ```bash
//! cargo run -p media --example list_audio_apps
//! ```
//!
//! Per the acceptance criteria for AUT-281: this prints every
//! currently-running audio-capable app with bundle id + display
//! name + PID. The bundle id is the canonical identifier used by
//! `AudioAppFilter::OnlyApps([..bundle_id..])` — if Spotify
//! crashes + restarts, the picker's persisted bundle-id selection
//! follows the new PID transparently.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "list_audio_apps is macOS-only — ScreenCaptureKit is not \
         available on this platform. Skipping."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    match media::sck_audio::list_audio_apps() {
        Ok(apps) if apps.is_empty() => {
            eprintln!(
                "no running apps surfaced by SCK — unusual; check Screen Recording permission"
            );
        }
        Ok(apps) => {
            println!("found {} app(s):", apps.len());
            for app in &apps {
                let icon = if app.icon_png_bytes.is_empty() {
                    "(no icon)"
                } else {
                    "(icon present)"
                };
                println!(
                    "  pid={:>6} bundle={:<40} name={} {icon}",
                    app.pid, app.bundle_id, app.display_name,
                );
            }
        }
        Err(err) => {
            eprintln!("error: {err}");
            eprintln!();
            eprintln!("most likely cause: macOS Screen Recording permission is not granted.");
            eprintln!("grant it under System Settings → Privacy & Security → Screen Recording,");
            eprintln!("then quit and re-launch this binary.");
            std::process::exit(1);
        }
    }
}
