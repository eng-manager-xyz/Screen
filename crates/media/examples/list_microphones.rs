//! Enumerate every microphone the OS exposes and print a one-line
//! summary per device.
//!
//! Acceptance criterion for M-MIC.0 (AUT-277): running this prints
//! every attached input with id + label + default flag + channels +
//! sample rate.
//!
//! ```bash
//! cargo run -p media --example list_microphones
//! ```

fn main() {
    let mics = media::list_microphones();
    if mics.is_empty() {
        eprintln!(
            "no microphones found — either none are attached or \
             `gst-device-monitor-1.0` is not on PATH \
             (try `brew install gstreamer`)"
        );
        return;
    }
    println!("found {} microphone(s):", mics.len());
    for mic in &mics {
        let default_tag = if mic.is_default { " (default)" } else { "" };
        println!(
            "  - {label}{default_tag}\n    id: {id}\n    channels: {channels}\n    sample_rate: {rate} Hz",
            label = mic.label,
            id = mic.id,
            channels = mic.channels,
            rate = mic.sample_rate_hz,
        );
    }
}
