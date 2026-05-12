//! Shared menu + popover primitives (M-UI.3 / AUT-123).
//!
//! Used by workspace switcher (UI-05), device pickers (UI-08),
//! system-audio picker (UI-09), on-screen options popover (UI-10),
//! tray record popover composition (UI-12), and any future menu.
//!
//! These primitives don't carry workspace / device semantics — they
//! render rows of arbitrary content with the right look. The concrete
//! menus compose them with the matching fixtures.

pub mod menu_footer;
pub mod menu_list;
pub mod menu_row;
pub mod menu_section;
pub mod popover_surface;

pub use menu_footer::MenuFooter;
pub use menu_list::MenuList;
pub use menu_row::{MenuBadgeView, MenuRow, MenuRowKind};
pub use menu_section::MenuSection;
pub use popover_surface::{PopoverPlacement, PopoverSurface};
