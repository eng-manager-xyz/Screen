//! `wisp-storybook` — interactive feature gallery, exposed as both a binary
//! (the eframe app) and a library (so integration tests can iterate over
//! every shipped story and verify it renders without errors).

pub mod app;
pub mod stories;
pub mod story;
