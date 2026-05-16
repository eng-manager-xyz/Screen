//! Tauri tray-icon plumbing (M-TRAY.0 / AUT-249).
//!
//! Owns the OS-level menubar / tray icon and the toggle state machine
//! that decides whether a left-click should show or hide the bound
//! popover window.
//!
//! The [`toggle`] submodule holds the pure-Rust state machine that is
//! testable cross-OS without a Tauri runtime; the integration with
//! Tauri's [`tauri::tray::TrayIconBuilder`] lives in `main.rs`.

pub mod toggle;
