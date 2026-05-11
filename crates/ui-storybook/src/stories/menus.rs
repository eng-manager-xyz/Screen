//! Menu / popover stories — filled in across UI-03 (menu primitives) /
//! UI-05 (workspace switcher) / UI-10 (on-screen options popover) / UI-12
//! (tray record popover composition).

use super::Story;

/// All menu-surface stories. Empty until UI-03 lands; the empty `Vec` keeps
/// the aggregator in [`super::all_stories`] uniform across surfaces.
#[must_use]
pub fn stories() -> Vec<Story> {
    Vec::new()
}
