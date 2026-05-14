# Chart gallery

Every chart `wisp-chart` can render in one place. Click a
thumbnail to open the chapter; the live WebGPU demos are
embedded inside each.

<style>
.wisp-chart-gallery {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 1rem;
  margin: 1.5rem 0;
}
.wisp-chart-gallery .card {
  display: block;
  text-decoration: none;
  color: inherit;
  border: 1px solid #e5e5e5;
  border-radius: 6px;
  overflow: hidden;
  background: #fafafa;
}
.wisp-chart-gallery .card:hover {
  border-color: #888;
}
.wisp-chart-gallery img {
  width: 100%;
  aspect-ratio: 4 / 3;
  object-fit: contain;
  background: #fafafa;
  display: block;
}
.wisp-chart-gallery .label {
  padding: 0.5rem 0.75rem;
  font-size: 0.85rem;
  font-weight: 600;
  border-top: 1px solid #e5e5e5;
}
.wisp-chart-gallery .desc {
  padding: 0 0.75rem 0.75rem;
  font-size: 0.75rem;
  color: #666;
}
</style>

## Cartesian marks

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/grouped-bar.html"><img loading="lazy" src="../assets/wisp-chart-web/grouped-bar.png" alt="Grouped bar"><div class="label">Grouped bar</div><div class="desc">Side-by-side comparison per X band</div></a>
  <a class="card" href="../wisp-chart/charts/stacked-bar.html"><img loading="lazy" src="../assets/wisp-chart-web/stacked-bar.png" alt="Stacked bar"><div class="label">Stacked bar</div><div class="desc">Composition within a total</div></a>
  <a class="card" href="../wisp-chart/charts/line.html"><img loading="lazy" src="../assets/wisp-chart-web/line.png" alt="Line"><div class="label">Line</div><div class="desc">Time-series + multi-series</div></a>
  <a class="card" href="../wisp-chart/charts/area.html"><img loading="lazy" src="../assets/wisp-chart-web/area.png" alt="Area"><div class="label">Area</div><div class="desc">Filled region under a curve</div></a>
  <a class="card" href="../wisp-chart/charts/scatter.html"><img loading="lazy" src="../assets/wisp-chart-web/scatter.png" alt="Scatter"><div class="label">Scatter</div><div class="desc">Continuous-x point cloud</div></a>
  <a class="card" href="../wisp-chart/charts/bubble.html"><img loading="lazy" src="../assets/wisp-chart-web/bubble.png" alt="Bubble"><div class="label">Bubble</div><div class="desc">Scatter + area-encoded size</div></a>
  <a class="card" href="../wisp-chart/charts/connected-scatter.html"><img loading="lazy" src="../assets/wisp-chart-web/connected-scatter.png" alt="Connected scatter"><div class="label">Connected scatter</div><div class="desc">Trajectory through 2D space</div></a>
  <a class="card" href="../wisp-chart/charts/baseline.html"><img loading="lazy" src="../assets/wisp-chart-web/baseline.png" alt="Baseline"><div class="label">Baseline</div><div class="desc">Area split above/below a threshold</div></a>
</div>

## Indicators

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/kpi.html"><img loading="lazy" src="../assets/wisp-chart-web/kpi.png" alt="KPI card"><div class="label">KPI card</div><div class="desc">Big number + delta + sparkline</div></a>
  <a class="card" href="../wisp-chart/charts/gauge.html"><img loading="lazy" src="../assets/wisp-chart-web/gauge.png" alt="Gauge"><div class="label">Gauge</div><div class="desc">Semicircle + threshold zones</div></a>
  <a class="card" href="../wisp-chart/charts/bullet.html"><img loading="lazy" src="../assets/wisp-chart-web/bullet.png" alt="Bullet"><div class="label">Bullet</div><div class="desc">Performance vs target</div></a>
</div>

## Finance

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/candlestick.html"><img loading="lazy" src="../assets/wisp-chart-web/candlestick.png" alt="Candlestick"><div class="label">Candlestick</div><div class="desc">OHLC body + wick</div></a>
  <a class="card" href="../wisp-chart/charts/ohlc.html"><img loading="lazy" src="../assets/wisp-chart-web/ohlc.png" alt="OHLC"><div class="label">OHLC bar</div><div class="desc">Range line + open/close ticks</div></a>
  <a class="card" href="../wisp-chart/charts/waterfall.html"><img loading="lazy" src="../assets/wisp-chart-web/waterfall.png" alt="Waterfall"><div class="label">Waterfall</div><div class="desc">Cumulative deltas bridge</div></a>
</div>

## Polar

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/pie.html"><img loading="lazy" src="../assets/wisp-chart-web/pie.png" alt="Pie"><div class="label">Pie / donut</div><div class="desc">Categorical proportions</div></a>
  <a class="card" href="../wisp-chart/charts/sunburst.html"><img loading="lazy" src="../assets/wisp-chart-web/sunburst.png" alt="Sunburst"><div class="label">Sunburst</div><div class="desc">Radial hierarchy</div></a>
  <a class="card" href="../wisp-chart/charts/radar.html"><img loading="lazy" src="../assets/wisp-chart-web/radar.png" alt="Radar"><div class="label">Radar</div><div class="desc">Multi-axis polygon overlay</div></a>
  <a class="card" href="../wisp-chart/charts/polar.html"><img loading="lazy" src="../assets/wisp-chart-web/polar.png" alt="Polar plot"><div class="label">Polar plot</div><div class="desc">Wind-rose / angular sectors</div></a>
</div>

## Heatmaps

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/table-heatmap.html"><img loading="lazy" src="../assets/wisp-chart-web/table-heatmap.png" alt="Table heatmap"><div class="label">Table heatmap</div><div class="desc">2D matrix as colour grid</div></a>
  <a class="card" href="../wisp-chart/charts/calendar-heatmap.html"><img loading="lazy" src="../assets/wisp-chart-web/calendar-heatmap.png" alt="Calendar heatmap"><div class="label">Calendar heatmap</div><div class="desc">Year-in-review 7×52 grid</div></a>
  <a class="card" href="../wisp-chart/charts/lasagna.html"><img loading="lazy" src="../assets/wisp-chart-web/lasagna.png" alt="Lasagna"><div class="label">Lasagna plot</div><div class="desc">Entity-time stripes</div></a>
</div>

## Topology + multi-view

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/treemap.html"><img loading="lazy" src="../assets/wisp-chart-web/treemap.png" alt="Treemap"><div class="label">Treemap</div><div class="desc">Nested rectangle hierarchy</div></a>
  <a class="card" href="../wisp-chart/charts/funnel.html"><img loading="lazy" src="../assets/wisp-chart-web/funnel.png" alt="Funnel"><div class="label">Funnel</div><div class="desc">Staged conversion bands</div></a>
  <a class="card" href="../wisp-chart/charts/splom.html"><img loading="lazy" src="../assets/wisp-chart-web/splom.png" alt="SPLOM"><div class="label">SPLOM</div><div class="desc">Pairwise scatter matrix</div></a>
</div>

## Distributions

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/boxplot.html"><img loading="lazy" src="../assets/wisp-chart-web/boxplot.png" alt="Box plot"><div class="label">Box plot</div><div class="desc">5-number summary per category</div></a>
  <a class="card" href="../wisp-chart/charts/parallel-coords.html"><img loading="lazy" src="../assets/wisp-chart-web/parallel-coords.png" alt="Parallel coordinates"><div class="label">Parallel coordinates</div><div class="desc">N axes polyline overlay</div></a>
  <a class="card" href="../wisp-chart/charts/error-bars.html"><img loading="lazy" src="../assets/wisp-chart-web/error-bars.png" alt="Error bars overlay"><div class="label">Error bars</div><div class="desc">Uncertainty whiskers on bar / point / line</div></a>
</div>

## Gantt

<div class="wisp-chart-gallery">
  <a class="card" href="../wisp-chart/charts/gantt/overview.html"><img loading="lazy" src="../assets/wisp-chart-web/gantt-demo.png" alt="Gantt"><div class="label">Gantt</div><div class="desc">Multi-row timeline</div></a>
</div>
