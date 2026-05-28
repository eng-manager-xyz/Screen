//! Gantt chart composition.
//!
//! v1 surface — data structs in, scene-graph subtree out.
//! Layout + render passes land in subsequent M-CHART.0 chunks
//! (AUT-180).

pub mod chrome;
pub mod data;
pub mod hit_regions;
pub mod kinetic;
pub mod layout;
pub mod pan;
pub mod popover;
pub mod render;
pub mod resolver;
pub mod scene;

pub use data::{Bar, DateRange, Gantt, GanttMarker, GanttRole, Person, PersonMap, Row, RowKind};
pub use hit_regions::GanttHitRegion;
pub use kinetic::{DEFAULT_TAU, KineticPan, MIN_VELOCITY_PX_PER_S};
pub use pan::{GanttPanController, GanttViewport};
pub use popover::{PopoverAnchor, PopoverKind, PopoverMetadata};
pub use resolver::GanttHitResolver;
pub use scene::{GanttScene, Pane, PaneScissor, pane_scissor};
