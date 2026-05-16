//! `recp` — M-RECORDER-V1 polish helpers (M-RECP.0..5 / AUT-261..266).
//!
//! Each sub-module lands a small pure-Rust scaffold for one of the
//! six polish tickets: deep-link to System Settings (M-RECP.0),
//! window positioning under tray-click (M-RECP.1), frame-rate budget
//! instrumentation (M-RECP.2), display-keep-awake (M-RECP.3),
//! resource-cleanup smoke (M-RECP.4), and hot-swap crossfade
//! (M-RECP.5).
//!
//! The OS-level wiring (objc2 / windows-rs / D-Bus / Tauri window
//! placement) is documented per module but **deferred** to follow-up
//! commits that need real hardware to verify. The state machines /
//! pure helpers / config types land now so:
//!
//! 1. The IPC + module surface is stable when the OS-wiring lands.
//! 2. Unit tests cover the testable surface today on every OS.
//! 3. M-RECORDER-V1 milestone close requires only swapping the
//!    deferred stubs for real OS calls.

pub mod crossfade;
pub mod fps_monitor;
pub mod keep_awake;
pub mod settings_deep_link;
pub mod tray_positioning;

pub use settings_deep_link::SettingsPane;
