//! Cursor Studio stories — UI-20 (`CursorStudioShell` + style picker) /
//! UI-21 (`CursorPreviewCanvas` + appearance controls).

use leptos::prelude::*;

use crate::components::cursor::{CursorStudioShell, CursorStyle, CursorStylePicker};
use crate::fixtures::cursor::{
    sample_cursor_studio_shell, sample_cursor_style_picker, sample_cursor_style_picker_disabled,
};

use super::{Story, StoryViewport, render};

const PICKER_W: u16 = 720;
const PICKER_H: u16 = 140;
const SHELL_W: u16 = 960;
const SHELL_H: u16 = 600;

fn fixed(w: u16, h: u16) -> StoryViewport {
    StoryViewport::Fixed {
        width: w,
        height: h,
    }
}

fn s(
    id: &'static str,
    title: &'static str,
    viewport: StoryViewport,
    render: fn() -> String,
) -> Story {
    Story {
        id,
        category: "Cursor",
        title,
        viewport,
        render,
    }
}

/// All cursor-surface stories.
#[must_use]
pub fn stories() -> Vec<Story> {
    vec![
        s(
            "cursor-style-picker-default",
            "Cursor style picker — default",
            fixed(PICKER_W, PICKER_H),
            render_picker_default,
        ),
        s(
            "cursor-style-picker-arrow-selected",
            "Cursor style picker — Arrow selected",
            fixed(PICKER_W, PICKER_H),
            render_picker_arrow,
        ),
        s(
            "cursor-style-picker-disabled",
            "Cursor style picker — all disabled",
            fixed(PICKER_W, PICKER_H),
            render_picker_disabled,
        ),
        s(
            "cursor-studio-shell",
            "Cursor Studio shell",
            fixed(SHELL_W, SHELL_H),
            render_shell,
        ),
    ]
}

fn render_picker_default() -> String {
    render(view! { <CursorStylePicker view=sample_cursor_style_picker(CursorStyle::System) /> })
}

fn render_picker_arrow() -> String {
    render(view! { <CursorStylePicker view=sample_cursor_style_picker(CursorStyle::Arrow) /> })
}

fn render_picker_disabled() -> String {
    render(view! { <CursorStylePicker view=sample_cursor_style_picker_disabled() /> })
}

fn render_shell() -> String {
    render(view! { <CursorStudioShell view=sample_cursor_studio_shell() /> })
}
