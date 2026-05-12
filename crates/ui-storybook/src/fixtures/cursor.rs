//! Cursor Studio fixtures — named cursor styles. Filled in across UI-20.

/// One cursor style the user could select in Cursor Studio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorStyleFixture {
    /// Stable id.
    pub id: &'static str,
    /// Display label.
    pub label: &'static str,
    /// `true` when this is the active style.
    pub selected: bool,
}

/// Sample cursor styles.
#[must_use]
pub fn sample_cursor_styles() -> Vec<CursorStyleFixture> {
    vec![
        CursorStyleFixture {
            id: "cursor-system",
            label: "System default",
            selected: true,
        },
        CursorStyleFixture {
            id: "cursor-soft",
            label: "Soft ring",
            selected: false,
        },
        CursorStyleFixture {
            id: "cursor-arrow",
            label: "Bold arrow",
            selected: false,
        },
        CursorStyleFixture {
            id: "cursor-spotlight",
            label: "Spotlight",
            selected: false,
        },
    ]
}

/// Sample cursor style picker (UI-20).
#[must_use]
pub fn sample_cursor_style_picker(
    selected: crate::components::cursor::CursorStyle,
) -> crate::components::cursor::CursorStylePickerView {
    use crate::components::cursor::{CursorStyle, CursorStylePickerView, CursorStyleTileView};
    let styles = [
        CursorStyle::System,
        CursorStyle::Arrow,
        CursorStyle::Soft,
        CursorStyle::Dot,
        CursorStyle::Ring,
        CursorStyle::Reticle,
        CursorStyle::Tactile,
        CursorStyle::Hide,
    ];
    let tiles = styles
        .iter()
        .map(|s| CursorStyleTileView {
            style: *s,
            label: None,
            selected: *s == selected,
            disabled: matches!(*s, CursorStyle::Tactile),
        })
        .collect();
    CursorStylePickerView { tiles }
}

/// Variant where every tile is disabled.
#[must_use]
pub fn sample_cursor_style_picker_disabled() -> crate::components::cursor::CursorStylePickerView {
    use crate::components::cursor::CursorStyle;
    let mut v = sample_cursor_style_picker(CursorStyle::Arrow);
    for t in &mut v.tiles {
        t.disabled = true;
    }
    v
}

/// Sample shell view (UI-20).
#[must_use]
pub fn sample_cursor_studio_shell() -> crate::components::cursor::CursorStudioShellView {
    use crate::components::cursor::{CursorStudioShellView, CursorStyle};
    CursorStudioShellView {
        picker: sample_cursor_style_picker(CursorStyle::Arrow),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_styles_non_empty_with_one_selected() {
        let styles = sample_cursor_styles();
        assert!(!styles.is_empty());
        assert_eq!(styles.iter().filter(|s| s.selected).count(), 1);
    }
}
