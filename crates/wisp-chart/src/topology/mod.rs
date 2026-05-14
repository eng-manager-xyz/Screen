//! Topology charts — treemap (hierarchical rectangles) +
//! funnel (staged conversion bands).

pub mod funnel;
pub mod treemap;

pub use funnel::{Funnel, FunnelStage};
pub use treemap::{Treemap, TreemapNode};
