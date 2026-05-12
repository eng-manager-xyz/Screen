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
