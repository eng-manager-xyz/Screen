//! Library fixtures — recordings list, storage meter values. Filled in
//! across UI-14 / UI-15.

/// One recording shown in the library grid.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordingFixture {
    /// Stable id (file uuid in production).
    pub id: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Captured-at label ("Captured 2026-05-09").
    pub captured_at: &'static str,
    /// Duration label ("1m 24s").
    pub duration: &'static str,
    /// Size label ("328 MB").
    pub size: &'static str,
    /// `true` when this card is currently selected.
    pub selected: bool,
}

/// Sample recordings for the library grid.
#[must_use]
pub fn sample_recordings() -> Vec<RecordingFixture> {
    vec![
        RecordingFixture {
            id: "rec-01",
            title: "Demo · auth login",
            captured_at: "Captured 2026-05-09",
            duration: "1m 24s",
            size: "328 MB",
            selected: true,
        },
        RecordingFixture {
            id: "rec-02",
            title: "Standup recap",
            captured_at: "Captured 2026-05-08",
            duration: "4m 02s",
            size: "1.1 GB",
            selected: false,
        },
        RecordingFixture {
            id: "rec-03",
            title: "Bug repro · payments",
            captured_at: "Captured 2026-05-07",
            duration: "0m 38s",
            size: "112 MB",
            selected: false,
        },
        RecordingFixture {
            id: "rec-04",
            title: "Onboarding tour",
            captured_at: "Captured 2026-05-06",
            duration: "2m 11s",
            size: "604 MB",
            selected: false,
        },
    ]
}

/// Storage meter fixture for the sidebar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StorageMeter {
    /// Bytes used.
    pub used_bytes: u64,
    /// Bytes available (the meter's max).
    pub total_bytes: u64,
}

impl StorageMeter {
    /// Fraction `used / total`, clamped to `[0, 1]`.
    #[must_use]
    pub fn fraction(self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            #[allow(
                clippy::cast_precision_loss,
                reason = "ratio fits f32 without meaningful loss"
            )]
            let used = self.used_bytes as f32;
            #[allow(
                clippy::cast_precision_loss,
                reason = "ratio fits f32 without meaningful loss"
            )]
            let total = self.total_bytes as f32;
            (used / total).clamp(0.0, 1.0)
        }
    }
}

/// Sample storage-meter values (~62% full of 50 GB).
#[must_use]
pub const fn sample_storage_meter() -> StorageMeter {
    StorageMeter {
        used_bytes: 33_285_996_544,
        total_bytes: 53_687_091_200,
    }
}

fn library_primary_nav(inbox_unread: u32) -> Vec<crate::components::library::LibraryNavItemView> {
    use crate::components::library::LibraryNavItemView;
    vec![
        LibraryNavItemView {
            id: "new",
            label: "New recording",
            icon: "＋",
            count: None,
            badge: None,
            selected: false,
            disabled: false,
        },
        LibraryNavItemView {
            id: "all",
            label: "All recordings",
            icon: "▦",
            count: Some(42),
            badge: None,
            selected: true,
            disabled: false,
        },
        LibraryNavItemView {
            id: "starred",
            label: "Starred",
            icon: "★",
            count: Some(7),
            badge: None,
            selected: false,
            disabled: false,
        },
        LibraryNavItemView {
            id: "shared",
            label: "Shared with me",
            icon: "↗",
            count: Some(3),
            badge: None,
            selected: false,
            disabled: false,
        },
        LibraryNavItemView {
            id: "inbox",
            label: "Inbox",
            icon: "✉",
            count: None,
            badge: (inbox_unread > 0).then_some(inbox_unread),
            selected: false,
            disabled: false,
        },
    ]
}

fn library_spaces_and_tags() -> Vec<crate::components::library::LibrarySectionView> {
    use crate::components::library::{LibraryNavItemView, LibrarySectionView};
    let spaces = LibrarySectionView {
        heading: "SPACES",
        items: vec![
            LibraryNavItemView {
                id: "space-team",
                label: "Northwind Studio",
                icon: "🟢",
                count: Some(24),
                badge: None,
                selected: false,
                disabled: false,
            },
            LibraryNavItemView {
                id: "space-product",
                label: "Product reviews",
                icon: "🟣",
                count: Some(8),
                badge: None,
                selected: false,
                disabled: false,
            },
        ],
    };
    let tags = LibrarySectionView {
        heading: "TAGS",
        items: vec![
            LibraryNavItemView {
                id: "tag-tutorial",
                label: "Tutorials",
                icon: "🟦",
                count: Some(12),
                badge: None,
                selected: false,
                disabled: false,
            },
            LibraryNavItemView {
                id: "tag-bug",
                label: "Bug repros",
                icon: "🟥",
                count: Some(6),
                badge: None,
                selected: false,
                disabled: false,
            },
            LibraryNavItemView {
                id: "tag-demo",
                label: "Customer demos",
                icon: "🟨",
                count: Some(3),
                badge: None,
                selected: false,
                disabled: false,
            },
        ],
    };
    vec![spaces, tags]
}

/// Sample sidebar view for UI-14. `inbox_unread` lets stories
/// sweep the badge count.
#[must_use]
pub fn sample_library_sidebar(inbox_unread: u32) -> crate::components::library::LibrarySidebarView {
    use crate::components::library::{LibrarySidebarView, StorageMeterView};
    LibrarySidebarView {
        primary: library_primary_nav(inbox_unread),
        sections: library_spaces_and_tags(),
        storage: StorageMeterView {
            used_bytes_label: "12.4 GB".to_owned(),
            quota_label: "50 GB".to_owned(),
            percent_used: sample_storage_meter().fraction(),
            plan_label: Some("Free"),
        },
    }
}

/// Variant — inbox selected (not "all"). Drives the `inbox-active`
/// story.
#[must_use]
pub fn sample_library_sidebar_inbox_active() -> crate::components::library::LibrarySidebarView {
    let mut v = sample_library_sidebar(3);
    for item in &mut v.primary {
        item.selected = item.id == "inbox";
    }
    v
}

/// Variant — near-full storage drives the warning style.
#[must_use]
pub fn sample_library_sidebar_high_storage() -> crate::components::library::LibrarySidebarView {
    let mut v = sample_library_sidebar(0);
    "47.6 GB".clone_into(&mut v.storage.used_bytes_label);
    v.storage.percent_used = 0.95;
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_sidebar_has_primary_and_sections() {
        let v = sample_library_sidebar(3);
        assert!(v.primary.iter().any(|i| i.id == "inbox"));
        assert_eq!(v.sections.len(), 2);
        assert_eq!(v.sections[0].heading, "SPACES");
        assert_eq!(v.sections[1].heading, "TAGS");
    }

    #[test]
    fn high_storage_variant_passes_warn_threshold() {
        let v = sample_library_sidebar_high_storage();
        assert!(v.storage.percent_used >= 0.85);
    }

    #[test]
    fn recordings_non_empty_with_one_selected() {
        let r = sample_recordings();
        assert!(!r.is_empty());
        assert_eq!(r.iter().filter(|x| x.selected).count(), 1);
    }

    #[test]
    fn storage_fraction_in_unit_interval() {
        let f = sample_storage_meter().fraction();
        assert!((0.0..=1.0).contains(&f));
    }
}
