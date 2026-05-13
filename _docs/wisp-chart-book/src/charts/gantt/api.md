# Gantt — data-struct API

v1 is **data-struct only**. No DSL, no parser, no builder.
Future M-CHART.1 may add a builder if ergonomics demand it.

## Top level

```rust,ignore
pub struct Gantt {
    pub range: DateRange,
    pub rows: Vec<Row>,
    pub bars: Vec<Bar>,
    pub people: PersonMap,
}
```

## `DateRange`

Half-open `[start, end)`. Two constructors:

```rust,ignore
DateRange::year(2026)                    // [Jan 1, Jan 1 of next year)
DateRange::from(date(2026, 2, 1)..date(2026, 3, 15))
```

The `From<Range<Date>>` impl makes inline construction
ergonomic in fixtures and tests.

## `Row`

```rust,ignore
Row::new("vec", "M-VEC")
```

`id` is the stable identifier; `Bar::row_id` references it.
`label` is the string drawn in the left gutter.

## `Bar`

```rust,ignore
Bar::new(
    "vec",
    date(2026, 2, 1)..date(2026, 3, 15),
    "Matt",
)
```

`Bar::label` and `Bar::group` are `Option<String>` and default
to `None`. v1 stores `group` but does not render it.

## `PersonMap`

```rust,ignore
let mut people = PersonMap::default();
people.insert(Person {
    name: "Matt".into(),
    color: Color::from_hex("#0072b2").unwrap(),
});
```

Explicit entries override the auto-assigned palette colour.
Owners without an entry fall back to `Theme::palette`
auto-assignment.

## Serde

The data structs are intentionally serde-friendly — adding
`#[derive(Serialize, Deserialize)]` later is a one-line change
per struct. The first ingest format will be CSV (M-CHART.2 /
follow-on ticket).

## API stability

The public surface is intentionally narrow. Additive changes
(more fields with `Default` impls, more constructors) ship at
minor versions. Renames / removals trip `cargo semver-checks`.
