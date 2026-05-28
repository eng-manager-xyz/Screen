//! Gantt chart composition.
//!
//! v1 surface — data structs in, scene-graph subtree out.
//! Layout + render passes land in subsequent M-CHART.0 chunks
//! (AUT-180).

pub mod data;
pub mod hit_regions;
pub mod layout;
pub mod pan;
pub mod render;

pub use data::{Bar, DateRange, Gantt, GanttMarker, GanttRole, Person, PersonMap, Row, RowKind};
pub use hit_regions::GanttHitRegion;
pub use pan::{GanttPanController, GanttViewport};
