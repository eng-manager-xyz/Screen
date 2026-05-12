//! Cursor Studio stories — UI-20 (`CursorStudioShell` + style picker) /
//! UI-21 (`CursorPreviewCanvas` + appearance controls).

use leptos::prelude::*;

use crate::components::cursor::{
    CursorAppearancePanel, CursorPreviewBackend, CursorPreviewCanvas, CursorStudioShell,
    CursorStyle, CursorStylePicker,
};
use crate::fixtures::cursor::{
    sample_cursor_appearance, sample_cursor_appearance_pulse, sample_cursor_appearance_spotlight,
    sample_cursor_appearance_trail_on, sample_cursor_studio_shell, sample_cursor_style_picker,
    sample_cursor_style_picker_disabled,
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
        s(
            "cursor-preview-arrow-ring",
            "Cursor preview — arrow with ring",
            fixed(360, 240),
            render_preview_light,
        ),
        s(
            "cursor-preview-dark-bg",
            "Cursor preview — dark background",
            fixed(360, 240),
            render_preview_dark,
        ),
        s(
            "cursor-appearance-panel-default",
            "Cursor appearance — default",
            fixed(320, 480),
            render_appearance_default,
        ),
        s(
            "cursor-appearance-panel-pulse",
            "Cursor appearance — Pulse effect",
            fixed(320, 480),
            render_appearance_pulse,
        ),
        s(
            "cursor-appearance-panel-spotlight",
            "Cursor appearance — Spotlight effect",
            fixed(320, 480),
            render_appearance_spotlight,
        ),
        s(
            "cursor-appearance-panel-trail-on",
            "Cursor appearance — trail on",
            fixed(320, 480),
            render_appearance_trail,
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

fn render_preview_light() -> String {
    render(view! {
        <CursorPreviewCanvas backend=CursorPreviewBackend::CssFallback />
    })
}

fn render_preview_dark() -> String {
    render(view! {
        <CursorPreviewCanvas
            backend=CursorPreviewBackend::CssFallback
            card_background="linear-gradient(135deg,#0f172a,#1e293b)"
        />
    })
}

fn render_appearance_default() -> String {
    render(view! { <CursorAppearancePanel view=sample_cursor_appearance() /> })
}

fn render_appearance_pulse() -> String {
    render(view! { <CursorAppearancePanel view=sample_cursor_appearance_pulse() /> })
}

fn render_appearance_spotlight() -> String {
    render(view! { <CursorAppearancePanel view=sample_cursor_appearance_spotlight() /> })
}

fn render_appearance_trail() -> String {
    render(view! { <CursorAppearancePanel view=sample_cursor_appearance_trail_on() /> })
}
