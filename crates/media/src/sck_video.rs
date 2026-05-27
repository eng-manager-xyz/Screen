//! macOS ScreenCaptureKit screen / window video capture
//! (M-SCK.0 / AUT-267).
//!
//! Spins up an `SCStream` configured for **video** output, attaches
//! an `SCStreamOutput` delegate that receives BGRA `CMSampleBuffer`
//! frames on the SCK dispatch queue, and tracks a frame counter +
//! a "Starting → Running" lifecycle transition off the first
//! received buffer. Direct sibling of [`crate::sck_audio`] — same
//! framework, same delegate pattern, video pixel buffers instead
//! of audio sample buffers.
//!
//! ```admonish important title="What this commit ships vs. what's deferred"
//! This module establishes the **capture path** end-to-end:
//!
//! - SCK initialisation + permission handshake (TCC)
//! - `SCContentFilter` targeting a specific display
//! - `SCStreamConfiguration` with width / height / target FPS
//! - `SCStreamOutput` delegate receiving video frames
//! - Atomic frame counter + lifecycle hook
//! - Drop-safe teardown
//!
//! What's intentionally **NOT** here:
//!
//! - **BGRA pixel extraction** from CMSampleBuffer's
//!   CVPixelBuffer / IOSurface. The delegate counts frames and
//!   discards the buffer — the M-SCK pipeline that pipes frames
//!   into wisp + reads back to a canvas is deferred per the user's
//!   "data delivered = skip for now" scoping. The capture worker
//!   is functional + verified (frame counter increments); the
//!   downstream consumer is a separate ticket.
//! - **Cursor capture toggling** — defaults to "show cursor"
//!   (`showsCursor = true`). The setting toggle is a settings
//!   follow-up.
//! - **Window capture** — this module targets a *display*; the
//!   window-capture path uses a different `SCContentFilter`
//!   constructor and lands as M-SCK.0.1 if needed.
//! ```
//!
//! Mirror of `sck_audio.rs`'s Drop discipline: `stopCapture` is
//! called synchronously on drop with a 500 ms safety timeout, and
//! the output delegate is removed first so late-firing callbacks
//! don't touch freed memory.

#![cfg(target_os = "macos")]
#![allow(
    unsafe_code,
    reason = "ScreenCaptureKit is FFI-by-design; every unsafe block has a SAFETY comment above it."
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_core_graphics::{CGDisplayCopyDisplayMode, CGDisplayMode, CGMainDisplayID};
use objc2_core_media::{CMSampleBuffer, CMTime};
use objc2_core_video::{
    CVPixelBufferGetBaseAddress, CVPixelBufferGetBytesPerRow, CVPixelBufferGetHeight,
    CVPixelBufferGetPixelFormatType, CVPixelBufferGetWidth, CVPixelBufferLockBaseAddress,
    CVPixelBufferLockFlags, CVPixelBufferUnlockBaseAddress, kCVPixelFormatType_32BGRA,
};
use objc2_foundation::{NSArray, NSObject, NSObjectProtocol, NSString};
use objc2_screen_capture_kit::{
    SCContentFilter, SCShareableContent, SCStream, SCStreamConfiguration, SCStreamOutput,
    SCStreamOutputType, SCWindow,
};
use serde::{Deserialize, Serialize};

use crate::sck_audio::{SystemAudioError, shareable_content_blocking};

/// Shared latest-frame slot the [`ScreenOutputHandler`] writes BGRA
/// bytes into (M-PIX.2). Plumbed in from the app crate (lives on
/// `crate::recording::RecordingState`); typed here as a free
/// `Arc<Mutex<Option<Vec<u8>>>>` so the `media` crate doesn't depend
/// on `app`.
pub type ScreenFrameSlot = Arc<std::sync::Mutex<Option<Vec<u8>>>>;

/// Result type — reuses [`SystemAudioError`] so the screen-capture
/// path surfaces failures with the same string conversion as the
/// audio path. Same TCC entry, same SCK error shapes.
pub type ScreenError = SystemAudioError;

/// Default capture width when [`ScreenCaptureConfig::width`] is `0`
/// (caller wants "native display size"). Matches Apple's
/// `SCStreamConfiguration` default.
pub const DEFAULT_WIDTH: u32 = 1920;

/// Default capture height. Pairs with [`DEFAULT_WIDTH`].
pub const DEFAULT_HEIGHT: u32 = 1080;

/// Default target frame rate. 30 fps balances "feels smooth" vs.
/// "doesn't melt the machine when readback lands." The recorder
/// can bump this later if user feedback wants 60.
pub const DEFAULT_TARGET_FPS: u32 = 30;

/// Which screen / window the capture session targets
/// (M-SCK.0.1 / AUT-291). `Default = PrimaryDisplay` so an existing
/// `ScreenCaptureConfig::default()` keeps the M-SCK.0 behaviour.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ScreenCaptureSource {
    /// First display in `SCShareableContent.displays` — M-SCK.0's
    /// historical behaviour.
    #[default]
    PrimaryDisplay,
    /// Specific display by id. Format matches what
    /// [`crate::screen::list_displays`] emits: `"display-<displayID>"`
    /// where `displayID` is the SCDisplay's macOS `CGDirectDisplayID`.
    Display(String),
    /// Specific window by id. Format matches what
    /// [`crate::screen::list_windows`] emits: `"window-<windowID>"`
    /// where `windowID` is the SCWindow's `CGWindowID`. Window IDs are
    /// **not stable across app launches** — the picker persists +
    /// recovers on no-match, see `<ScreenPicker />`.
    Window(String),
}

/// Screen-capture configuration. Each field has a sensible default;
/// pass `Default::default()` for "primary display, 1920×1080, 30
/// fps." Override `width` / `height` to a specific source-rect
/// sub-region or to the display's actual size; override
/// `target_fps` to match a 60Hz display; override `source` to capture
/// a non-primary display or a specific window (M-SCK.0.1 / AUT-291).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenCaptureConfig {
    /// Capture width in pixels. `0` falls back to [`DEFAULT_WIDTH`].
    pub width: u32,
    /// Capture height in pixels. `0` falls back to [`DEFAULT_HEIGHT`].
    pub height: u32,
    /// Target frame rate. `0` falls back to [`DEFAULT_TARGET_FPS`].
    pub target_fps: u32,
    /// `true` to include the mouse cursor in captured frames.
    /// Defaults `true` because that's what the user expects for
    /// every screencast they've ever seen.
    pub shows_cursor: bool,
    /// Which display / window to capture (M-SCK.0.1 / AUT-291). The
    /// `Default::default()` value is [`ScreenCaptureSource::PrimaryDisplay`]
    /// so legacy callers keep the M-SCK.0 behaviour.
    pub source: ScreenCaptureSource,
    /// `CGWindowID`s to exclude from the capture (display-source only;
    /// ignored for [`ScreenCaptureSource::Window`] since that filter
    /// targets a single specific window). Each ID is the integer
    /// value returned by `NSWindow.windowNumber` for the window to
    /// keep OUT of the captured frame — typically the caller's own
    /// UI windows that the user is monitoring on screen but doesn't
    /// want recorded. Unknown IDs (window no longer present) are
    /// silently dropped at filter-build time.
    pub excluded_window_ids: Vec<u32>,
}

impl Default for ScreenCaptureConfig {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            target_fps: DEFAULT_TARGET_FPS,
            shows_cursor: true,
            source: ScreenCaptureSource::PrimaryDisplay,
            excluded_window_ids: Vec::new(),
        }
    }
}

impl ScreenCaptureConfig {
    /// Construct a config that captures `source` with the M-SCK.0
    /// defaults for everything else (1920×1080 @ 30 fps, cursor on).
    /// Most call sites use this rather than literal struct init since
    /// the source is the field that actually varies per-session.
    #[must_use]
    pub fn for_source(source: ScreenCaptureSource) -> Self {
        Self {
            source,
            ..Self::default()
        }
    }
}

/// Resolve `source`'s native backing-pixel dimensions for recording
/// (M-QUAL.2). Returns even-rounded `(width, height)` — the display's
/// true Retina pixel resolution, so capture is 1:1 with **no
/// downscale** and no aspect squish (the old fixed 1920×1080 both
/// halved a Retina panel's detail *and* distorted its non-16:9 aspect).
///
/// Falls back to `(DEFAULT_WIDTH, DEFAULT_HEIGHT)` when the
/// CoreGraphics query fails, or for window sources — a window's pixel
/// size isn't a display mode, so per-window native sizing is a
/// follow-up.
#[must_use]
pub fn resolve_native_screen_dims(source: &ScreenCaptureSource) -> (u32, u32) {
    let display_id = match source {
        ScreenCaptureSource::PrimaryDisplay => CGMainDisplayID(),
        ScreenCaptureSource::Display(id) => match parse_display_id(id) {
            Some(did) => did,
            None => return (DEFAULT_WIDTH, DEFAULT_HEIGHT),
        },
        ScreenCaptureSource::Window(_) => return (DEFAULT_WIDTH, DEFAULT_HEIGHT),
    };
    let Some(mode) = CGDisplayCopyDisplayMode(display_id) else {
        return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
    };
    // `pixel_*` (vs `width`/`height`) returns the true backing pixels —
    // 3024×1964 on a 14" MBP, not the 1512×982 "looks like" point size.
    let w = CGDisplayMode::pixel_width(Some(&mode));
    let h = CGDisplayMode::pixel_height(Some(&mode));
    sanitize_dims(w, h)
}

/// Even-round (H.264 requires mod-2 dimensions) + clamp to a sane
/// ceiling so a misreported display mode can't yield an invalid encode
/// pipeline. Pure — unit-tested without CoreGraphics.
fn sanitize_dims(w: usize, h: usize) -> (u32, u32) {
    // H.264 level-6 max edge. No real Retina panel reaches it, so this
    // guards against a bogus reading rather than downscaling anything.
    const MAX_EDGE: u32 = 7680;
    let clamp_even = |v: usize| -> u32 {
        let v = u32::try_from(v).unwrap_or(MAX_EDGE).clamp(2, MAX_EDGE);
        v & !1 // round down to even
    };
    (clamp_even(w), clamp_even(h))
}

/// Atomic counters surfaced for diagnostics (M-CAM.3's
/// `PreviewDiagnostics` pattern). Used by the lifecycle helper to
/// flip `Starting → Running` on first frame and by future ticks /
/// frame-rate-monitors to observe throughput.
#[derive(Default)]
pub struct ScreenCaptureCounters {
    /// Cumulative frames received from the SCK delegate.
    pub frames_received: AtomicU64,
}

impl ScreenCaptureCounters {
    /// Read the current frame count (atomic, lock-free).
    #[must_use]
    pub fn frames_received(&self) -> u64 {
        self.frames_received.load(Ordering::Relaxed)
    }
}

// `SCStreamOutput` delegate for video output. Receives SCK sample
// buffers on a dispatch queue, increments the shared counter, and
// drops the buffer. Stays minimal so the delegate doesn't block
// SCK's queue — pixel extraction is a separate ticket per the
// module-level admonish.
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "ScreenVideoOutputHandler"]
    #[ivars = ScreenOutputIvars]
    pub(crate) struct ScreenOutputHandler;

    impl ScreenOutputHandler {}

    unsafe impl NSObjectProtocol for ScreenOutputHandler {}

    unsafe impl SCStreamOutput for ScreenOutputHandler {
        #[unsafe(method(stream:didOutputSampleBuffer:ofType:))]
        #[allow(
            non_snake_case,
            reason = "method name must match Apple selector exactly for the runtime to dispatch"
        )]
        unsafe fn stream_didOutputSampleBuffer_ofType(
            &self,
            _stream: &SCStream,
            sample_buffer: &CMSampleBuffer,
            r#type: SCStreamOutputType,
        ) {
            if r#type != SCStreamOutputType::Screen {
                return;
            }
            self.ivars()
                .counters
                .frames_received
                .fetch_add(1, Ordering::Relaxed);

            // M-PIX.2 — extract BGRA bytes + write to the shared
            // frame slot, if a recording session is wired. No-op
            // when the slot is absent (slot is None on the ivars
            // when SCK is running for diagnostics only).
            let Some(ref slot) = self.ivars().frame_slot else {
                return;
            };
            // SAFETY: SCK guarantees the sample buffer is live for
            // the callback's duration; CMSampleBufferGetImageBuffer
            // is documented thread-safe on the SCK dispatch queue.
            let Some(image_buffer) = (unsafe { sample_buffer.image_buffer() }) else {
                return;
            };
            // CVImageBuffer === CVPixelBuffer (typedef in
            // objc2-core-video).
            let pixel_buffer: &objc2_core_video::CVPixelBuffer = &image_buffer;
            if let Some(bytes) = extract_bgra_from_pixel_buffer(pixel_buffer) {
                let mut guard = slot
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                *guard = Some(bytes);
            }
        }
    }
);

pub(crate) struct ScreenOutputIvars {
    counters: Arc<ScreenCaptureCounters>,
    /// M-PIX.2 — optional shared slot the delegate writes BGRA
    /// bytes into. `None` keeps the M-SCK.0 frame-counting-only
    /// behaviour for tests + standalone diagnostics.
    frame_slot: Option<ScreenFrameSlot>,
}

impl ScreenOutputHandler {
    fn new(
        counters: Arc<ScreenCaptureCounters>,
        frame_slot: Option<ScreenFrameSlot>,
    ) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ScreenOutputIvars {
            counters,
            frame_slot,
        });
        // SAFETY: `init` on NSObject takes Allocated<Self> + returns
        // Retained<Self>. Class derives NSObject via define_class!.
        unsafe { msg_send![super(this), init] }
    }
}

/// Extract a freshly-allocated BGRA `Vec<u8>` from a `CVPixelBuffer`,
/// in standard CoreVideo top-down row order. Many IOSurfaces have
/// `bytes_per_row > width * 4` (per-row trailing padding); this
/// helper strips that padding so the output is a tight
/// `width * height * 4` byte buffer.
///
/// Returns `None` when the pixel format isn't `32BGRA` (the
/// `SCStreamConfiguration` pixelFormat is set to `32BGRA` in
/// [`ScreenCaptureStream::new`], so this branch is defensive — if
/// we somehow get a YUV / non-BGRA buffer, drop it rather than
/// corrupting the encoder feed).
///
/// The recording pump pushes the resulting bytes through wisp's
/// `RecordingScene::set_screen_frame`, which converts to wisp's
/// internal sprite-Y convention; callers that don't go through
/// `RecordingScene` get plain top-down bytes.
pub(crate) fn extract_bgra_from_pixel_buffer(
    pixel_buffer: &objc2_core_video::CVPixelBuffer,
) -> Option<Vec<u8>> {
    let format = CVPixelBufferGetPixelFormatType(pixel_buffer);
    if format != kCVPixelFormatType_32BGRA {
        tracing::trace!(
            format = format,
            "extract_bgra_from_pixel_buffer: non-BGRA format, skipping"
        );
        return None;
    }
    // SAFETY: CVPixelBufferLockBaseAddress is documented thread-safe
    // for read-only locks. We pair with Unlock before return.
    let lock_result =
        unsafe { CVPixelBufferLockBaseAddress(pixel_buffer, CVPixelBufferLockFlags::ReadOnly) };
    if lock_result != 0 {
        tracing::warn!(
            cv_return = lock_result,
            "CVPixelBufferLockBaseAddress failed"
        );
        return None;
    }
    let width = CVPixelBufferGetWidth(pixel_buffer);
    let height = CVPixelBufferGetHeight(pixel_buffer);
    let bytes_per_row = CVPixelBufferGetBytesPerRow(pixel_buffer);
    let base = CVPixelBufferGetBaseAddress(pixel_buffer);
    let result = if base.is_null() || width == 0 || height == 0 {
        None
    } else {
        // SAFETY: per Apple's docs the locked pixel-buffer memory
        // spans `bytes_per_row * height` bytes contiguously from
        // `base`. The lock keeps it live until the matching Unlock
        // below. The slice borrows for the body of this branch only,
        // which finishes before we Unlock.
        let src_slice = unsafe {
            std::slice::from_raw_parts(base.cast::<u8>(), bytes_per_row.saturating_mul(height))
        };
        Some(copy_bgra_rows_packed(
            src_slice,
            width,
            height,
            bytes_per_row,
        ))
    };
    // SAFETY: matching unlock for the lock above.
    unsafe {
        let _ = CVPixelBufferUnlockBaseAddress(pixel_buffer, CVPixelBufferLockFlags::ReadOnly);
    }
    result
}

/// Copy `height` rows of BGRA from `src` — which may have per-row
/// trailing padding (`bytes_per_row > width * 4`) — into a tightly
/// packed `Vec<u8>` (`width * height * 4` bytes, no padding). Row
/// order is preserved (top-down stays top-down).
///
/// The IOSurfaces backing SCK CVPixelBuffers commonly add stride
/// padding for SIMD-friendly row alignment; downstream consumers
/// (wisp `VideoTexture::upload_bgra`, the encoder feed thread) want
/// a tight buffer with `bytes_per_row = width * 4`.
fn copy_bgra_rows_packed(src: &[u8], width: usize, height: usize, bytes_per_row: usize) -> Vec<u8> {
    let row_bytes_packed = width.saturating_mul(4);
    debug_assert!(
        bytes_per_row >= row_bytes_packed,
        "bytes_per_row {bytes_per_row} < packed row width {row_bytes_packed}"
    );
    debug_assert!(
        src.len() >= height.saturating_mul(bytes_per_row),
        "src len {} < height*stride {}",
        src.len(),
        height.saturating_mul(bytes_per_row)
    );
    let mut out: Vec<u8> = Vec::with_capacity(row_bytes_packed.saturating_mul(height));
    for row in 0..height {
        let start = row * bytes_per_row;
        out.extend_from_slice(&src[start..start + row_bytes_packed]);
    }
    out
}

/// Active SCK screen-capture session. Owns the `SCStream` + the
/// `Retained<ScreenOutputHandler>` for the lifetime of the session.
/// Drop is fully synchronous: removes the output delegate, then
/// calls `stopCapture` with a 500 ms safety timeout.
///
/// # `Send` + `Sync`
///
/// Same justification as [`crate::sck_audio::SystemAudioStream`] —
/// objc2's conservative auto-impl doesn't see CFRetain/CFRelease
/// as thread-safe even though Apple documents them as such. We
/// declare both manually so the field can sit inside Tauri-managed
/// state.
unsafe impl Send for ScreenCaptureStream {}
unsafe impl Sync for ScreenCaptureStream {}

/// Live SCK screen-capture session. See module docs for the
/// architecture overview + what's intentionally NOT here.
pub struct ScreenCaptureStream {
    config: ScreenCaptureConfig,
    stream: Retained<SCStream>,
    delegate: Retained<ScreenOutputHandler>,
    counters: Arc<ScreenCaptureCounters>,
}

/// Resolve [`ScreenCaptureSource`] to an `SCContentFilter`
/// (M-SCK.0.1 / AUT-291). Extracted from `ScreenCaptureStream::new`
/// so that function stays under the `clippy::too_many_lines` cap.
///
/// `excluded_window_ids` (display-source only) is the list of
/// `CGWindowID`s to keep OUT of the captured frame — used to hide
/// the recorder's own webcam-bubble window so it doesn't duplicate
/// the wisp-composited cam. Unknown IDs (window has since closed)
/// are dropped silently.
fn build_content_filter(
    content: &SCShareableContent,
    source: &ScreenCaptureSource,
    excluded_window_ids: &[u32],
) -> Result<Retained<SCContentFilter>, ScreenError> {
    match source {
        ScreenCaptureSource::PrimaryDisplay => {
            let displays = unsafe { content.displays() };
            let Some(display) = displays.iter().next() else {
                return Err(ScreenError::NoDisplays);
            };
            let excluded = resolve_excluded_windows(content, excluded_window_ids);
            Ok(unsafe {
                SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &display,
                    &excluded,
                )
            })
        }
        ScreenCaptureSource::Display(id) => {
            let target_id = parse_display_id(id).ok_or_else(|| {
                ScreenError::StreamCreationFailed(format!(
                    "malformed display source id `{id}` (expected `display-<displayID>`)"
                ))
            })?;
            let displays = unsafe { content.displays() };
            let matched = displays
                .iter()
                .find(|d| unsafe { d.displayID() } == target_id)
                .ok_or_else(|| {
                    ScreenError::StreamCreationFailed(format!(
                        "display id `{id}` not present (was the display unplugged?)"
                    ))
                })?;
            let excluded = resolve_excluded_windows(content, excluded_window_ids);
            Ok(unsafe {
                SCContentFilter::initWithDisplay_excludingWindows(
                    SCContentFilter::alloc(),
                    &matched,
                    &excluded,
                )
            })
        }
        ScreenCaptureSource::Window(id) => {
            let target_id = parse_window_id(id).ok_or_else(|| {
                ScreenError::StreamCreationFailed(format!(
                    "malformed window source id `{id}` (expected `window-<windowID>`)"
                ))
            })?;
            let windows = unsafe { content.windows() };
            let matched = windows
                .iter()
                .find(|w| unsafe { w.windowID() } == target_id)
                .ok_or_else(|| {
                    ScreenError::StreamCreationFailed(format!(
                        "window id `{id}` not present (was it closed?)"
                    ))
                })?;
            // Window-source filter targets a single window; exclusion
            // is N/A here. Silently ignore `excluded_window_ids` so
            // callers don't have to branch.
            Ok(unsafe {
                SCContentFilter::initWithDesktopIndependentWindow(
                    SCContentFilter::alloc(),
                    &matched,
                )
            })
        }
    }
}

/// Walk `content.windows()` and collect the `SCWindow` objects whose
/// `windowID` matches one in `excluded_window_ids`. Returns an
/// `NSArray<SCWindow>` suitable for SCContentFilter's
/// `excludingWindows:` parameter.
fn resolve_excluded_windows(
    content: &SCShareableContent,
    excluded_window_ids: &[u32],
) -> Retained<NSArray<SCWindow>> {
    if excluded_window_ids.is_empty() {
        return NSArray::new();
    }
    let all_windows = unsafe { content.windows() };
    let matched: Vec<Retained<SCWindow>> = all_windows
        .iter()
        .filter(|w| excluded_window_ids.contains(&unsafe { w.windowID() }))
        .collect();
    if matched.is_empty() {
        tracing::debug!(
            requested = ?excluded_window_ids,
            "resolve_excluded_windows: no SCWindow matched the requested IDs (windows closed?)"
        );
    }
    NSArray::from_retained_slice(&matched)
}

impl ScreenCaptureStream {
    /// Build + start a capture session on the **first available
    /// display**. Triggers the macOS Screen Recording TCC prompt
    /// on first run if not yet granted.
    ///
    /// # Errors
    ///
    /// Same shape as [`crate::sck_audio::SystemAudioStream::new`] —
    /// `EnumerationFailed` / `NoDisplays` / `StreamCreationFailed` /
    /// `StartFailed`.
    pub fn new(config: ScreenCaptureConfig) -> Result<Self, ScreenError> {
        Self::new_with_frame_slot(config, None)
    }

    /// Same as [`Self::new`] but plumbs the M-PIX.2 frame slot into
    /// the delegate so each captured frame's BGRA bytes are written
    /// to the slot for the encoder feed thread to pick up.
    pub fn new_with_frame_slot(
        config: ScreenCaptureConfig,
        frame_slot: Option<ScreenFrameSlot>,
    ) -> Result<Self, ScreenError> {
        let counters = Arc::new(ScreenCaptureCounters::default());
        let delegate = ScreenOutputHandler::new(Arc::clone(&counters), frame_slot);

        // 1+2. Get shareable content + resolve `config.source` to the
        //      right SCContentFilter (M-SCK.0.1 / AUT-291). Three
        //      filter constructors → one helper to keep `new` short.
        //      `excluded_window_ids` is forwarded so display-source
        //      filters can keep specific windows out of the capture
        //      (callers use this for their own overlay UI).
        let content = shareable_content_blocking()?;
        let filter = build_content_filter(&content, &config.source, &config.excluded_window_ids)?;

        // 3. Build the SCStreamConfiguration. Width/height/fps
        //    fall back to defaults when caller passed 0 (so a
        //    plain `Default::default()` works).
        let stream_config = unsafe {
            let alloc = SCStreamConfiguration::alloc();
            let cfg: Retained<SCStreamConfiguration> = msg_send![alloc, init];
            let width = if config.width == 0 {
                DEFAULT_WIDTH
            } else {
                config.width
            };
            let height = if config.height == 0 {
                DEFAULT_HEIGHT
            } else {
                config.height
            };
            let fps = if config.target_fps == 0 {
                DEFAULT_TARGET_FPS
            } else {
                config.target_fps
            };
            cfg.setWidth(width as usize);
            cfg.setHeight(height as usize);
            cfg.setShowsCursor(config.shows_cursor);
            // M-PIX.2 — pin pixel format to 32BGRA so the
            // delegate's `extract_bgra_from_pixel_buffer` doesn't
            // have to do YUV→BGRA conversion. SCK normally returns
            // 420v (YUV) on Apple Silicon; setting this flips to
            // BGRA at the cost of a slight extra GPU copy in SCK.
            cfg.setPixelFormat(kCVPixelFormatType_32BGRA);
            // minimumFrameInterval is the seconds-between-frames
            // cap. `CMTime { value: 1, timescale: fps, ... }`
            // means 1/fps seconds per frame.
            cfg.setMinimumFrameInterval(CMTime {
                value: 1,
                timescale: fps.try_into().unwrap_or(30),
                flags: objc2_core_media::CMTimeFlags(1), // kCMTimeFlags_Valid
                epoch: 0,
            });
            cfg
        };

        // 4. Construct the SCStream with no stream-delegate (we
        //    don't consume `streamDidStopWithError`).
        let stream = unsafe {
            SCStream::initWithFilter_configuration_delegate(
                SCStream::alloc(),
                &filter,
                &stream_config,
                None,
            )
        };

        // 5. Attach the video output delegate. None for the queue
        //    lets SCK use its default dispatch queue.
        let output_proto: &ProtocolObject<dyn SCStreamOutput> =
            ProtocolObject::from_ref(&*delegate);
        unsafe {
            stream
                .addStreamOutput_type_sampleHandlerQueue_error(
                    output_proto,
                    SCStreamOutputType::Screen,
                    None,
                )
                .map_err(|err| ScreenError::StreamCreationFailed(ns_error_description(&err)))?;
        }

        // 6. Start capture. SCK runs startCapture asynchronously;
        //    bridge to sync via a oneshot channel.
        start_capture_blocking(&stream)?;

        tracing::info!(
            width = config.width,
            height = config.height,
            target_fps = config.target_fps,
            shows_cursor = config.shows_cursor,
            "sck_video: capture started"
        );

        Ok(Self {
            config,
            stream,
            delegate,
            counters,
        })
    }

    /// Configuration the stream was built with. Returns a clone so
    /// callers can hold it without taking a borrow on the live
    /// session (was `Copy` until M-SCK.0.1 added the `String`-bearing
    /// `source` field).
    #[must_use]
    pub fn config(&self) -> ScreenCaptureConfig {
        self.config.clone()
    }

    /// Atomic counters readable from any thread — see
    /// [`ScreenCaptureCounters::frames_received`].
    #[must_use]
    pub fn counters(&self) -> Arc<ScreenCaptureCounters> {
        Arc::clone(&self.counters)
    }
}

impl Drop for ScreenCaptureStream {
    fn drop(&mut self) {
        let stream = &self.stream;
        // Remove the output delegate FIRST so late-firing callbacks
        // don't touch the about-to-be-dropped counter Arc.
        let output_proto: &ProtocolObject<dyn SCStreamOutput> =
            ProtocolObject::from_ref(&*self.delegate);
        // SAFETY: removeStreamOutput is documented thread-safe;
        // delegate is still alive (we hold the Retained).
        unsafe {
            let _ = stream.removeStreamOutput_type_error(output_proto, SCStreamOutputType::Screen);
        }

        // Synchronous stop with a 500 ms safety cap — mirrors the
        // sck_audio Drop pattern.
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
        // SAFETY: stopCapture is the documented teardown path.
        unsafe {
            stream.stopCaptureWithCompletionHandler(Some(&block));
        }
        let _ = rx.recv_timeout(Duration::from_millis(500));
        tracing::info!("sck_video: capture stopped");
    }
}

/// Sync wrapper around `SCStream.startCapture`. Same shape as the
/// audio path's `start_capture_blocking`.
fn start_capture_blocking(stream: &SCStream) -> Result<(), ScreenError> {
    let (tx, rx) = channel::<Result<(), String>>();
    #[allow(
        clippy::arc_with_non_send_sync,
        reason = "Sender<Result<(), String>> is Send; clippy can't see the inner String through Mutex/Option. Mirror of sck_audio.rs's same allow."
    )]
    let tx_arc = Arc::new(std::sync::Mutex::new(Some(tx)));
    let tx_for_block = Arc::clone(&tx_arc);
    let block = RcBlock::new(move |error: *mut objc2_foundation::NSError| {
        let mut slot = tx_for_block
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(sender) = slot.take() {
            let result = if error.is_null() {
                Ok(())
            } else {
                // SAFETY: non-null per the branch.
                Err(unsafe { ns_error_description(&*error) })
            };
            let _ = sender.send(result);
        }
    });
    // SAFETY: startCapture is documented + the block signature matches.
    unsafe {
        stream.startCaptureWithCompletionHandler(Some(&block));
    }
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(msg)) => Err(ScreenError::StartFailed(msg)),
        Err(_) => Err(ScreenError::StartFailed(
            "startCapture completion timed out after 10s".into(),
        )),
    }
}

/// Extract `localizedDescription` from an `NSError`. Tiny helper
/// duplicated from `sck_audio` rather than exposed publicly — the
/// modules are siblings + nothing else needs it.
fn ns_error_description(error: &objc2_foundation::NSError) -> String {
    let description: Retained<NSString> = unsafe { msg_send![error, localizedDescription] };
    description.to_string()
}

/// Parse `"display-<u32>"` → `Some(u32)`. Returns `None` on any
/// non-matching prefix or non-numeric tail (M-SCK.0.1 / AUT-291).
#[must_use]
pub fn parse_display_id(id: &str) -> Option<u32> {
    id.strip_prefix("display-").and_then(|s| s.parse().ok())
}

/// Parse `"window-<u32>"` → `Some(u32)`. Returns `None` on any
/// non-matching prefix or non-numeric tail (M-SCK.0.1 / AUT-291).
#[must_use]
pub fn parse_window_id(id: &str) -> Option<u32> {
    id.strip_prefix("window-").and_then(|s| s.parse().ok())
}

/// View-model for an active screen-capture session (M-SCK.2 / AUT-269).
/// Mirrors `MicLifecycle`'s shape — kept here so the Tauri command
/// surface can return + render it without importing internal types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScreenLifecycle {
    /// No capture running.
    #[default]
    Idle,
    /// `start_screen_capture` invoked; awaiting first frame.
    Starting,
    /// Capture is producing frames.
    Running,
    /// `stop_screen_capture` invoked; tearing down.
    Stopping,
}

impl ScreenLifecycle {
    /// Idle → Starting; other states unchanged.
    #[must_use]
    pub fn try_start(self) -> Self {
        match self {
            Self::Idle => Self::Starting,
            other => other,
        }
    }

    /// Starting → Running on first frame; idempotent on Running.
    #[must_use]
    pub fn mark_running(self) -> Self {
        match self {
            Self::Starting => Self::Running,
            other => other,
        }
    }

    /// Starting / Running → Stopping; Idle / Stopping unchanged.
    #[must_use]
    pub fn try_stop(self) -> Self {
        match self {
            Self::Running | Self::Starting => Self::Stopping,
            other => other,
        }
    }

    /// Stopping → Idle; other states unchanged.
    #[must_use]
    pub fn finish_stop(self) -> Self {
        match self {
            Self::Stopping => Self::Idle,
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_uses_1920_1080_30fps_with_cursor() {
        let cfg = ScreenCaptureConfig::default();
        assert_eq!(cfg.width, DEFAULT_WIDTH);
        assert_eq!(cfg.height, DEFAULT_HEIGHT);
        assert_eq!(cfg.target_fps, DEFAULT_TARGET_FPS);
        assert!(cfg.shows_cursor);
        assert_eq!(cfg.source, ScreenCaptureSource::PrimaryDisplay);
    }

    #[test]
    fn for_source_overrides_only_source() {
        let cfg =
            ScreenCaptureConfig::for_source(ScreenCaptureSource::Display("display-12345".into()));
        assert_eq!(cfg.width, DEFAULT_WIDTH);
        assert_eq!(cfg.target_fps, DEFAULT_TARGET_FPS);
        assert_eq!(
            cfg.source,
            ScreenCaptureSource::Display("display-12345".into())
        );
    }

    #[test]
    fn screen_capture_source_default_is_primary_display() {
        assert_eq!(
            ScreenCaptureSource::default(),
            ScreenCaptureSource::PrimaryDisplay
        );
    }

    #[test]
    fn screen_capture_source_serde_round_trips_all_variants() {
        // M-SCK.0.1 — source crosses the IPC seam, so all 3 variants
        // must round-trip.
        for v in [
            ScreenCaptureSource::PrimaryDisplay,
            ScreenCaptureSource::Display("display-987654321".into()),
            ScreenCaptureSource::Window("window-42".into()),
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: ScreenCaptureSource = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    #[test]
    fn parse_display_id_extracts_numeric_suffix() {
        assert_eq!(parse_display_id("display-1"), Some(1));
        assert_eq!(parse_display_id("display-1234567890"), Some(1_234_567_890));
        assert_eq!(parse_display_id("display-0"), Some(0));
    }

    #[test]
    fn parse_display_id_rejects_malformed_ids() {
        assert_eq!(parse_display_id(""), None);
        assert_eq!(parse_display_id("display-"), None);
        assert_eq!(parse_display_id("display-abc"), None);
        assert_eq!(parse_display_id("window-42"), None);
        assert_eq!(parse_display_id("not-a-display"), None);
        // Don't accept negatives — CGDirectDisplayID is u32.
        assert_eq!(parse_display_id("display--1"), None);
    }

    #[test]
    fn parse_window_id_extracts_numeric_suffix() {
        assert_eq!(parse_window_id("window-1"), Some(1));
        assert_eq!(parse_window_id("window-99999"), Some(99_999));
    }

    // ---- M-QUAL.2 — native-resolution capture ----

    #[test]
    fn sanitize_dims_rounds_down_to_even() {
        // H.264 needs mod-2 dims; odd values round down.
        assert_eq!(sanitize_dims(1921, 1081), (1920, 1080));
        // Already-even native dims (14" MBP) pass through untouched.
        assert_eq!(sanitize_dims(3024, 1964), (3024, 1964));
        assert_eq!(sanitize_dims(5120, 2880), (5120, 2880));
    }

    #[test]
    fn sanitize_dims_clamps_degenerate_and_absurd_values() {
        // 0 can't make a valid pipeline — floor at 2.
        assert_eq!(sanitize_dims(0, 0), (2, 2));
        // A bogus huge reading is clamped to the H.264 ceiling (even).
        assert_eq!(sanitize_dims(999_999, 999_999), (7680, 7680));
    }

    /// On real macOS hardware the primary display resolves to its
    /// native backing pixels; on a headless CI runner `CGMainDisplayID`
    /// has no mode and we fall back to `DEFAULT_WIDTH/HEIGHT`. Either
    /// way the result must be even, non-zero, and within bounds — so
    /// the encoder caps + compose canvas are always valid.
    #[test]
    fn resolve_native_primary_display_is_even_nonzero_bounded() {
        let (w, h) = resolve_native_screen_dims(&ScreenCaptureSource::PrimaryDisplay);
        eprintln!("resolved primary-display native dims: {w}x{h}");
        assert!(w >= 2 && h >= 2, "non-zero: {w}x{h}");
        assert!(w <= 7680 && h <= 7680, "within H.264 ceiling: {w}x{h}");
        assert_eq!(w % 2, 0, "even width: {w}");
        assert_eq!(h % 2, 0, "even height: {h}");
    }

    #[test]
    fn resolve_native_window_source_falls_back_to_default() {
        // A window's pixel size isn't a display mode — keep the default
        // until per-window native sizing lands.
        let dims = resolve_native_screen_dims(&ScreenCaptureSource::Window("window-1".into()));
        assert_eq!(dims, (DEFAULT_WIDTH, DEFAULT_HEIGHT));
    }

    #[test]
    fn parse_window_id_rejects_malformed_ids() {
        assert_eq!(parse_window_id(""), None);
        assert_eq!(parse_window_id("window-"), None);
        assert_eq!(parse_window_id("window-abc"), None);
        assert_eq!(parse_window_id("display-42"), None);
    }

    #[test]
    fn counters_start_at_zero() {
        let c = ScreenCaptureCounters::default();
        assert_eq!(c.frames_received(), 0);
    }

    #[test]
    fn counters_increment_via_atomic() {
        let c = ScreenCaptureCounters::default();
        c.frames_received.fetch_add(7, Ordering::Relaxed);
        assert_eq!(c.frames_received(), 7);
    }

    #[test]
    fn lifecycle_full_round_trip() {
        let mut s = ScreenLifecycle::default();
        assert_eq!(s, ScreenLifecycle::Idle);
        s = s.try_start();
        assert_eq!(s, ScreenLifecycle::Starting);
        s = s.mark_running();
        assert_eq!(s, ScreenLifecycle::Running);
        s = s.try_stop();
        assert_eq!(s, ScreenLifecycle::Stopping);
        s = s.finish_stop();
        assert_eq!(s, ScreenLifecycle::Idle);
    }

    #[test]
    fn lifecycle_re_entrant_start_is_noop() {
        assert_eq!(
            ScreenLifecycle::Starting.try_start(),
            ScreenLifecycle::Starting
        );
        assert_eq!(
            ScreenLifecycle::Running.try_start(),
            ScreenLifecycle::Running
        );
    }

    #[test]
    fn lifecycle_serde_round_trip() {
        for v in [
            ScreenLifecycle::Idle,
            ScreenLifecycle::Starting,
            ScreenLifecycle::Running,
            ScreenLifecycle::Stopping,
        ] {
            let json = serde_json::to_string(&v).unwrap();
            let back: ScreenLifecycle = serde_json::from_str(&json).unwrap();
            assert_eq!(back, v);
        }
    }

    /// 4×2 BGRA, no row padding: row 0 is `0xAA…`, row 1 is `0xCC…`.
    /// Row order is preserved (top-down stays top-down); the helper
    /// only strips stride padding.
    #[test]
    fn copy_bgra_rows_packed_preserves_row_order() {
        let row_0 = [0xAAu8; 4 * 4];
        let row_1 = [0xCCu8; 4 * 4];
        let mut src = Vec::with_capacity(32);
        src.extend_from_slice(&row_0);
        src.extend_from_slice(&row_1);

        let out = copy_bgra_rows_packed(&src, 4, 2, 16);

        assert_eq!(out.len(), 4 * 2 * 4);
        assert_eq!(&out[0..16], &row_0, "row 0 (top) stays first");
        assert_eq!(&out[16..32], &row_1, "row 1 (bottom) stays last");
    }

    /// IOSurface rows commonly have trailing padding so
    /// `bytes_per_row > width * 4`. The padding bytes must be stripped
    /// (NOT carried into the output) so the encoder sees a tight BGRA
    /// buffer matching `width * height * 4`.
    #[test]
    fn copy_bgra_rows_packed_strips_row_padding() {
        // 3 cols × 4 = 12 packed bytes per row; stride 16 = 4 bytes pad.
        let row_0_packed = [
            0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C,
        ];
        let row_1_packed = [
            0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C,
        ];
        let pad = [0xFFu8; 4];

        let mut src = Vec::with_capacity(32);
        src.extend_from_slice(&row_0_packed);
        src.extend_from_slice(&pad);
        src.extend_from_slice(&row_1_packed);
        src.extend_from_slice(&pad);

        let out = copy_bgra_rows_packed(&src, 3, 2, 16);

        assert_eq!(out.len(), 24, "packed output: width*height*4 = 3*2*4");
        assert_eq!(&out[0..12], &row_0_packed, "row 0 first, padding stripped");
        assert_eq!(&out[12..24], &row_1_packed, "row 1 last, padding stripped");
        assert!(!out.contains(&0xFF), "no padding bytes should leak through");
    }

    #[test]
    fn copy_bgra_rows_packed_handles_single_row() {
        let row = [0x42u8; 8];
        let out = copy_bgra_rows_packed(&row, 2, 1, 8);
        assert_eq!(out, row, "1-row input is identical");
    }
}
