//! JS-bridge bindings for the M-SAVE.0 recorder settings — the
//! persisted output directory + the native folder picker.
//!
//! Mirror of [`crate::recording_ipc`]. The `__screenPickOutputDir` /
//! `__screenGetOutputDir` / `__screenSetOutputDir` helpers in
//! `index.html` wrap `window.__TAURI__.core.invoke(...)`. Consumed by
//! the post-record Save panel (M-SAVE.3) and the "Recording folder"
//! settings row (M-SAVE.4).
//!
//! The `extern "C"` block is intentionally **not** `cfg`-gated to
//! wasm32: `#[wasm_bindgen]` externs compile on native targets too
//! (they just can't be invoked there), which keeps
//! `cargo check --workspace` green on the host triple. The wrapper
//! functions are only ever called from the browser.

use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    /// `__screenPickOutputDir()` — opens the native folder dialog.
    /// `Promise<string | null>` (null on cancel).
    #[wasm_bindgen(js_name = __screenPickOutputDir, catch)]
    async fn pick_output_dir_js() -> Result<JsValue, JsValue>;

    /// `__screenGetOutputDir()` — `Promise<string>` returning the
    /// configured output directory (persisted override or per-OS
    /// default; never empty).
    #[wasm_bindgen(js_name = __screenGetOutputDir, catch)]
    async fn get_output_dir_js() -> Result<JsValue, JsValue>;

    /// `__screenSetOutputDir(dir)` — `Promise<void>`. Persists `dir`
    /// as the default; empty string clears the override.
    #[wasm_bindgen(js_name = __screenSetOutputDir, catch)]
    async fn set_output_dir_js(dir: String) -> Result<JsValue, JsValue>;
}

/// Open the native folder picker. Returns the chosen absolute path, or
/// `None` when the user cancelled (or on IPC failure — the caller
/// treats both the same: keep the current directory).
pub async fn pick_output_dir() -> Option<String> {
    match pick_output_dir_js().await {
        Ok(value) => value.as_string().filter(|s| !s.is_empty()),
        Err(_) => None,
    }
}

/// Read the currently-configured output directory. Returns an empty
/// string on IPC failure (caller falls back to a placeholder label).
pub async fn get_output_dir() -> String {
    match get_output_dir_js().await {
        Ok(value) => value.as_string().unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// Persist `dir` as the default output directory. Empty string clears
/// the override (reverts to the per-OS default).
///
/// # Errors
///
/// Returns the IPC error string when the persist fails.
pub async fn set_output_dir(dir: &str) -> Result<(), String> {
    set_output_dir_js(dir.to_owned())
        .await
        .map(|_| ())
        .map_err(|err| err.as_string().unwrap_or_else(|| format!("{err:?}")))
}
