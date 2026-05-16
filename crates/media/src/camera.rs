//! Camera device enumeration (M-CAM.1 / AUT-255) — CLI-pipe pattern.
//!
//! Spawns `gst-device-monitor-1.0 Video/Source` and parses its
//! human-readable text output into a [`Vec<CameraDevice>`]. Preserves
//! the project's CLI-pipe-over-`gstreamer-rs` convention (CLAUDE.md:
//! "Upgrading to `gstreamer-rs` Rust bindings is a later chunk").
//!
//! ```admonish note title="Option 1 from the ticket decision"
//! The ticket spec offered three enumeration backends: gst CLI
//! subprocess (chosen), platform-native (`AVCaptureDevice` /
//! `IMFActivate` / `udev`), or the `gstreamer-rs` `DeviceMonitor`.
//! Option 1 minimises new surface area — no new Rust deps, no
//! per-OS code paths. The cost is parsing a loosely-specified text
//! output, mitigated by the fixture-driven parser tests below.
//! ```

use std::process::{Command, Stdio};

use serde::{Deserialize, Serialize};

/// One attached camera device with a stable ID, a human-readable
/// label, and a flag indicating whether the OS treats it as the
/// default.
///
/// `id` is derived from the device name via FNV-1a hashing so the
/// same camera produces the same ID across reboots even when the
/// OS's underlying device-id string is non-stable (macOS
/// AVFoundation has a history of doing this).
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct CameraDevice {
    /// Stable identifier — used by M-CAM.4's "last-used camera"
    /// persistence and by M-CAM.2's `start_preview(camera_id)` IPC.
    pub id: String,
    /// Human-readable label (e.g. `"FaceTime HD Camera"`).
    pub label: String,
    /// `true` for the first device the OS lists. There's no canonical
    /// "default camera" concept in the gst output, so we fall back to
    /// "first in the list" — which generally matches the macOS /
    /// Windows default-device selection.
    pub is_default: bool,
}

/// Enumerate every camera the OS exposes via `gst-device-monitor-1.0`.
///
/// Returns an empty `Vec` (not an error) if the host has no cameras
/// or the binary isn't on `PATH` — matches the M-CAM.0 probe
/// convention. Integration tests should runtime-skip when the
/// returned slice is empty.
#[must_use]
pub fn list_cameras() -> Vec<CameraDevice> {
    let output = Command::new("gst-device-monitor-1.0")
        .args(["Video/Source"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let stdout = match output {
        Ok(out) if out.status.success() => out.stdout,
        _ => return Vec::new(),
    };
    let text = String::from_utf8_lossy(&stdout);
    parse_device_monitor_output(&text)
}

/// Pure-Rust parser for `gst-device-monitor-1.0 Video/Source` text
/// output. Split out from [`list_cameras`] so the parser is testable
/// against captured fixtures without needing gst installed.
#[must_use]
pub fn parse_device_monitor_output(text: &str) -> Vec<CameraDevice> {
    let mut devices = Vec::new();
    let mut current_name: Option<String> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with("Device found:") {
            // New device block — emit the previous one if pending.
            if let Some(label) = current_name.take() {
                devices.push(make_device(label, devices.is_empty()));
            }
            continue;
        }
        // The `name :` line carries the human-readable label. gst
        // formats this with variable whitespace + colon padding.
        if let Some(rest) = line.strip_prefix("name") {
            let value = rest.trim_start_matches([' ', '\t', ':']);
            if !value.is_empty() && current_name.is_none() {
                current_name = Some(value.to_string());
            }
        }
    }
    if let Some(label) = current_name.take() {
        devices.push(make_device(label, devices.is_empty()));
    }
    devices
}

fn make_device(label: String, is_first: bool) -> CameraDevice {
    let id = stable_id_for(&label);
    CameraDevice {
        id,
        label,
        is_default: is_first,
    }
}

/// Derive a stable ID for a camera from its human-readable label
/// using FNV-1a. Deterministic, dependency-free.
#[must_use]
pub fn stable_id_for(label: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in label.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("cam-{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Real-world `gst-device-monitor-1.0 Video/Source` output
    /// captured from a macOS dev box with one built-in camera.
    const MACOS_SINGLE_CAM: &str = "Probing devices...

Device found:

\tname  : FaceTime HD Camera
\tclass : Video/Source
\tcaps  : video/x-raw, format=(string)NV12, width=(int)1280, height=(int)720, framerate=(fraction)30/1
\tproperties:
\t\tdevice.api = avfvideosrc
\tgst-launch-1.0 avfvideosrc device-index=0 ! ...
";

    /// Synthetic output for the two-camera case used to verify the
    /// `is_default` flag goes to the first listed device only.
    const TWO_CAMS: &str = "Device found:

\tname  : FaceTime HD Camera
\tclass : Video/Source

Device found:

\tname  : External USB Cam
\tclass : Video/Source
";

    #[test]
    fn parser_extracts_single_camera() {
        let cams = parse_device_monitor_output(MACOS_SINGLE_CAM);
        assert_eq!(cams.len(), 1);
        assert_eq!(cams[0].label, "FaceTime HD Camera");
        assert!(cams[0].is_default);
    }

    #[test]
    fn parser_extracts_multiple_cameras_with_default_first() {
        let cams = parse_device_monitor_output(TWO_CAMS);
        assert_eq!(cams.len(), 2);
        assert_eq!(cams[0].label, "FaceTime HD Camera");
        assert!(cams[0].is_default);
        assert_eq!(cams[1].label, "External USB Cam");
        assert!(!cams[1].is_default);
    }

    #[test]
    fn parser_returns_empty_for_empty_input() {
        assert!(parse_device_monitor_output("").is_empty());
        assert!(parse_device_monitor_output("Probing devices...").is_empty());
    }

    #[test]
    fn stable_id_is_deterministic_per_label() {
        assert_eq!(
            stable_id_for("FaceTime HD Camera"),
            stable_id_for("FaceTime HD Camera")
        );
        // Different labels → different IDs.
        assert_ne!(
            stable_id_for("FaceTime HD Camera"),
            stable_id_for("External USB Cam")
        );
    }

    #[test]
    fn stable_id_prefix_is_cam() {
        assert!(stable_id_for("any").starts_with("cam-"));
    }

    #[test]
    fn camera_device_serde_round_trip() {
        let cam = CameraDevice {
            id: "cam-feedface".into(),
            label: "Test Cam".into(),
            is_default: true,
        };
        let json = serde_json::to_string(&cam).unwrap();
        let parsed: CameraDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cam);
    }
}
