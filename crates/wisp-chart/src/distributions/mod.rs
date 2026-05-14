//! Distribution charts — box plot, parallel coordinates,
//! histogram, KDE.

pub mod boxplot;
pub mod histogram;
pub mod kde;
pub mod parallel;

pub use boxplot::{Box, BoxPlot};
pub use histogram::{BinCount, Histogram, HistogramBin};
pub use kde::{BandwidthRule, KdePlot};
pub use parallel::{ParallelAxis, ParallelCoords, ParallelRow};
