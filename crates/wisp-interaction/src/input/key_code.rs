//! `KeyCode` — wisp-interaction's physical-key enum.
//!
//! Mirrors the subset of `winit::keyboard::KeyCode` (which itself
//! mirrors W3C UI Events code values) that actually shows up in
//! desktop / web applications. We deliberately don't carry
//! `Unidentified`, `Dead*`, or the long tail of localised punctuation
//! keys — adapters drop unsupported keys silently rather than
//! widening this enum past usefulness.
//!
//! The 1:1 mapping table to / from winit `KeyCode` lives in the
//! `adapter::winit` module (WI.6). The corresponding mapping to
//! browser `KeyboardEvent.code` strings lives in `adapter::web`
//! (WI.7) — `code` is the physical-key string per
//! <https://www.w3.org/TR/uievents-code/>.

/// Physical key identifier.
///
/// Naming follows winit's `PhysicalKey::Code` — `KeyW` for the W
/// key, `Digit1` for `1`, etc. — so adapter translation is a single
/// `match` statement.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(
    missing_docs,
    reason = "letter-key variants are self-explanatory; documenting each adds noise without value"
)]
pub enum KeyCode {
    // Letters
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // Digits (top row, not numpad).
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // Function row.
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Whitespace / line-ending.
    Space,
    Enter,
    Tab,
    Backspace,
    Escape,

    // Navigation.
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    Delete,

    // Modifiers (also reported via ModifierState for "is held").
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,
    SuperLeft,
    SuperRight,
    CapsLock,

    // Punctuation that ships in 99% of layouts.
    Backquote,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
}
