//! Project persistence — the `.screenproj` file format (ED.23 / M-EDIT).
//!
//! An [`EditProject`] is "never cut the negative" all the way down: it holds
//! only value types — segment ranges, zoom windows, framing config, a source
//! reference — so saving a project is just serializing that decision list to
//! JSON, and reopening it is parsing the JSON back. No media is copied; the
//! `.screenproj` sits beside the recording and points at it. Because the
//! whole model is `serde`, save↔load is a pure, lossless round-trip — proven
//! here without touching the filesystem.
//!
//! The file carries a [`SCHEMA_VERSION`](crate::project::SCHEMA_VERSION) so a
//! future format change can migrate rather than mis-parse.

use crate::project::{EditProject, SCHEMA_VERSION};

/// File extension for a saved editor project.
pub const SCREENPROJ_EXTENSION: &str = "screenproj";

/// Serialize a project to its `.screenproj` JSON (pretty-printed so the file
/// is human-readable + diff-friendly).
///
/// # Errors
///
/// Returns a message if serialization fails (not expected for valid data).
pub fn to_screenproj(project: &EditProject) -> Result<String, String> {
    serde_json::to_string_pretty(project).map_err(|e| format!("serialize project: {e}"))
}

/// Parse a project from its `.screenproj` JSON.
///
/// # Errors
///
/// Returns a message if the JSON is malformed or doesn't match the schema.
pub fn from_screenproj(json: &str) -> Result<EditProject, String> {
    let project: EditProject =
        serde_json::from_str(json).map_err(|e| format!("parse project: {e}"))?;
    if project.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported .screenproj schema version {} (this build reads {SCHEMA_VERSION})",
            project.schema_version
        ));
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::ClipRef;
    use crate::ops::EditOp;
    use crate::project::SCHEMA_VERSION;
    use crate::zoom::{ZoomId, ZoomSegment};
    use std::path::PathBuf;

    #[test]
    fn round_trips_an_edited_project() {
        let mut p = EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/rec.mp4"),
            1920,
            1080,
            30,
            900,
        ));
        // Non-trivial edits so the round-trip exercises real state, not just
        // the default shell: a split, a speed change, and a zoom.
        p.apply(&EditOp::Split { at: 300 }).unwrap();
        p.apply(&EditOp::SetSpeed {
            index: 0,
            timescale: 2.0,
        })
        .unwrap();
        p.apply(&EditOp::AddZoom {
            zoom: ZoomSegment::manual(ZoomId(0), 100, 200, 1.6),
        })
        .unwrap();

        let json = to_screenproj(&p).expect("serialize");
        let back = from_screenproj(&json).expect("deserialize");
        assert_eq!(back, p, "project round-trips identically");
        assert_eq!(back.schema_version, SCHEMA_VERSION);
    }

    #[test]
    fn malformed_json_errors() {
        assert!(from_screenproj("not valid json {").is_err());
        // Well-formed JSON of the wrong shape (an array, not the project
        // object) is rejected too.
        assert!(from_screenproj("[1, 2, 3]").is_err());
    }

    #[test]
    fn screenproj_is_pretty_and_versioned() {
        let p = EditProject::from_recording(ClipRef::new(
            PathBuf::from("/tmp/a.mp4"),
            640,
            480,
            30,
            100,
        ));
        let json = to_screenproj(&p).unwrap();
        assert!(json.contains('\n'), "pretty-printed (multi-line)");
        assert!(json.contains("schema_version"));
    }

    #[test]
    fn rejects_a_future_schema_version() {
        let p =
            EditProject::from_recording(ClipRef::new(PathBuf::from("/tmp/a.mp4"), 64, 64, 30, 10));
        let json = to_screenproj(&p).unwrap();
        assert!(from_screenproj(&json).is_ok(), "current version loads");
        // A newer on-disk version is refused (explicit error, not mis-parse).
        let bumped = json.replace("\"schema_version\": 1", "\"schema_version\": 999");
        assert!(from_screenproj(&bumped).is_err(), "future version rejected");
    }
}
