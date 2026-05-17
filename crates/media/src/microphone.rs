//! Microphone device enumeration (M-MIC.0 / AUT-277) — CLI-pipe pattern.
//!
//! Spawns `gst-device-monitor-1.0 Audio/Source` and parses its
//! human-readable text output into a [`Vec<MicrophoneDevice>`].
//! Direct sister of [`crate::camera`] (M-CAM.1 / AUT-255).
//!
//! ```admonish note title="Differences from `camera`"
//! Two interesting deltas from the camera enumerator:
//!
//! - `gst-device-monitor-1.0 Audio/Source` exposes a real
//!   `is-default = true|false` line in the `properties:` block on
//!   macOS. That's authoritative — we use it instead of falling back
//!   to "first device listed." When *no* device carries
//!   `is-default = true` (e.g. the property is absent on some
//!   Linux backends) we degrade to the first-listed heuristic so
//!   the picker still has a reasonable preselection.
//! - The first `caps` line carries `rate=` and `channels=` for the
//!   device's preferred native format. Those are parsed into
//!   [`MicrophoneDevice::sample_rate_hz`] and
//!   [`MicrophoneDevice::channels`] respectively. Either field
//!   degrades to `0` ("unknown") if the parser can't find it —
//!   downstream code (M-MIC.1's capture pipeline) defaults to
//!   `48 kHz` / `2 channels` when the value is `0`.
//! ```

use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

/// One attached microphone with a stable ID, a human-readable label,
/// a default-device flag, and the native channel + sample-rate hint
/// reported by GStreamer.
///
/// `id` is derived from the device label via FNV-1a hashing so the
/// same mic produces the same ID across reboots even when the OS's
/// underlying device-id string is non-stable (macOS AVFoundation has
/// a history of doing this). Matches [`crate::camera::stable_id_for`].
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MicrophoneDevice {
    /// Stable identifier — used by M-MIC.2's "last-used mic"
    /// persistence and by M-MIC.1's `start_mic_capture(mic_id)` IPC.
    pub id: String,
    /// Human-readable label (e.g. `"MacBook Pro Microphone"`,
    /// `"Shure MV7"`, `"AirPods Pro"`).
    pub label: String,
    /// `true` when GStreamer flagged this device as the OS-level
    /// default. Falls back to "first device in the enumeration"
    /// when no device carries the `is-default = true` property.
    pub is_default: bool,
    /// Native channel count from the first reported `caps` line
    /// (1 = mono, 2 = stereo). `0` means the parser couldn't extract
    /// the value — downstream capture defaults to 2.
    pub channels: u8,
    /// Native sample rate in Hz (typically `48000` or `44100`).
    /// `0` means the parser couldn't extract the value — downstream
    /// capture defaults to 48000. GStreamer's `audioresample` will
    /// convert as needed for the encoder.
    pub sample_rate_hz: u32,
}

/// Enumerate every microphone the OS exposes via
/// `gst-device-monitor-1.0`.
///
/// Returns an empty `Vec` (not an error) if the host has no
/// microphones or the binary isn't on `PATH` — matches the
/// [`crate::camera::list_cameras`] convention. Integration tests
/// should runtime-skip when the returned slice is empty.
#[must_use]
pub fn list_microphones() -> Vec<MicrophoneDevice> {
    let path_env = std::env::var("PATH").unwrap_or_else(|_| "<unset>".to_owned());
    let output = Command::new("gst-device-monitor-1.0")
        .args(["Audio/Source"])
        .stdout(Stdio::piped())
        // Mirror camera.rs: capture stderr so a permission-denied /
        // no-mic / missing-binary failure isn't silent. GUI-launched
        // binaries on macOS sometimes have a sanitised PATH and we
        // couldn't otherwise tell why the Vec came back empty.
        .stderr(Stdio::piped())
        .output();
    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            let devices = parse_device_monitor_output(&text);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if devices.is_empty() {
                tracing::warn!(
                    stdout_bytes = out.stdout.len(),
                    stderr_bytes = out.stderr.len(),
                    %path_env,
                    "list_microphones: gst-device-monitor exited 0 but parser found 0 mics"
                );
                if !text.is_empty() {
                    tracing::warn!(stdout = %text, "raw gst-device-monitor stdout");
                }
                if !stderr.is_empty() {
                    tracing::warn!(stderr = %stderr, "raw gst-device-monitor stderr");
                }
            } else {
                tracing::info!(
                    count = devices.len(),
                    labels = ?devices.iter().map(|d| &d.label).collect::<Vec<_>>(),
                    "list_microphones: gst-device-monitor returned mics"
                );
            }
            devices
        }
        Ok(out) => {
            tracing::warn!(
                status = ?out.status,
                stderr = %String::from_utf8_lossy(&out.stderr),
                %path_env,
                "list_microphones: gst-device-monitor exited non-zero"
            );
            Vec::new()
        }
        Err(err) => {
            tracing::warn!(
                ?err,
                %path_env,
                "list_microphones: failed to spawn gst-device-monitor-1.0 \
                 (probably missing from PATH for the launched binary)"
            );
            Vec::new()
        }
    }
}

/// Pure-Rust parser for `gst-device-monitor-1.0 Audio/Source` text
/// output. Split out from [`list_microphones`] so the parser is
/// testable against captured fixtures without needing gst installed.
///
/// Resolution rule for [`MicrophoneDevice::is_default`]:
/// 1. If any device's `properties:` block contains
///    `is-default = true`, that device alone wins the flag.
/// 2. Otherwise the first-listed device is marked default — same
///    fallback shape as [`crate::camera::parse_device_monitor_output`].
#[must_use]
pub fn parse_device_monitor_output(text: &str) -> Vec<MicrophoneDevice> {
    let blocks = split_into_device_blocks(text);
    let mut parsed: Vec<ParsedDevice> = blocks
        .into_iter()
        .filter_map(|block| parse_one_device_block(&block))
        .collect();

    let any_explicit_default = parsed.iter().any(|d| d.explicit_default);
    if !any_explicit_default && let Some(first) = parsed.first_mut() {
        first.explicit_default = true;
    }

    parsed
        .into_iter()
        .map(|p| MicrophoneDevice {
            id: stable_id_for(&p.label),
            label: p.label,
            is_default: p.explicit_default,
            channels: p.channels,
            sample_rate_hz: p.sample_rate_hz,
        })
        .collect()
}

/// Derive a stable ID for a microphone from its human-readable label
/// using FNV-1a. Deterministic, dependency-free. Mirrors
/// [`crate::camera::stable_id_for`] but emits a `mic-` prefix so the
/// two ID spaces never collide.
#[must_use]
pub fn stable_id_for(label: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("mic-{hash:016x}")
}

/// Per-device intermediate parse result. `explicit_default` carries
/// only the `is-default = true` signal at first; the fallback
/// "first-listed" rule is applied in [`parse_device_monitor_output`]
/// after every block has been parsed.
struct ParsedDevice {
    label: String,
    explicit_default: bool,
    channels: u8,
    sample_rate_hz: u32,
}

/// Cut the gst output into one `String` per `Device found:` block.
/// Anything before the first `Device found:` line (the
/// `Probing devices...` banner) is discarded.
fn split_into_device_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for raw_line in text.lines() {
        if raw_line.trim().starts_with("Device found:") {
            if let Some(lines) = current.take() {
                blocks.push(lines.join("\n"));
            }
            current = Some(Vec::new());
            continue;
        }
        if let Some(lines) = current.as_mut() {
            lines.push(raw_line);
        }
    }
    if let Some(lines) = current.take() {
        blocks.push(lines.join("\n"));
    }
    blocks
}

fn parse_one_device_block(block: &str) -> Option<ParsedDevice> {
    let mut label: Option<String> = None;
    let mut explicit_default = false;
    let mut channels: u8 = 0;
    let mut sample_rate_hz: u32 = 0;
    let mut first_caps_line: Option<String> = None;

    for raw_line in block.lines() {
        let line = raw_line.trim();

        if let Some(rest) = line.strip_prefix("name") {
            let value = rest.trim_start_matches([' ', '\t', ':']).trim();
            if label.is_none() && !value.is_empty() {
                label = Some(value.to_string());
            }
            continue;
        }

        if let Some(rest) = line.strip_prefix("caps") {
            // First caps line wins — it's the device's preferred
            // native format. Subsequent lines list every supported
            // permutation and would muddy the picture.
            if first_caps_line.is_none() {
                let value = rest.trim_start_matches([' ', '\t', ':']).trim();
                if !value.is_empty() {
                    first_caps_line = Some(value.to_string());
                }
            }
            continue;
        }

        if let Some(value) = parse_property(line, "is-default")
            && value.eq_ignore_ascii_case("true")
        {
            explicit_default = true;
        }
    }

    if let Some(caps_line) = first_caps_line.as_deref() {
        if let Some(rate) = extract_caps_int_field(caps_line, "rate") {
            sample_rate_hz = u32::try_from(rate).unwrap_or(0);
        }
        if let Some(ch) = extract_caps_int_field(caps_line, "channels") {
            channels = u8::try_from(ch).unwrap_or(0);
        }
    }

    label.map(|label| ParsedDevice {
        label,
        explicit_default,
        channels,
        sample_rate_hz,
    })
}

/// Match a `properties:`-block line shaped like `key = value` and
/// return the trimmed value. gst pads with tabs and a single `=`.
fn parse_property<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.strip_prefix(key)?;
    let trimmed = rest.trim_start();
    let after_eq = trimmed.strip_prefix('=')?;
    Some(after_eq.trim())
}

/// Extract an `int`-valued field from a gst caps string. Caps fields
/// look like `rate=(int)48000` or `rate=48000`; both shapes appear in
/// the wild depending on the gst version. Continues past non-matching
/// tokens (`format=F32LE`, `layout=interleaved`, …) and stops as soon
/// as it finds the first match, so a trailing `channel-mask=0x…` for
/// the `channels` query never bleeds in (`strip_prefix("channels")`
/// fails on `channel-mask`).
fn extract_caps_int_field(caps: &str, field: &str) -> Option<u64> {
    for token in caps.split(',') {
        let trimmed = token.trim();
        let Some(after_key) = trimmed.strip_prefix(field) else {
            continue;
        };
        // Reject prefix-matches like `channel-mask` when searching for
        // `channels` — the next char must be either `=` or whitespace.
        let after_key_trimmed = after_key.trim_start();
        let Some(after_eq) = after_key_trimmed.strip_prefix('=') else {
            continue;
        };
        let value = after_eq.trim();
        // Strip optional `(int)` / `(string)` type annotation.
        let value = value.strip_prefix("(int)").unwrap_or(value).trim();
        // Stop at the first non-digit so trailing field-list cruft
        // doesn't bleed in.
        let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
        if !digits.is_empty() {
            return digits.parse::<u64>().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real macOS `gst-device-monitor-1.0 Audio/Source` output —
    /// virtual loopback + USB webcam mic + Bluetooth headset. The
    /// Bluetooth headset (`MOMENTUM 4`) is the OS default *despite*
    /// being the third listed, so this fixture exercises the
    /// `is-default = true` branch versus the first-listed fallback.
    const MACOS_THREE_MICS: &str = "Probing devices...


Device found:

\tname  : LoomAudioDevice
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=F32LE, layout=interleaved, rate=48000, channels=2, channel-mask=0x0000000000000003
\t        audio/x-raw, format={ (string)F64LE, (string)S16LE }, layout=interleaved, rate=48000, channels=2, channel-mask=0x0000000000000003
\tproperties:
\t\tis-default = false
\t\tunique-id = com.loom.desktop.audio-device.device
\tgst-launch-1.0 osxaudiosrc device=97 ! ...


Device found:

\tname  : Insta360 Link
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=F32LE, layout=interleaved, rate=48000, channels=1
\tproperties:
\t\tis-default = false
\t\tunique-id = AppleUSBAudioEngine:Insta360:Insta360 Link:100000:3
\tgst-launch-1.0 osxaudiosrc device=103 ! ...


Device found:

\tname  : MOMENTUM 4
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=F32LE, layout=interleaved, rate=16000, channels=1
\tproperties:
\t\tis-default = true
\t\tunique-id = 80-C3-BA-87-28-6D:input
\tgst-launch-1.0 osxaudiosrc device=113 ! ...
";

    /// Synthetic single-mic output — built-in mic, default flag set,
    /// stereo 48 kHz. Exercises the common-case happy path.
    const MACOS_BUILTIN_ONLY: &str = "Probing devices...


Device found:

\tname  : MacBook Pro Microphone
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=F32LE, layout=interleaved, rate=48000, channels=2
\tproperties:
\t\tis-default = true
\t\tunique-id = BuiltInMicrophoneDevice
\tgst-launch-1.0 osxaudiosrc device=42 ! ...
";

    /// Synthetic Linux/Pulse-shaped output that *omits* the
    /// `is-default` property entirely. Exercises the first-listed
    /// fallback.
    const PULSE_NO_DEFAULT_PROPERTY: &str = "Probing devices...


Device found:

\tname  : Built-in Audio Analog Stereo
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=(string)S16LE, layout=(string)interleaved, rate=(int)44100, channels=(int)2
\tproperties:
\t\tdevice.api = pulse
\tgst-launch-1.0 pulsesrc device=alsa_input.pci ! ...


Device found:

\tname  : USB Audio Device Mono
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=(string)S16LE, layout=(string)interleaved, rate=(int)48000, channels=(int)1
\tproperties:
\t\tdevice.api = pulse
\tgst-launch-1.0 pulsesrc device=alsa_input.usb ! ...
";

    #[test]
    fn parser_extracts_three_macos_mics_with_explicit_default() {
        let mics = parse_device_monitor_output(MACOS_THREE_MICS);
        assert_eq!(mics.len(), 3, "{mics:#?}");

        assert_eq!(mics[0].label, "LoomAudioDevice");
        assert!(!mics[0].is_default);
        assert_eq!(mics[0].channels, 2);
        assert_eq!(mics[0].sample_rate_hz, 48_000);

        assert_eq!(mics[1].label, "Insta360 Link");
        assert!(!mics[1].is_default);
        assert_eq!(mics[1].channels, 1);
        assert_eq!(mics[1].sample_rate_hz, 48_000);

        // MOMENTUM 4 is the BLUETOOTH device flagged is-default=true
        // even though it's listed third — proves we use the explicit
        // signal, not "first in list."
        assert_eq!(mics[2].label, "MOMENTUM 4");
        assert!(mics[2].is_default);
        assert_eq!(mics[2].channels, 1);
        assert_eq!(mics[2].sample_rate_hz, 16_000);
    }

    #[test]
    fn parser_extracts_builtin_only() {
        let mics = parse_device_monitor_output(MACOS_BUILTIN_ONLY);
        assert_eq!(mics.len(), 1);
        assert_eq!(mics[0].label, "MacBook Pro Microphone");
        assert!(mics[0].is_default);
        assert_eq!(mics[0].channels, 2);
        assert_eq!(mics[0].sample_rate_hz, 48_000);
    }

    #[test]
    fn parser_falls_back_to_first_listed_when_no_explicit_default() {
        let mics = parse_device_monitor_output(PULSE_NO_DEFAULT_PROPERTY);
        assert_eq!(mics.len(), 2);
        assert_eq!(mics[0].label, "Built-in Audio Analog Stereo");
        assert!(
            mics[0].is_default,
            "no device carries is-default=true, so first-listed wins"
        );
        assert_eq!(mics[0].channels, 2);
        assert_eq!(mics[0].sample_rate_hz, 44_100);

        assert_eq!(mics[1].label, "USB Audio Device Mono");
        assert!(!mics[1].is_default);
        assert_eq!(mics[1].channels, 1);
        assert_eq!(mics[1].sample_rate_hz, 48_000);
    }

    #[test]
    fn parser_returns_empty_for_no_inputs() {
        assert!(parse_device_monitor_output("").is_empty());
        assert!(parse_device_monitor_output("Probing devices...").is_empty());
        assert!(parse_device_monitor_output("Probing devices...\n\n").is_empty());
    }

    #[test]
    fn parser_handles_caps_without_rate_or_channels() {
        // Hypothetical loose-form caps line that omits both keys.
        // Should land channels=0, sample_rate_hz=0 (the documented
        // "unknown" sentinel) rather than crashing.
        let text = "Device found:

\tname  : Weird Mic
\tclass : Audio/Source
\tcaps  : audio/x-raw, format=F32LE
\tproperties:
\t\tis-default = true
";
        let mics = parse_device_monitor_output(text);
        assert_eq!(mics.len(), 1);
        assert_eq!(mics[0].label, "Weird Mic");
        assert!(mics[0].is_default);
        assert_eq!(mics[0].channels, 0);
        assert_eq!(mics[0].sample_rate_hz, 0);
    }

    #[test]
    fn extract_caps_int_field_handles_both_typed_and_untyped_int() {
        assert_eq!(
            extract_caps_int_field("rate=(int)48000, channels=(int)2", "rate"),
            Some(48_000)
        );
        assert_eq!(
            extract_caps_int_field("rate=48000, channels=2", "channels"),
            Some(2)
        );
        // channel-mask field shouldn't pollute channels.
        assert_eq!(
            extract_caps_int_field("channels=2, channel-mask=0x0000000000000003", "channels"),
            Some(2)
        );
        assert_eq!(extract_caps_int_field("format=F32LE", "rate"), None);
    }

    #[test]
    fn stable_id_is_deterministic_per_label() {
        assert_eq!(
            stable_id_for("MacBook Pro Microphone"),
            stable_id_for("MacBook Pro Microphone")
        );
        // Different labels → different IDs.
        assert_ne!(
            stable_id_for("MacBook Pro Microphone"),
            stable_id_for("Shure MV7")
        );
    }

    #[test]
    fn stable_id_prefix_is_mic_not_cam() {
        // M-MIC.0 explicitly uses a `mic-` prefix so a hypothetical
        // ID collision with a camera (same label, different kind)
        // can't happen at the IPC layer.
        assert!(stable_id_for("MacBook Pro Microphone").starts_with("mic-"));
        assert_ne!(
            stable_id_for("Some Device"),
            crate::camera::stable_id_for("Some Device")
        );
    }

    #[test]
    fn microphone_device_serde_round_trip() {
        let mic = MicrophoneDevice {
            id: "mic-feedface".into(),
            label: "Test Mic".into(),
            is_default: true,
            channels: 2,
            sample_rate_hz: 48_000,
        };
        let json = serde_json::to_string(&mic).unwrap();
        let parsed: MicrophoneDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, mic);
    }

    #[test]
    fn types_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MicrophoneDevice>();
    }
}
