//! Shell components — chrome that frames the application: drop-zone, status
//! bar, navigation rail (forthcoming UI-02), and friends.

pub mod drop_zone;
pub mod status_bar;

pub use drop_zone::{DropZone, DropZoneState};
pub use status_bar::{StatusBar, StatusKind};
