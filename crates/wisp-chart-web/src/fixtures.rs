//! Per-chart fixtures consumed by both the native render-to-PNG
//! integration tests and the browser WebGPU demo. Keeping them
//! here in `lib.rs` so the iframe demos render the same data the
//! committed `.png` snapshots show.

use jiff::civil::date;

use wisp_chart::baseline::BaselineChart;
use wisp_chart::color::Color as ChartColor;
use wisp_chart::finance::{Candlestick, Ohlc, Period, Waterfall, WaterfallRow};
use wisp_chart::heatmap::{CalendarHeatmap, CalendarValue, LasagnaHeatmap, TableHeatmap};
use wisp_chart::indicator::{Bullet, Delta, DeltaKind, Gauge, Kpi, Orientation, Zone};
use wisp_chart::plot::{DataFrame, Value};
use wisp_chart::polar::{Pie, Radar, RadarAxis, RadarSeries, Slice, Sunburst, SunburstNode};

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

/// Pie fixture — traffic-source mix.
#[must_use]
pub fn pie_fixture() -> Pie {
    Pie::new(vec![
        Slice::new(45.0, "Organic", ChartColor::from_hex("#0072b2").unwrap()),
        Slice::new(25.0, "Paid", ChartColor::from_hex("#d55e00").unwrap()),
        Slice::new(15.0, "Social", ChartColor::from_hex("#009e73").unwrap()),
        Slice::new(10.0, "Referral", ChartColor::from_hex("#cc79a7").unwrap()),
        Slice::new(5.0, "Direct", ChartColor::from_hex("#f0e442").unwrap()),
    ])
}

/// Donut fixture — same as pie, 50% hollow.
#[must_use]
pub fn donut_fixture() -> Pie {
    pie_fixture().hollow_ratio(0.5)
}

/// Sunburst fixture — 2-level org / category breakdown.
#[must_use]
pub fn sunburst_fixture() -> Sunburst {
    let c = |hex: &str| ChartColor::from_hex(hex).unwrap();
    Sunburst::new(SunburstNode::group(
        "root",
        c("#888888"),
        vec![
            SunburstNode::group(
                "Sales",
                c("#0072b2"),
                vec![
                    SunburstNode::leaf("NA", 30.0, c("#56b4e9")),
                    SunburstNode::leaf("EU", 20.0, c("#7faedc")),
                    SunburstNode::leaf("APAC", 15.0, c("#a3c7ea")),
                ],
            ),
            SunburstNode::group(
                "Marketing",
                c("#d55e00"),
                vec![
                    SunburstNode::leaf("Brand", 12.0, c("#e8853d")),
                    SunburstNode::leaf("Perf", 18.0, c("#eea063")),
                ],
            ),
            SunburstNode::group(
                "Eng",
                c("#009e73"),
                vec![
                    SunburstNode::leaf("Platform", 25.0, c("#3eb893")),
                    SunburstNode::leaf("App", 20.0, c("#71cba8")),
                ],
            ),
        ],
    ))
    .ring_width_px(30.0)
}

/// Candlestick fixture — 8 OHLC periods.
#[must_use]
pub fn candlestick_fixture() -> Candlestick {
    Candlestick::new(vec![
        Period::new(100.0, 110.0, 95.0, 108.0),
        Period::new(108.0, 115.0, 105.0, 102.0),
        Period::new(102.0, 109.0, 100.0, 107.0),
        Period::new(107.0, 112.0, 103.0, 111.0),
        Period::new(111.0, 118.0, 109.0, 116.0),
        Period::new(116.0, 119.0, 112.0, 113.0),
        Period::new(113.0, 117.0, 110.0, 109.0),
        Period::new(109.0, 114.0, 106.0, 113.0),
    ])
}

/// OHLC fixture — same periods as candlestick.
#[must_use]
pub fn ohlc_fixture() -> Ohlc {
    Ohlc::new(vec![
        Period::new(100.0, 110.0, 95.0, 108.0),
        Period::new(108.0, 115.0, 105.0, 102.0),
        Period::new(102.0, 109.0, 100.0, 107.0),
        Period::new(107.0, 112.0, 103.0, 111.0),
        Period::new(111.0, 118.0, 109.0, 116.0),
        Period::new(116.0, 119.0, 112.0, 113.0),
        Period::new(113.0, 117.0, 110.0, 109.0),
        Period::new(109.0, 114.0, 106.0, 113.0),
    ])
}

/// Waterfall fixture — P&L bridge.
#[must_use]
pub fn waterfall_fixture() -> Waterfall {
    Waterfall::new(vec![
        WaterfallRow::summary("Start", 100.0),
        WaterfallRow::contribution("Revenue", 80.0),
        WaterfallRow::contribution("COGS", -30.0),
        WaterfallRow::contribution("Opex", -25.0),
        WaterfallRow::contribution("Tax", -10.0),
        WaterfallRow::summary("End", 115.0),
    ])
}

/// Table heatmap fixture — 5×8 activity grid.
#[must_use]
pub fn table_heatmap_fixture() -> TableHeatmap {
    let rows = vec![
        "Mon".into(),
        "Tue".into(),
        "Wed".into(),
        "Thu".into(),
        "Fri".into(),
    ];
    let cols = (0..8).map(|h| format!("{}h", h * 3)).collect();
    let values = (0..5_usize)
        .map(|r| {
            (0..8_usize)
                .map(|c| {
                    let modulo = u16::try_from((r * 3 + c * 7) % 11).unwrap_or(0);
                    let base = f32::from(modulo) / 11.0;
                    base * 100.0
                })
                .collect()
        })
        .collect();
    TableHeatmap::new(rows, cols, values)
}

/// Calendar heatmap fixture — 2025 with a few hot days.
#[must_use]
pub fn calendar_heatmap_fixture() -> CalendarHeatmap {
    let mut values = Vec::new();
    // Synth a wave of activity through the year.
    for month in 1i8..=12 {
        for day in [3i8, 10, 17, 24] {
            #[allow(
                clippy::cast_precision_loss,
                clippy::cast_lossless,
                reason = "month + day bounded"
            )]
            let v = (f32::from(month) * 2.0 + f32::from(day) * 0.3) % 10.0;
            values.push(CalendarValue::new(date(2025, month, day), v));
        }
    }
    CalendarHeatmap::new(2025, values)
}

/// Lasagna fixture — 6 entities × 24 hours.
#[must_use]
pub fn lasagna_fixture() -> LasagnaHeatmap {
    let entities = (1..=6).map(|i| format!("entity-{i}")).collect();
    let times = (0..24).map(|h| format!("{h:02}h")).collect();
    let values = (0..6_usize)
        .map(|r| {
            (0..24_usize)
                .map(|c| {
                    let modulo = u16::try_from((r * 5 + c * 3) % 13).unwrap_or(0);
                    let v = f32::from(modulo) / 13.0;
                    v * 100.0
                })
                .collect()
        })
        .collect();
    LasagnaHeatmap::new(entities, times, values)
}

/// Baseline fixture — signal crossing zero a few times.
#[must_use]
pub fn baseline_fixture() -> BaselineChart {
    BaselineChart::new(
        vec![
            (0.0, 10.0),
            (1.0, 25.0),
            (2.0, 15.0),
            (3.0, -10.0),
            (4.0, -25.0),
            (5.0, -5.0),
            (6.0, 15.0),
            (7.0, 30.0),
        ],
        0.0,
    )
}

/// Radar fixture — two products across 5 dimensions.
#[must_use]
pub fn radar_fixture() -> Radar {
    Radar::new(
        vec![
            RadarAxis::new("speed", (0.0, 100.0)),
            RadarAxis::new("range", (0.0, 100.0)),
            RadarAxis::new("comfort", (0.0, 100.0)),
            RadarAxis::new("efficiency", (0.0, 100.0)),
            RadarAxis::new("price", (0.0, 100.0)),
        ],
        vec![
            RadarSeries::new(
                "Model A",
                vec![80.0, 70.0, 60.0, 90.0, 50.0],
                ChartColor::from_hex("#0072b2").unwrap(),
            ),
            RadarSeries::new(
                "Model B",
                vec![60.0, 85.0, 80.0, 70.0, 75.0],
                ChartColor::from_hex("#d55e00").unwrap(),
            ),
        ],
    )
}
