//! Editor components — editor shell, Wisp canvas host, inspector panel,
//! timeline skeleton, dope sheet, player controls. Filled in across UI-16
//! through UI-19; pre-existing dope-sheet and player-controls live here.

pub mod dope_sheet;
pub mod player_controls;

pub use dope_sheet::{DopeSheet, DopeSheetKeyframe, DopeSheetTrack, KeyframeKind, TrackKind};
pub use player_controls::{PlayState, PlayerControls};
