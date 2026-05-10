//! Registry of all stories. Each chunk that introduces a renderable feature
//! adds one entry here.

mod s_blur_filter;
mod s_clip_rounded;
mod s_color_matrix;
mod s_dim_outside;
mod s_drop_shadow;
mod s_graphics_ellipse;
mod s_graphics_gradients;
mod s_graphics_rounded;
mod s_hello_quad;
mod s_mesh_perspective;
mod s_motion_blur;
mod s_privacy_blur_rect;
mod s_privacy_blur_rounded;
mod s_privacy_blur_strength;
mod s_solid_redaction;
mod s_spotlight;
mod s_sprite_batcher;
mod s_text;
mod s_transform_nesting;
mod s_webcam_shapes;

use crate::story::Story;

/// Build the full story list. Order = display order within a category.
#[must_use]
pub fn all_stories() -> Vec<Story> {
    vec![
        s_hello_quad::story(),
        s_sprite_batcher::story(),
        s_transform_nesting::story(),
        s_text::story(),
        s_graphics_rounded::story(),
        s_graphics_ellipse::story(),
        s_graphics_gradients::story(),
        s_blur_filter::story(),
        s_drop_shadow::story(),
        s_motion_blur::story(),
        s_color_matrix::story(),
        s_mesh_perspective::story(),
        s_clip_rounded::story(),
        s_privacy_blur_rect::story(),
        s_privacy_blur_rounded::story(),
        s_privacy_blur_strength::story(),
        s_solid_redaction::story(),
        s_spotlight::story(),
        s_dim_outside::story(),
        s_webcam_shapes::story(),
    ]
}
