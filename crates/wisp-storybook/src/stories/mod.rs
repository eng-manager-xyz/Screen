//! Registry of all stories. Each chunk that introduces a renderable feature
//! adds one entry here.

mod s_graphics_ellipse;
mod s_graphics_rounded;
mod s_sprite_batcher;

use crate::story::Story;

/// Build the full story list. Order = display order within a category.
pub fn all_stories() -> Vec<Story> {
    vec![
        s_sprite_batcher::story(),
        s_graphics_rounded::story(),
        s_graphics_ellipse::story(),
    ]
}
