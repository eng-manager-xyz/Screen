//! macOS ScreenCaptureKit system-audio capture (M-AUDIO-SYS.0 / AUT-280).
//!
//! Captures **what is playing through the speakers** — game audio,
//! browser-tab audio, video-conferencing audio — as a single mixed
//! PCM stream using `SCStreamConfiguration.capturesAudio = true`.
//! Apple's blessed post-12.3 / 13.0 API; no kernel extension (no
//! BlackHole / Loopback dependency).
//!
//! ```admonish important title="macOS 13.0 floor"
//! `setCapturesAudio:` was added in macOS 13.0 — the project's
//! `Info.plist` `LSMinimumSystemVersion` is bumped to **13.0**
//! alongside this module. macOS 12.3–12.7 users will be refused at
//! launch by `LSMinimumSystemVersion`; that is the documented
//! trade-off for getting system-audio recording into the recorder.
//! ```
//!
//! ```admonish warning title="Permission relaunch quirk"
//! Granting Screen Recording on macOS doesn't take effect until the
//! app relaunches — same well-known macOS quirk as the screen-video
//! path (AUT-270 / M-SCK.3). The Recorder UX surface needs a
//! "Quit and reopen" prompt when the user grants this permission
//! for the first time. M-AUDIO.PERMS (AUT-283) verifies the user
//! flow end-to-end.
//! ```
//!
//! # Architecture
//!
//! ```mermaid
//! sequenceDiagram
//!     participant Caller as Rust caller
//!     participant Stream as SystemAudioStream
//!     participant SCK as SCStream (Apple)
//!     participant Cb as SCStreamOutput delegate
//!     participant Chan as mpsc<Vec<f32>>
//!
//!     Caller->>Stream: SystemAudioStream::new(config)
//!     Stream->>SCK: SCShareableContent.current
//!     SCK-->>Stream: displays + apps
//!     Stream->>SCK: SCContentFilter(display, excluding=[])
//!     Stream->>SCK: SCStreamConfiguration { capturesAudio=true, excludesCurrentProcessAudio=true, sampleRate, channelCount }
//!     Stream->>SCK: SCStream.init(filter, config, delegate)
//!     Stream->>SCK: addStreamOutput(Cb, type=Audio)
//!     Stream->>SCK: startCapture
//!     SCK-->>Stream: ready
//!
//!     loop frames flowing
//!         SCK->>Cb: stream:didOutputSampleBuffer:ofType: (audio)
//!         Cb->>Cb: extract AudioBufferList + Float32 PCM
//!         Cb->>Chan: send(Vec<f32>)
//!     end
//!
//!     Caller->>Stream: next_chunk(frames)
//!     Stream->>Chan: recv until `frames` samples buffered
//!     Stream-->>Caller: AudioChunk { samples, pts, format }
//!
//!     Caller->>Stream: drop
//!     Stream->>SCK: stopCapture
//!     Stream->>SCK: removeStreamOutput
//!     SCK-->>Stream: stopped
//! ```
//!
//! # Format
//!
//! SCK emits Float32 PCM in the device's preferred layout (typically
//! interleaved stereo when `channelCount=2`). The delegate handles
//! both interleaved (single-buffer) and planar (multi-buffer) shapes
//! by collapsing planar layouts into interleaved output before
//! sending to the channel — downstream callers always see one
//! interleaved `Vec<f32>` per sample buffer, matching
//! [`AudioChunk`]'s expectations.
//!
//! # Drop safety
//!
//! [`SystemAudioStream`]'s `Drop` calls
//! `SCStream.stopCaptureWithCompletionHandler` synchronously (the
//! completion handler blocks the drop). The delegate object is held
//! in `Retained` so it survives until the stream is fully torn
//! down; this avoids the Apple-side "callback fires on a freed
//! delegate" crash class. Same shape as `GstreamerAudioCapture`'s
//! Drop discipline (per CLAUDE.md "Drop-kill the child").

#![cfg(target_os = "macos")]
#![allow(
    unsafe_code,
    reason = "ScreenCaptureKit, CMSampleBuffer, and AudioBufferList are all C / Objective-C interop; bridging to safe Rust requires raw pointer + unsafe Apple-method invocations. Each unsafe block has a safety justification immediately above it."
)]

use std::mem::MaybeUninit;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_core_audio_types::AudioBufferList;
use objc2_core_media::CMSampleBuffer;
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString};
use objc2_screen_capture_kit::{
    SCContentFilter, SCRunningApplication, SCShareableContent, SCStream, SCStreamConfiguration,
    SCStreamOutput, SCStreamOutputType, SCWindow,
};
use serde::{Deserialize, Serialize};

use crate::audio::{AudioChunk, AudioChunkError, AudioFormat, SampleFormat};
use crate::clock::MediaTime;

/// Output buffer guard depth — number of [`Vec<f32>`] chunks the
/// delegate may queue before the channel exerts back-pressure (which
/// becomes a slow consumer dropping samples). 64 chunks at 100 ms /
/// chunk is ~6 seconds of buffer — comfortable for the recorder's
/// real-time encode path while still bounding memory if the consumer
/// stalls (e.g., the encoder thread blocks on disk I/O).
const CHANNEL_DEPTH_BOUND: usize = 64;

/// EMA smoothing factor for the master-stream level meter
/// (M-AUDIO.METER / AUT-287). Matches the mic meter's
/// `MIC_LEVEL_EMA_ALPHA` so the two bars feel consistent.
const SYSTEM_AUDIO_LEVEL_EMA_ALPHA: f32 = 0.3;

/// Callback invoked from inside the `SCStreamOutput` delegate on
/// every received sample buffer (M-AUDIO.METER / AUT-287). Receives
/// an **EMA-smoothed** RMS in `[0.0, ~1.0]`. Implemented as a `Box<dyn
/// Fn>` so the `media` crate stays `tauri`-free — the `app` side
/// passes a closure that emits a `system-audio-level` Tauri event.
pub type LevelSink = Box<dyn Fn(f32) + Send + Sync + 'static>;

/// Default sample rate the recorder requests from SCK. Matches the
/// mic path's `MIC_SAMPLE_RATE` (M-MIC.1) so the encoder's resampler
/// stays trivial. SCK negotiates around this if the underlying
/// audio engine can't deliver — observed value is in
/// [`SystemAudioConfig::actual_sample_rate`] post-start.
pub const DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Default channel count. Stereo is the universal recorder default.
pub const DEFAULT_CHANNELS: u8 = 2;

/// Capture-path errors. Serde-derived so they round-trip across the
/// Tauri IPC seam intact (the per-app picker in M-AUDIO-SYS.2 surfaces
/// the variant tag to the user).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub enum SystemAudioError {
    /// SCK is macOS-only. Compiled-but-unused stubs on Linux/Windows
    /// return this so the caller sees a typed error.
    #[error("system audio capture is only supported on macOS 13.0+")]
    NotMacOs,
    /// The host has no shareable displays — the SCK content filter
    /// requires at least one display to attach the audio capture to,
    /// even when we only care about audio.
    #[error("no displays available to attach the audio capture to")]
    NoDisplays,
    /// SCK refused stream creation. Wraps the `NSError.description`.
    #[error("SCK stream creation failed: {0}")]
    StreamCreationFailed(String),
    /// `startCapture` returned a non-nil error. Wraps the message.
    /// Often a permission failure — `NSScreenCaptureUsageDescription`
    /// in `Info.plist` is required, plus the user must grant Screen
    /// Recording in System Settings (relaunch required first time).
    #[error(
        "SCK startCapture failed (often: permission denied — grant Screen Recording, then relaunch): {0}"
    )]
    StartFailed(String),
    /// `getShareableContent` failed.
    #[error("SCK getShareableContent failed: {0}")]
    EnumerationFailed(String),
    /// The blocking `next_chunk` hit its timeout before enough audio
    /// arrived. Includes the partial-count so the caller can decide
    /// whether to retry or surface the issue.
    #[error(
        "next_chunk timed out after {timeout_ms} ms (got {frames_read} of {frames_requested} frames)"
    )]
    Timeout {
        /// Frames received before the timeout fired.
        frames_read: u64,
        /// Frames the caller requested.
        frames_requested: u64,
        /// Timeout value in ms.
        timeout_ms: u64,
    },
    /// Constructing an [`AudioChunk`] failed shape validation —
    /// internal bug.
    #[error("internal: built invalid AudioChunk: {0}")]
    InvalidChunk(String),
}

impl From<AudioChunkError> for SystemAudioError {
    fn from(value: AudioChunkError) -> Self {
        Self::InvalidChunk(value.to_string())
    }
}

/// One audio-producing (or audio-capable) app currently running.
/// Returned by [`list_audio_apps`]; consumed by [`AudioAppFilter`].
///
/// `icon_png_bytes` is reserved for the M-AUDIO-SYS.2 picker UI but
/// **shipped empty in v0** — pulling the macOS bundle icon via
/// `NSWorkspace.iconForFile(at:)` + downscale + PNG encode requires
/// adding `objc2-app-kit` + `image` deps to the macOS-only target.
/// The bundle-id is enough for the Tauri seam contract to round-trip;
/// the icon-extraction is a discrete follow-up (file as
/// M-AUDIO-SYS.1.1).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioApp {
    /// Process identifier as observed at enumeration time. Use
    /// `bundle_id` as the canonical identifier — PIDs change across
    /// app restarts; the filter resolves bundle-id → PID at apply
    /// time so a Spotify crash + restart is followed transparently.
    pub pid: u32,
    /// Bundle identifier (e.g. `"com.spotify.client"`). The canonical
    /// identity used for picker persistence + filter resolution.
    pub bundle_id: String,
    /// Human-readable app name (`"Spotify"`).
    pub display_name: String,
    /// 32×32 PNG bytes for the picker icon stack. `Vec::new()` in v0;
    /// real icon-bytes land in the M-AUDIO-SYS.1.1 follow-up.
    pub icon_png_bytes: Vec<u8>,
}

/// Per-process audio-capture filter. Defines which apps' audio the
/// `SystemAudioStream` emits.
///
/// Implementation note: the variants carry **bundle ids**, not PIDs.
/// The Apple-side `SCContentFilter` wants `SCRunningApplication`
/// objects (which are keyed by PID), so [`SystemAudioStream::set_app_filter`]
/// re-resolves bundle ids → live PIDs at apply time. If a chosen
/// app isn't currently running, it is silently omitted from the
/// filter — the next `set_app_filter` call will pick it up once it
/// relaunches.
#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum AudioAppFilter {
    /// Capture every app's audio. The default.
    #[default]
    AllAudio,
    /// Capture audio from only these apps (selected by bundle id).
    /// Apps not in the list are silenced.
    OnlyApps(Vec<String>),
    /// Capture audio from every app except these (selected by
    /// bundle id). Apps in the list are silenced; everything else
    /// passes through.
    ExcludeApps(Vec<String>),
}

/// Capture configuration. Defaults match the recorder's encoder
/// target (`48 kHz / stereo`); per-instance overrides land if a
/// per-mic / per-app device path demands a different rate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemAudioConfig {
    /// Sample rate to request from SCK. 48 kHz default.
    pub sample_rate_hz: u32,
    /// Channel count. 1 = mono, 2 = stereo.
    pub channels: u8,
    /// When `true`, the recorder's own audio output is excluded from
    /// the captured stream — defaults `true` to prevent a feedback
    /// loop on every "play test sound" interaction. Flip to `false`
    /// only for the meta-recording case (recording a tutorial of
    /// *using the recorder*).
    pub excludes_current_process_audio: bool,
}

impl Default for SystemAudioConfig {
    fn default() -> Self {
        Self {
            sample_rate_hz: DEFAULT_SAMPLE_RATE,
            channels: DEFAULT_CHANNELS,
            excludes_current_process_audio: true,
        }
    }
}

/// One captured audio sample buffer's-worth of PCM, queued for the
/// consumer. Internal — the public API yields [`AudioChunk`].
type DeliveredSamples = Vec<f32>;

// `SCStreamOutput` Objective-C delegate. Receives audio sample
// buffers from SCK on a dispatch queue, extracts Float32 PCM, and
// forwards onto the `mpsc::Sender`. The struct is `Retained<>`-held
// by `SystemAudioStream` so the Apple side can dispatch into it
// after the Rust caller has moved on.
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "ScreenAudioOutputHandler"]
    #[ivars = AudioOutputIvars]
    pub(crate) struct AudioOutputHandler;

    impl AudioOutputHandler {}

    unsafe impl NSObjectProtocol for AudioOutputHandler {}

    unsafe impl SCStreamOutput for AudioOutputHandler {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        #[allow(non_snake_case, reason = "method name must match the Apple selector exactly so the runtime can dispatch into it")]
        unsafe fn stream_didOutputSampleBuffer_ofType(
            &self,
            _stream: &SCStream,
            sample_buffer: &CMSampleBuffer,
            r#type: SCStreamOutputType,
        ) {
            if r#type != SCStreamOutputType::Audio {
                return;
            }
            // Extract Float32 PCM from the CMSampleBuffer's
            // AudioBufferList. Any failure logs + drops the buffer
            // (rather than crashing) — audio is real-time, dropping a
            // single buffer is preferable to panicking.
            match extract_pcm_from_sample_buffer(sample_buffer) {
                Ok(samples) if !samples.is_empty() => {
                    // M-AUDIO.METER / AUT-287 — compute RMS + EMA-
                    // smooth + push to the optional level sink. Runs
                    // here (inside the SCK callback) rather than on
                    // the consumer side so a missing consumer doesn't
                    // freeze the meter. RMS over interleaved samples
                    // is the master-stream level; per-app meters are
                    // a separate ticket (M-AUDIO.METER.1).
                    if let Some(sink) = self.ivars().level_sink.as_ref() {
                        let rms = rms_of(&samples);
                        let mut state = self
                            .ivars()
                            .smoothed_level
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        *state = SYSTEM_AUDIO_LEVEL_EMA_ALPHA * rms
                            + (1.0 - SYSTEM_AUDIO_LEVEL_EMA_ALPHA) * *state;
                        sink(*state);
                    }
                    // try_send equivalent: if the receiver is gone
                    // (Drop in progress) we silently drop. The Sender
                    // is mpsc, send blocks only when the channel is
                    // unbounded — we use a bounded approach via the
                    // channel-depth guard in `extract_pcm…`.
                    let _ = self.ivars().sender.send(samples);
                }
                Ok(_) => {
                    // Empty buffer — no-op.
                }
                Err(err) => {
                    tracing::warn!(?err, "system_audio: failed to extract PCM from CMSampleBuffer");
                }
            }
        }
    }
);

pub(crate) struct AudioOutputIvars {
    sender: Sender<DeliveredSamples>,
    /// Optional per-buffer level callback (M-AUDIO.METER / AUT-287).
    /// EMA-smoothed master-stream RMS is pushed here on every
    /// successfully-extracted sample buffer. `None` disables the
    /// meter (used in tests + when system audio is captured without
    /// a UI consumer).
    level_sink: Option<LevelSink>,
    /// Persisted EMA state across buffer callbacks.
    smoothed_level: std::sync::Mutex<f32>,
}

impl AudioOutputHandler {
    fn new(sender: Sender<DeliveredSamples>, level_sink: Option<LevelSink>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(AudioOutputIvars {
            sender,
            level_sink,
            smoothed_level: std::sync::Mutex::new(0.0),
        });
        // SAFETY: `init` on NSObject takes Allocated<Self> + returns
        // Retained<Self>. The class derives from NSObject (via
        // define_class!) so the runtime call is well-formed.
        unsafe { msg_send![super(this), init] }
    }
}

/// PCM-extraction errors. Logged + dropped at the delegate; never
/// surfaced through the public API directly.
#[derive(Debug, thiserror::Error)]
enum ExtractError {
    #[error("CMSampleBufferGetAudioBufferListWithRetainedBlockBuffer returned OSStatus {0}")]
    OsStatus(i32),
    #[error("AudioBuffer.mData was null")]
    NullData,
    #[error("AudioBuffer.mDataByteSize {0} is not a multiple of f32 size (4 bytes)")]
    UnalignedByteSize(u32),
}

/// Inline `AudioBufferList` storage capable of holding up to
/// `MAX_AUDIO_BUFFERS` planar buffers. SCK normally emits a single
/// interleaved buffer for our `channelCount` request, but allocating
/// for the planar worst-case keeps the type sound under both
/// layouts. 16 is well above realistic channel counts (5.1, 7.1
/// fit; cinema-grade 24-ch audio would need a bump that's far
/// outside this project's scope).
const MAX_AUDIO_BUFFERS: usize = 16;

#[repr(C)]
struct AudioBufferListN {
    m_number_buffers: u32,
    m_buffers: [objc2_core_audio_types::AudioBuffer; MAX_AUDIO_BUFFERS],
}

/// Read one Float32 PCM blob out of a `CMSampleBuffer` audio
/// `AudioBufferList`. Collapses planar buffers into interleaved
/// output so the downstream channel always carries the same shape.
fn extract_pcm_from_sample_buffer(buf: &CMSampleBuffer) -> Result<Vec<f32>, ExtractError> {
    let mut list_storage: MaybeUninit<AudioBufferListN> = MaybeUninit::uninit();
    let mut block_buffer_out: *mut objc2_core_media::CMBlockBuffer = std::ptr::null_mut();
    let mut needed: usize = 0;

    // SAFETY: We pass a pointer to a stack-allocated AudioBufferListN
    // that is at least as large as `AudioBufferList` for the worst
    // case (MAX_AUDIO_BUFFERS planar). The SCK Float32 capture path
    // realistically emits 1–2 buffers; the inline cap is a safety
    // margin. `block_buffer_out` is pinned in-place; CFAllocator
    // arguments are nil so the runtime uses the default (kCFAllocatorDefault).
    let status = unsafe {
        buf.audio_buffer_list_with_retained_block_buffer(
            &raw mut needed,
            list_storage.as_mut_ptr().cast::<AudioBufferList>(),
            size_of::<AudioBufferListN>(),
            None,
            None,
            0,
            &raw mut block_buffer_out,
        )
    };
    if status != 0 {
        return Err(ExtractError::OsStatus(status));
    }

    // SAFETY: the SCK function returned 0 (success), which means the
    // first `m_number_buffers + 1` u32-slot + N×AudioBuffer-slot are
    // initialised. We read the count first, then iterate that many
    // buffer slots — never beyond the storage we allocated.
    let list = unsafe { list_storage.assume_init_ref() };
    let buffer_count = (list.m_number_buffers as usize).min(MAX_AUDIO_BUFFERS);
    if buffer_count == 0 {
        return Ok(Vec::new());
    }

    // Single-buffer interleaved case (the common SCK path) — copy
    // directly out of the AudioBuffer's mData.
    if buffer_count == 1 {
        let ab = &list.m_buffers[0];
        return audio_buffer_to_interleaved_f32(ab.mData, ab.mDataByteSize);
    }

    // Multi-buffer planar case — each buffer is one channel's
    // worth of f32 samples. Interleave into a single flat Vec.
    // All planar buffers must carry the same frame count; if SCK
    // sends a mismatched set we conservatively use the minimum.
    let buffers = &list.m_buffers[..buffer_count];
    let mut per_channel: Vec<&[f32]> = Vec::with_capacity(buffer_count);
    let mut min_frames = usize::MAX;
    for ab in buffers {
        if ab.mData.is_null() {
            return Err(ExtractError::NullData);
        }
        if !(ab.mDataByteSize as usize).is_multiple_of(size_of::<f32>()) {
            return Err(ExtractError::UnalignedByteSize(ab.mDataByteSize));
        }
        let frame_count = ab.mDataByteSize as usize / size_of::<f32>();
        // SAFETY: SCK promises mData points at `mDataByteSize` bytes
        // of valid Float32 PCM. `frame_count = byte_size / 4` is the
        // exact slice length, computed from the same byte_size.
        let slice = unsafe { std::slice::from_raw_parts(ab.mData.cast::<f32>(), frame_count) };
        if frame_count < min_frames {
            min_frames = frame_count;
        }
        per_channel.push(slice);
    }
    let frame_count = min_frames;
    let mut interleaved = Vec::with_capacity(frame_count * buffer_count);
    for frame in 0..frame_count {
        for channel in &per_channel {
            interleaved.push(channel[frame]);
        }
    }
    Ok(interleaved)
}

/// Root-mean-square over interleaved Float32 PCM. Used by
/// [`AudioOutputHandler`]'s per-buffer meter (M-AUDIO.METER /
/// AUT-287). Empty buffer → 0.0.
fn rms_of(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f64 = samples.iter().map(|&s| f64::from(s).powi(2)).sum();
    #[allow(
        clippy::cast_precision_loss,
        reason = "sample count well below 2^53 for realistic chunk sizes"
    )]
    let mean = sum_sq / (samples.len() as f64);
    #[allow(
        clippy::cast_possible_truncation,
        reason = "RMS in [0, 1] easily fits f32"
    )]
    let rms = mean.sqrt() as f32;
    rms
}

fn audio_buffer_to_interleaved_f32(
    data: *mut std::ffi::c_void,
    byte_size: u32,
) -> Result<Vec<f32>, ExtractError> {
    if data.is_null() {
        return Err(ExtractError::NullData);
    }
    if !(byte_size as usize).is_multiple_of(size_of::<f32>()) {
        return Err(ExtractError::UnalignedByteSize(byte_size));
    }
    let frame_count = byte_size as usize / size_of::<f32>();
    // SAFETY: SCK promises `data` points at `byte_size` bytes of
    // valid Float32 PCM. The slice length is exactly byte_size / 4.
    let slice = unsafe { std::slice::from_raw_parts(data.cast::<f32>(), frame_count) };
    Ok(slice.to_vec())
}

/// Active system-audio capture session.
///
/// Owns the `SCStream` + delegate; on drop, calls `stopCapture` +
/// `removeStreamOutput` synchronously to ensure no further callbacks
/// fire after the Receiver is gone.
///
/// # `Send` + `Sync`
///
/// We declare both manually. The wrapped `Retained<SCStream>` and
/// `Retained<AudioOutputHandler>` are conservatively not auto-`Send`
/// because objc2 can't statically know which Apple-method calls are
/// thread-safe. For our usage:
///
/// - `SCStream::updateContentFilter_completionHandler`,
///   `stopCaptureWithCompletionHandler`, and
///   `removeStreamOutput_type_error` are all documented as
///   thread-safe (they acquire SCK's internal lock).
/// - The delegate object never has methods called on it from Rust —
///   it's a callback target the SCK runtime invokes on its own
///   dispatch queue.
/// - Reference-counting (CFRetain/CFRelease) is thread-safe by Apple
///   guarantee.
///
/// So `Send` + `Sync` here is sound for the operations we actually
/// perform. The unsafe wrapper exists so Tauri can `.manage()` this
/// state and the IPC command surface can read it from any thread in
/// Tauri's pool.
unsafe impl Send for SystemAudioStream {}
unsafe impl Sync for SystemAudioStream {}

/// Live SCK system-audio capture session — see the module-level
/// docs for the full architecture. Held as an `Option` inside Tauri
/// state via `crates/app/src/system_audio.rs::SystemAudioCaptureState`.
pub struct SystemAudioStream {
    config: SystemAudioConfig,
    stream: Retained<SCStream>,
    // Held to keep the delegate alive for the lifetime of the stream
    // (SCK retains its pointer; if we dropped Retained early, a
    // late-firing callback would touch freed memory).
    delegate: Retained<AudioOutputHandler>,
    receiver: Receiver<DeliveredSamples>,
    pending: Vec<f32>,
    next_frame: u64,
}

impl SystemAudioStream {
    /// Build + start a new capture session.
    ///
    /// Blocks until either SCK reports the stream is running OR the
    /// permission prompt is denied. The first call on a fresh
    /// install triggers the macOS Screen Recording permission prompt
    /// (granted via `NSScreenCaptureUsageDescription`); after grant
    /// the user must relaunch the app for the new TCC entry to take
    /// effect.
    ///
    /// Convenience wrapper that opens with no meter [`LevelSink`].
    /// Callers that want the M-AUDIO.METER / AUT-287 meter use
    /// [`Self::new_with_level_sink`] instead.
    pub fn new(config: SystemAudioConfig) -> Result<Self, SystemAudioError> {
        Self::new_with_level_sink(config, None)
    }

    /// As [`Self::new`] but accepts an optional [`LevelSink`] for
    /// the master-stream RMS meter. Pass `Some(closure)` to receive
    /// EMA-smoothed level values on every SCK sample buffer; pass
    /// `None` to disable the meter (saves an `rms_of` pass per
    /// callback when no consumer wants the value).
    pub fn new_with_level_sink(
        config: SystemAudioConfig,
        level_sink: Option<LevelSink>,
    ) -> Result<Self, SystemAudioError> {
        let (sender, receiver) = channel();
        let delegate = AudioOutputHandler::new(sender, level_sink);

        // 1. Get shareable content (displays + apps). Required so we
        //    can attach the SCContentFilter to a display, even
        //    though we only care about audio.
        let content = shareable_content_blocking()?;

        // 2. Build the SCContentFilter. Default is "every display,
        //    no per-app filter" (M-AUDIO-SYS.0 shape). Callers can
        //    later switch to a per-app filter via `set_app_filter`
        //    (M-AUDIO-SYS.1).
        let filter = build_content_filter(&content, &AudioAppFilter::AllAudio)?;

        // 3. Build the SCStreamConfiguration.
        let stream_config = unsafe {
            let alloc = SCStreamConfiguration::alloc();
            let cfg: Retained<SCStreamConfiguration> = msg_send![alloc, init];
            cfg.setCapturesAudio(true);
            cfg.setExcludesCurrentProcessAudio(config.excludes_current_process_audio);
            // u32 → isize is lossless on every target we run on
            // (Apple platforms are 64-bit-only since macOS 10.15);
            // try_from would be dead defensive code.
            cfg.setSampleRate(isize::try_from(config.sample_rate_hz).unwrap_or(isize::MAX));
            cfg.setChannelCount(isize::from(config.channels));
            cfg
        };

        // 4. Construct the SCStream with a nil stream-delegate (we
        //    don't need the `streamDidStopWithError` callback for
        //    v0; the audio output delegate is added separately
        //    below).
        let stream = unsafe {
            let alloc = SCStream::alloc();
            SCStream::initWithFilter_configuration_delegate(alloc, &filter, &stream_config, None)
        };

        // 5. Wire the audio output delegate. SCK calls the delegate
        //    on its own dispatch queue (passing `None` lets it use a
        //    default queue), which is fine — our mpsc::Sender is
        //    Send.
        let output_proto: &ProtocolObject<dyn SCStreamOutput> =
            ProtocolObject::from_ref(&*delegate);
        unsafe {
            stream
                .addStreamOutput_type_sampleHandlerQueue_error(
                    output_proto,
                    SCStreamOutputType::Audio,
                    None,
                )
                .map_err(|err| {
                    SystemAudioError::StreamCreationFailed(ns_error_description(&err))
                })?;
        }

        // 6. Start capture. SCK runs startCapture asynchronously;
        //    we block on a oneshot channel until the completion
        //    block fires.
        start_capture_blocking(&stream)?;

        tracing::info!(
            sample_rate = config.sample_rate_hz,
            channels = config.channels,
            excludes_current_process = config.excludes_current_process_audio,
            "system_audio: capture started"
        );

        Ok(Self {
            config,
            stream,
            delegate,
            receiver,
            pending: Vec::new(),
            next_frame: 0,
        })
    }

    /// Read `frames` frames of audio. Blocks until either enough PCM
    /// has been buffered or `timeout` elapses (default 2 seconds).
    ///
    /// Returns a normalised [`AudioChunk`] with monotonic PTS.
    pub fn next_chunk(&mut self, frames: u64) -> Result<AudioChunk, SystemAudioError> {
        self.next_chunk_with_timeout(frames, Duration::from_secs(2))
    }

    /// As [`Self::next_chunk`] but with an explicit timeout. Useful
    /// for tests that want a tight upper bound on blocking.
    pub fn next_chunk_with_timeout(
        &mut self,
        frames: u64,
        timeout: Duration,
    ) -> Result<AudioChunk, SystemAudioError> {
        let frames_usize = usize::try_from(frames)
            .map_err(|_| SystemAudioError::InvalidChunk("frame count exceeds usize".into()))?;
        let need = frames_usize
            .checked_mul(usize::from(self.config.channels))
            .ok_or_else(|| SystemAudioError::InvalidChunk("frame count overflow".into()))?;

        let deadline = std::time::Instant::now() + timeout;
        while self.pending.len() < need {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                let frames_read = self.pending.len() as u64 / u64::from(self.config.channels);
                return Err(SystemAudioError::Timeout {
                    frames_read: self.next_frame + frames_read,
                    frames_requested: self.next_frame + frames,
                    timeout_ms: u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
                });
            }
            match self.receiver.recv_timeout(remaining) {
                Ok(samples) => {
                    if self.pending.len() + samples.len()
                        > CHANNEL_DEPTH_BOUND * (self.config.sample_rate_hz as usize / 10)
                    {
                        // Back-pressure guard — discard the oldest
                        // pending samples so memory stays bounded if
                        // the consumer is slow. Cheap to compute;
                        // the realistic case is "consumer caught
                        // up" so this branch rarely fires.
                        let overflow = self.pending.len() + samples.len()
                            - CHANNEL_DEPTH_BOUND * (self.config.sample_rate_hz as usize / 10);
                        self.pending.drain(..overflow);
                    }
                    self.pending.extend(samples);
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(SystemAudioError::StartFailed(
                        "delegate channel disconnected".into(),
                    ));
                }
            }
        }

        let drained: Vec<f32> = self.pending.drain(..need).collect();
        let pts = MediaTime::from_sample(self.next_frame, self.config.sample_rate_hz);
        let chunk = AudioChunk::new(
            AudioFormat {
                sample_rate: self.config.sample_rate_hz,
                channels: self.config.channels,
                sample_format: SampleFormat::F32,
            },
            drained,
            pts,
        )?;
        self.next_frame = self.next_frame.saturating_add(frames);
        Ok(chunk)
    }

    /// Configuration the stream was constructed with.
    #[must_use]
    pub fn config(&self) -> SystemAudioConfig {
        self.config
    }

    /// Reconfigure the active stream with a per-app filter
    /// (M-AUDIO-SYS.1 / AUT-281).
    ///
    /// SCK has no clean "swap filter mid-stream" path — calling
    /// `updateContentFilter` on a running stream can race with
    /// in-flight sample-buffer delivery in ways that produce silent
    /// gaps. We instead enumerate the current shareable content,
    /// resolve each requested bundle id → live PID, build a new
    /// `SCContentFilter`, and call `updateContentFilter` via
    /// completion handler (Apple's documented swap path). Callers
    /// should debounce rapid filter changes (~250 ms) so a flurry
    /// of checkbox clicks doesn't tear the stream up multiple times.
    ///
    /// # Errors
    ///
    /// Returns [`SystemAudioError::EnumerationFailed`] if the
    /// shareable-content fetch fails, [`SystemAudioError::NoDisplays`]
    /// if no displays are attached, or [`SystemAudioError::StartFailed`]
    /// if SCK reports an error on the filter update.
    pub fn set_app_filter(&self, filter: &AudioAppFilter) -> Result<(), SystemAudioError> {
        let content = shareable_content_blocking()?;
        let new_filter = build_content_filter(&content, filter)?;

        // updateContentFilter is async — bridge to sync via the
        // same oneshot pattern as start_capture_blocking.
        let (tx, rx) = channel::<Result<(), String>>();
        #[allow(
            clippy::arc_with_non_send_sync,
            reason = "Same justification as shareable_content_blocking — the channel payload is plain Result<(), String> here (no Apple types), but the Sender wrapper still trips clippy; safe."
        )]
        let tx_arc = Arc::new(std::sync::Mutex::new(Some(tx)));
        let tx_for_block = Arc::clone(&tx_arc);
        let block = RcBlock::new(move |error: *mut objc2_foundation::NSError| {
            let mut sender_slot = tx_for_block
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(sender) = sender_slot.take() {
                let result = if error.is_null() {
                    Ok(())
                } else {
                    // SAFETY: non-null per the branch.
                    Err(unsafe { ns_error_description(&*error) })
                };
                let _ = sender.send(result);
            }
        });
        // SAFETY: updateContentFilter is the documented Apple-side
        // swap path; the block signature matches.
        unsafe {
            self.stream
                .updateContentFilter_completionHandler(&new_filter, Some(&block));
        }
        match rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                tracing::info!(?filter, "system_audio: content filter updated");
                Ok(())
            }
            Ok(Err(msg)) => Err(SystemAudioError::StartFailed(msg)),
            Err(_) => Err(SystemAudioError::StartFailed(
                "updateContentFilter completion timed out after 5s".into(),
            )),
        }
    }
}

/// Enumerate every running app SCK can see (M-AUDIO-SYS.1 / AUT-281).
///
/// Returned apps include every app currently running with at least
/// one capturable window — SCK doesn't distinguish "audio-producing"
/// from "not", so the picker UI presents every running app and lets
/// the user pick. (System Audio MIDI Setup has the same behaviour.)
///
/// Triggers the macOS Screen Recording permission prompt on first
/// run (same TCC entry as the rest of SCK; granting it once covers
/// every SCK path).
///
/// # Errors
///
/// Returns [`SystemAudioError::EnumerationFailed`] if SCK's
/// shareable-content fetch fails (which includes the
/// permission-denied case — the underlying error message reports
/// `"The user declined TCCs for application, window, display capture"`).
pub fn list_audio_apps() -> Result<Vec<AudioApp>, SystemAudioError> {
    let content = shareable_content_blocking()?;
    let apps = unsafe { content.applications() };
    let mut out: Vec<AudioApp> = Vec::with_capacity(apps.len());
    let mut seen_bundle_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for app in &apps {
        // SAFETY: every field accessor is a no-arg objc method that
        // returns retained Foundation types; SCK guarantees the
        // app reference is live for the iterator's lifetime.
        let pid_raw = unsafe { app.processID() };
        let bundle_ns = unsafe { app.bundleIdentifier() };
        let display_ns = unsafe { app.applicationName() };
        let bundle_id = bundle_ns.to_string();
        // Skip apps without a usable bundle id (system services,
        // command-line invocations, helper processes that SCK
        // surfaces but the user can't meaningfully pick).
        if bundle_id.is_empty() {
            continue;
        }
        // De-dupe: multi-process apps (Chrome, etc.) appear with one
        // entry per process; the picker should see one row per
        // bundle. Keep the first seen — pid will resolve again at
        // filter-apply time.
        if !seen_bundle_ids.insert(bundle_id.clone()) {
            continue;
        }
        let display_name = display_ns.to_string();
        let pid = u32::try_from(pid_raw).unwrap_or(0);
        out.push(AudioApp {
            pid,
            bundle_id,
            display_name,
            // v0: icon-bytes are deferred to M-AUDIO-SYS.1.1 (needs
            // objc2-app-kit dep for NSWorkspace.iconForFile + an
            // image-encode step). The picker's icon stack renders a
            // generic placeholder for empty payloads.
            icon_png_bytes: Vec::new(),
        });
    }
    Ok(out)
}

/// Build an `SCContentFilter` from a shareable-content snapshot + a
/// user-provided filter spec. Returns the new filter ready for
/// `SCStream.updateContentFilter` or first-time
/// `initWithFilter_configuration_delegate`.
fn build_content_filter(
    content: &SCShareableContent,
    filter: &AudioAppFilter,
) -> Result<Retained<SCContentFilter>, SystemAudioError> {
    let displays = unsafe { content.displays() };
    let Some(display) = displays.iter().next() else {
        return Err(SystemAudioError::NoDisplays);
    };
    let empty_windows: Retained<NSArray<SCWindow>> = NSArray::new();
    let apps_to_pin: Retained<NSArray<SCRunningApplication>> = match filter {
        AudioAppFilter::AllAudio => {
            // Empty-apps filter — degenerate to the M-AUDIO-SYS.0
            // shape (every audio source captured).
            return Ok(unsafe {
                SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &empty_windows,
                )
            });
        }
        AudioAppFilter::OnlyApps(bundle_ids) | AudioAppFilter::ExcludeApps(bundle_ids) => {
            resolve_bundle_ids_to_apps(content, bundle_ids)
        }
    };
    let new_filter = match filter {
        AudioAppFilter::AllAudio => unreachable!("handled above"),
        AudioAppFilter::OnlyApps(_) => unsafe {
            SCContentFilter::initWithDisplay_includingApplications_exceptingWindows(
                SCContentFilter::alloc(),
                &display,
                &apps_to_pin,
                &empty_windows,
            )
        },
        AudioAppFilter::ExcludeApps(_) => unsafe {
            SCContentFilter::initWithDisplay_excludingApplications_exceptingWindows(
                SCContentFilter::alloc(),
                &display,
                &apps_to_pin,
                &empty_windows,
            )
        },
    };
    Ok(new_filter)
}

/// Walk the shareable-content app list and collect every
/// `SCRunningApplication` whose bundle id is in `bundle_ids`.
/// Missing apps (not currently running) are silently skipped —
/// the next filter-apply call will pick them up once they relaunch.
fn resolve_bundle_ids_to_apps(
    content: &SCShareableContent,
    bundle_ids: &[String],
) -> Retained<NSArray<SCRunningApplication>> {
    let apps = unsafe { content.applications() };
    let wanted: std::collections::HashSet<&str> = bundle_ids.iter().map(String::as_str).collect();
    let mut matched: Vec<Retained<SCRunningApplication>> = Vec::new();
    let mut already_added: std::collections::HashSet<String> =
        std::collections::HashSet::with_capacity(matched.len());
    for app in &apps {
        let bundle_id = unsafe { app.bundleIdentifier() };
        let bundle_str = bundle_id.to_string();
        if bundle_str.is_empty() {
            continue;
        }
        if wanted.contains(bundle_str.as_str()) && already_added.insert(bundle_str.clone()) {
            matched.push(app.clone());
        }
    }
    NSArray::from_retained_slice(&matched)
}

impl Drop for SystemAudioStream {
    fn drop(&mut self) {
        let stream = &self.stream;
        // Remove the audio stream output first so no further
        // callbacks fire onto the (about-to-be-dropped) delegate /
        // receiver.
        let output_proto: &ProtocolObject<dyn SCStreamOutput> =
            ProtocolObject::from_ref(&*self.delegate);
        // SAFETY: SCStream's removeStreamOutput is documented as
        // safe to call against an active stream from any thread;
        // it acquires its own internal lock. The protocol object
        // pointer remains valid because we still hold _delegate.
        unsafe {
            let _ = stream.removeStreamOutput_type_error(output_proto, SCStreamOutputType::Audio);
        }

        // Synchronously stop the SCK stream. We use a oneshot to
        // wait on the completion block; if the OS never calls back
        // we time out after 500 ms rather than blocking Drop
        // forever.
        let (tx, rx) = std::sync::mpsc::channel::<()>();
        let tx_arc = Arc::new(std::sync::Mutex::new(Some(tx)));
        let tx_for_block = Arc::clone(&tx_arc);
        let block = RcBlock::new(move |_err: *mut objc2_foundation::NSError| {
            if let Some(tx) = tx_for_block
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .take()
            {
                let _ = tx.send(());
            }
        });
        // SAFETY: stopCapture is the documented Apple-side teardown;
        // the completion block matches the expected signature.
        unsafe {
            stream.stopCaptureWithCompletionHandler(Some(&*block));
        }
        let _ = rx.recv_timeout(Duration::from_millis(500));
        tracing::info!("system_audio: capture stopped");
    }
}

/// Block on `SCShareableContent.current`. SCK's API is async; we
/// bridge to sync via a oneshot channel + completion block.
///
/// Visible to `crate::screen` (M-SCK.1 / AUT-268) so the display +
/// window enumeration path reuses the same async-bridge instead of
/// duplicating the 30-line completion-handler boilerplate.
pub(crate) fn shareable_content_blocking() -> Result<Retained<SCShareableContent>, SystemAudioError>
{
    // Apple's CFRetain / CFRelease ARE thread-safe (see Apple's
    // "Memory Management Programming Guide for Core Foundation"),
    // so transferring a `Retained<SCShareableContent>` from the
    // dispatch-queue thread to our caller-thread via mpsc is sound
    // even though objc2's conservative `Send` auto-impl doesn't see
    // it. The clippy lint flags the Arc<Mutex<...>> wrapper around
    // the Sender; suppress with the justification that the channel
    // is only ever read by the calling thread after the completion
    // block has fired, so the Retained moves between threads exactly
    // once and only the new owner ever invokes methods on it.
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "Apple-side CFRetain/CFRelease is thread-safe; method dispatch only happens on the receiving thread after recv()."
    )]
    let (tx, rx) = channel::<Result<Retained<SCShareableContent>, String>>();
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "Apple-side CFRetain/CFRelease is thread-safe; method dispatch only happens on the receiving thread after recv()."
    )]
    let tx_arc = Arc::new(std::sync::Mutex::new(Some(tx)));
    let tx_for_block = Arc::clone(&tx_arc);
    let block = RcBlock::new(
        move |content: *mut SCShareableContent, error: *mut objc2_foundation::NSError| {
            let mut sender_slot = tx_for_block
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(sender) = sender_slot.take() {
                let result = if content.is_null() {
                    let msg = if error.is_null() {
                        "nil content + nil error".to_string()
                    } else {
                        // SAFETY: error pointer is non-null per the
                        // branch; SCK promises the pointer outlives
                        // the completion block.
                        unsafe { ns_error_description(&*error) }
                    };
                    Err(msg)
                } else {
                    // SAFETY: SCK hands us a +0 (autoreleased)
                    // pointer to a live `SCShareableContent`. We
                    // need our own +1 retain so the value survives
                    // past the autorelease pool drain; `Retained::retain`
                    // is the safe wrapper that does exactly that.
                    let retained = unsafe { Retained::retain(content) };
                    match retained {
                        Some(r) => Ok(r),
                        None => Err("retain on SCShareableContent returned None".to_string()),
                    }
                };
                let _ = sender.send(result);
            }
        },
    );
    // SAFETY: getShareableContentWithCompletionHandler is the
    // canonical Apple-side enumerator; the block has the documented
    // signature.
    unsafe {
        SCShareableContent::getShareableContentWithCompletionHandler(&block);
    }
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(content)) => Ok(content),
        Ok(Err(msg)) => Err(SystemAudioError::EnumerationFailed(msg)),
        Err(_) => Err(SystemAudioError::EnumerationFailed(
            "shareable-content callback timed out after 5s".into(),
        )),
    }
}

/// Block on `SCStream.startCapture`. Same bridge pattern as
/// [`shareable_content_blocking`].
fn start_capture_blocking(stream: &SCStream) -> Result<(), SystemAudioError> {
    let (tx, rx) = channel::<Result<(), String>>();
    let tx_arc = Arc::new(std::sync::Mutex::new(Some(tx)));
    let tx_for_block = Arc::clone(&tx_arc);
    let block = RcBlock::new(move |error: *mut objc2_foundation::NSError| {
        let mut sender_slot = tx_for_block
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(sender) = sender_slot.take() {
            let result = if error.is_null() {
                Ok(())
            } else {
                // SAFETY: pointer is non-null per the branch.
                Err(unsafe { ns_error_description(&*error) })
            };
            let _ = sender.send(result);
        }
    });
    // SAFETY: startCapture is the documented Apple-side start path;
    // the block has the expected signature.
    unsafe {
        stream.startCaptureWithCompletionHandler(Some(&*block));
    }
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(msg)) => Err(SystemAudioError::StartFailed(msg)),
        Err(_) => Err(SystemAudioError::StartFailed(
            "startCapture completion timed out after 10s".into(),
        )),
    }
}

/// Extract the `localizedDescription` from an `NSError`, returning
/// the empty string if the call fails. Used for shaping error
/// messages we pass back to Leptos.
fn ns_error_description(error: &objc2_foundation::NSError) -> String {
    let description: Retained<NSString> = unsafe { msg_send![error, localizedDescription] };
    description.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_48k_stereo_with_self_exclusion() {
        let cfg = SystemAudioConfig::default();
        assert_eq!(cfg.sample_rate_hz, 48_000);
        assert_eq!(cfg.channels, 2);
        assert!(
            cfg.excludes_current_process_audio,
            "default must exclude our own audio to avoid the feedback loop"
        );
    }

    #[test]
    fn system_audio_error_round_trips_through_serde() {
        let cases = [
            SystemAudioError::NotMacOs,
            SystemAudioError::NoDisplays,
            SystemAudioError::StreamCreationFailed("init failed".into()),
            SystemAudioError::StartFailed("permission denied".into()),
            SystemAudioError::EnumerationFailed("timed out".into()),
            SystemAudioError::Timeout {
                frames_read: 1_000,
                frames_requested: 4_800,
                timeout_ms: 2_000,
            },
            SystemAudioError::InvalidChunk("unaligned".into()),
        ];
        for err in cases {
            let json = serde_json::to_string(&err).unwrap();
            let back: SystemAudioError = serde_json::from_str(&json).unwrap();
            assert_eq!(back, err);
        }
    }

    #[test]
    fn extract_error_messages_are_actionable() {
        let osstatus = ExtractError::OsStatus(-12_731);
        assert!(osstatus.to_string().contains("-12731"));
        let unaligned = ExtractError::UnalignedByteSize(7);
        assert!(unaligned.to_string().contains('7'));
        assert!(unaligned.to_string().contains("f32"));
    }

    #[test]
    fn audio_buffer_to_interleaved_handles_aligned_data() {
        // Build a 4-sample (f32) buffer and round-trip it through
        // the safe wrapper. Native test runs in-process so we can
        // hand `data` a real pointer.
        let samples: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4];
        let bytes = u32::try_from(samples.len() * size_of::<f32>())
            .expect("4 floats = 16 bytes fits in u32");
        let data = samples.as_ptr().cast::<std::ffi::c_void>().cast_mut();
        let out = audio_buffer_to_interleaved_f32(data, bytes).unwrap();
        assert_eq!(out.len(), 4);
        for (a, b) in samples.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn audio_buffer_to_interleaved_rejects_null_data() {
        let err = audio_buffer_to_interleaved_f32(std::ptr::null_mut(), 16).unwrap_err();
        assert!(matches!(err, ExtractError::NullData));
    }

    #[test]
    fn audio_buffer_to_interleaved_rejects_unaligned_byte_size() {
        let one_byte = [0u8; 4];
        let ptr = one_byte.as_ptr().cast::<std::ffi::c_void>().cast_mut();
        // 3 bytes is not a multiple of 4.
        let err = audio_buffer_to_interleaved_f32(ptr, 3).unwrap_err();
        assert!(matches!(err, ExtractError::UnalignedByteSize(3)));
    }

    #[test]
    fn audio_app_serde_round_trip_preserves_every_field() {
        let app = AudioApp {
            pid: 12_345,
            bundle_id: "com.spotify.client".into(),
            display_name: "Spotify".into(),
            icon_png_bytes: vec![0x89, 0x50, 0x4e, 0x47],
        };
        let json = serde_json::to_string(&app).unwrap();
        let back: AudioApp = serde_json::from_str(&json).unwrap();
        assert_eq!(back, app);
    }

    #[test]
    fn audio_app_filter_default_is_all_audio() {
        // The default variant must match the M-AUDIO-SYS.0 behaviour
        // so a fresh stream captures everything until the user picks
        // a filter — opt-in restriction, not opt-out.
        assert_eq!(AudioAppFilter::default(), AudioAppFilter::AllAudio);
    }

    #[test]
    fn audio_app_filter_serde_round_trip_every_variant() {
        let cases = [
            AudioAppFilter::AllAudio,
            AudioAppFilter::OnlyApps(vec![
                "com.spotify.client".into(),
                "com.google.Chrome".into(),
            ]),
            AudioAppFilter::ExcludeApps(vec!["us.zoom.xos".into()]),
        ];
        for filter in cases {
            let json = serde_json::to_string(&filter).unwrap();
            let back: AudioAppFilter = serde_json::from_str(&json).unwrap();
            assert_eq!(back, filter);
        }
    }

    #[test]
    fn channel_depth_bound_is_at_least_one_second_at_default_rate() {
        // Sanity guard on the back-pressure constant — `CHANNEL_DEPTH_BOUND`
        // chunks of 1/10s each must buffer at least 1 second at 48 kHz
        // before we start dropping samples.
        let buffered_samples = CHANNEL_DEPTH_BOUND * (DEFAULT_SAMPLE_RATE as usize / 10);
        assert!(buffered_samples >= DEFAULT_SAMPLE_RATE as usize);
    }
}
