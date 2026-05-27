//! Keyboard / mouse / touch state vocabulary (WI.1 / AUT-304).
//!
//! The `input::` module is the "raw inputs" layer of `wisp-
//! interaction`. It owns:
//!
//! - [`ButtonInput<T>`] — Bevy-shaped three-set state machine
//!   (`pressed` / `just_pressed` / `just_released`) generic over a
//!   button kind. Pre-instantiated as [`KeyboardInput`] (for
//!   [`KeyCode`]) and [`MouseButtonInput`] (for [`MouseButton`]).
//! - [`KeyCode`] / [`MouseButton`] — input-key enums mirroring
//!   winit's shape closely enough that the WI.6 adapter is a
//!   1:1 translation table.
//! - [`ModifierState`] — packed-bool struct for shift/ctrl/alt/super.
//! - [`AccumulatedMouseMotion`] — per-frame motion delta (separate
//!   from per-event raw stream).
//! - [`InputEvent`] — sum-type for the raw per-event channel, for
//!   consumers that need per-event granularity (rare; `ButtonInput`
//!   is the 80% path).
//!
//! This module is platform-free. WI.6 / WI.7 adapters wire raw winit
//! / web-sys events into these types.

pub mod button_input;
pub mod key_code;
pub mod modifier;
pub mod mouse;
pub mod raw_event;

pub use button_input::{ButtonInput, KeyboardInput, MouseButtonInput};
pub use key_code::KeyCode;
pub use modifier::ModifierState;
pub use mouse::{AccumulatedMouseMotion, MouseButton};
pub use raw_event::{
    InputEvent, KeyboardEvent, MouseButtonEvent, MouseMotionEvent, MouseWheelEvent, TouchEvent,
    TouchPhase, WheelDelta,
};
