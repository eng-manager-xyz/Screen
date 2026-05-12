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

#[cfg(test)]
mod tests {
    use super::*;

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
