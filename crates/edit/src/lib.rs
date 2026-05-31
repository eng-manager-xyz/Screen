//! `edit` — the pure, serializable non-destructive edit model that
//! powers the Screen recorder's editor (milestone M-EDIT).
//!
//! # The segment-list design
//!
//! An [`EditProject`] is the serialized source of truth for one
//! editing session. Following the design proven by Cap's
//! `project::configuration`, the edit is encoded as **lists of
//! lightweight value types**, not a mutated copy of the media:
//!
//! - [`TimelineSegment`]s — an ordered list of slices of the source
//!   recording. **Trim** adjusts a slice's `source_start` /
//!   `source_end`; **split** replaces one slice with two adjacent
//!   ones; **speed** changes a slice's `timescale`. The project's
//!   video is the ordered concatenation of its segments.
//! - [`ZoomSegment`]s — cinematic punch-in regions on the project
//!   timeline, each a zoom `amount` at an [`ZoomMode`] target. At the
//!   render boundary (ED.16) each compiles to a keyframed transform.
//! - [`BackgroundConfig`] / [`CursorConfig`] / [`CropRect`] /
//!   [`AspectRatio`] — the "cinematic framing" layer (wallpaper,
//!   padding, corner radius, shadow, cursor smoothing, crop/reframe).
//!
//! Nothing here touches a GPU or a media pipeline. The renderer
//! (`wisp`) and the encoder (`media` / `GStreamer`) consume this model at
//! preview + export time, re-deriving every frame from the lists. That
//! keeps editing non-destructive (the source file is never rewritten)
//! and keeps this crate exhaustively unit-testable on any machine.
//!
//! # Time model
//!
//! Project time is **frame-indexed** at a fixed [`EditProject::project_fps`].
//! [`EditProject::source_time`] walks the segment list applying each
//! segment's `timescale` to map a project frame to the source frame the
//! renderer should decode. See [`segment`] for the arithmetic.

pub mod clip;
pub mod history;
pub mod ops;
pub mod persist;
pub mod project;
pub mod segment;
pub mod style;
pub mod telemetry;
pub mod zoom;
pub mod zoom_anim;

pub use clip::ClipRef;
pub use history::History;
pub use ops::{EditError, EditOp, TrimEdge};
pub use project::{DEFAULT_PROJECT_FPS, EditProject, SCHEMA_VERSION};
pub use segment::{Frame, TimelineSegment};
pub use style::{
    AspectRatio, AutoZoomConfig, BackgroundConfig, BackgroundSource, CropRect, CursorConfig,
};
pub use telemetry::{ClickEvent, CursorSample};
pub use zoom::{EditEase, ZoomId, ZoomMode, ZoomSegment};
