//! Registry of all stories. Each chunk that introduces a renderable feature
//! adds one entry here.

mod s_graphics_ellipse;
mod s_graphics_gradients;
mod s_graphics_rounded;
mod s_hello_quad;
mod s_sprite_batcher;
mod s_text;
mod s_transform_nesting;

use crate::story::Story;

/// Build the full story list. Order = display order within a category.
pub fn all_stories() -> Vec<Story> {
    vec![
        s_hello_quad::story(),
        s_sprite_batcher::story(),
        s_transform_nesting::story(),
        s_text::story(),
        s_graphics_rounded::story(),
        s_graphics_ellipse::story(),
        s_graphics_gradients::story(),
    ]
}
