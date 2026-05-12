//! Foundational primitives — `Button`, `Card`, `Surface`, `Badge`,
//! `Divider`, `Kbd`, `IconTile`, `IconButton`, `ToggleSwitch`,
//! `SegmentedControl`, `Slider`, `SelectPill`, `ColorSwatch`, `Meter`.
//! The most reusable layer; downstream feature components in
//! [`shell`](super::shell) / [`recorder`](super::recorder) /
//! [`editor`](super::editor) / [`cursor`](super::cursor) /
//! [`menus`](super::menus) / [`library`](super::library) consume them.

pub mod badge;
pub mod button;
pub mod card;
pub mod color_swatch;
pub mod divider;
pub mod icon_button;
pub mod icon_tile;
pub mod kbd;
pub mod meter;
pub mod segmented_control;
pub mod select_pill;
pub mod slider;
pub mod surface;
pub mod toggle_switch;

pub use badge::{Badge, BadgeKind};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardBody, CardHeader};
pub use color_swatch::ColorSwatch;
pub use divider::{Divider, DividerOrientation};
pub use icon_button::{IconButton, IconButtonVariant};
pub use icon_tile::{IconTile, IconTileKind};
pub use kbd::Kbd;
pub use meter::{Meter, lit_segments};
pub use segmented_control::{Segment, SegmentedControl};
pub use select_pill::SelectPill;
pub use slider::{Slider, slider_percent};
pub use surface::{Surface, SurfaceKind};
pub use toggle_switch::ToggleSwitch;
