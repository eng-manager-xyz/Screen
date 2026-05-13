//! Pure data structs — no rendering, no layout math. The
//! ingest-friendly shape per AUT-180.

use std::collections::HashMap;

use crate::color::Color;
use jiff::civil::Date;

/// Inclusive-start, exclusive-end date range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateRange {
    /// Inclusive start (00:00 of the day).
    pub start: Date,
    /// Exclusive end (00:00 of the day AFTER the last day).
    pub end: Date,
}

impl DateRange {
    /// Full calendar year `[Jan 1, Jan 1 of next year)`.
    ///
    /// `y` is constrained to `i16` to keep the public surface
    /// free of accidental `i32`-sized year values; `jiff` accepts
    /// the full negative range internally.
    #[must_use]
    pub fn year(y: i16) -> Self {
        Self {
            start: Date::constant(y, 1, 1),
            end: Date::constant(y + 1, 1, 1),
        }
    }

    /// Build from a half-open Rust `Range<Date>` for ergonomic
    /// fixture construction:
    ///
    /// ```ignore
    /// DateRange::from(date(2026, 2, 1)..date(2026, 3, 15))
    /// ```
    #[must_use]
    pub fn from_range(range: std::ops::Range<Date>) -> Self {
        Self {
            start: range.start,
            end: range.end,
        }
    }
}

impl From<std::ops::Range<Date>> for DateRange {
    fn from(range: std::ops::Range<Date>) -> Self {
        Self::from_range(range)
    }
}

/// One Gantt row — a horizontal track. Rows render top-to-bottom
/// in the order they appear in [`Gantt::rows`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// Stable identifier referenced by [`Bar::row_id`].
    pub id: String,
    /// Label drawn in the left gutter.
    pub label: String,
}

impl Row {
    /// Convenience constructor.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// One bar inside a row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bar {
    /// Which row this bar belongs to (matches a [`Row::id`]).
    pub row_id: String,
    /// Time span the bar covers.
    pub range: DateRange,
    /// Owner name — looked up in [`Gantt::people`] for colour.
    pub owner: String,
    /// Optional in-bar label override; defaults to `owner`.
    pub label: Option<String>,
    /// Optional grouping tag — v1 stores but does not render it.
    pub group: Option<String>,
}

impl Bar {
    /// Convenience constructor — most fixtures only set the
    /// required four fields.
    #[must_use]
    pub fn new(
        row_id: impl Into<String>,
        range: impl Into<DateRange>,
        owner: impl Into<String>,
    ) -> Self {
        Self {
            row_id: row_id.into(),
            range: range.into(),
            owner: owner.into(),
            label: None,
            group: None,
        }
    }
}

/// One person — name + explicit colour override (or fallback to
/// auto-assignment when the entry is missing).
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    /// Display name (matches [`Bar::owner`]).
    pub name: String,
    /// Bar fill colour for this person's bars.
    pub color: Color,
}

/// Map of owner name → explicit `Person` entry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PersonMap(HashMap<String, Person>);

impl PersonMap {
    /// Insert (or replace) a person entry.
    pub fn insert(&mut self, person: Person) {
        self.0.insert(person.name.clone(), person);
    }

    /// Look up an explicit entry by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Person> {
        self.0.get(name)
    }

    /// Iterate over every registered person.
    pub fn iter(&self) -> impl Iterator<Item = &Person> {
        self.0.values()
    }

    /// `true` if no explicit entries are registered (all owners
    /// will resolve via the palette's auto-assignment).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Top-level Gantt model.
#[derive(Clone, Debug, PartialEq)]
pub struct Gantt {
    /// Time span the chart covers.
    pub range: DateRange,
    /// Rows in display order (top to bottom).
    pub rows: Vec<Row>,
    /// Bars laid out across the rows.
    pub bars: Vec<Bar>,
    /// Explicit owner overrides (auto-assignment fills the rest).
    pub people: PersonMap,
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn year_range_spans_jan_to_jan() {
        let r = DateRange::year(2026);
        assert_eq!(r.start, date(2026, 1, 1));
        assert_eq!(r.end, date(2027, 1, 1));
    }

    #[test]
    fn date_range_from_rust_range_is_half_open() {
        let r: DateRange = (date(2026, 2, 1)..date(2026, 3, 15)).into();
        assert_eq!(r.start, date(2026, 2, 1));
        assert_eq!(r.end, date(2026, 3, 15));
    }

    #[test]
    fn row_new_takes_anything_into_string() {
        let r = Row::new("vec", "M-VEC");
        assert_eq!(r.id, "vec");
        assert_eq!(r.label, "M-VEC");
    }

    #[test]
    fn bar_new_defaults_label_and_group() {
        let b = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 15), "Matt");
        assert_eq!(b.label, None);
        assert_eq!(b.group, None);
    }

    #[test]
    fn person_map_round_trip() {
        let mut pm = PersonMap::default();
        assert!(pm.is_empty());
        pm.insert(Person {
            name: "Matt".into(),
            color: Color::WHITE,
        });
        assert!(!pm.is_empty());
        assert_eq!(pm.get("Matt").map(|p| &p.name), Some(&"Matt".to_string()));
        assert_eq!(pm.get("Alice"), None);
    }
}
