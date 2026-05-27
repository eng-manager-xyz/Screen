//! web-sys (browser) adapter — translate DOM `PointerEvent`,
//! `WheelEvent`, `KeyboardEvent`, `TouchEvent` into our vocabulary.
//!
//! Feature-gated under `web` AND restricted to `target_arch = "wasm32"`
//! so the symbol table never appears on native builds.
//!
//! Translation conventions (matches the W3C specs where possible):
//! - **Pixel positions** come from `PointerEvent.client_x() / client_y()`
//!   — CSS pixels relative to the viewport. DPR scaling is the host
//!   adapter's job (chart-web's `wisp_chart_web::web` is the
//!   reference); we pass through whatever the browser reports.
//! - **`PointerEvent.button`** is the W3C button index (0=left,
//!   1=middle, 2=right, 3=back, 4=forward) — we map to our
//!   [`MouseButton`] enum.
//! - **`PointerEvent.pointer_type()`** distinguishes mouse vs touch
//!   vs pen — we collapse `"touch"` + `"pen"` into [`PointerId::Touch`]
//!   keyed by `pointer_id()`.
//! - **`WheelEvent.delta_mode()`** is `0`=pixel, `1`=line, `2`=page;
//!   we map 0→[`WheelDelta::Pixel`], 1→[`WheelDelta::Line`], 2→Pixel
//!   (treating page as oversized pixel — browsers rarely emit it).
//! - **`KeyboardEvent.code()`** is the W3C UI Events Code value —
//!   string form like `"KeyW"`, `"Digit1"`, `"ArrowUp"`. We parse
//!   into our [`KeyCode`] enum (drop unknown silently).
//! - **`KeyboardEvent.repeat()`** carries the auto-repeat flag.
//!
//! Pure translation functions — no `addEventListener` plumbing here;
//! that's the host (`wisp-interaction-web` for the chapter demos,
//! `screen-app` for the recorder). See `wisp_chart_web::web` for
//! a worked example of the listener-side wiring pattern.

use glam::Vec2;
use wasm_bindgen::JsCast;
use web_sys::{
    KeyboardEvent as WebKeyboardEvent, PointerEvent as WebPointerEvent, WheelEvent as WebWheelEvent,
};

use crate::input::{
    InputEvent, KeyCode, KeyboardEvent, ModifierState, MouseButton, MouseButtonEvent,
    MouseMotionEvent, MouseWheelEvent, TouchEvent, TouchPhase, WheelDelta,
};
use crate::pointer::{PointerId, PointerLocation};

/// Map a W3C `PointerEvent.button` index to our [`MouseButton`].
#[must_use]
pub fn translate_button(button: i16) -> MouseButton {
    match button {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::Back,
        4 => MouseButton::Forward,
        #[allow(
            clippy::cast_sign_loss,
            clippy::cast_possible_truncation,
            reason = "browser button indices are 0..15; sign loss + truncation cannot happen for any realistic value"
        )]
        other => MouseButton::Other(other as u16),
    }
}

/// Map the DOM `WheelEvent.deltaMode` enum + `delta_x / delta_y`.
#[must_use]
pub fn translate_wheel(event: &WebWheelEvent) -> WheelDelta {
    #[allow(
        clippy::cast_possible_truncation,
        reason = "browser delta values are within f32 range for any realistic wheel event"
    )]
    let delta = Vec2::new(event.delta_x() as f32, event.delta_y() as f32);
    match event.delta_mode() {
        WebWheelEvent::DOM_DELTA_LINE => WheelDelta::Line(delta),
        // DOM_DELTA_PAGE (2) is rare; treat as pixel rather than
        // adding a separate Page variant to WheelDelta — consumers
        // can always scale further if they need to.
        _ => WheelDelta::Pixel(delta),
    }
}

/// Modifier snapshot from any DOM event that exposes the standard
/// `shiftKey` / `ctrlKey` / `altKey` / `metaKey` properties.
#[must_use]
pub fn modifiers_from_mouse(event: &web_sys::MouseEvent) -> ModifierState {
    ModifierState {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        super_key: event.meta_key(),
    }
}

/// Same as [`modifiers_from_mouse`] but for `KeyboardEvent`.
#[must_use]
pub fn modifiers_from_keyboard(event: &WebKeyboardEvent) -> ModifierState {
    ModifierState {
        shift: event.shift_key(),
        ctrl: event.ctrl_key(),
        alt: event.alt_key(),
        super_key: event.meta_key(),
    }
}

/// Build a `PointerLocation` from a `PointerEvent`. Position uses
/// `client_x / client_y` — CSS pixels relative to the viewport.
#[must_use]
pub fn pointer_location(event: &WebPointerEvent) -> PointerLocation {
    #[allow(
        clippy::cast_precision_loss,
        reason = "client_x/y are i32; for any plausible viewport the value fits in f32"
    )]
    PointerLocation {
        viewport: Vec2::new(event.client_x() as f32, event.client_y() as f32),
        modifiers: modifiers_from_mouse(event.unchecked_ref()),
    }
}

/// Map a `PointerEvent` to our internal [`PointerId`].
///
/// `"mouse"` → [`PointerId::Mouse`]; `"touch"` and `"pen"` get the
/// `pointer_id()` (i32, cast to u64) under [`PointerId::Touch`].
#[must_use]
pub fn pointer_id_from_event(event: &WebPointerEvent) -> PointerId {
    let kind = event.pointer_type();
    if kind == "mouse" {
        PointerId::Mouse
    } else {
        #[allow(
            clippy::cast_sign_loss,
            reason = "pointer_id is browser-internal opaque; we just need a stable u64 for the lifetime of the contact"
        )]
        PointerId::Touch(event.pointer_id() as u64)
    }
}

/// Parse W3C UI Events `code` strings into our [`KeyCode`]. Returns
/// `None` for any code outside the enum. Reference:
/// <https://www.w3.org/TR/uievents-code/>.
#[must_use]
pub fn translate_key_code(code: &str) -> Option<KeyCode> {
    Some(match code {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "Enter" => KeyCode::Enter,
        "Space" => KeyCode::Space,
        "Escape" => KeyCode::Escape,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "MetaLeft" => KeyCode::SuperLeft,
        "MetaRight" => KeyCode::SuperRight,
        "CapsLock" => KeyCode::CapsLock,
        "Backquote" => KeyCode::Backquote,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Backslash" => KeyCode::Backslash,
        "Semicolon" => KeyCode::Semicolon,
        "Quote" => KeyCode::Quote,
        "Comma" => KeyCode::Comma,
        "Period" => KeyCode::Period,
        "Slash" => KeyCode::Slash,
        _ => return None,
    })
}

/// Synthesise an [`InputEvent::Keyboard`] from a DOM `KeyboardEvent`.
/// Returns `None` if the code isn't in our enum.
#[must_use]
pub fn keyboard_input_event(event: &WebKeyboardEvent, pressed: bool) -> Option<InputEvent> {
    let key = translate_key_code(&event.code())?;
    Some(InputEvent::Keyboard(KeyboardEvent {
        key,
        pressed,
        repeat: event.repeat(),
        modifiers: modifiers_from_keyboard(event),
    }))
}

/// Synthesise an [`InputEvent::MouseButton`] from a DOM `PointerEvent`.
#[must_use]
pub fn mouse_button_input_event(event: &WebPointerEvent, pressed: bool) -> InputEvent {
    InputEvent::MouseButton(MouseButtonEvent {
        button: translate_button(event.button()),
        pressed,
        modifiers: modifiers_from_mouse(event.unchecked_ref()),
    })
}

/// Synthesise an [`InputEvent::MouseMotion`] from a delta.
#[must_use]
pub fn mouse_motion_input_event(delta: Vec2) -> InputEvent {
    InputEvent::MouseMotion(MouseMotionEvent { delta })
}

/// Synthesise an [`InputEvent::MouseWheel`] from a DOM `WheelEvent`.
#[must_use]
pub fn mouse_wheel_input_event(event: &WebWheelEvent) -> InputEvent {
    InputEvent::MouseWheel(MouseWheelEvent {
        delta: translate_wheel(event),
        modifiers: modifiers_from_mouse(event.unchecked_ref()),
    })
}

/// Synthesise an [`InputEvent::Touch`] from a DOM `PointerEvent`.
/// The `phase` is the caller's responsibility (the listener that fired
/// knows whether this was a `pointerdown` / `pointermove` / `pointerup`
/// / `pointercancel`).
#[must_use]
pub fn touch_input_event(event: &WebPointerEvent, phase: TouchPhase) -> InputEvent {
    #[allow(
        clippy::cast_sign_loss,
        reason = "pointer_id is browser-opaque; we just need a stable u64"
    )]
    #[allow(
        clippy::cast_precision_loss,
        reason = "client_x/y are i32; fits in f32 for any plausible viewport"
    )]
    InputEvent::Touch(TouchEvent {
        id: event.pointer_id() as u64,
        position: Vec2::new(event.client_x() as f32, event.client_y() as f32),
        phase,
    })
}
