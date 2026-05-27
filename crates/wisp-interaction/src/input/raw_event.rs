//! Per-event raw input stream — `InputEvent` enum for consumers that
//! need per-event granularity (rare; `ButtonInput<T>` is the 80%
//! path).
//!
//! Use cases for the raw stream:
//!
//! - Auto-repeat detection (`ButtonInput` filters out repeats)
//! - Text input with diacritics / IME (`ButtonInput` is physical-key
//!   only; a future `TextInputEvent` belongs here)
//! - Recording / playback for tests
//!
//! Adapters push into a `std::sync::mpsc::Sender`; consumers read
//! from the matching `Receiver`. Single-producer-single-consumer is
//! the expected pattern (one adapter, one app loop).

use glam::Vec2;

use crate::input::key_code::KeyCode;
use crate::input::modifier::ModifierState;
use crate::input::mouse::MouseButton;

/// All raw per-event input notifications, before per-frame
/// `ButtonInput` aggregation.
#[derive(Debug, Clone)]
pub enum InputEvent {
    /// Key went down or up.
    Keyboard(KeyboardEvent),
    /// Mouse button went down or up.
    MouseButton(MouseButtonEvent),
    /// Pointer moved. Coordinates are in CSS / logical pixels
    /// relative to the host surface's top-left.
    MouseMotion(MouseMotionEvent),
    /// Wheel rotated.
    MouseWheel(MouseWheelEvent),
    /// Single touch point updated.
    Touch(TouchEvent),
    /// Window / canvas lost input focus — every consumer should
    /// `release_all()` to avoid stuck keys. Adapters synthesise this.
    FocusLost,
}

/// Per-event keyboard payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyboardEvent {
    /// Which key.
    pub key: KeyCode,
    /// Down (true) or up (false).
    pub pressed: bool,
    /// Whether the OS reported this as a "repeat" autopress
    /// (different from a genuine new press). `ButtonInput` filters
    /// these out; the raw stream preserves them.
    pub repeat: bool,
    /// Modifier snapshot at the moment of the press.
    pub modifiers: ModifierState,
}

/// Per-event mouse-button payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MouseButtonEvent {
    /// Which button.
    pub button: MouseButton,
    /// Down (true) or up (false).
    pub pressed: bool,
    /// Modifier snapshot at the moment of the click.
    pub modifiers: ModifierState,
}

/// Per-event mouse-motion payload. Delta is "since the last motion
/// event" not "since the last frame" — for per-frame totals, use
/// [`crate::input::AccumulatedMouseMotion`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseMotionEvent {
    /// Motion delta in CSS / logical pixels.
    pub delta: Vec2,
}

/// Per-event wheel payload. `delta` semantics depend on the
/// platform's wheel unit — most browsers report PIXEL; macOS
/// trackpads report PIXEL; native Linux + some mice report LINE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MouseWheelEvent {
    /// Scroll delta. X = horizontal, Y = vertical. Positive Y is
    /// "scroll down" (matches DOM `WheelEvent.deltaY`); positive X
    /// is "scroll right".
    pub delta: WheelDelta,
    /// Modifier snapshot.
    pub modifiers: ModifierState,
}

/// Wheel delta units. Matches DOM `WheelEvent.deltaMode`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelDelta {
    /// Delta in CSS pixels (most common).
    Pixel(Vec2),
    /// Delta in line-heights (some mice, some Linux setups).
    Line(Vec2),
}

/// Per-event touch payload.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchEvent {
    /// OS-assigned touch identifier; multi-touch gestures key off
    /// this. Stable for the lifetime of a single finger contact.
    pub id: u64,
    /// Position in CSS / logical pixels.
    pub position: Vec2,
    /// Lifecycle phase.
    pub phase: TouchPhase,
}

/// Touch lifecycle phase. Matches winit + DOM Touch Events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    /// Finger landed.
    Started,
    /// Finger moved while still touching.
    Moved,
    /// Finger lifted normally.
    Ended,
    /// OS interrupted the touch (incoming notification, etc.).
    Cancelled,
}
