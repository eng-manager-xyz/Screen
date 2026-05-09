//! `Story` trait + helper types.

use wisp::Stage;
use wisp::application::Application;

/// One interactive demo registered in the storybook.
///
/// Each story is responsible for building a fresh `Stage` when its scene is
/// requested. The optional `tick` hook lets a story animate per-frame (called
/// before each render).
pub struct Story {
    /// Stable identifier — reserved for future URL-style navigation and
    /// cross-session story bookmarking.
    #[allow(
        dead_code,
        reason = "reserved for upcoming URL navigation + bookmark state"
    )]
    pub id: &'static str,
    /// Category — groups stories in the picker (e.g. "Renderer Foundation",
    /// "Scene Graph", "Graphics", "Filters").
    pub category: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Milestone tag (e.g. "M0.5").
    pub milestone: &'static str,
    /// Markdown-flavored write-up shown in the right sidebar.
    pub writeup: &'static str,
    /// Build the scene from scratch. Called when the story is first selected.
    pub build: fn(&Application, &mut Stage),
    /// Optional per-frame animation hook. `t` is seconds since story start.
    pub tick: Option<fn(&mut Stage, f32)>,
}

impl Story {
    /// Resolve the per-frame hook, treating `None` as a no-op.
    pub fn tick(&self, stage: &mut Stage, t: f32) {
        if let Some(f) = self.tick {
            f(stage, t);
        }
    }
}
