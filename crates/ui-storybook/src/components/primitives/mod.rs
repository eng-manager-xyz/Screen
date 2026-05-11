//! Foundational primitives — `Button`, `Card`, `Surface`, `Badge`,
//! `Divider`, `Kbd`, `IconTile`. The most reusable layer; downstream
//! feature components in [`shell`](super::shell) /
//! [`recorder`](super::recorder) / [`editor`](super::editor) /
//! [`cursor`](super::cursor) / [`menus`](super::menus) /
//! [`library`](super::library) consume them.

pub mod badge;
pub mod button;
pub mod card;
pub mod divider;
pub mod icon_tile;
pub mod kbd;
pub mod surface;

pub use badge::{Badge, BadgeKind};
pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardBody, CardHeader};
pub use divider::{Divider, DividerOrientation};
pub use icon_tile::{IconTile, IconTileKind};
pub use kbd::Kbd;
pub use surface::{Surface, SurfaceKind};
