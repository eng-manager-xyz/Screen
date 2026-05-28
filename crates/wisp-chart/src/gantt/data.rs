//! Pure data structs — no rendering, no layout math. The
//! ingest-friendly shape per AUT-180 + WG.1 (AUT-321) planning
//! parity extensions.
//!
//! ## Backward-compatible extension philosophy
//!
//! The original AUT-180 shape (`Row { id, label }`, `Bar { row_id,
//! range, owner, label?, group? }`) is preserved. WG.1 adds new
//! optional fields (`Row::kind`, `Row::subtitle`, …) and a new
//! `GanttMarker` model alongside it. Constructors keep the old
//! short forms (`Row::new(id, label)`, `Bar::new(row_id, range,
//! owner)`) and default the new fields to a sensible empty state,
//! so existing fixtures continue to compile unchanged.
//!
//! ## Planning concepts added in WG.1
//!
//! - [`RowKind`] distinguishes project rows from groupings + slack
//!   rows so headers can render hierarchies (e.g. M-CHART grouping
//!   six leaf milestones).
//! - [`Row::parent_id`] lets the host build a tree of rows without
//!   forcing the renderer to do nested layout — flat rows still
//!   draw in `Gantt::rows` order; the renderer + interaction
//!   layers consult `parent_id` for labelling, alt-row tinting,
//!   and roll-up tooltips.
//! - [`Bar::id`] gives every assignment a stable id callers can
//!   reference from semantic events (vs. relying on the row + range
//!   tuple).
//! - [`Bar::allocation_pct`] feeds the end-cap badge.
//! - [`Bar::lane`] places concurrent bars in a row's vertical lanes.
//! - [`Bar::roles`] carries tech-lead + future role markers.
//! - [`GanttMarker`] models milestone overlays (current date,
//!   quarter start, holidays, planning overlays). Markers are
//!   first-class so the WG.4 frozen-pane scene graph can route them
//!   to the correct pane (header vs body).

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

    /// Construct a single-day range `[d, d+1)`. Useful for
    /// holiday markers and the current-date overlay.
    #[must_use]
    pub fn day(d: Date) -> Self {
        Self {
            start: d,
            end: d.checked_add(jiff::Span::new().days(1)).unwrap_or(d),
        }
    }

    /// `true` if `d` falls inside the half-open range.
    #[must_use]
    pub fn contains(&self, d: Date) -> bool {
        d >= self.start && d < self.end
    }
}

impl From<std::ops::Range<Date>> for DateRange {
    fn from(range: std::ops::Range<Date>) -> Self {
        Self::from_range(range)
    }
}

/// What kind of row this is. Drives label rendering, gutter
/// indentation, and whether the row band is pickable.
///
/// `Project` is the v1 default — back-compat with AUT-180 fixtures.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum RowKind {
    /// A leaf project row that holds assignment bars. Default.
    #[default]
    Project,
    /// A grouping / parent header (e.g. "M-CHART" grouping the six
    /// underlying milestone rows). Visually styled like a category
    /// heading.
    Group,
    /// A slack / capacity row showing aggregated bandwidth instead
    /// of project bars. Reserved for future use; treated like
    /// `Project` for layout today.
    Slack,
}

/// Per-bar role marker — extends the bar's payload with semantic
/// roles the renderer + tooltips care about.
///
/// `#[non_exhaustive]` so we can add roles (DRI, Reviewer,
/// On-call, etc.) without breaking downstream callers.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GanttRole {
    /// Person leads engineering on this assignment. Renders the
    /// tech-lead marker (default: diamond) on the bar.
    TechLead,
}

/// A milestone / overlay that lives on the timeline alongside bars.
///
/// Markers carry their own date or range and render in either the
/// header pane (e.g. `QuarterStart`) or the body pane (e.g.
/// `CurrentDate` overlays the body across all rows). The
/// renderer (WG.2) decides per-variant where each marker pane
/// belongs.
///
/// `#[non_exhaustive]` so we can add planning concepts (release
/// cuts, freeze windows, ship dates) without breaking callers.
/// `Hash` + `Eq` are NOT derived because [`PlanningOverlay`]
/// carries an `f32`-component [`Color`] payload.
///
/// [`PlanningOverlay`]: GanttMarker::PlanningOverlay
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum GanttMarker {
    /// Vertical dashed line at the current date (typically inside
    /// the body pane). Position is the date the host computes at
    /// load time.
    CurrentDate {
        /// The date the marker sits on.
        date: Date,
    },
    /// Quarter-start tick on the header axis (e.g. Q1 → Q2). Hosts
    /// pre-compute the quarter boundaries; the renderer just
    /// places them.
    QuarterStart {
        /// First date of the new quarter.
        date: Date,
        /// Optional label (e.g. "Q2 2026"). Defaults to none.
        label: Option<String>,
    },
    /// Single-day or multi-day holiday band. Holiday pips render
    /// only in the header row (hosts may also dim the body cells
    /// in a follow-up).
    Holiday {
        /// Day span of the holiday.
        range: DateRange,
        /// Display name (e.g. "Thanksgiving"). Always required —
        /// it's what the tooltip / pip badge shows.
        label: String,
    },
    /// Generic planning overlay (year-end slowdown, code freeze,
    /// release-cut blackout). Renders behind bars with stable
    /// alpha; the host supplies the colour.
    PlanningOverlay {
        /// Span the overlay covers.
        range: DateRange,
        /// Display label.
        label: String,
        /// Fill colour (host-chosen).
        color: Color,
    },
}

/// One Gantt row — a horizontal track. Rows render top-to-bottom
/// in the order they appear in [`Gantt::rows`].
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Stable identifier referenced by [`Bar::row_id`].
    pub id: String,
    /// Label drawn in the left gutter.
    pub label: String,
    /// Row kind — drives label rendering and pickable-band shape.
    pub kind: RowKind,
    /// Optional second line in the gutter (e.g. program/team).
    pub subtitle: Option<String>,
    /// Optional owning team / program tag for tooltips.
    pub owner_team: Option<String>,
    /// Optional human-readable effort summary (e.g. "12 wk").
    pub effort_label: Option<String>,
    /// Optional engineering-week estimate. Used by WG.3 allocation
    /// math + tooltip computations.
    pub estimated_weeks: Option<f32>,
    /// Optional reference to a parent row's `id`, for grouping.
    pub parent_id: Option<String>,
}

impl Row {
    /// Convenience constructor — back-compatible with the AUT-180
    /// `Row::new(id, label)` short form. New fields default to
    /// empty / `Project`.
    #[must_use]
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: RowKind::default(),
            subtitle: None,
            owner_team: None,
            effort_label: None,
            estimated_weeks: None,
            parent_id: None,
        }
    }

    /// Builder — set the row kind (Project / Group / Slack).
    #[must_use]
    pub fn with_kind(mut self, kind: RowKind) -> Self {
        self.kind = kind;
        self
    }

    /// Builder — set the gutter subtitle.
    #[must_use]
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Builder — set the owning team tag.
    #[must_use]
    pub fn with_owner_team(mut self, team: impl Into<String>) -> Self {
        self.owner_team = Some(team.into());
        self
    }

    /// Builder — set the effort label (e.g. `"12 wk"`).
    #[must_use]
    pub fn with_effort_label(mut self, label: impl Into<String>) -> Self {
        self.effort_label = Some(label.into());
        self
    }

    /// Builder — set the engineering-week estimate.
    #[must_use]
    pub fn with_estimated_weeks(mut self, weeks: f32) -> Self {
        self.estimated_weeks = Some(weeks);
        self
    }

    /// Builder — set the parent row id (for grouping).
    #[must_use]
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }
}

/// One bar inside a row.
#[derive(Clone, Debug, PartialEq)]
pub struct Bar {
    /// Stable identifier — added in WG.1 so semantic events can
    /// reference bars without using `row_id + range` tuples. If
    /// the caller doesn't supply one, [`Bar::new`] mints
    /// `"{row_id}-{lane}-{start}"` from the constructor args.
    pub id: String,
    /// Which row this bar belongs to (matches a [`Row::id`]).
    pub row_id: String,
    /// Time span the bar covers.
    pub range: DateRange,
    /// Owner name — looked up in [`Gantt::people`] for colour.
    pub owner: String,
    /// Optional explicit person id (decouples display owner from
    /// the underlying assignee). Falls back to `owner` when None.
    pub person_id: Option<String>,
    /// Optional in-bar label override; defaults to `owner`.
    pub label: Option<String>,
    /// Optional allocation percent (0..=100). Drives the end-cap
    /// badge rendered by WG.3.
    pub allocation_pct: Option<f32>,
    /// Role markers (tech-lead, …). Empty by default.
    pub roles: Vec<GanttRole>,
    /// Optional lane index inside the row. `None` defaults to
    /// lane 0. WG.3 stacks concurrent bars by lane.
    pub lane: Option<u16>,
    /// Optional grouping tag — stored, not rendered by v1.
    pub group: Option<String>,
}

impl Bar {
    /// Convenience constructor — back-compatible with the AUT-180
    /// `Bar::new(row_id, range, owner)` short form. New fields
    /// default to empty.
    #[must_use]
    pub fn new(
        row_id: impl Into<String>,
        range: impl Into<DateRange>,
        owner: impl Into<String>,
    ) -> Self {
        let row_id: String = row_id.into();
        let range: DateRange = range.into();
        let owner: String = owner.into();
        // Mint a deterministic id from constructor args so callers
        // get something usable for hit-test reverse lookups
        // without having to set `id` explicitly. Callers that need
        // their own id space override via `with_id`.
        let id = format!("{row_id}::{}", range.start);
        Self {
            id,
            row_id,
            range,
            owner,
            person_id: None,
            label: None,
            allocation_pct: None,
            roles: Vec::new(),
            lane: None,
            group: None,
        }
    }

    /// Builder — override the bar id.
    #[must_use]
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Builder — set the explicit person id (vs falling back to
    /// `owner`).
    #[must_use]
    pub fn with_person_id(mut self, person_id: impl Into<String>) -> Self {
        self.person_id = Some(person_id.into());
        self
    }

    /// Builder — override the in-bar label.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Builder — set allocation percent (0..=100). Values are
    /// clamped on insert so callers can pass raw inputs.
    #[must_use]
    pub fn with_allocation_pct(mut self, pct: f32) -> Self {
        self.allocation_pct = Some(pct.clamp(0.0, 100.0));
        self
    }

    /// Builder — add a role marker.
    #[must_use]
    pub fn with_role(mut self, role: GanttRole) -> Self {
        self.roles.push(role);
        self
    }

    /// Builder — set the lane (vertical stack position within a row).
    #[must_use]
    pub fn with_lane(mut self, lane: u16) -> Self {
        self.lane = Some(lane);
        self
    }

    /// Builder — set the grouping tag.
    #[must_use]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
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
    /// Timeline markers (current date, quarter ticks, holidays,
    /// planning overlays). Empty by default; back-compatible.
    pub markers: Vec<GanttMarker>,
}

impl Default for Gantt {
    fn default() -> Self {
        Self {
            range: DateRange::year(2026),
            rows: Vec::new(),
            bars: Vec::new(),
            people: PersonMap::default(),
            markers: Vec::new(),
        }
    }
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
    fn date_range_day_is_single_day_half_open() {
        let r = DateRange::day(date(2026, 7, 4));
        assert_eq!(r.start, date(2026, 7, 4));
        assert_eq!(r.end, date(2026, 7, 5));
    }

    #[test]
    fn date_range_contains_is_half_open() {
        let r = DateRange::from_range(date(2026, 2, 1)..date(2026, 2, 5));
        assert!(r.contains(date(2026, 2, 1)));
        assert!(r.contains(date(2026, 2, 4)));
        assert!(!r.contains(date(2026, 2, 5)));
        assert!(!r.contains(date(2026, 1, 31)));
    }

    #[test]
    fn row_new_defaults_kind_and_optionals() {
        let r = Row::new("vec", "M-VEC");
        assert_eq!(r.id, "vec");
        assert_eq!(r.label, "M-VEC");
        assert_eq!(r.kind, RowKind::Project);
        assert!(r.subtitle.is_none());
        assert!(r.parent_id.is_none());
    }

    #[test]
    fn row_builders_compose() {
        let r = Row::new("chart", "M-CHART")
            .with_kind(RowKind::Group)
            .with_subtitle("Charting milestones")
            .with_owner_team("graphics")
            .with_effort_label("12 wk")
            .with_estimated_weeks(12.0);
        assert_eq!(r.kind, RowKind::Group);
        assert_eq!(r.subtitle.as_deref(), Some("Charting milestones"));
        assert_eq!(r.owner_team.as_deref(), Some("graphics"));
        assert_eq!(r.effort_label.as_deref(), Some("12 wk"));
        assert!((r.estimated_weeks.unwrap() - 12.0).abs() < 1e-6);
    }

    #[test]
    fn row_with_parent_models_grouping() {
        let r = Row::new("vec.add", "Vector add").with_parent("vec");
        assert_eq!(r.parent_id.as_deref(), Some("vec"));
    }

    #[test]
    fn bar_new_defaults_label_and_group_and_mints_id() {
        let b = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 15), "Matt");
        assert_eq!(b.label, None);
        assert_eq!(b.group, None);
        assert_eq!(b.allocation_pct, None);
        assert!(b.roles.is_empty());
        assert_eq!(b.lane, None);
        // Id should be deterministic + non-empty.
        assert_eq!(b.id, "vec::2026-02-01");
    }

    #[test]
    fn bar_with_allocation_pct_clamps_to_0_100() {
        let b =
            Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_allocation_pct(150.0);
        assert!((b.allocation_pct.unwrap() - 100.0).abs() < 1e-6);
        let b =
            Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_allocation_pct(-10.0);
        assert!((b.allocation_pct.unwrap() - 0.0).abs() < 1e-6);
    }

    #[test]
    fn bar_with_role_techlead_collected_in_roles_vec() {
        let b = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt")
            .with_role(GanttRole::TechLead);
        assert_eq!(b.roles.len(), 1);
        assert_eq!(b.roles[0], GanttRole::TechLead);
    }

    #[test]
    fn bar_with_lane_places_concurrent_assignments() {
        let b0 = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_lane(0);
        let b1 = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Alice").with_lane(1);
        assert_eq!(b0.lane, Some(0));
        assert_eq!(b1.lane, Some(1));
    }

    #[test]
    fn bar_with_id_overrides_auto_id() {
        let b = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt").with_id("custom-id-42");
        assert_eq!(b.id, "custom-id-42");
    }

    #[test]
    fn bar_with_person_id_decouples_owner_from_assignee() {
        let b = Bar::new("vec", date(2026, 2, 1)..date(2026, 3, 1), "Matt")
            .with_person_id("matt@example.com");
        assert_eq!(b.person_id.as_deref(), Some("matt@example.com"));
        assert_eq!(b.owner, "Matt");
    }

    #[test]
    fn gantt_default_is_empty_with_2026_range() {
        let g = Gantt::default();
        assert_eq!(g.range, DateRange::year(2026));
        assert!(g.rows.is_empty());
        assert!(g.bars.is_empty());
        assert!(g.markers.is_empty());
    }

    #[test]
    fn gantt_markers_round_trip_all_variants() {
        let mut g = Gantt {
            range: DateRange::year(2026),
            rows: Vec::new(),
            bars: Vec::new(),
            people: PersonMap::default(),
            markers: Vec::new(),
        };
        g.markers.push(GanttMarker::CurrentDate {
            date: date(2026, 6, 15),
        });
        g.markers.push(GanttMarker::QuarterStart {
            date: date(2026, 4, 1),
            label: Some("Q2 2026".into()),
        });
        g.markers.push(GanttMarker::Holiday {
            range: DateRange::day(date(2026, 7, 4)),
            label: "Independence Day".into(),
        });
        g.markers.push(GanttMarker::PlanningOverlay {
            range: DateRange::from_range(date(2026, 12, 20)..date(2026, 12, 31)),
            label: "Year-end slowdown".into(),
            color: Color::WHITE,
        });
        assert_eq!(g.markers.len(), 4);
        // Variant discrimination via match.
        let has_current = g.markers.iter().any(|m| {
            matches!(
                m,
                GanttMarker::CurrentDate { date: d } if *d == date(2026, 6, 15)
            )
        });
        assert!(has_current);
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
