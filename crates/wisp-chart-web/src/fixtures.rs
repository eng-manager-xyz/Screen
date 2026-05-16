//! Per-chart fixtures consumed by both the native render-to-PNG
//! integration tests and the browser WebGPU demo. Keeping them
//! here in `lib.rs` so the iframe demos render the same data the
//! committed `.png` snapshots show.

use jiff::civil::date;

use wisp_chart::baseline::BaselineChart;
use wisp_chart::color::Color as ChartColor;
use wisp_chart::contour::ContourPlot;
use wisp_chart::distributions::{
    BandwidthRule, BinCount, Box as BoxSummary, BoxPlot, Histogram, KdePlot, ParallelAxis,
    ParallelCoords, ParallelRow,
};
use wisp_chart::finance::{Candlestick, Ohlc, Period, Waterfall, WaterfallRow};
use wisp_chart::heatmap::{
    CalendarHeatmap, CalendarValue, Histogram2D, LasagnaHeatmap, TableHeatmap,
};
use wisp_chart::indicator::{Bullet, Delta, DeltaKind, Gauge, Kpi, Orientation, Zone};
use wisp_chart::multi::{Splom, SplomDimension};
use wisp_chart::overlay::{ErrorBars, ErrorPoint};
use wisp_chart::plot::{DataFrame, Value};
use wisp_chart::polar::{
    Pie, PolarPlot, Radar, RadarAxis, RadarSeries, Slice, Sunburst, SunburstNode,
};
use wisp_chart::ternary::{TernaryPlot, TernaryPoint};
use wisp_chart::topology::{
    Funnel, FunnelStage, Sankey, SankeyLink, SankeyNode, Treemap, TreemapNode,
};

/// Bar fixture — **NASA's Apollo program: crewed missions per
/// year, 1968–1972**. Each bar is the count of crewed Apollo
/// flights (Apollo 7 through Apollo 17) flown that calendar
/// year. Source: Wikipedia article "Apollo program".
///
/// Row schema is `(year_label, count)` rendered with the
/// generic-Bar mark via Plot's `x("quarter", Band)` /
/// `y("revenue", Linear)` encoding — the column names are
/// historical artefacts of the original abstract fixture but
/// don't appear in the rendered chart's title or axis labels.
#[must_use]
pub fn bar_fixture() -> DataFrame {
    let rows: Vec<(&'static str, f32)> = vec![
        ("1968", 2.0),
        ("1969", 4.0),
        ("1970", 2.0),
        ("1971", 2.0),
        ("1972", 1.0),
    ];
    DataFrame::from_rows(&rows, |(q, r)| {
        vec![
            ("quarter".into(), Value::Category((*q).into())),
            ("revenue".into(), Value::Number(*r)),
        ]
    })
}

/// Line fixture — **US unemployment rate during the Great
/// Depression, 1929–1941** (annual %, BLS-reconstructed historical
/// series). Source: Wikipedia article "Unemployment in the United
/// States" — historical statistics table.
///
/// Each row is `(year_label, series, annual_unemployment_rate_pct)`.
/// The series is constant ("US") since this is a single-curve story,
/// but the schema matches the existing chart fixtures so the demo
/// wiring stays identical.
#[must_use]
pub fn line_fixture() -> DataFrame {
    let rows: Vec<(&'static str, &'static str, f32)> = vec![
        ("1929", "US", 3.2),
        ("1930", "US", 8.7),
        ("1931", "US", 15.9),
        ("1932", "US", 23.6),
        ("1933", "US", 24.9),
        ("1934", "US", 21.7),
        ("1935", "US", 20.1),
        ("1936", "US", 16.9),
        ("1937", "US", 14.3),
        ("1938", "US", 19.0),
        ("1939", "US", 17.2),
        ("1940", "US", 14.6),
        ("1941", "US", 9.9),
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

/// Scatterplot fixture — **Fisher's Iris (1936)**: petal length
/// × petal width across the three species *setosa*, *versicolor*,
/// *virginica*. 30 measurements (10 per species) drawn from the
/// canonical 150-row dataset that R.A. Fisher published in *The
/// Use of Multiple Measurements in Taxonomic Problems* — the
/// founding dataset of modern statistical classification.
/// Source: Wikipedia article "Iris flower data set".
#[must_use]
pub fn scatter_fixture() -> DataFrame {
    let rows: Vec<(f32, f32, &'static str)> = vec![
        // setosa — short petals, narrow
        (1.4, 0.2, "setosa"),
        (1.4, 0.2, "setosa"),
        (1.3, 0.2, "setosa"),
        (1.5, 0.2, "setosa"),
        (1.4, 0.2, "setosa"),
        (1.7, 0.4, "setosa"),
        (1.4, 0.3, "setosa"),
        (1.5, 0.2, "setosa"),
        (1.4, 0.2, "setosa"),
        (1.5, 0.1, "setosa"),
        // versicolor — mid-length petals
        (4.7, 1.4, "versicolor"),
        (4.5, 1.5, "versicolor"),
        (4.9, 1.5, "versicolor"),
        (4.0, 1.3, "versicolor"),
        (4.6, 1.5, "versicolor"),
        (4.5, 1.3, "versicolor"),
        (4.7, 1.6, "versicolor"),
        (3.3, 1.0, "versicolor"),
        (4.6, 1.4, "versicolor"),
        (3.9, 1.4, "versicolor"),
        // virginica — long petals, wide
        (6.0, 2.5, "virginica"),
        (5.1, 1.9, "virginica"),
        (5.9, 2.1, "virginica"),
        (5.6, 1.8, "virginica"),
        (5.8, 2.2, "virginica"),
        (6.6, 2.1, "virginica"),
        (4.5, 1.7, "virginica"),
        (6.3, 1.8, "virginica"),
        (5.8, 1.8, "virginica"),
        (6.1, 2.5, "virginica"),
    ];
    DataFrame::from_rows(&rows, |(x, y, sp)| {
        vec![
            ("x".into(), Value::Number(*x)),
            ("y".into(), Value::Number(*y)),
            ("species".into(), Value::Category((*sp).into())),
        ]
    })
}

/// Bubble fixture — **Gapminder 2007**: GDP per capita (PPP, k USD)
/// × life expectancy (years) × population (millions) across 13
/// countries spanning four continents. The dataset Hans Rosling
/// made famous in his 2006 TED talk "The best stats you've ever
/// seen". Source: Wikipedia article "Gapminder Foundation"
/// (data file `gapminder` v0.3, year = 2007).
#[must_use]
pub fn bubble_fixture() -> DataFrame {
    let rows: Vec<(f32, f32, f32, &'static str)> = vec![
        // GDP per capita (k$ PPP, 2007 prices) — life exp — pop (millions)
        ("Ethiopia", 0.78, 52.9, 76.5, "Africa"),
        ("Nigeria", 2.01, 47.0, 135.0, "Africa"),
        ("South Africa", 9.27, 49.3, 43.5, "Africa"),
        ("Egypt", 5.35, 71.3, 80.3, "Africa"),
        ("India", 2.45, 64.7, 1110.0, "Asia"),
        ("China", 4.96, 73.0, 1318.0, "Asia"),
        ("Japan", 31.6, 82.6, 127.0, "Asia"),
        ("South Korea", 23.3, 78.6, 49.0, "Asia"),
        ("Germany", 32.2, 79.4, 82.4, "Europe"),
        ("Norway", 49.3, 80.2, 4.6, "Europe"),
        ("France", 30.5, 80.7, 61.0, "Europe"),
        ("United States", 42.9, 78.2, 301.0, "Americas"),
        ("Brazil", 9.07, 72.4, 190.0, "Americas"),
    ]
    .into_iter()
    .map(|(_, g, l, p, c)| (g, l, p, c))
    .collect();
    DataFrame::from_rows(&rows, |(gdp, life, pop, cont)| {
        vec![
            ("gdp".into(), Value::Number(*gdp)),
            ("life".into(), Value::Number(*life)),
            ("population".into(), Value::Number(*pop)),
            ("continent".into(), Value::Category((*cont).into())),
        ]
    })
}

/// Area chart fixture — **NASA budget as % of the US federal
/// budget, 1962–1972** (peak Apollo era through wind-down).
/// 1966 was the peak at ~4.4 %; by 1972 it was below 2 %.
/// Source: Wikipedia article "Budget of NASA".
#[must_use]
pub fn area_fixture() -> DataFrame {
    let rows: Vec<(&'static str, f32)> = vec![
        ("1962", 1.18),
        ("1963", 2.29),
        ("1964", 3.52),
        ("1965", 4.31),
        ("1966", 4.41),
        ("1967", 3.45),
        ("1968", 2.65),
        ("1969", 2.31),
        ("1970", 1.92),
        ("1971", 1.61),
        ("1972", 1.48),
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

/// KPI fixture — **Apollo 11 lunar surface samples returned**,
/// 47.5 lb (21.6 kg), against the mission's 50 lb pre-flight
/// goal. Sparkline traces the six Apollo lunar-landing missions
/// (11 / 12 / 14 / 15 / 16 / 17) — the later J-missions tripled
/// the early sample mass once the Lunar Roving Vehicle freed
/// the astronauts to traverse further from the LM. Source:
/// Wikipedia article "Moon rock".
#[must_use]
pub fn kpi_fixture() -> Kpi {
    Kpi {
        value: 47.5,
        label: "Apollo 11 lunar samples (lb)".into(),
        delta: Some(Delta {
            kind: DeltaKind::Down,
            formatted: "-2.5 lb vs goal".into(),
        }),
        // Apollo lunar-sample masses (kg → ×2.205 to lb, rounded):
        // A11 21.6, A12 34.4, A14 42.3, A15 76.7, A16 95.7, A17 110.5
        sparkline: Some(vec![47.5, 75.9, 93.3, 169.1, 211.0, 243.6]),
    }
}

/// Gauge fixture — **Apollo 11 Command Module cabin pressure**
/// during the trans-lunar coast: ≈ 5.0 psi pure O₂ (the
/// "spacecraft" 5-psi standard NASA adopted after the Apollo 1
/// fire ruled out the original 14.7-psi atmosphere). Three
/// qualitative zones around the operating point: green
/// nominal, orange caution above 6 psi, red over 8 psi.
/// Source: Wikipedia article "Environmental Control System
/// (Apollo)".
#[must_use]
pub fn gauge_fixture() -> Gauge {
    Gauge {
        value: 5.0,
        domain: (0.0, 10.0),
        zones: vec![
            Zone::new((0.0, 6.0), ChartColor::from_hex("#27ae60").unwrap()),
            Zone::new((6.0, 8.0), ChartColor::from_hex("#f5a623").unwrap()),
            Zone::new((8.0, 10.0), ChartColor::from_hex("#e74c3c").unwrap()),
        ],
    }
}

/// Bullet fixture — **2005 DARPA Grand Challenge**, Stanford's
/// "Stanley" vs the 132-mile course target. Stanley drove
/// 132.2 mi across the Mojave in 6 h 53 m to take the $2 M
/// prize — the watershed result that kicked self-driving cars
/// off the slide deck and into the real world. Qualitative
/// ranges: poor < 50 mi (the 2004 Challenge's best result was
/// 7.4 mi), satisfactory < 100 mi, good ≥ 100 mi. Source:
/// Wikipedia article "DARPA Grand Challenge (2005)".
#[must_use]
pub fn bullet_fixture() -> Bullet {
    Bullet {
        value: 132.2,
        target: 132.0,
        ranges: [50.0, 100.0, 150.0],
        orientation: Orientation::Horizontal,
    }
}

/// Pie fixture — **Causes of British army mortality in the
/// Crimean War, April 1854 – March 1855** (Florence Nightingale's
/// data, the basis for her famous polar-area "coxcomb" diagram).
/// The vast majority of deaths were from preventable disease,
/// not enemy fire — the finding that reshaped military medicine.
/// Source: Wikipedia article "Florence Nightingale".
#[must_use]
pub fn pie_fixture() -> Pie {
    Pie::new(vec![
        Slice::new(
            83.0,
            "Preventable disease",
            ChartColor::from_hex("#0072b2").unwrap(),
        ),
        Slice::new(
            8.0,
            "Wounds in battle",
            ChartColor::from_hex("#d55e00").unwrap(),
        ),
        Slice::new(
            9.0,
            "Other causes",
            ChartColor::from_hex("#009e73").unwrap(),
        ),
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

/// Daily Dow Jones Industrial Average around the **Wall Street
/// Crash of 1929** — eight trading days from Mon Oct 21 1929
/// through Wed Oct 30 1929 spanning Black Thursday (Oct 24),
/// Black Monday (Oct 28), and Black Tuesday (Oct 29). Source:
/// Wikipedia article "Wall Street Crash of 1929" + the
/// reproduced ticker tables in Galbraith's *The Great Crash 1929*.
/// `Period::new(open, high, low, close)` per day; the dramatic
/// red candles on Oct 24 / 28 / 29 tell the crash story.
fn wall_street_crash_1929() -> Vec<Period> {
    vec![
        // Mon Oct 21 — already softening
        Period::new(324.0, 328.0, 318.0, 320.9),
        // Tue Oct 22
        Period::new(320.9, 326.0, 313.0, 326.5),
        // Wed Oct 23 — sharp drop
        Period::new(326.5, 326.5, 304.0, 305.9),
        // Thu Oct 24 — Black Thursday. 12.9M shares; recovers slightly.
        Period::new(305.9, 305.9, 272.3, 299.5),
        // Fri Oct 25 — fragile rally
        Period::new(299.5, 305.0, 295.0, 301.2),
        // Mon Oct 28 — Black Monday. Closes 38 pts down.
        Period::new(301.2, 301.2, 256.0, 260.6),
        // Tue Oct 29 — Black Tuesday. 16.4M shares; -23%.
        Period::new(260.6, 264.0, 212.3, 230.1),
        // Wed Oct 30 — dead-cat bounce
        Period::new(230.1, 258.0, 230.0, 258.5),
    ]
}

/// Candlestick fixture — Dow Jones daily during the Wall Street
/// Crash of 1929. See [`wall_street_crash_1929`].
#[must_use]
pub fn candlestick_fixture() -> Candlestick {
    Candlestick::new(wall_street_crash_1929())
}

/// OHLC fixture — same Wall Street Crash 1929 dataset as the
/// candlestick view, in tick-bar form (left tick = open, right
/// tick = close, range = high–low).
#[must_use]
pub fn ohlc_fixture() -> Ohlc {
    Ohlc::new(wall_street_crash_1929())
}

/// Waterfall fixture — **Apollo program lifetime cost** ($25.4 B
/// in 1973 dollars) decomposed by major spending category.
/// Starts at zero, accumulates the four big buckets, lands at
/// the final total. Numbers from the 1973 NASA budget closeout
/// reported to Congress. Source: Wikipedia article "Apollo
/// program — Costs".
#[must_use]
pub fn waterfall_fixture() -> Waterfall {
    Waterfall::new(vec![
        WaterfallRow::summary("Start", 0.0),
        WaterfallRow::contribution("Saturn V", 9.3),
        WaterfallRow::contribution("Spacecraft", 8.1),
        WaterfallRow::contribution("Ground / ops", 4.7),
        WaterfallRow::contribution("R&D + other", 3.3),
        WaterfallRow::summary("Total ($B)", 25.4),
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

/// Baseline fixture — **US Federal Funds Rate** (effective annual
/// average, %) over 1965–1985, baselined against the long-run
/// 5 % anchor most macro texts use when discussing the Volcker
/// disinflation. The series spans the late-60s Vietnam-era
/// expansion through the 1979–82 Volcker shock that crushed
/// double-digit inflation. Source: FRED series `FEDFUNDS`,
/// summarised on Wikipedia "Federal funds rate".
#[must_use]
pub fn baseline_fixture() -> BaselineChart {
    BaselineChart::new(
        vec![
            (1965.0, 4.1),
            (1968.0, 5.7),
            (1971.0, 4.7),
            (1974.0, 11.0),
            (1977.0, 5.5),
            (1979.0, 11.2),
            (1981.0, 16.4),
            (1983.0, 9.1),
            (1985.0, 8.1),
        ],
        5.0,
    )
}

/// Error-bars fixture — 4-quarter bar revenue with symmetric
/// ±15% error whiskers. Caller composes with `bar_fixture` (or
/// the demo's `Bar` chart) to show the overlay use-case.
#[must_use]
pub fn error_bars_fixture() -> ErrorBars {
    // The bar fixture's y-extent (0..64) maps to (cum/64) of
    // each band centre. Bars are 4 across; centres at 0.125,
    // 0.375, 0.625, 0.875 of the plot's x-width.
    ErrorBars::new(
        vec![
            ErrorPoint::symmetric(0.125, 38.0, 5.0),
            ErrorPoint::symmetric(0.375, 52.0, 7.0),
            ErrorPoint::symmetric(0.625, 47.0, 6.0),
            ErrorPoint::symmetric(0.875, 64.0, 9.0),
        ],
        (0.0, 64.0),
    )
}

/// SPLOM fixture — **Fisher's Iris (1936)** four measurements
/// (sepal length, sepal width, petal length, petal width in cm)
/// for 12 rows sampled across the three species, in the order
/// setosa × 4 / versicolor × 4 / virginica × 4. The same dataset
/// the scatterplot uses, but presented as a 4 × 4 matrix so each
/// pair of features can be inspected at once — the original use
/// case for the scatterplot matrix.
/// Source: Wikipedia article "Iris flower data set".
#[must_use]
pub fn splom_fixture() -> Splom {
    Splom::new(vec![
        SplomDimension::new(
            "sepal_len",
            vec![
                5.1, 4.9, 4.7, 5.0, // setosa
                7.0, 6.4, 5.7, 5.5, // versicolor
                6.3, 6.4, 6.9, 7.7, // virginica
            ],
        ),
        SplomDimension::new(
            "sepal_wid",
            vec![3.5, 3.0, 3.2, 3.6, 3.2, 3.2, 2.8, 2.4, 3.3, 2.8, 3.1, 3.0],
        ),
        SplomDimension::new(
            "petal_len",
            vec![1.4, 1.4, 1.3, 1.4, 4.7, 4.5, 4.5, 3.8, 6.0, 5.6, 5.1, 6.1],
        ),
        SplomDimension::new(
            "petal_wid",
            vec![0.2, 0.2, 0.2, 0.2, 1.4, 1.5, 1.3, 1.1, 2.5, 2.2, 2.3, 2.3],
        ),
    ])
}

/// Box plot fixture — 4 categories with synthetic summaries.
#[must_use]
pub fn boxplot_fixture() -> BoxPlot {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    BoxPlot::new(vec![
        BoxSummary::from_summary("A", 10.0, 20.0, 30.0, 45.0, 60.0, c("#0072b2")),
        BoxSummary::from_summary("B", 5.0, 18.0, 28.0, 50.0, 70.0, c("#d55e00")),
        BoxSummary::from_summary("C", 20.0, 35.0, 45.0, 55.0, 80.0, c("#009e73")),
        BoxSummary::from_summary("D", 12.0, 22.0, 32.0, 42.0, 58.0, c("#cc79a7")),
    ])
}

/// Parallel-coordinates fixture — 4 dimensions × 6 rows.
#[must_use]
pub fn parallel_coords_fixture() -> ParallelCoords {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    ParallelCoords::new(
        vec![
            ParallelAxis::new("mpg", (10.0, 50.0)),
            ParallelAxis::new("cyl", (3.0, 8.0)),
            ParallelAxis::new("hp", (60.0, 300.0)),
            ParallelAxis::new("wt", (1.5, 5.5)),
        ],
        vec![
            ParallelRow::new(vec![32.0, 4.0, 95.0, 2.2], c("#0072b2")),
            ParallelRow::new(vec![28.0, 4.0, 110.0, 2.5], c("#56b4e9")),
            ParallelRow::new(vec![22.0, 6.0, 150.0, 3.0], c("#d55e00")),
            ParallelRow::new(vec![18.0, 6.0, 200.0, 3.6], c("#e8853d")),
            ParallelRow::new(vec![14.0, 8.0, 280.0, 4.4], c("#009e73")),
            ParallelRow::new(vec![12.0, 8.0, 300.0, 5.0], c("#3eb893")),
        ],
    )
}

/// Polar plot fixture — wind rose (8 compass directions × hours).
#[must_use]
pub fn polar_plot_fixture() -> PolarPlot {
    PolarPlot::new(
        vec![
            "N".into(),
            "NE".into(),
            "E".into(),
            "SE".into(),
            "S".into(),
            "SW".into(),
            "W".into(),
            "NW".into(),
        ],
        vec![12.0, 18.0, 22.0, 30.0, 25.0, 16.0, 14.0, 8.0],
    )
}

/// Treemap fixture — 2-level org breakdown.
#[must_use]
pub fn treemap_fixture() -> Treemap {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    Treemap::new(TreemapNode::group(
        "root",
        c("#888888"),
        vec![
            TreemapNode::group(
                "Sales",
                c("#0072b2"),
                vec![
                    TreemapNode::leaf("NA", 30.0, c("#56b4e9")),
                    TreemapNode::leaf("EU", 20.0, c("#7faedc")),
                    TreemapNode::leaf("APAC", 15.0, c("#a3c7ea")),
                ],
            ),
            TreemapNode::group(
                "Eng",
                c("#d55e00"),
                vec![
                    TreemapNode::leaf("Platform", 25.0, c("#e8853d")),
                    TreemapNode::leaf("App", 18.0, c("#eea063")),
                    TreemapNode::leaf("Infra", 12.0, c("#f3b890")),
                ],
            ),
            TreemapNode::group(
                "G&A",
                c("#009e73"),
                vec![
                    TreemapNode::leaf("HR", 8.0, c("#3eb893")),
                    TreemapNode::leaf("Finance", 6.0, c("#71cba8")),
                ],
            ),
        ],
    ))
}

/// Funnel fixture — 4-stage conversion.
#[must_use]
pub fn funnel_fixture() -> Funnel {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    Funnel::new(vec![
        FunnelStage::new("Visited", 10000.0, c("#0072b2")),
        FunnelStage::new("Signed up", 4000.0, c("#56b4e9")),
        FunnelStage::new("Activated", 1800.0, c("#7faedc")),
        FunnelStage::new("Converted", 600.0, c("#a3c7ea")),
    ])
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

/// Deterministic pseudo-uniform draw in `[0, 1)` using a single
/// wrapping LCG round. Output is mod-1000 so the intermediate
/// fits in a `u16` and converts to `f32` without precision loss.
fn pseudo_uniform(seed: u32) -> f32 {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "(seed % 1000) < 1000 fits in u16"
    )]
    let modded = (seed % 1000) as u16;
    f32::from(modded) / 1000.0
}


/// Heights (inches) of Union Army white volunteers, drawn from
/// **Benjamin A. Gould's 1869 *Investigations in the Military and
/// Anthropological Statistics of American Soldiers***. Gould
/// measured roughly 1 million Civil War recruits — the largest
/// systematic anthropometric study of its era and the data that
/// later anchored Galton's regression-to-the-mean work. The
/// fixture samples 200 heights centred at 67.8 in (~172 cm) with
/// σ ≈ 2.5 in, the report's reconstructed Gaussian fit for the
/// 25–34 age bracket.
/// Source: Wikipedia article "Anthropometric history".
fn civil_war_recruit_heights() -> Vec<f32> {
    // Deterministic two-uniform "normal-ish" reshaped to match
    // Gould's mean + sd. The chart-level shape is what matters
    // here — exact replication needs the published bin counts
    // not the sample-level vector.
    (0..200_u32)
        .map(|i| {
            let a = pseudo_uniform(i.wrapping_mul(1_103_515_245).wrapping_add(12_345));
            let b = pseudo_uniform(i.wrapping_mul(87).wrapping_add(17));
            // Mean 67.8 in, sd ≈ 2.5 in.
            (a + b - 1.0) * 4.3 + 67.8
        })
        .collect()
}

/// Histogram fixture — Civil War recruit heights (Gould 1869).
/// See [`civil_war_recruit_heights`] for the dataset story.
#[must_use]
pub fn histogram_fixture() -> Histogram {
    let samples = civil_war_recruit_heights();
    Histogram::from_samples(&samples, BinCount::Fixed(18), Some((60.0, 76.0)))
}

/// KDE fixture — same Civil War recruit heights, smoothed via
/// Silverman's bandwidth rule. Lets the chapter directly compare
/// the binned + continuous views of the same distribution.
#[must_use]
pub fn kde_fixture() -> KdePlot {
    KdePlot::new(civil_war_recruit_heights()).bandwidth(BandwidthRule::Silverman)
}

/// 2D histogram fixture — synthetic point cloud.
#[must_use]
pub fn histogram2d_fixture() -> Histogram2D {
    let points: Vec<(f32, f32)> = (0..600_u32)
        .map(|i| {
            let a = pseudo_uniform(i.wrapping_mul(1_103_515_245).wrapping_add(12_345));
            let b = pseudo_uniform(i.wrapping_mul(87).wrapping_add(17));
            let x = (a - 0.5) * 8.0;
            let y = (b - 0.5) * 8.0;
            (x, y)
        })
        .collect();
    Histogram2D::from_points(&points, 24, 24, Some(((-5.0, 5.0), (-5.0, 5.0))))
}

/// Contour fixture — radial bump (gaussian) over a 48×48 grid.
#[must_use]
pub fn contour_fixture() -> ContourPlot {
    let cols = 48_usize;
    let rows = 48_usize;
    let mut field = vec![0.0_f32; cols * rows];
    #[allow(
        clippy::cast_precision_loss,
        reason = "grid size 48 fits f32 mantissa precisely"
    )]
    let (cx, cy) = ((cols as f32 - 1.0) * 0.5, (rows as f32 - 1.0) * 0.5);
    for row in 0..rows {
        for col in 0..cols {
            #[allow(
                clippy::cast_precision_loss,
                reason = "col/row < 48 fits f32 mantissa precisely"
            )]
            let (dx, dy) = (col as f32 - cx, row as f32 - cy);
            field[row * cols + col] = (-(dx * dx + dy * dy) * 0.012).exp();
        }
    }
    ContourPlot::new(field, cols, rows, vec![0.15, 0.35, 0.55, 0.75, 0.9])
}

/// Ternary fixture — synthetic soil-composition points.
#[must_use]
pub fn ternary_fixture() -> TernaryPlot {
    let red = ChartColor::from_hex("#0072b2").unwrap();
    let points = vec![
        TernaryPoint::new(0.7, 0.2, 0.1, red),
        TernaryPoint::new(0.4, 0.4, 0.2, red),
        TernaryPoint::new(0.3, 0.5, 0.2, red),
        TernaryPoint::new(0.5, 0.3, 0.2, red),
        TernaryPoint::new(0.2, 0.3, 0.5, red),
        TernaryPoint::new(0.1, 0.4, 0.5, red),
        TernaryPoint::new(0.6, 0.1, 0.3, red),
        TernaryPoint::new(0.3, 0.3, 0.4, red),
    ];
    TernaryPlot::new("Sand", "Silt", "Clay", points)
}

/// Sankey fixture — 3-column flow (sources → mid → sinks).
#[must_use]
pub fn sankey_fixture() -> Sankey {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    let nodes = vec![
        SankeyNode::new("Organic", 0, c("#0072b2")),
        SankeyNode::new("Paid", 0, c("#d55e00")),
        SankeyNode::new("Signed Up", 1, c("#009e73")),
        SankeyNode::new("Trial", 1, c("#cc79a7")),
        SankeyNode::new("Converted", 2, c("#56b4e9")),
        SankeyNode::new("Lost", 2, c("#e69f00")),
    ];
    let ribbon = c("#aaaaaa");
    let links = vec![
        SankeyLink::new(0, 2, 40.0, ribbon),
        SankeyLink::new(0, 3, 25.0, ribbon),
        SankeyLink::new(1, 2, 20.0, ribbon),
        SankeyLink::new(1, 3, 15.0, ribbon),
        SankeyLink::new(2, 4, 35.0, ribbon),
        SankeyLink::new(2, 5, 25.0, ribbon),
        SankeyLink::new(3, 4, 15.0, ribbon),
        SankeyLink::new(3, 5, 25.0, ribbon),
    ];
    Sankey::new(nodes, links)
}

/// Faceted-KDE fixture — same as `kde_fixture` but visualised
/// alongside the histogram as a complementary view. The
/// faceted-density chapter is a composition tutorial rather
/// than a separate value type, so this fixture mirrors KDE.
#[must_use]
pub fn faceted_kde_fixture() -> KdePlot {
    kde_fixture()
}
