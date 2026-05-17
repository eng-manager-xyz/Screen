#![allow(
    clippy::doc_markdown,
    reason = "scaffolded module docs use bare GStreamer / MediaTime / MediaClock in prose; M-MEDIA.1..22 chunks rewrite each module's docs as the real types land"
)]

//! `media` — GStreamer-backed audio + video capture, playback orchestration,
//! and the data models that `wisp` (renderer) and `app` (Tauri/Leptos
//! shell) consume.
//!
//! # Three-way split
//!
//! Boundaries are load-bearing. Crossing them once would bloat every wisp
//! consumer (storybook, headless export, future plugins) with GStreamer's
//! build + license footprint.
//!
//! - **`media` (this crate)** owns: GStreamer capture (audio + video),
//!   GStreamer playback, audio/video data models ([`audio::AudioChunk`],
//!   `decode::VideoFrame` re-exported under [`video`]), the
//!   [`clock::MediaClock`] + [`clock::MediaTime`] timing model,
//!   [`histogram::AudioHistogram`] quantization, the
//!   `manifest::RecordingManifest` session descriptor (planned —
//!   see [`manifest`] for the scaffolded module), and the device
//!   enumeration model.
//! - **`wisp`** owns visual composition only. It **must not** depend on
//!   this crate's GStreamer integration. It receives `VideoFrame` /
//!   `Texture` handles, waveform-bar geometry, cursor state, and timeline
//!   timestamps via typed structs — and renders them through its sprite +
//!   graphics pipelines.
//! - **`app` (Tauri + Leptos)** owns UI orchestration. It calls this
//!   crate's [Leptos/Tauri integration seam](#leptostauri-seam) and the
//!   wisp renderer; it does not poke GStreamer or `wgpu` directly.
//!
//! # Layering
//!
//! ```text
//!  ┌──────────────────────────────────────────────┐
//!  │ app  (Tauri shell + Leptos UI)              │
//!  │   - calls `media::commands::*`              │
//!  │   - feeds `wisp` typed data                 │
//!  └────────────┬──────────────────┬──────────────┘
//!               │                  │
//!     ┌─────────▼────────┐   ┌─────▼────────────┐
//!     │   media (this)   │   │     wisp         │
//!     │  - GStreamer     │   │  - render        │
//!     │  - timing model  │   │  - sprite/graphics│
//!     │  - histogram     │   │  - text + mask   │
//!     │  - manifest      │   │  - filter + blend│
//!     └────────┬─────────┘   └──────────────────┘
//!              │      typed data (VideoFrame,
//!              │      WaveformBarRect, MediaTime, …)
//!              ▼
//!         (no direct dep on wisp)
//! ```
//!
//! # Build-on-decode
//!
//! [`decode::VideoFrame`] + [`decode::VideoStream`] already carry the
//! BGRA-frame contract used throughout the project. This crate re-exports
//! them under [`video`] rather than reinventing the wheel. M-MEDIA.6
//! (`gstreamer::video_test_source`) builds on `decode`'s
//! `GstreamerPipeStream`.
//!
//! # GStreamer integration choice
//!
//! CLI-pipe pattern — spawn `gst-launch-1.0` as a child process and pipe
//! raw bytes through `fdsrc` / `fdsink`. Documented in
//! [`gstreamer`]. No compile-time `libgstreamer` dependency.
//!
//! # Leptos/Tauri seam
//!
//! M-MEDIA.21 (AUT-117) lands the command/service surface at
//! `media::commands::*` — `list_devices`, `start_capture`,
//! `stop_capture`, `current_clock`, `latest_histogram_window`,
//! `latest_video_handle`. Until then, this module is a placeholder.
//!
//! # Track status
//!
//! Each M-MEDIA chunk lands as its own commit on `m-media` branch.
//! Sub-modules fill in chunk-by-chunk:
//!
//! | Module | Chunk | Status |
//! |---|---|---|
//! | [`gstreamer`] | M-MEDIA.1 (AUT-97) | scaffolded |
//! | [`clock`] | M-MEDIA.2 (AUT-98) | scaffolded |
//! | [`audio`] | M-MEDIA.3 (AUT-99) | scaffolded |
//! | [`video`] | (re-export) | scaffolded |
//! | [`histogram`] | M-MEDIA.8 (AUT-104) | scaffolded |
//! | [`manifest`] | M-MEDIA.20 (AUT-116) | scaffolded |

pub mod audio;
pub mod camera;
pub mod clock;
pub mod gstreamer;
pub mod gstreamer_audio;
pub mod gstreamer_video;
pub mod histogram;
pub mod manifest;
pub mod microphone;
pub mod mock_audio;
pub mod sync;
pub mod video;
pub mod waveform;

pub use audio::{AudioChunk, AudioChunkError, AudioFormat, SampleFormat};
pub use camera::{CameraDevice, list_cameras};
pub use clock::{MediaClock, MediaDuration, MediaTime, Timestamped};
pub use histogram::{AudioBar, AudioHistogram};
pub use microphone::{MicrophoneDevice, list_microphones};
pub use mock_audio::{SilenceSource, SineWaveSource, StepPulseSource};
pub use video::{VideoFrame, VideoStream};
pub use waveform::{
    BarMetric, WaveformBarRect, WaveformDisplayMode, WaveformLayout, mono_bars, stereo_bars,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_re_exports_decode_video_frame() {
        // Smoke: the crate boundary surfaces VideoFrame so downstream
        // callers don't have to depend on `decode` directly.
        let _: fn() -> Option<VideoFrame> = || None;
    }

    #[test]
    fn crate_re_exports_decode_video_stream_trait() {
        // Smoke: VideoStream is the contract that gstreamer + mock
        // implementations both satisfy.
        fn assert_object_safe<T: ?Sized>(_: &T) {}
        // Trait existence check — compile-time only.
        let _ = std::marker::PhantomData::<dyn VideoStream>;
        assert_object_safe(&std::marker::PhantomData::<dyn VideoStream>);
    }
}
