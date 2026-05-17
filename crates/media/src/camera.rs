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
    /// gst-launch source-element tokens that pin capture to *this*
    /// physical device — extracted from `gst-device-monitor-1.0`'s
    /// per-device hint line (e.g. `"avfvideosrc device-index=0"`).
    /// `None` when the parser couldn't find a hint line; callers
    /// should fall back to `autovideosrc` in that case. Used by
    /// [`super::gstreamer_video::GstreamerVideoCapture::from_camera`]
    /// to actually route to the picked camera (M-CAM.4).
    #[serde(default)]
    pub gst_source: Option<String>,
}

/// Enumerate every camera the OS exposes via `gst-device-monitor-1.0`.
///
/// Returns an empty `Vec` (not an error) if the host has no cameras
/// or the binary isn't on `PATH` — matches the M-CAM.0 probe
/// convention. Integration tests should runtime-skip when the
/// returned slice is empty.
#[must_use]
pub fn list_cameras() -> Vec<CameraDevice> {
    let path_env = std::env::var("PATH").unwrap_or_else(|_| "<unset>".to_owned());
    let output = Command::new("gst-device-monitor-1.0")
        .args(["Video/Source"])
        .stdout(Stdio::piped())
        // Capture stderr so a permission-denied / no-camera /
        // missing-binary failure isn't silent. Logged via `tracing`
        // below if non-empty — the M-CAM.0/1 lift uncovered that
        // GUI-launched binaries on macOS sometimes have a sanitised
        // PATH and we couldn't tell from a silent empty Vec.
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
                    "list_cameras: gst-device-monitor exited 0 but parser found 0 cameras"
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
                    "list_cameras: gst-device-monitor returned cameras"
                );
            }
            devices
        }
        Ok(out) => {
            // Non-zero exit — surface what gst said and which PATH
            // we used so the failure mode is debuggable.
            tracing::warn!(
                status = ?out.status,
                stderr = %String::from_utf8_lossy(&out.stderr),
                %path_env,
                "list_cameras: gst-device-monitor exited non-zero"
            );
            Vec::new()
        }
        Err(err) => {
            // Spawn itself failed — almost always "binary not on
            // PATH". Log PATH so the user can see what was searched.
            tracing::warn!(
                ?err,
                %path_env,
                "list_cameras: failed to spawn gst-device-monitor-1.0 \
                 (probably missing from PATH for the launched binary)"
            );
            Vec::new()
        }
    }
}

/// Pure-Rust parser for `gst-device-monitor-1.0 Video/Source` text
/// output. Split out from [`list_cameras`] so the parser is testable
/// against captured fixtures without needing gst installed.
///
/// Captures (per device block): the `name : ...` line as `label`, and
/// the `gst-launch-1.0 <src-element> [<props>] ! ...` example line as
/// [`CameraDevice::gst_source`] (verbatim source-element tokens).
#[must_use]
pub fn parse_device_monitor_output(text: &str) -> Vec<CameraDevice> {
    let mut devices = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_source: Option<String> = None;
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with("Device found:") {
            // New device block — emit the previous one if pending.
            if let Some(label) = current_name.take() {
                devices.push(make_device(label, current_source.take(), devices.is_empty()));
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
            continue;
        }
        // Per-device hint line: `gst-launch-1.0 <src> [<props>] ! ...`.
        // Extract everything between `gst-launch-1.0 ` and ` ! `; if
        // there's no ` ! ` (one-element pipelines hint), take the rest
        // of the line. M-CAM.4 uses this verbatim as the routing
        // source so `device-index=N` actually pins capture.
        if let Some(rest) = line.strip_prefix("gst-launch-1.0 ")
            && current_source.is_none()
        {
            let source = rest.split(" ! ").next().unwrap_or(rest).trim();
            if !source.is_empty() {
                current_source = Some(source.to_string());
            }
        }
    }
    if let Some(label) = current_name.take() {
        devices.push(make_device(label, current_source.take(), devices.is_empty()));
    }
    devices
}

fn make_device(label: String, gst_source: Option<String>, is_first: bool) -> CameraDevice {
    let id = stable_id_for(&label);
    CameraDevice {
        id,
        label,
        is_default: is_first,
        gst_source,
    }
}

/// Locate the [`CameraDevice`] whose stable id matches `id` by
/// re-probing the OS via [`list_cameras`]. Used by
/// [`super::gstreamer_video::GstreamerVideoCapture::from_camera`] to
/// resolve the picker's camera id back to its OS-native source
/// element on every recording start (M-CAM.4). Returns `None` when
/// the camera has been unplugged since enumeration.
#[must_use]
pub fn find_by_id(id: &str) -> Option<CameraDevice> {
    list_cameras().into_iter().find(|d| d.id == id)
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
    fn parser_extracts_macos_gst_source_with_device_index() {
        // M-CAM.4 — the `gst-launch-1.0 avfvideosrc device-index=0 ! ...`
        // hint line is what routes capture to *this* camera.
        let cams = parse_device_monitor_output(MACOS_SINGLE_CAM);
        assert_eq!(
            cams[0].gst_source.as_deref(),
            Some("avfvideosrc device-index=0")
        );
    }

    #[test]
    fn parser_extracts_per_device_gst_source_for_each_block() {
        // Synthetic two-cam output where each device has a distinct
        // gst-launch hint. Confirms the per-device state machine
        // emits a fresh `gst_source` per `Device found:` block (vs.
        // accidentally reusing the first device's source for both).
        let two_cams_with_hints = "Device found:

\tname  : FaceTime HD Camera
\tclass : Video/Source
\tgst-launch-1.0 avfvideosrc device-index=0 ! ...

Device found:

\tname  : External USB Cam
\tclass : Video/Source
\tgst-launch-1.0 avfvideosrc device-index=1 ! ...
";
        let cams = parse_device_monitor_output(two_cams_with_hints);
        assert_eq!(cams.len(), 2);
        assert_eq!(
            cams[0].gst_source.as_deref(),
            Some("avfvideosrc device-index=0")
        );
        assert_eq!(
            cams[1].gst_source.as_deref(),
            Some("avfvideosrc device-index=1")
        );
    }

    #[test]
    fn parser_emits_none_gst_source_when_hint_absent() {
        // The legacy TWO_CAMS fixture has no `gst-launch-1.0` line —
        // the parser should report `None` so `from_camera` falls back
        // to `autovideosrc` rather than building a malformed pipeline.
        let cams = parse_device_monitor_output(TWO_CAMS);
        assert!(cams.iter().all(|d| d.gst_source.is_none()));
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
            gst_source: Some("avfvideosrc device-index=2".into()),
        };
        let json = serde_json::to_string(&cam).unwrap();
        let parsed: CameraDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, cam);
    }

    #[test]
    fn camera_device_serde_back_compat_when_gst_source_absent() {
        // Pre-M-CAM.4 persisted records (or external JSON fixtures)
        // won't carry `gst_source`. The `#[serde(default)]` on the
        // field must let those deserialize cleanly — otherwise we'd
        // silently break any external consumer.
        let legacy_json = r#"{"id":"cam-feedface","label":"Test Cam","is_default":true}"#;
        let parsed: CameraDevice = serde_json::from_str(legacy_json).unwrap();
        assert_eq!(parsed.id, "cam-feedface");
        assert!(parsed.gst_source.is_none());
    }
}
