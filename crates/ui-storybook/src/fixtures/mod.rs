//! Fixture data for storybook stories.
//!
//! Stories must not hand-roll their own mock devices / workspaces /
//! recordings inline. Define each fixture once here so multiple stories
//! share a single source of truth — when UI-08 / UI-14 / UI-19 land and
//! refine these structs, every consuming story sees the updated shape
//! without touching its own body.
//!
//! Fixtures return **owned** values (not signals) so callers can
//! freely clone or mutate without leaking app state into presentational
//! components.

pub mod cursor;
pub mod devices;
pub mod editor;
pub mod library;
pub mod recorder;
pub mod shell;
pub mod workspaces;
