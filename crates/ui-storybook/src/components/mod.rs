//! UI components — Leptos `#[component]` definitions matching rust-ui's
//! visual aesthetic (dark zinc palette, subtle borders, soft shadows).
//!
//! Convention: every component re-exported from here must have a story in
//! [`crate::stories`]. The story drives the SSR snapshot test.

// Leptos `#[component]` rewrites the function into a builder-pattern struct
// plus a wrapper fn; clippy's `must_use_candidate` and `needless_pass_by_value`
// fire on the rewritten code regardless of attribute placement on the source
// fn. Allowing them at the module level keeps every component clean without
// per-fn pragma noise.
#![allow(
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    reason = "Leptos `#[component]` macro rewrites these patterns; lints fire on generated code"
)]
// UI components are documented via mdBook stories under `_docs/book/src/ui/`,
// not via rustdoc — every visible variant has a screenshot + use case there.
// `missing_docs` would otherwise fire on macro-generated builder structs we
// don't author directly.
#![allow(
    missing_docs,
    reason = "components documented in mdBook stories; rustdoc gate would fire on macro-generated builder structs"
)]

pub mod button;
pub mod card;
pub mod dope_sheet;
pub mod drop_zone;
pub mod player_controls;
pub mod recording_toolbar;
pub mod status_bar;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardBody, CardHeader};
pub use dope_sheet::{DopeSheet, DopeSheetKeyframe, DopeSheetTrack, KeyframeKind, TrackKind};
pub use drop_zone::{DropZone, DropZoneState};
pub use player_controls::{PlayState, PlayerControls};
pub use recording_toolbar::{RecordingState, RecordingToolbar};
pub use status_bar::{StatusBar, StatusKind};
