//! Shell fixtures — nav rail items, workspace badge, user avatar
//! (M-UI.2 / AUT-122).

use crate::components::shell::{AppSection, NavItemView, UserAvatarView, WorkspaceBadgeView};

/// Default nav rail items for Screen Studio.
///
/// `extra_count` adds a notification badge to the Library item so the
/// `nav-rail-with-counts` story can demonstrate that variant without
/// owning the variant logic inside the story body.
#[must_use]
pub fn sample_nav_items(extra_count: bool) -> Vec<NavItemView> {
    vec![
        NavItemView {
            section: AppSection::Record,
            label: "Record",
            count: None,
            disabled: false,
        },
        NavItemView {
            section: AppSection::Library,
            label: "Library",
            count: if extra_count { Some(3) } else { None },
            disabled: false,
        },
        NavItemView {
            section: AppSection::Editor,
            label: "Editor",
            count: None,
            disabled: false,
        },
        NavItemView {
            section: AppSection::Cursor,
            label: "Cursor",
            count: None,
            disabled: false,
        },
        NavItemView {
            section: AppSection::Prefs,
            label: "Prefs",
            count: None,
            disabled: false,
        },
    ]
}

/// Sample workspace badge.
#[must_use]
pub fn sample_workspace_badge() -> WorkspaceBadgeView {
    WorkspaceBadgeView {
        id: "ws-personal",
        monogram: "PE",
        label: "Personal",
        accent: Some("#ef4444"),
    }
}

/// Sample user avatar (monogram fallback, no image).
#[must_use]
pub fn sample_user_avatar() -> UserAvatarView {
    UserAvatarView {
        monogram: "M",
        src: None,
        name: "Matthew Harwood",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nav_items_have_one_per_section() {
        let items = sample_nav_items(false);
        assert_eq!(items.len(), 5);
        let sections: Vec<_> = items.iter().map(|i| i.section).collect();
        for section in [
            AppSection::Record,
            AppSection::Library,
            AppSection::Editor,
            AppSection::Cursor,
            AppSection::Prefs,
        ] {
            assert!(sections.contains(&section), "missing section {section:?}");
        }
    }

    #[test]
    fn extra_count_adds_a_library_count_only() {
        let plain = sample_nav_items(false);
        let extra = sample_nav_items(true);
        assert!(plain.iter().all(|i| i.count.is_none()));
        let library_count = extra
            .iter()
            .find(|i| i.section == AppSection::Library)
            .and_then(|i| i.count);
        assert_eq!(library_count, Some(3));
    }
}
