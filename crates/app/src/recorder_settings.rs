//! Persistent recorder preferences (M-SAVE.0).
//!
//! Two pieces of cross-session state the recorder remembers:
//!
//! - **`output_dir`** — the directory new recordings export into.
//!   `None` means "use the per-OS default"
//!   ([`recording_paths::default_output_dir`](crate::recording_paths::default_output_dir)).
//! - **`last_format`** — the export format slug the user last chose in
//!   the Save panel (`"mp4-h264"` / `"webm-vp9"` / …). `None` means
//!   "use the default format". Restored as the dropdown's initial
//!   value so the user doesn't re-pick every recording.
//!
//! Stored as JSON at `<app-config-dir>/recorder-settings.json`. We use
//! `serde_json` here (unlike the bubble-position file, which hand-rolls
//! a two-integer text format) because a filesystem path can contain
//! the `:` / `,` / newline characters that an ad-hoc `key:value` format
//! would choke on — JSON escapes them correctly. Both fields are
//! `#[serde(default)]` + `Option`, so an older or partially-written
//! file still deserializes (missing keys fall back to `None`); unknown
//! keys from a future version are ignored. No format-version field is
//! needed.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::Manager;

/// File name under the app-config dir. See module docs.
const SETTINGS_FILE: &str = "recorder-settings.json";

/// Cross-session recorder preferences. See module docs for the
/// per-field semantics.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecorderSettings {
    /// User-chosen export directory. `None` → per-OS default.
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
    /// Last-used export format slug. `None` → default format.
    #[serde(default)]
    pub last_format: Option<String>,
}

/// Parse settings from the JSON at `path`. Returns
/// [`RecorderSettings::default`] (no overrides) on any failure —
/// missing file (first launch), unreadable file, or malformed JSON.
/// A corrupt settings file degrades to defaults rather than blocking
/// the recorder.
#[must_use]
pub fn load_from(path: &Path) -> RecorderSettings {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return RecorderSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

/// Serialize `settings` to the JSON at `path`, creating the parent
/// directory if it doesn't exist yet (first-ever save).
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] when the directory can't
/// be created or the file can't be written. JSON serialization of two
/// `Option` fields can't realistically fail, but a serialize error is
/// also surfaced as an `io::Error` of kind `InvalidData`.
pub fn save_to(path: &Path, settings: &RecorderSettings) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(settings)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(path, json)
}

/// Resolve the settings-file path under Tauri's app-config dir.
/// `None` if the platform path resolver fails (extremely rare —
/// missing `$HOME` with no fallback).
fn settings_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    Some(app.path().app_config_dir().ok()?.join(SETTINGS_FILE))
}

/// Load the persisted settings for this app handle. Defaults on any
/// failure (see [`load_from`]).
#[must_use]
pub fn load(app: &tauri::AppHandle) -> RecorderSettings {
    settings_path(app)
        .map(|p| load_from(&p))
        .unwrap_or_default()
}

/// Persist `settings` for this app handle.
///
/// # Errors
///
/// Returns an error string when the app-config dir is unavailable or
/// the write fails (see [`save_to`]).
pub fn save(app: &tauri::AppHandle, settings: &RecorderSettings) -> Result<(), String> {
    let path = settings_path(app).ok_or("app config dir unavailable")?;
    save_to(&path, settings).map_err(|err| format!("failed to write recorder settings: {err}"))
}

/// The directory new recordings should export into: the persisted
/// override if set, otherwise the per-OS default. This is the single
/// resolver every output-path computation should call so the chosen
/// directory is honored everywhere.
#[must_use]
pub fn resolved_output_dir(app: &tauri::AppHandle) -> PathBuf {
    load(app)
        .output_dir
        .unwrap_or_else(crate::recording_paths::default_output_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("m-save-settings-{tag}.json"))
    }

    #[test]
    fn load_from_missing_file_is_default() {
        let path = temp_path("missing-never-created");
        let _ = std::fs::remove_file(&path);
        assert_eq!(load_from(&path), RecorderSettings::default());
    }

    #[test]
    fn load_from_malformed_json_is_default() {
        let path = temp_path("malformed");
        std::fs::write(&path, "{ this is not json").expect("write");
        assert_eq!(load_from(&path), RecorderSettings::default());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_then_load_round_trips() {
        let path = temp_path("round-trip");
        let _ = std::fs::remove_file(&path);
        let settings = RecorderSettings {
            output_dir: Some(PathBuf::from("/Users/x/Desktop/My Recordings")),
            last_format: Some("webm-vp9".to_owned()),
        };
        save_to(&path, &settings).expect("save");
        assert_eq!(load_from(&path), settings);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn save_creates_parent_dir() {
        let dir = std::env::temp_dir().join("m-save-settings-nested-parent");
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join("sub").join(SETTINGS_FILE);
        save_to(&path, &RecorderSettings::default()).expect("save into nested dir");
        assert!(path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_with_special_chars_survives_round_trip() {
        // A path with a comma + colon would break a naive key:value
        // text format; JSON escapes it. This is why the module uses
        // serde_json rather than the bubble-position text scheme.
        let path = temp_path("special-chars");
        let _ = std::fs::remove_file(&path);
        let settings = RecorderSettings {
            output_dir: Some(PathBuf::from("/tmp/odd, name: with chars")),
            last_format: None,
        };
        save_to(&path, &settings).expect("save");
        assert_eq!(load_from(&path), settings);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn partial_json_fills_missing_field_with_none() {
        // A file written by an older build that only knew `output_dir`
        // must still parse, leaving `last_format` as None.
        let path = temp_path("partial");
        std::fs::write(&path, r#"{"output_dir":"/tmp/only-dir"}"#).expect("write");
        let loaded = load_from(&path);
        assert_eq!(loaded.output_dir, Some(PathBuf::from("/tmp/only-dir")));
        assert_eq!(loaded.last_format, None);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn default_has_no_overrides() {
        let d = RecorderSettings::default();
        assert!(d.output_dir.is_none());
        assert!(d.last_format.is_none());
    }
}
