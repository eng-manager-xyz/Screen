//! Per-chart render functions consumed by both native tests and
//! the browser WebGPU demo. Each `render_*` builds the chart from
//! the matching fixture in [`crate::fixtures`] and pushes its
//! `Graphics` + optional `Text` nodes onto the stage, then runs
//! `Renderer::render_stage` against the supplied `TextureView`.

use glam::Vec2;
use wgpu::TextureView;
use wisp::Font;
use wisp::application::Application;
use wisp::render::Renderer;
use wisp_chart::Theme;
use wisp_chart::plot::{
    self, Interpolation, Mark, Plot, PointShape, PointStyle, ScaleKind, SizeMapping, Transform,
};

use crate::fixtures;

/// Identifier of one of the demo charts. Parsed from the
/// `?chart=<id>` URL parameter in the browser, or hand-picked by
/// integration tests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChartId {
    /// Gantt fixture — same as the original wisp-chart-web demo.
    #[default]
    Gantt,
    /// Single-series bar chart.
    Bar,
    /// Multi-series line chart.
    Line,
    /// Grouped bar — 4 quarters × 3 regions.
    GroupedBar,
    /// Stacked bar — same data as grouped, accumulated.
    StackedBar,
    /// Scatterplot — 28 points across 3 species.
    Scatter,
    /// Bubble chart — area-encoded size.
    Bubble,
    /// Area chart — single-series filled curve.
    Area,
    /// Connected scatterplot — Linear X + Order encoding.
    ConnectedScatter,
    /// KPI / indicator card.
    Kpi,
    /// Semicircular gauge.
    Gauge,
    /// Bullet chart.
    Bullet,
    /// Pie chart.
    Pie,
    /// Donut variant of the pie chart.
    Donut,
    /// Radial hierarchy chart.
    Sunburst,
    /// Multi-axis radar / spider chart.
    Radar,
    /// Candlestick price chart.
    Candlestick,
    /// OHLC bar chart.
    Ohlc,
    /// Waterfall (cumulative deltas).
    Waterfall,
    /// Baseline chart (area split at threshold).
    Baseline,
    /// Table heatmap.
    TableHeatmap,
    /// Calendar heatmap (year-in-review).
    CalendarHeatmap,
    /// Lasagna heatmap.
    Lasagna,
    /// Treemap (hierarchical rectangles).
    Treemap,
    /// Funnel (staged conversion bands).
    Funnel,
    /// Box plot.
    BoxPlot,
    /// Parallel-coordinates plot.
    ParallelCoords,
    /// Scatterplot matrix.
    Splom,
    /// Bar chart with error-bar overlay.
    ErrorBars,
}

impl ChartId {
    /// Parse from a URL-param string (case-insensitive). Returns
    /// `None` for unknown values so callers can default cleanly.
    #[must_use]
    pub fn parse(id: &str) -> Option<Self> {
        match id.to_ascii_lowercase().as_str() {
            "gantt" => Some(Self::Gantt),
            "bar" => Some(Self::Bar),
            "line" => Some(Self::Line),
            "grouped-bar" | "grouped_bar" | "groupedbar" => Some(Self::GroupedBar),
            "stacked-bar" | "stacked_bar" | "stackedbar" => Some(Self::StackedBar),
            "scatter" => Some(Self::Scatter),
            "bubble" => Some(Self::Bubble),
            "area" => Some(Self::Area),
            "connected-scatter" | "connected_scatter" | "connectedscatter" => {
                Some(Self::ConnectedScatter)
            }
            "kpi" => Some(Self::Kpi),
            "gauge" => Some(Self::Gauge),
            "bullet" => Some(Self::Bullet),
            "pie" => Some(Self::Pie),
            "donut" => Some(Self::Donut),
            "sunburst" => Some(Self::Sunburst),
            "radar" => Some(Self::Radar),
            "candlestick" => Some(Self::Candlestick),
            "ohlc" => Some(Self::Ohlc),
            "waterfall" => Some(Self::Waterfall),
            "baseline" => Some(Self::Baseline),
            "table-heatmap" | "heatmap" => Some(Self::TableHeatmap),
            "calendar-heatmap" | "calendar" => Some(Self::CalendarHeatmap),
            "lasagna" => Some(Self::Lasagna),
            "treemap" => Some(Self::Treemap),
            "funnel" => Some(Self::Funnel),
            "box-plot" | "boxplot" => Some(Self::BoxPlot),
            "parallel-coords" | "parallel" => Some(Self::ParallelCoords),
            "splom" => Some(Self::Splom),
            "error-bars" | "errorbars" => Some(Self::ErrorBars),
            _ => None,
        }
    }
}

/// Dispatch render to the right per-chart fn.
///
/// # Errors
///
/// Returns the underlying [`wisp::Error`] if `Renderer::new`
/// fails (e.g. the surface format can't compile the pipelines).
#[allow(
    clippy::too_many_lines,
    reason = "one match arm per chart variant — each arm is a small Plot::new() builder chain. Extracting them into per-variant fns would 10× the indirection without simplifying anything."
)]
pub fn render_chart_to_view(
    chart: ChartId,
    app: &mut Application,
    target_view: &TextureView,
    surface_format: wgpu::TextureFormat,
    viewport_px: Vec2,
) -> Result<(), wisp::Error> {
    let renderer = Renderer::new(app, surface_format)?;
    let theme = Theme::light();
    let root = app.stage().root();

    match chart {
        ChartId::Gantt => {
            let gantt = crate::sample_gantt();
            let graphics = gantt.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Bar => {
            let plot = Plot::new(fixtures::bar_fixture())
                .mark(Mark::Bar {
                    value_labels: false,
                })
                .x_title("Quarter")
                .y_title("Revenue")
                .encode(plot::x("quarter", ScaleKind::Band))
                .encode(plot::y("revenue", ScaleKind::Linear));
            attach_plot(app, root, &plot, &theme, viewport_px);
        }
        ChartId::Line => {
            let plot = Plot::new(fixtures::line_fixture())
                .mark(Mark::Line {
                    interpolation: Interpolation::Linear,
                    marker: Some(PointStyle::Circle),
                })
                .x_title("Quarter")
                .y_title("Revenue")
                .encode(plot::x("quarter", ScaleKind::Band))
                .encode(plot::y("revenue", ScaleKind::Linear))
                .encode(plot::color("region"));
            attach_plot(app, root, &plot, &theme, viewport_px);
        }
        ChartId::GroupedBar => {
            let plot = Plot::new(fixtures::region_bar_fixture())
                .mark(Mark::Bar {
                    value_labels: false,
                })
                .x_title("Quarter")
                .y_title("Revenue")
                .encode(plot::x("quarter", ScaleKind::Band))
                .encode(plot::y("revenue", ScaleKind::Linear))
                .encode(plot::color("region"))
                .encode(plot::x_offset("region"));
            attach_plot(app, root, &plot, &theme, viewport_px);
        }
        ChartId::StackedBar => {
            let plot = Plot::new(fixtures::region_bar_fixture())
                .mark(Mark::Bar {
                    value_labels: false,
                })
                .x_title("Quarter")
                .y_title("Revenue")
                .encode(plot::x("quarter", ScaleKind::Band))
                .encode(plot::y("revenue", ScaleKind::Linear))
                .encode(plot::color("region"))
                .transform(Transform::Stack { normalize: false });
            attach_plot(app, root, &plot, &theme, viewport_px);
        }
        ChartId::Scatter => {
            let plot = Plot::new(fixtures::scatter_fixture())
                .mark(Mark::Point {
                    shape: PointShape::Circle,
                })
                .encode(plot::x("x", ScaleKind::Linear))
                .encode(plot::y("y", ScaleKind::Linear))
                .encode(plot::color("species"));
            let graphics = plot.render(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Bubble => {
            let plot = Plot::new(fixtures::bubble_fixture())
                .mark(Mark::Point {
                    shape: PointShape::Circle,
                })
                .encode(plot::x("gdp", ScaleKind::Linear))
                .encode(plot::y("life", ScaleKind::Linear))
                .encode(plot::size("population").size_mapping(SizeMapping::Area))
                .encode(plot::color("continent"));
            let graphics = plot.render(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Area => {
            let plot = Plot::new(fixtures::area_fixture())
                .mark(Mark::Area {
                    interpolation: Interpolation::Linear,
                })
                .x_title("Period")
                .y_title("Revenue")
                .encode(plot::x("quarter", ScaleKind::Band))
                .encode(plot::y("value", ScaleKind::Linear));
            attach_plot(app, root, &plot, &theme, viewport_px);
        }
        ChartId::ConnectedScatter => {
            let plot = Plot::new(fixtures::connected_scatter_fixture())
                .mark(Mark::Line {
                    interpolation: Interpolation::Linear,
                    marker: Some(PointStyle::Circle),
                })
                .encode(plot::x("inflation", ScaleKind::Linear))
                .encode(plot::y("unemployment", ScaleKind::Linear))
                .encode(plot::order("step"));
            let graphics = plot.render(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Kpi => {
            let kpi = fixtures::kpi_fixture();
            let graphics = kpi.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
            let font = Font::bitmap_8x8(app);
            for t in kpi.emit_text_labels(&theme, viewport_px, &font) {
                let _ = app.stage_mut().add_child(root, t);
            }
        }
        ChartId::Gauge => {
            let gauge = fixtures::gauge_fixture();
            let graphics = gauge.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
            let font = Font::bitmap_8x8(app);
            for t in gauge.emit_text_labels(&theme, viewport_px, &font) {
                let _ = app.stage_mut().add_child(root, t);
            }
        }
        ChartId::Bullet => {
            let bullet = fixtures::bullet_fixture();
            let graphics = bullet.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Pie => {
            let pie = fixtures::pie_fixture();
            let graphics = pie.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Donut => {
            let pie = fixtures::donut_fixture();
            let graphics = pie.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Sunburst => {
            let s = fixtures::sunburst_fixture();
            let graphics = s.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Radar => {
            let r = fixtures::radar_fixture();
            let graphics = r.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Candlestick => {
            let c = fixtures::candlestick_fixture();
            let graphics = c.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Ohlc => {
            let o = fixtures::ohlc_fixture();
            let graphics = o.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Waterfall => {
            let w = fixtures::waterfall_fixture();
            let graphics = w.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Baseline => {
            let b = fixtures::baseline_fixture();
            let graphics = b.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::TableHeatmap => {
            let h = fixtures::table_heatmap_fixture();
            let graphics = h.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::CalendarHeatmap => {
            let cal = fixtures::calendar_heatmap_fixture();
            let graphics = cal.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Lasagna => {
            let l = fixtures::lasagna_fixture();
            let graphics = l.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Treemap => {
            let t = fixtures::treemap_fixture();
            let graphics = t.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Funnel => {
            let f = fixtures::funnel_fixture();
            let graphics = f.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::BoxPlot => {
            let bp = fixtures::boxplot_fixture();
            let graphics = bp.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::ParallelCoords => {
            let pc = fixtures::parallel_coords_fixture();
            let graphics = pc.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::Splom => {
            let s = fixtures::splom_fixture();
            let graphics = s.emit_graphics(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, graphics);
        }
        ChartId::ErrorBars => {
            // Compose a Bar chart + matching ErrorBars overlay
            // in the SAME plot rect so the whiskers line up
            // with the bar centres. Bar uses Plot's
            // cartesian_layout (60-px gutter + 40-px header +
            // 40-px footer); ErrorBars matches by passing the
            // same rect.
            let plot = Plot::new(fixtures::bar_fixture())
                .mark(Mark::Bar {
                    value_labels: false,
                })
                .x_title("Quarter")
                .y_title("Revenue")
                .axes(false)
                .encode(plot::x("quarter", ScaleKind::Band))
                .encode(plot::y("revenue", ScaleKind::Linear));
            let bar_graphics = plot.render(&theme, viewport_px);
            let _ = app.stage_mut().add_child(root, bar_graphics);
            // Match the plot's internal layout — gutter 60,
            // header 40, footer 40, right pad 20.
            let plot_rect =
                wisp::math::Rect::new(60.0, 40.0, viewport_px.x - 80.0, viewport_px.y - 80.0);
            let bars = fixtures::error_bars_fixture();
            let overlay = bars.emit_graphics_in_rect(&theme, viewport_px, plot_rect);
            let _ = app.stage_mut().add_child(root, overlay);
        }
    }

    let _stats = renderer.render_stage(
        app,
        target_view,
        // Bright magenta clear — any uncovered region pops.
        wisp::Color::rgba(1.0, 0.0, 1.0, 1.0),
        app.stage(),
    );
    Ok(())
}

/// Helper: attach a `Plot`'s `Graphics` + axis text labels to
/// the stage. Used by cartesian-band plots that need axis titles
/// rendered as `Text` nodes.
fn attach_plot(
    app: &mut Application,
    root: wisp::scene::NodeId,
    plot: &Plot,
    theme: &Theme,
    viewport_px: Vec2,
) {
    let graphics = plot.render(theme, viewport_px);
    let _ = app.stage_mut().add_child(root, graphics);
    let font = Font::bitmap_8x8(app);
    for t in plot.axis_text_labels(theme, viewport_px, &font) {
        let _ = app.stage_mut().add_child(root, t);
    }
}
