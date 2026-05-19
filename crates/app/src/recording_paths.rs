//! Default output path resolution + "Reveal in Finder/Explorer/Files"
//! plumbing for M-EXPORT.4 (M-RECORD-EXPORT).
//!
//! Pure-Rust path math + a `std::process::Command` wrapper for the
//! per-OS file-manager reveal. No Tauri-plugin-dialog dep — the
//! recorder's v0 UX is "auto-pick a sensible default location"
//! rather than "always show a save dialog." A future M-EXPORT.4.1
//! follow-up can add `tauri-plugin-dialog` for explicit Save-As.

#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Howard Hinnant civil-from-days algorithm operates in i64 intermediate values + casts back to u32 / u64 at known-safe range boundaries. The algorithm is exact for any positive Unix timestamp through year 9999 (way past any realistic recorder use)."
)]

use std::path::{Path, PathBuf};

use media::encode::OutputFormat;

/// Default output directory for new recordings. macOS uses
/// `~/Movies/Screen/`; Windows + Linux use `~/Videos/Screen/`.
/// Honors the `SCREEN_RECORDER_OUTPUT_DIR` env var for tests and
/// power users.
#[must_use]
pub fn default_output_dir() -> PathBuf {
    if let Some(override_dir) = std::env::var_os("SCREEN_RECORDER_OUTPUT_DIR") {
        return PathBuf::from(override_dir);
    }
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from);
    if cfg!(target_os = "macos") {
        home.join("Movies").join("Screen")
    } else {
        home.join("Videos").join("Screen")
    }
}

/// Compose a default output filename for a session that started at
/// `started_at` (Unix epoch seconds). Format
/// `Screen-YYYY-MM-DD-HHMMSS.<ext>` — sortable, unambiguous, file-
/// system-safe on every OS.
#[must_use]
pub fn default_filename(started_at_unix_secs: u64, format: OutputFormat) -> String {
    let (year, month, day, hour, minute, second) = unix_to_ymdhms(started_at_unix_secs);
    format!(
        "Screen-{year:04}-{month:02}-{day:02}-{hour:02}{minute:02}{second:02}.{ext}",
        ext = format.extension(),
    )
}

/// Compose the full default output path. The directory is created
/// on first use by the encoder; this just returns the path.
#[must_use]
pub fn default_output_path(started_at_unix_secs: u64, format: OutputFormat) -> PathBuf {
    default_output_dir().join(default_filename(started_at_unix_secs, format))
}

/// Ensure the parent directory of `path` exists, creating it
/// recursively if needed. Used by the encoder before opening the
/// scratch files.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] when directory creation
/// fails (permission denied, read-only filesystem, etc.).
pub fn ensure_parent_dir(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// Spawn the per-OS file-manager "reveal this file" command.
/// macOS uses `open -R <path>`, Windows uses
/// `explorer /select,<path>`, Linux uses `xdg-open <parent-dir>`
/// (most distros don't have a portable "select" verb).
///
/// # Errors
///
/// Returns an error string if the spawn itself failed (binary not on
/// PATH). Non-zero exit from the spawned command is treated as
/// success because the file manager may legitimately background the
/// window and return immediately.
pub fn reveal_in_file_manager(path: &Path) -> Result<(), String> {
    let cmd_result = if cfg!(target_os = "macos") {
        std::process::Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
    } else if cfg!(target_os = "windows") {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
    } else {
        let dir = path.parent().unwrap_or(Path::new("."));
        std::process::Command::new("xdg-open").arg(dir).spawn()
    };
    cmd_result
        .map(|_child| ())
        .map_err(|err| format!("failed to spawn file-manager command: {err}"))
}

/// Convert Unix epoch seconds to (year, month, day, hour, minute,
/// second) via a small pure-Rust algorithm. Avoids pulling in
/// `chrono` for this single-purpose use.
///
/// Accurate for any positive timestamp through year 9999. Returns
/// `(1970, 1, 1, 0, 0, 0)` for negative inputs (impossible in
/// practice since session timestamps are always now-ish).
fn unix_to_ymdhms(unix_secs: u64) -> (u32, u32, u32, u32, u32, u32) {
    let second = (unix_secs % 60) as u32;
    let minutes_total = unix_secs / 60;
    let minute = (minutes_total % 60) as u32;
    let hours_total = minutes_total / 60;
    let hour = (hours_total % 24) as u32;
    let days_since_epoch = hours_total / 24;

    // Civil-from-days algorithm (Howard Hinnant) — exact for the
    // proleptic Gregorian calendar.
    let z = days_since_epoch as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y } as u32;

    (year, m as u32, d as u32, hour, minute, second)
}

#[cfg(test)]
#[allow(
    unsafe_code,
    reason = "Tests need to set/unset SCREEN_RECORDER_OUTPUT_DIR to exercise both env-var-honored and HOME-default branches. The wider `unsafe_code = warn` lint is workspace-wide; tests scope the safety justification."
)]
mod tests {
    use super::*;

    #[test]
    fn default_output_dir_uses_home() {
        // Save + clear the override so we exercise the HOME path.
        let saved_override = std::env::var_os("SCREEN_RECORDER_OUTPUT_DIR");
        // SAFETY: unit tests are single-threaded per nextest's
        // default; the env-var dance is fine here.
        unsafe {
            std::env::remove_var("SCREEN_RECORDER_OUTPUT_DIR");
        }
        let dir = default_output_dir();
        // Either Movies (macOS) or Videos (Win/Linux) subfolder.
        let s = dir.to_string_lossy();
        assert!(
            s.contains("Movies/Screen")
                || s.contains("Videos/Screen")
                || s.contains("Videos\\Screen"),
            "unexpected default dir: {s}"
        );
        // Restore.
        if let Some(val) = saved_override {
            // SAFETY: same justification as the remove above.
            unsafe {
                std::env::set_var("SCREEN_RECORDER_OUTPUT_DIR", val);
            }
        }
    }

    #[test]
    fn default_output_dir_honors_override_env_var() {
        // SAFETY: see above.
        unsafe {
            std::env::set_var("SCREEN_RECORDER_OUTPUT_DIR", "/tmp/m-export-override");
        }
        assert_eq!(
            default_output_dir(),
            PathBuf::from("/tmp/m-export-override")
        );
        // SAFETY: see above.
        unsafe {
            std::env::remove_var("SCREEN_RECORDER_OUTPUT_DIR");
        }
    }

    #[test]
    fn default_filename_uses_mp4_extension_for_h264() {
        // 2026-05-17 18:00:00 UTC = 1763402400
        let name = default_filename(1_763_402_400, OutputFormat::Mp4H264Aac);
        assert_eq!(name, "Screen-2025-11-17-180000.mp4");
        // (Note: the "2025" is correct — 1763402400 is November 2025,
        // not May 2026. The unix timestamp doesn't lie even if the
        // user's wall clock does.)
    }

    #[test]
    fn default_filename_uses_webm_extension_for_vp9() {
        let name = default_filename(1_763_402_400, OutputFormat::WebmVp9Opus);
        assert!(
            std::path::Path::new(&name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("webm"))
        );
    }

    #[test]
    fn default_filename_pads_single_digits() {
        // 1970-01-01 00:01:02 = 62 seconds.
        let name = default_filename(62, OutputFormat::Mp4H264Aac);
        assert_eq!(name, "Screen-1970-01-01-000102.mp4");
    }

    #[test]
    fn default_filename_handles_unix_epoch() {
        let name = default_filename(0, OutputFormat::Mp4H264Aac);
        assert_eq!(name, "Screen-1970-01-01-000000.mp4");
    }

    #[test]
    fn default_output_path_joins_dir_and_filename() {
        // SAFETY: single-threaded test env.
        unsafe {
            std::env::set_var("SCREEN_RECORDER_OUTPUT_DIR", "/tmp/screen-test");
        }
        let p = default_output_path(0, OutputFormat::Mp4H264Aac);
        assert_eq!(
            p,
            PathBuf::from("/tmp/screen-test/Screen-1970-01-01-000000.mp4")
        );
        // SAFETY: see above.
        unsafe {
            std::env::remove_var("SCREEN_RECORDER_OUTPUT_DIR");
        }
    }

    #[test]
    fn ensure_parent_dir_creates_nested_path() {
        let test_dir = std::env::temp_dir().join("m-export-ensure-parent-test");
        let _ = std::fs::remove_dir_all(&test_dir);
        let nested = test_dir.join("sub").join("deeper").join("file.mp4");
        ensure_parent_dir(&nested).expect("create_dir_all");
        assert!(nested.parent().unwrap().exists());
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn ensure_parent_dir_is_idempotent() {
        let test_dir = std::env::temp_dir().join("m-export-ensure-parent-idempotent");
        let _ = std::fs::remove_dir_all(&test_dir);
        let target = test_dir.join("file.mp4");
        ensure_parent_dir(&target).expect("first call");
        ensure_parent_dir(&target).expect("second call");
        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn unix_to_ymdhms_reference_points() {
        // 0 → Unix epoch.
        assert_eq!(unix_to_ymdhms(0), (1970, 1, 1, 0, 0, 0));
        // 86400 → next day.
        assert_eq!(unix_to_ymdhms(86_400), (1970, 1, 2, 0, 0, 0));
        // 1700000000 ≈ 2023-11-14 22:13:20 UTC (Howard Hinnant's
        // formula is exact; spot-check against a known value).
        assert_eq!(unix_to_ymdhms(1_700_000_000), (2023, 11, 14, 22, 13, 20));
    }

    #[test]
    fn unix_to_ymdhms_handles_leap_year_feb_29() {
        // 2024-02-29 00:00:00 UTC = 1709164800
        let (y, m, d, h, _, _) = unix_to_ymdhms(1_709_164_800);
        assert_eq!((y, m, d, h), (2024, 2, 29, 0));
    }
}
