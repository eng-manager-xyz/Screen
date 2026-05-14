//! Per-chart fixtures consumed by both the native render-to-PNG
//! integration tests and the browser WebGPU demo. Keeping them
//! here in `lib.rs` so the iframe demos render the same data the
//! committed `.png` snapshots show.

use wisp_chart::color::Color as ChartColor;
use wisp_chart::indicator::{Bullet, Delta, DeltaKind, Gauge, Kpi, Orientation, Zone};
use wisp_chart::plot::{DataFrame, Value};

/// Bar fixture — 4-quarter single-series revenue.
#[must_use]
pub fn bar_fixture() -> DataFrame {
    let rows: Vec<(&'static str, f32)> =
        vec![("Q1", 38.0), ("Q2", 52.0), ("Q3", 47.0), ("Q4", 64.0)];
    DataFrame::from_rows(&rows, |(q, r)| {
        vec![
            ("quarter".into(), Value::Category((*q).into())),
            ("revenue".into(), Value::Number(*r)),
        ]
    })
}

/// Line fixture — 2 series (NA, EU) across 4 quarters.
#[must_use]
pub fn line_fixture() -> DataFrame {
    let rows: Vec<(&'static str, &'static str, f32)> = vec![
        ("Q1", "NA", 38.0),
        ("Q2", "NA", 52.0),
        ("Q3", "NA", 47.0),
        ("Q4", "NA", 64.0),
        ("Q1", "EU", 22.0),
        ("Q2", "EU", 30.0),
        ("Q3", "EU", 42.0),
        ("Q4", "EU", 48.0),
    ];
    DataFrame::from_rows(&rows, |(q, r, v)| {
        vec![
            ("quarter".into(), Value::Category((*q).into())),
            ("region".into(), Value::Category((*r).into())),
            ("revenue".into(), Value::Number(*v)),
        ]
    })
}

/// Grouped / stacked bar fixture — 4 quarters × 3 regions.
#[must_use]
pub fn region_bar_fixture() -> DataFrame {
    let rows: Vec<(&'static str, &'static str, f32)> = vec![
        ("Q1", "NA", 38.0),
        ("Q1", "EU", 22.0),
        ("Q1", "APAC", 14.0),
        ("Q2", "NA", 52.0),
        ("Q2", "EU", 27.0),
        ("Q2", "APAC", 18.0),
        ("Q3", "NA", 47.0),
        ("Q3", "EU", 33.0),
        ("Q3", "APAC", 22.0),
        ("Q4", "NA", 64.0),
        ("Q4", "EU", 40.0),
        ("Q4", "APAC", 28.0),
    ];
    DataFrame::from_rows(&rows, |(q, r, v)| {
        vec![
            ("quarter".into(), Value::Category((*q).into())),
            ("region".into(), Value::Category((*r).into())),
            ("revenue".into(), Value::Number(*v)),
        ]
    })
}

/// Scatterplot fixture — 28 samples × 3 species.
#[must_use]
pub fn scatter_fixture() -> DataFrame {
    let rows: Vec<(f32, f32, &'static str)> = vec![
        (1.5, 2.1, "A"),
        (2.2, 2.8, "A"),
        (3.1, 4.0, "A"),
        (4.5, 5.2, "A"),
        (5.0, 5.9, "A"),
        (6.3, 7.1, "A"),
        (7.0, 8.2, "A"),
        (8.5, 9.4, "A"),
        (9.1, 9.8, "A"),
        (10.0, 11.2, "A"),
        (1.0, 4.0, "B"),
        (2.5, 5.0, "B"),
        (3.7, 6.4, "B"),
        (4.8, 7.2, "B"),
        (5.5, 8.1, "B"),
        (6.8, 9.0, "B"),
        (7.9, 10.1, "B"),
        (9.0, 11.3, "B"),
        (10.5, 12.0, "B"),
        (11.0, 13.0, "B"),
        (1.2, 6.0, "C"),
        (3.0, 7.5, "C"),
        (4.5, 9.0, "C"),
        (6.0, 10.5, "C"),
        (7.5, 12.0, "C"),
        (9.0, 13.5, "C"),
        (10.5, 14.5, "C"),
        (12.0, 15.0, "C"),
    ];
    DataFrame::from_rows(&rows, |(x, y, sp)| {
        vec![
            ("x".into(), Value::Number(*x)),
            ("y".into(), Value::Number(*y)),
            ("species".into(), Value::Category((*sp).into())),
        ]
    })
}

/// Bubble fixture — Gapminder-style GDP × life × population.
#[must_use]
pub fn bubble_fixture() -> DataFrame {
    let rows: Vec<(f32, f32, f32, &'static str)> = vec![
        (2.0, 65.0, 100.0, "Africa"),
        (3.0, 68.0, 200.0, "Africa"),
        (4.5, 72.0, 80.0, "Africa"),
        (6.0, 70.0, 300.0, "Africa"),
        (7.5, 76.0, 50.0, "Asia"),
        (10.0, 75.0, 1400.0, "Asia"),
        (12.0, 78.0, 200.0, "Asia"),
        (15.0, 82.0, 600.0, "Asia"),
        (18.0, 81.0, 100.0, "Europe"),
        (22.0, 83.0, 80.0, "Europe"),
        (28.0, 84.0, 60.0, "Europe"),
        (35.0, 81.5, 330.0, "Americas"),
        (42.0, 83.5, 50.0, "Americas"),
    ];
    DataFrame::from_rows(&rows, |(gdp, life, pop, cont)| {
        vec![
            ("gdp".into(), Value::Number(*gdp)),
            ("life".into(), Value::Number(*life)),
            ("population".into(), Value::Number(*pop)),
            ("continent".into(), Value::Category((*cont).into())),
        ]
    })
}

/// Area chart fixture — single 7-period series.
#[must_use]
pub fn area_fixture() -> DataFrame {
    let rows: Vec<(&'static str, f32)> = vec![
        ("Q1", 24.0),
        ("Q2", 38.0),
        ("Q3", 32.0),
        ("Q4", 56.0),
        ("Q5", 48.0),
        ("Q6", 64.0),
        ("Q7", 72.0),
    ];
    DataFrame::from_rows(&rows, |(q, v)| {
        vec![
            ("quarter".into(), Value::Category((*q).into())),
            ("value".into(), Value::Number(*v)),
        ]
    })
}

/// Connected-scatter fixture — Phillips-curve trajectory.
#[must_use]
pub fn connected_scatter_fixture() -> DataFrame {
    let rows: Vec<(f32, f32, f32)> = vec![
        (3.0, 5.5, 3.0),
        (2.5, 6.0, 1.0),
        (2.8, 5.8, 2.0),
        (3.5, 5.2, 4.0),
        (4.2, 4.9, 5.0),
        (5.0, 4.5, 6.0),
        (4.5, 4.7, 7.0),
        (5.8, 4.3, 8.0),
    ];
    DataFrame::from_rows(&rows, |(infl, unemp, step)| {
        vec![
            ("inflation".into(), Value::Number(*infl)),
            ("unemployment".into(), Value::Number(*unemp)),
            ("step".into(), Value::Number(*step)),
        ]
    })
}

/// KPI fixture — Monthly Active Users with sparkline.
#[must_use]
pub fn kpi_fixture() -> Kpi {
    Kpi {
        value: 1_234_567.0,
        label: "Monthly Active Users".into(),
        delta: Some(Delta {
            kind: DeltaKind::Up,
            formatted: "+12.4% vs last mo".into(),
        }),
        sparkline: Some(vec![
            100.0, 105.0, 102.0, 110.0, 108.0, 115.0, 112.0, 118.0, 120.0, 125.0,
        ]),
    }
}

/// Gauge fixture — 73% with green/orange/red zones.
#[must_use]
pub fn gauge_fixture() -> Gauge {
    Gauge {
        value: 73.0,
        domain: (0.0, 100.0),
        zones: vec![
            Zone::new((0.0, 60.0), ChartColor::from_hex("#27ae60").unwrap()),
            Zone::new((60.0, 85.0), ChartColor::from_hex("#f5a623").unwrap()),
            Zone::new((85.0, 100.0), ChartColor::from_hex("#e74c3c").unwrap()),
        ],
    }
}

/// Bullet fixture — value 270 vs target 250.
#[must_use]
pub fn bullet_fixture() -> Bullet {
    Bullet {
        value: 270.0,
        target: 250.0,
        ranges: [150.0, 225.0, 300.0],
        orientation: Orientation::Horizontal,
    }
}
