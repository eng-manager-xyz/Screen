//! `<CameraDiagnostics />` — small text overlay in the Recorder
//! surface that polls the M-CAM.3 worker for proof-of-life
//! (M-CAM.3 / AUT-257 diagnostic addition).
//!
//! Renders four lines while the camera pipeline is alive:
//!
//! ```text
//! Camera pipeline
//!   Status:   Running
//!   Source:   480×480 @ 30 fps
//!   Frames:   1247   (incrementing)
//! ```
//!
//! Plus, if the first-frame PNG dump succeeded this session, the
//! absolute path of the file so the user can open it and confirm
//! real pixel data hit Rust.
//!
//! Polls `__screenPreviewDiagnostics` every 500ms via
//! `setInterval`. Two reasons for 500ms over 1000ms or 100ms:
//!
//! * **500ms feels alive** without being twitchy. The frame counter
//!   moves visibly each tick (~15 frames at 30fps source).
//! * **2 Hz is well below the worker's 30 Hz frame rate**, so the
//!   IPC poll never serialises against the hot path (atomic reads
//!   only on the Rust side).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;

/// Mirror of `screen-app/src/preview/diagnostics.rs::DiagnosticsSnapshot`.
/// Kept in sync by hand; the IPC wire shape is the contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticsSnapshot {
    /// Total frames since `start_preview`.
    pub frames_received: u64,
    /// Source width in pixels; `0` until first frame.
    pub source_width: u32,
    /// Source height in pixels; `0` until first frame.
    pub source_height: u32,
    /// Source fps × 100; `0` until known.
    pub source_fps_hundredths: u32,
    /// First-frame PNG dump path; `None` until dump succeeds.
    pub first_frame_dump_path: Option<String>,
}

#[wasm_bindgen]
extern "C" {
    /// `__screenPreviewDiagnostics()` in `index.html` — resolves to
    /// the latest `DiagnosticsSnapshot` (or `null` outside Tauri).
    #[wasm_bindgen(js_name = __screenPreviewDiagnostics, catch)]
    pub async fn preview_diagnostics_js() -> Result<JsValue, JsValue>;
}

/// Polling interval in milliseconds. 500ms = 2 Hz, well below the
/// worker's 30 Hz frame rate so the IPC poll never serialises
/// against the hot path.
const POLL_INTERVAL_MS: i32 = 500;

/// `<CameraDiagnostics />` — proof-of-life overlay for the camera
/// pipeline. Renders nothing useful when running outside Tauri
/// (in `trunk serve`); the IPC returns `null` and the overlay
/// renders "(no pipeline)".
#[component]
pub fn CameraDiagnostics() -> impl IntoView {
    let snapshot = RwSignal::new(None::<DiagnosticsSnapshot>);

    // Set up the polling loop. `setInterval` via web-sys; we don't
    // bother cleaning up the handle on component unmount — the
    // Recorder surface is the AppShell's sole consumer + lives for
    // the whole window lifetime, so leak-on-unmount isn't a real
    // concern. (If we ever mount/unmount the Recorder surface, we'd
    // wire `on_cleanup` here.)
    Effect::new(move |_| {
        let Some(window) = web_sys::window() else {
            return;
        };
        let callback = Closure::<dyn FnMut()>::new(move || {
            wasm_bindgen_futures::spawn_local(async move {
                match preview_diagnostics_js().await {
                    Ok(value) if !value.is_null() && !value.is_undefined() => {
                        if let Ok(parsed) = serde_wasm_bindgen::from_value(value) {
                            snapshot.set(Some(parsed));
                        }
                    }
                    _ => {}
                }
            });
        });
        let _ = window.set_interval_with_callback_and_timeout_and_arguments_0(
            callback.as_ref().unchecked_ref(),
            POLL_INTERVAL_MS,
        );
        // Leak the closure — see the lifetime note above.
        callback.forget();
    });

    view! {
        <section class="camera-diagnostics" aria-label="Camera pipeline diagnostics">
            <header class="camera-diagnostics-header">"Camera pipeline"</header>
            {move || match snapshot.get() {
                None => view! {
                    <p class="camera-diagnostics-empty">"(waiting for first poll…)"</p>
                }.into_any(),
                Some(s) => render_snapshot(&s).into_any(),
            }}
        </section>
    }
}

/// Render a single non-empty snapshot. Pulled out of the component
/// so the formatting logic is testable without a Leptos runtime.
fn render_snapshot(s: &DiagnosticsSnapshot) -> impl IntoView {
    let source_text = if s.source_width == 0 && s.source_height == 0 {
        "(waiting for first frame…)".to_owned()
    } else {
        format!(
            "{w}×{h} @ {fps} fps",
            w = s.source_width,
            h = s.source_height,
            fps = format_fps(s.source_fps_hundredths),
        )
    };
    let frames_text = format!("{}", s.frames_received);
    let dump_path = s.first_frame_dump_path.clone();
    view! {
        <dl class="camera-diagnostics-rows">
            <dt>"Source:"</dt>
            <dd>{source_text}</dd>
            <dt>"Frames:"</dt>
            <dd class="camera-diagnostics-frames">{frames_text}</dd>
            {move || dump_path.clone().map(|p| view! {
                <>
                    <dt>"First-frame PNG:"</dt>
                    <dd class="camera-diagnostics-dump">{p}</dd>
                </>
            })}
        </dl>
    }
}

/// Format a fps-as-hundredths integer (e.g. `2997`) as a display
/// string (e.g. `"29.97"`). Avoids floating-point arithmetic so the
/// rendered string is byte-stable for testing.
#[must_use]
pub fn format_fps(fps_hundredths: u32) -> String {
    let whole = fps_hundredths / 100;
    let frac = fps_hundredths % 100;
    if frac == 0 {
        format!("{whole}")
    } else if frac.is_multiple_of(10) {
        format!("{whole}.{frac:01}", frac = frac / 10)
    } else {
        format!("{whole}.{frac:02}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_fps_displays_30_as_integer() {
        assert_eq!(format_fps(3000), "30");
    }

    #[test]
    fn format_fps_handles_29_97() {
        assert_eq!(format_fps(2997), "29.97");
    }

    #[test]
    fn format_fps_handles_24_drop() {
        assert_eq!(format_fps(2400), "24");
    }

    #[test]
    fn format_fps_handles_one_decimal() {
        // 29.5 fps → 2950 → "29.5".
        assert_eq!(format_fps(2950), "29.5");
    }

    #[test]
    fn format_fps_handles_zero() {
        assert_eq!(format_fps(0), "0");
    }

    #[test]
    fn snapshot_round_trips_serde() {
        let s = DiagnosticsSnapshot {
            frames_received: 42,
            source_width: 640,
            source_height: 480,
            source_fps_hundredths: 2997,
            first_frame_dump_path: Some("/tmp/first.png".to_owned()),
        };
        let json = serde_json::to_string(&s).unwrap();
        let back: DiagnosticsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back, s);
    }
}
