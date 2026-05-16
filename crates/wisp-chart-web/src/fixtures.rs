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
    // NASA Apollo program annual outlays in $B (1973 dollars),
    // by NASA centre. Source: 1973 NASA Budget Estimates;
    // summarised by year on Wikipedia "Apollo program — Costs".
    //
    // Marshall (MSFC) ran Saturn V development, Manned Spacecraft
    // Center (MSC, Houston) ran the CSM + LM; Kennedy ran launch
    // operations + tracking. The 1966 peak corresponds to the
    // late-stage Saturn V flight-hardware build.
    let rows: Vec<(&'static str, &'static str, f32)> = vec![
        ("1962", "MSFC", 0.2),
        ("1962", "MSC", 0.2),
        ("1962", "KSC", 0.1),
        ("1964", "MSFC", 1.6),
        ("1964", "MSC", 1.0),
        ("1964", "KSC", 0.5),
        ("1966", "MSFC", 2.6),
        ("1966", "MSC", 1.7),
        ("1966", "KSC", 0.8),
        ("1969", "MSFC", 1.6),
        ("1969", "MSC", 1.6),
        ("1969", "KSC", 0.7),
        ("1972", "MSFC", 0.4),
        ("1972", "MSC", 0.7),
        ("1972", "KSC", 0.2),
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

/// Connected-scatter fixture — **the US Phillips-curve
/// trajectory, 1960 → 1980**, plotted as annual `(inflation,
/// unemployment)` pairs in time order. Through the 1960s the
/// classical Phillips downward-sloping curve held (low
/// unemployment ↔ higher inflation). The 1970s **stagflation**
/// shock punched the line out into the upper-right — both axes
/// climbing at once, the empirical observation that broke
/// Keynesian consensus and powered Friedman's natural-rate
/// theory. Numbers from BLS CPI + unemployment series.
/// Source: Wikipedia article "Phillips curve".
#[must_use]
pub fn connected_scatter_fixture() -> DataFrame {
    // (inflation %, unemployment %, year)
    let rows: Vec<(f32, f32, f32)> = vec![
        (1.7, 5.5, 1960.0),
        (1.0, 6.7, 1961.0),
        (3.0, 4.5, 1966.0),
        (4.2, 3.5, 1968.0),
        (5.5, 5.0, 1970.0),
        (11.0, 5.6, 1974.0), // stagflation begins
        (9.1, 7.7, 1975.0),
        (13.5, 5.8, 1980.0),
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

/// Sunburst fixture — **Apollo program lifetime cost ($25.4 B,
/// 1973 dollars) decomposed by major NASA centre and then by
/// program element**. Two-level hierarchy: the outer ring is
/// the four big buckets (Saturn V, Spacecraft, Ground / ops,
/// R&D + other) and the inner ring groups them by the centre
/// that owned the work (MSFC for the rockets, MSC / Houston for
/// the spacecraft, KSC for ground ops, HQ for everything else).
/// Numbers from the 1973 NASA budget closeout. Source: Wikipedia
/// article "Apollo program — Costs".
#[must_use]
pub fn sunburst_fixture() -> Sunburst {
    let c = |hex: &str| ChartColor::from_hex(hex).unwrap();
    Sunburst::new(SunburstNode::group(
        "Apollo $25.4 B",
        c("#888888"),
        vec![
            SunburstNode::group(
                "Marshall (MSFC)",
                c("#0072b2"),
                vec![
                    SunburstNode::leaf("Saturn V", 9.3, c("#56b4e9")),
                    SunburstNode::leaf("S-IB / S-II", 1.4, c("#7faedc")),
                ],
            ),
            SunburstNode::group(
                "Manned Spacecraft Center",
                c("#d55e00"),
                vec![
                    SunburstNode::leaf("CSM", 5.1, c("#e8853d")),
                    SunburstNode::leaf("LM", 3.0, c("#eea063")),
                ],
            ),
            SunburstNode::group(
                "Kennedy (KSC)",
                c("#009e73"),
                vec![
                    SunburstNode::leaf("Launch ops", 3.1, c("#3eb893")),
                    SunburstNode::leaf("Tracking", 1.6, c("#71cba8")),
                ],
            ),
            SunburstNode::group(
                "NASA HQ + R&D",
                c("#cc79a7"),
                vec![SunburstNode::leaf("R&D + ops", 1.9, c("#d99dbf"))],
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

/// Table heatmap fixture — **1918 influenza pandemic weekly
/// excess-mortality rate (per 1 000)** for five US cities across
/// the eight-week peak from late Sep through mid-Nov 1918. Cities
/// that imposed early non-pharmaceutical interventions
/// (St. Louis, San Francisco) cap noticeably below cities that
/// delayed (Philadelphia, Pittsburgh). Numbers approximated from
/// Markel et al., *JAMA* 2007, "Nonpharmaceutical interventions
/// implemented by US cities during the 1918–1919 influenza
/// pandemic." Source: Wikipedia article "1918 flu pandemic in
/// the United States".
#[must_use]
pub fn table_heatmap_fixture() -> TableHeatmap {
    let rows = vec![
        "Philadelphia".into(),
        "Pittsburgh".into(),
        "NYC".into(),
        "St. Louis".into(),
        "San Francisco".into(),
    ];
    let cols = vec![
        "wk 1".into(),
        "wk 2".into(),
        "wk 3".into(),
        "wk 4".into(),
        "wk 5".into(),
        "wk 6".into(),
        "wk 7".into(),
        "wk 8".into(),
    ];
    let values = vec![
        // Philadelphia — delayed NPIs; peaks fastest + hardest.
        vec![0.4, 1.8, 8.5, 14.2, 6.0, 2.5, 1.4, 0.9],
        // Pittsburgh — also delayed.
        vec![0.5, 2.0, 5.6, 9.2, 7.3, 3.1, 1.5, 1.0],
        // NYC — moderate timing.
        vec![0.6, 1.4, 3.1, 4.8, 4.6, 3.0, 2.0, 1.4],
        // St. Louis — early action; flat peak.
        vec![0.4, 0.7, 1.2, 2.1, 2.6, 2.2, 1.5, 1.0],
        // San Francisco — early action.
        vec![0.3, 0.6, 1.0, 1.8, 2.4, 2.0, 1.3, 0.8],
    ];
    TableHeatmap::new(rows, cols, values)
}

/// Calendar heatmap fixture — **1918 weekly mortality from
/// influenza + pneumonia in NYC**, charted as a calendar grid
/// across the full year. The October peak (~12 weekly deaths
/// per 10 k population, the autumnal "second wave") is the
/// chart's high cell; the spring "first wave" is the lighter
/// March / April band. Approximated from the NYC Department of
/// Health weekly mortality reports. Source: Wikipedia article
/// "1918 flu pandemic in the United States — New York City".
#[must_use]
pub fn calendar_heatmap_fixture() -> CalendarHeatmap {
    // Approximate weekly excess-mortality rate (per 10 000) by
    // ISO week of 1918, plotted on the Wednesday of each week.
    // The October peak is well documented at ~13 / 10 k.
    let weekly_rate: [f32; 52] = [
        0.6, 0.7, 0.8, 1.0, 1.4, 1.8, 2.2, 2.6, // Jan–Feb early bumps
        2.4, 2.0, 1.6, 1.3, 1.0, 0.8, 0.7, 0.6, // Mar–Apr first wave
        0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.6, 0.6, // May–Jun quiet
        0.7, 0.8, 1.0, 1.4, 2.2, 3.6, 6.4, 9.8, // Jul–Sep second wave climb
        12.5, 13.0, 11.6, 8.8, 5.6, 3.4, 2.2, 1.8, // Oct peak + decline
        1.6, 1.4, 1.2, 1.0, 0.9, 0.8, 0.8, 0.7, // Nov tail
        0.7, 0.6, 0.6, 0.6, // Dec
    ];
    let mut values = Vec::with_capacity(52);
    for (i, rate) in weekly_rate.iter().enumerate() {
        // Wednesday of ISO week (i+1).
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "i bounded by 52"
        )]
        let week_index = i as i32;
        let day_of_year = week_index * 7 + 3;
        let (month, day) = day_of_year_to_md(day_of_year + 1);
        values.push(CalendarValue::new(date(1918, month, day), *rate));
    }
    CalendarHeatmap::new(1918, values)
}

/// Convert 1..=365 day-of-year into (month, day-of-month) for
/// non-leap year 1918. Caller bounds the input.
fn day_of_year_to_md(doy: i32) -> (i8, i8) {
    const DAYS_PER_MONTH: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut remaining = doy.clamp(1, 365);
    for (idx, &days) in DAYS_PER_MONTH.iter().enumerate() {
        if remaining <= days {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "month 1..=12 / day 1..=31 fit i8"
            )]
            return ((idx as i8) + 1, remaining as i8);
        }
        remaining -= days;
    }
    (12, 31)
}

/// Lasagna fixture — **US polio incidence per 100 k population**
/// by state × half-year 1952 → 1956 (six states × eight
/// half-years). The Salk inactivated polio vaccine was approved
/// 12 Apr 1955 and rolled out nationally that spring; the
/// post-1955 columns collapse to near-zero across every state,
/// the public-health victory the lasagna chart was invented to
/// visualise. Numbers approximated from CDC historical
/// surveillance summaries. Source: Wikipedia article
/// "Polio vaccine — Polio eradication".
#[must_use]
pub fn lasagna_fixture() -> LasagnaHeatmap {
    let entities: Vec<String> = vec![
        "California".into(),
        "New York".into(),
        "Texas".into(),
        "Massachusetts".into(),
        "Illinois".into(),
        "Pennsylvania".into(),
    ];
    let times: Vec<String> = vec![
        "1952 H1".into(),
        "1952 H2".into(),
        "1953 H1".into(),
        "1953 H2".into(),
        "1954 H1".into(),
        "1954 H2".into(),
        "1955 H1".into(),
        "1955 H2".into(),
        "1956 H1".into(),
        "1956 H2".into(),
    ];
    // Rough cases-per-100k by half-year. 1952 pandemic peak ~57/100k
    // nationally; falls by ~80 % within two years of the Salk vaccine.
    let values = vec![
        vec![26.0, 50.0, 18.0, 42.0, 15.0, 36.0, 12.0, 8.0, 3.5, 1.4],
        vec![22.0, 47.0, 16.0, 38.0, 14.0, 32.0, 10.0, 6.0, 2.5, 1.0],
        vec![28.0, 55.0, 19.0, 44.0, 16.0, 38.0, 11.0, 7.0, 3.0, 1.2],
        vec![20.0, 41.0, 14.0, 33.0, 11.0, 28.0, 9.0, 5.0, 2.0, 0.9],
        vec![24.0, 48.0, 17.0, 40.0, 14.0, 34.0, 10.5, 6.5, 2.8, 1.1],
        vec![26.0, 51.0, 18.0, 41.0, 15.0, 35.0, 11.0, 6.5, 2.7, 1.0],
    ];
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

/// Error-bars fixture — **Millikan oil-drop electron-charge
/// measurements**. Four of Millikan's annual published values
/// for the elementary charge (×10⁻¹⁰ statcoulomb), 1909→1913,
/// with the published uncertainty bands. Millikan's 1913 figure
/// (4.774) won him the 1923 Nobel and held as the canonical
/// value until Bridgman / Birge re-examined the data in the
/// 1930s. The visible drift across years — and Feynman's famous
/// commentary about it in *Cargo Cult Science* — is the
/// textbook reminder that error bars don't include systematic
/// bias. Source: Wikipedia article "Oil drop experiment".
#[must_use]
pub fn error_bars_fixture() -> ErrorBars {
    // X = year mapped to a 0..1 band position (4 measurements).
    // Y = measured charge (×10⁻¹⁰ statcoulomb).
    ErrorBars::new(
        vec![
            // 1909, first published estimate; large uncertainty.
            ErrorPoint::symmetric(0.125, 4.65, 0.08),
            // 1910, refined apparatus.
            ErrorPoint::symmetric(0.375, 4.70, 0.06),
            // 1911, "Determination of e" paper.
            ErrorPoint::symmetric(0.625, 4.74, 0.05),
            // 1913, the Nobel-cited value.
            ErrorPoint::symmetric(0.875, 4.774, 0.05),
        ],
        (4.5, 4.9),
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

/// Box plot fixture — **Boston Marathon men's winning times by
/// decade** (1900s → 1960s). Each box summarises the spread of
/// winning times within that decade — min / Q1 / median / Q3 /
/// max in minutes. The progression from a 2:55 median in the
/// 1900s to a 2:18 median in the 1960s tracks training science
/// + course modernisation; the wide 1920s box reflects the
/// crowd-tactic era where pace strategy was still being
/// invented. Numbers from the marathon's published results
/// archive. Source: Wikipedia article "List of Boston Marathon
/// winners".
#[must_use]
pub fn boxplot_fixture() -> BoxPlot {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    BoxPlot::new(vec![
        // min, q1, median, q3, max — all in minutes.
        BoxSummary::from_summary("1900s", 153.0, 156.0, 158.0, 160.0, 167.0, c("#0072b2")),
        BoxSummary::from_summary("1920s", 138.0, 142.0, 147.0, 150.0, 161.0, c("#d55e00")),
        BoxSummary::from_summary("1940s", 138.0, 140.0, 145.0, 149.0, 155.0, c("#009e73")),
        BoxSummary::from_summary("1960s", 133.0, 136.0, 138.0, 141.0, 146.0, c("#cc79a7")),
    ])
}

/// Parallel-coordinates fixture — **Apollo crewed lunar missions
/// 11–17** across four mission dimensions: total mission
/// duration (days), EVA hours on the lunar surface, kilometres
/// traversed (by foot Apollo 11–14 / by LRV Apollo 15–17), and
/// sample mass returned (kg). The break between Apollo 14 and
/// 15 is the LRV arriving — every dimension steps up. Numbers
/// from the NASA Apollo mission summary tables. Source:
/// Wikipedia article "Apollo program — Lunar missions".
#[must_use]
pub fn parallel_coords_fixture() -> ParallelCoords {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    ParallelCoords::new(
        vec![
            ParallelAxis::new("duration (d)", (8.0, 13.0)),
            ParallelAxis::new("EVA (h)", (0.0, 22.0)),
            ParallelAxis::new("traverse (km)", (0.0, 36.0)),
            ParallelAxis::new("samples (kg)", (0.0, 115.0)),
        ],
        vec![
            // Apollo 11 (Jul 1969)
            ParallelRow::new(vec![8.1, 2.5, 0.25, 21.6], c("#0072b2")),
            // Apollo 12 (Nov 1969)
            ParallelRow::new(vec![10.2, 7.8, 1.35, 34.4], c("#56b4e9")),
            // Apollo 14 (Feb 1971)
            ParallelRow::new(vec![9.0, 9.4, 3.45, 42.3], c("#7faedc")),
            // Apollo 15 (Jul 1971) — first LRV
            ParallelRow::new(vec![12.3, 18.5, 27.9, 76.7], c("#d55e00")),
            // Apollo 16 (Apr 1972)
            ParallelRow::new(vec![11.1, 20.2, 26.7, 95.7], c("#e8853d")),
            // Apollo 17 (Dec 1972) — peak J-mission
            ParallelRow::new(vec![12.6, 22.0, 35.7, 110.5], c("#eea063")),
        ],
    )
}

/// Polar plot fixture — **Florence Nightingale's monthly disease
/// deaths in the British Army, Crimean War, April – November
/// 1854**. The per-month breakdown that anchored her coxcomb /
/// rose diagram in *Notes on Matters Affecting the Health,
/// Efficiency, and Hospital Administration of the British Army*
/// (1858). The pie chart in [`pie_fixture`] shows the aggregate
/// summary — this per-month decomposition makes the Sep–Jan
/// peak (the Scutari hospital sanitation crisis) starkly
/// visible. Source: Wikipedia article "Florence Nightingale".
#[must_use]
pub fn polar_plot_fixture() -> PolarPlot {
    PolarPlot::new(
        vec![
            "Apr".into(),
            "May".into(),
            "Jun".into(),
            "Jul".into(),
            "Aug".into(),
            "Sep".into(),
            "Oct".into(),
            "Nov".into(),
        ],
        // British Army disease deaths per 1 000 in theatre,
        // Apr-Nov 1854 — approximated from Nightingale's report.
        vec![60.0, 110.0, 180.0, 250.0, 410.0, 1100.0, 920.0, 510.0],
    )
}

/// Treemap fixture — same **Apollo program $25.4 B cost
/// hierarchy** as [`sunburst_fixture`], squarified into nested
/// rectangles so total-area = total-cost. Saturn V dominates
/// (the biggest leaf) — the treemap shape makes that ratio
/// visible at a glance. Source: Wikipedia article "Apollo
/// program — Costs".
#[must_use]
pub fn treemap_fixture() -> Treemap {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    Treemap::new(TreemapNode::group(
        "Apollo $25.4 B",
        c("#888888"),
        vec![
            TreemapNode::group(
                "Marshall",
                c("#0072b2"),
                vec![
                    TreemapNode::leaf("Saturn V", 9.3, c("#56b4e9")),
                    TreemapNode::leaf("S-IB / S-II", 1.4, c("#7faedc")),
                ],
            ),
            TreemapNode::group(
                "Spacecraft",
                c("#d55e00"),
                vec![
                    TreemapNode::leaf("CSM", 5.1, c("#e8853d")),
                    TreemapNode::leaf("LM", 3.0, c("#eea063")),
                ],
            ),
            TreemapNode::group(
                "Kennedy",
                c("#009e73"),
                vec![
                    TreemapNode::leaf("Launch ops", 3.1, c("#3eb893")),
                    TreemapNode::leaf("Tracking", 1.6, c("#71cba8")),
                ],
            ),
            TreemapNode::group(
                "HQ + R&D",
                c("#cc79a7"),
                vec![TreemapNode::leaf("R&D + ops", 1.9, c("#d99dbf"))],
            ),
        ],
    ))
}

/// Funnel fixture — **Mercury Seven astronaut selection,
/// 1958–59**. NASA invited 508 military test pilots, qualified
/// 110 on records review, brought 32 to Lovelace Clinic + Wright-
/// Patterson AFB for the physical / psychological screening, cut
/// to 18 finalists, and announced **7** on 9 April 1959. The
/// most-selective hiring funnel in spaceflight history.
/// Source: Wikipedia article "Mercury Seven — Selection".
#[must_use]
pub fn funnel_fixture() -> Funnel {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    Funnel::new(vec![
        FunnelStage::new("Records reviewed", 110.0, c("#0072b2")),
        FunnelStage::new("Lovelace + WPAFB tests", 32.0, c("#56b4e9")),
        FunnelStage::new("Finalists", 18.0, c("#7faedc")),
        FunnelStage::new("Mercury Seven", 7.0, c("#a3c7ea")),
    ])
}

/// Radar fixture — **1960 Rome Olympics medal table**: USA vs
/// USSR across five medal categories. The Soviet Union topped
/// the table for the first time at a Summer Games it attended,
/// foreshadowing the Cold War's two-decade medal-count rivalry.
/// Numbers from the 1960 final medal tables (Wikipedia article
/// "1960 Summer Olympics medal table"). Axes range to 50 (gold)
/// or 110 (total) so neither superpower's axis pegs at max.
#[must_use]
pub fn radar_fixture() -> Radar {
    Radar::new(
        vec![
            RadarAxis::new("Gold", (0.0, 50.0)),
            RadarAxis::new("Silver", (0.0, 35.0)),
            RadarAxis::new("Bronze", (0.0, 35.0)),
            RadarAxis::new("Athletics", (0.0, 20.0)),
            RadarAxis::new("Total", (0.0, 110.0)),
        ],
        vec![
            RadarSeries::new(
                "USSR",
                vec![43.0, 29.0, 31.0, 11.0, 103.0],
                ChartColor::from_hex("#d55e00").unwrap(),
            ),
            RadarSeries::new(
                "USA",
                vec![34.0, 21.0, 16.0, 12.0, 71.0],
                ChartColor::from_hex("#0072b2").unwrap(),
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

/// 2D histogram fixture — **the Hertzsprung-Russell diagram**:
/// stellar effective temperature (log Kelvin, X) vs absolute
/// magnitude (M_V, Y). Each binned cell counts how many of the
/// ~600 synthesised stars fall there. The dense diagonal
/// running top-right → bottom-left is the **main sequence**;
/// the secondary cluster top-left is the white-dwarf branch;
/// the upper-right scatter is the giants + supergiants. The
/// chart was published independently by Hertzsprung (1911) and
/// Russell (1913) and is the single most important diagram in
/// stellar astrophysics. Source: Wikipedia article
/// "Hertzsprung–Russell diagram".
#[must_use]
pub fn histogram2d_fixture() -> Histogram2D {
    // X = log10(T_eff / K); plot-conventional axis runs HIGH-TEMP
    // LEFT, so we expose 3.4 (cool) .. 4.6 (hot). Y = absolute V
    // magnitude with brighter (negative) UP.
    let mut points: Vec<(f32, f32)> = Vec::with_capacity(640);
    // Main sequence — ~500 stars, descending diagonal from O5 (4.55, -6)
    // through G2 Sun (3.76, 4.8) to M5 (3.5, 12).
    for i in 0..500_u32 {
        let t = pseudo_uniform(i.wrapping_mul(2_147_483_647).wrapping_add(101));
        let x = 4.55 - t * 1.05; // 4.55 → 3.50
        let scatter = pseudo_uniform(i.wrapping_mul(31).wrapping_add(7)) - 0.5;
        // M_V along main sequence: -6 → 12 with mild scatter
        let y = -6.0 + t * 18.0 + scatter * 1.0;
        points.push((x, y));
    }
    // White dwarfs — ~70 stars, top-left low-luminosity cluster.
    for i in 0..70_u32 {
        let t = pseudo_uniform(i.wrapping_mul(8_191).wrapping_add(53));
        let x = 4.2 - t * 0.4;
        let y = 11.5 + (pseudo_uniform(i.wrapping_mul(47).wrapping_add(19)) - 0.5) * 1.4;
        points.push((x, y));
    }
    // Giants + supergiants — ~70 stars, upper right scatter.
    for i in 0..70_u32 {
        let t = pseudo_uniform(i.wrapping_mul(4_421).wrapping_add(29));
        let x = 3.85 - t * 0.35;
        let y = -1.0 - t * 4.0 + (pseudo_uniform(i.wrapping_mul(53).wrapping_add(11)) - 0.5) * 1.6;
        points.push((x, y));
    }
    Histogram2D::from_points(&points, 28, 28, Some(((3.4, 4.6), (-7.0, 14.0))))
}

/// Contour fixture — a single radial Gaussian peak over a 48×48
/// grid, mathematically equivalent to **the bivariate normal
/// density** Sir Francis Galton drew on his 1885 quincunx +
/// regression-board demonstration. Five level-sets at 0.15, 0.35,
/// 0.55, 0.75, 0.9 of the peak height. Source: Wikipedia article
/// "Galton board".
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

/// Ternary fixture — **soil-texture composition (Sand / Silt /
/// Clay) from the USDA soil-texture triangle**, eight reference
/// soil types sampled across the diagram. The USDA triangle is
/// the canonical figure agronomists use to classify soils
/// (sandy loam, silty clay, clay loam, etc.) by their fraction
/// of each particle size — published since 1951 in the *Soil
/// Survey Manual*. Source: Wikipedia article "Soil texture".
#[must_use]
pub fn ternary_fixture() -> TernaryPlot {
    let red = ChartColor::from_hex("#0072b2").unwrap();
    let points = vec![
        // Sand
        TernaryPoint::new(0.85, 0.12, 0.03, red),
        // Loamy sand
        TernaryPoint::new(0.75, 0.20, 0.05, red),
        // Sandy loam
        TernaryPoint::new(0.65, 0.25, 0.10, red),
        // Loam (the agronomist's ideal)
        TernaryPoint::new(0.40, 0.40, 0.20, red),
        // Silt loam
        TernaryPoint::new(0.20, 0.65, 0.15, red),
        // Silty clay loam
        TernaryPoint::new(0.10, 0.55, 0.35, red),
        // Clay loam
        TernaryPoint::new(0.30, 0.35, 0.35, red),
        // Clay
        TernaryPoint::new(0.15, 0.20, 0.65, red),
    ];
    TernaryPlot::new("Sand", "Silt", "Clay", points)
}

/// Sankey fixture — **NASA astronaut career flow, Groups 1–3
/// (Mercury / Gemini / Apollo eras)**. Of the 30 astronauts
/// across the three pre-1965 groups: roughly 17 came from Air
/// Force backgrounds and 13 from Navy / Marine; they sorted into
/// Mercury or Gemini training; and ultimately ~12 walked on the
/// Moon during the Apollo programme while the rest did not (the
/// "did not" includes Gus Grissom and Ed White, lost in Apollo 1,
/// and the back-up rotation that never flew lunar missions).
/// Counts approximated from the NASA Astronaut Group articles.
/// Source: Wikipedia "NASA Astronaut Group 1 / 2 / 3".
#[must_use]
pub fn sankey_fixture() -> Sankey {
    let c = |hex| ChartColor::from_hex(hex).unwrap();
    let nodes = vec![
        SankeyNode::new("USAF", 0, c("#0072b2")),
        SankeyNode::new("Navy / USMC", 0, c("#d55e00")),
        SankeyNode::new("Mercury group", 1, c("#009e73")),
        SankeyNode::new("Gemini group", 1, c("#cc79a7")),
        SankeyNode::new("Walked on Moon", 2, c("#56b4e9")),
        SankeyNode::new("Did not walk", 2, c("#e69f00")),
    ];
    let ribbon = c("#aaaaaa");
    let links = vec![
        SankeyLink::new(0, 2, 4.0, ribbon),
        SankeyLink::new(0, 3, 13.0, ribbon),
        SankeyLink::new(1, 2, 3.0, ribbon),
        SankeyLink::new(1, 3, 10.0, ribbon),
        SankeyLink::new(2, 4, 1.0, ribbon),
        SankeyLink::new(2, 5, 6.0, ribbon),
        SankeyLink::new(3, 4, 11.0, ribbon),
        SankeyLink::new(3, 5, 12.0, ribbon),
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
