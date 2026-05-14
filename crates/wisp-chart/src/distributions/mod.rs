//! Distribution charts — box plot + parallel coordinates.

pub mod boxplot;
pub mod parallel;

pub use boxplot::{Box, BoxPlot};
pub use parallel::{ParallelAxis, ParallelCoords, ParallelRow};
