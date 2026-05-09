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

pub mod button;
pub mod card;
pub mod dope_sheet;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardBody, CardHeader};
pub use dope_sheet::{DopeSheet, DopeSheetKeyframe, DopeSheetTrack, KeyframeKind, TrackKind};
