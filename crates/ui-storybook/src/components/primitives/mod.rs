//! Foundational primitives — `Button`, `Card`, and forthcoming surface +
//! control elements that the rest of the UI composes from. These are the
//! most reusable layer; downstream feature components in
//! [`shell`](super::shell) / [`recorder`](super::recorder) /
//! [`editor`](super::editor) / [`cursor`](super::cursor) /
//! [`menus`](super::menus) / [`library`](super::library) consume them.

pub mod button;
pub mod card;

pub use button::{Button, ButtonSize, ButtonVariant};
pub use card::{Card, CardBody, CardHeader};
