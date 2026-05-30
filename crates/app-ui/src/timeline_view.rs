//! Timeline coordinate system + ruler (ED.8 / M-EDIT).
//!
//! [`TimelineViewport`] is the pure frame↔pixel mapping — zoom
//! (`px_per_frame`), scroll (`scroll_frame`), and "nice" ruler-tick
//! generation — that the whole timeline (ruler, lanes, playhead, snapping)
//! hangs off. It is GPU/DOM-free and exhaustively unit-tested at multiple
//! zoom levels.
//!
//! [`TimelineRuler`] renders a fit-to-width ruler (the full-clip "global
//! progress" view, decoupled from per-lane zoom) with frame-correct tick
//! labels, a reactive playhead, and click-to-seek.

use leptos::prelude::*;
use wasm_bindgen::JsCast;

use crate::editor_ipc::{self, EditorStatus, TransportAction};

/// Zoom bounds, in pixels per frame.
const MIN_PX_PER_FRAME: f64 = 0.01;
const MAX_PX_PER_FRAME: f64 = 40.0;
/// Target minimum spacing between ruler tick labels, in pixels.
const MIN_TICK_SPACING_PX: f64 = 64.0;

/// One labeled tick on the ruler.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RulerTick {
    /// Project frame the tick sits at.
    pub frame: u64,
    /// `M:SS` label.
    pub label: String,
}

/// Maps between project frames and timeline pixels at a zoom + scroll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimelineViewport {
    px_per_frame: f64,
    scroll_frame: f64,
    width_px: f64,
    fps: u32,
    duration_frames: u64,
}

#[allow(
    clippy::cast_precision_loss,
    reason = "frame counts are well under 2^52; u64→f64 is lossless at these magnitudes"
)]
fn frames_f64(frames: u64) -> f64 {
    frames as f64
}

impl TimelineViewport {
    /// A viewport zoomed so the whole clip fits in `width_px`.
    #[must_use]
    pub fn fit(duration_frames: u64, fps: u32, width_px: f64) -> Self {
        let width_px = width_px.max(1.0);
        let dur = frames_f64(duration_frames.max(1));
        let px_per_frame = (width_px / dur).clamp(MIN_PX_PER_FRAME, MAX_PX_PER_FRAME);
        Self {
            px_per_frame,
            scroll_frame: 0.0,
            width_px,
            fps: fps.max(1),
            duration_frames,
        }
    }

    /// Pixels per frame (the zoom level).
    #[must_use]
    pub fn px_per_frame(&self) -> f64 {
        self.px_per_frame
    }

    /// Leftmost visible frame.
    #[must_use]
    pub fn scroll_frame(&self) -> f64 {
        self.scroll_frame
    }

    /// Pixel x of a frame within the viewport.
    #[must_use]
    pub fn frame_to_px(&self, frame: f64) -> f64 {
        (frame - self.scroll_frame) * self.px_per_frame
    }

    /// Frame at a pixel x within the viewport.
    #[must_use]
    pub fn px_to_frame(&self, px: f64) -> f64 {
        self.scroll_frame + px / self.px_per_frame
    }

    /// Fraction `0..=1` of the viewport width for `frame` (for percent-based
    /// CSS positioning of a responsive, fit-to-width ruler).
    #[must_use]
    pub fn frame_to_fraction(&self, frame: f64) -> f64 {
        if self.width_px <= 0.0 {
            return 0.0;
        }
        (self.frame_to_px(frame) / self.width_px).clamp(0.0, 1.0)
    }

    /// Visible frame range `(first, last)`.
    #[must_use]
    pub fn visible_range(&self) -> (f64, f64) {
        (
            self.scroll_frame,
            self.scroll_frame + self.width_px / self.px_per_frame,
        )
    }

    /// Zoom by `factor` (>1 = in) keeping the frame under `anchor_px` fixed —
    /// so zooming centred on the playhead keeps the playhead put.
    pub fn zoom_at(&mut self, factor: f64, anchor_px: f64) {
        let anchor_frame = self.px_to_frame(anchor_px);
        self.px_per_frame = (self.px_per_frame * factor).clamp(MIN_PX_PER_FRAME, MAX_PX_PER_FRAME);
        self.scroll_frame = anchor_frame - anchor_px / self.px_per_frame;
        self.clamp_scroll();
    }

    /// Pan by `dx` pixels (positive `dx` scrolls the content left).
    pub fn pan_px(&mut self, dx: f64) {
        self.scroll_frame -= dx / self.px_per_frame;
        self.clamp_scroll();
    }

    fn clamp_scroll(&mut self) {
        let visible = self.width_px / self.px_per_frame;
        let max_scroll = (frames_f64(self.duration_frames) - visible).max(0.0);
        self.scroll_frame = self.scroll_frame.clamp(0.0, max_scroll);
    }

    /// Labeled ruler ticks across the visible range, at a "nice" second
    /// interval chosen so labels stay at least [`MIN_TICK_SPACING_PX`] apart.
    /// Frame-correct at every zoom level.
    #[must_use]
    pub fn ruler_ticks(&self) -> Vec<RulerTick> {
        let px_per_second = self.px_per_frame * f64::from(self.fps);
        let interval_frames = nice_second_interval(px_per_second) * u64::from(self.fps);
        if interval_frames == 0 {
            return Vec::new();
        }
        let (first, last) = self.visible_range();
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "first is clamped non-negative; frame counts fit u64"
        )]
        let first_frame = first.max(0.0) as u64;
        let start = (first_frame / interval_frames) * interval_frames;
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "last is positive and bounded by the clip duration"
        )]
        let last_frame = (last.ceil() as u64).min(self.duration_frames);
        let mut ticks = Vec::new();
        let mut frame = start;
        while frame <= last_frame {
            ticks.push(RulerTick {
                frame,
                label: format_clock(frame, self.fps),
            });
            frame += interval_frames;
            if ticks.len() > 1024 {
                break; // safety net against a degenerate interval
            }
        }
        ticks
    }
}

/// Smallest "nice" second interval whose pixel width is at least
/// [`MIN_TICK_SPACING_PX`] — so ruler labels never crowd.
fn nice_second_interval(px_per_second: f64) -> u64 {
    const CANDIDATES: [u64; 11] = [1, 2, 5, 10, 15, 30, 60, 120, 300, 600, 1800];
    for &secs in &CANDIDATES {
        #[allow(
            clippy::cast_precision_loss,
            reason = "interval candidates are tiny; exact in f64"
        )]
        let width = px_per_second * secs as f64;
        if width >= MIN_TICK_SPACING_PX {
            return secs;
        }
    }
    *CANDIDATES.last().unwrap_or(&1)
}

/// `M:SS` clock label for a frame.
fn format_clock(frame: u64, fps: u32) -> String {
    let secs = frame / u64::from(fps.max(1));
    format!("{}:{:02}", secs / 60, secs % 60)
}

/// A fit-to-width ruler: frame-correct tick labels + a reactive playhead +
/// click-to-seek. Reads the playhead from the editor-status context.
#[component]
pub fn TimelineRuler() -> impl IntoView {
    let status = use_context::<RwSignal<EditorStatus>>()
        .unwrap_or_else(|| RwSignal::new(EditorStatus::default()));
    // A nominal width is used only to pick the tick interval; positioning is
    // percent-based so the ruler is responsive to the real element width.
    let viewport = move || {
        let st = status.get();
        TimelineViewport::fit(st.duration_frames, st.fps, 1000.0)
    };
    view! {
        <div
            class="timeline-ruler"
            on:click=move |ev| {
                let Some(target) = ev.current_target() else { return };
                let Ok(el) = target.dyn_into::<web_sys::Element>() else { return };
                let width = el.client_width();
                if width <= 0 {
                    return;
                }
                let st = status.get_untracked();
                let frac = (f64::from(ev.offset_x()) / f64::from(width)).clamp(0.0, 1.0);
                #[allow(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "frac in [0,1]; product with the (u64) duration is non-negative and in range"
                )]
                let frame = (frac * frames_f64(st.duration_frames)) as u64;
                editor_ipc::editor_transport(&TransportAction::Seek { frame });
            }
        >
            {move || {
                let vp = viewport();
                vp.ruler_ticks()
                    .into_iter()
                    .map(|tick| {
                        let left = vp.frame_to_fraction(frames_f64(tick.frame)) * 100.0;
                        view! {
                            <span class="timeline-tick" style=format!("left:{left:.3}%")>
                                {tick.label}
                            </span>
                        }
                    })
                    .collect_view()
            }}
            <div
                class="timeline-playhead"
                style=move || {
                    let vp = viewport();
                    let left = vp.frame_to_fraction(frames_f64(status.get().current_frame)) * 100.0;
                    format!("left:{left:.3}%")
                }
            ></div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_px_round_trips() {
        let vp = TimelineViewport::fit(900, 30, 600.0);
        for frame in [0.0, 1.0, 100.0, 450.0, 899.0] {
            let px = vp.frame_to_px(frame);
            assert!(
                (vp.px_to_frame(px) - frame).abs() < 1e-6,
                "round-trip {frame}"
            );
        }
    }

    #[test]
    fn fit_spans_full_width() {
        let vp = TimelineViewport::fit(900, 30, 600.0);
        assert!((vp.frame_to_fraction(0.0)).abs() < 1e-9);
        assert!((vp.frame_to_fraction(900.0) - 1.0).abs() < 1e-9);
        assert!((vp.frame_to_fraction(450.0) - 0.5).abs() < 1e-3);
    }

    #[test]
    fn zoom_keeps_anchor_frame_put() {
        let mut vp = TimelineViewport::fit(9000, 30, 600.0);
        let anchor_px = 300.0;
        let before = vp.px_to_frame(anchor_px);
        vp.zoom_at(4.0, anchor_px);
        let after = vp.px_to_frame(anchor_px);
        assert!(
            (before - after).abs() < 1e-6,
            "anchor frame stayed put under zoom"
        );
        assert!(vp.px_per_frame() > TimelineViewport::fit(9000, 30, 600.0).px_per_frame());
    }

    #[test]
    fn scroll_clamps_within_clip() {
        let mut vp = TimelineViewport::fit(9000, 30, 600.0);
        vp.zoom_at(8.0, 0.0);
        vp.pan_px(-1_000_000.0); // pan way past the end
        let (_first, last) = vp.visible_range();
        assert!(
            last <= 9000.0 + 1.0,
            "can't scroll past the clip end, last={last}"
        );
        vp.pan_px(1_000_000.0); // and back past the start
        assert!(vp.scroll_frame() >= -1e-9, "can't scroll before frame 0");
    }

    #[test]
    fn ruler_ticks_are_frame_correct_and_spaced() {
        // 5s clip @ 30fps fit into 600px → ~4px/frame → ~120px/s.
        let vp = TimelineViewport::fit(150, 30, 600.0);
        let ticks = vp.ruler_ticks();
        assert!(!ticks.is_empty());
        // First tick at frame 0, labeled 0:00.
        assert_eq!(
            ticks[0],
            RulerTick {
                frame: 0,
                label: "0:00".into()
            }
        );
        // Ticks land on whole-second multiples (interval = N×fps).
        for t in &ticks {
            assert_eq!(t.frame % 30, 0, "tick {t:?} on a second boundary");
        }
    }

    #[test]
    fn nice_interval_widens_as_we_zoom_out() {
        // Lots of px/sec → 1s ticks; very few px/sec → coarse ticks.
        assert_eq!(nice_second_interval(200.0), 1);
        assert_eq!(nice_second_interval(10.0), 10); // 10px/s → 10s interval ≈100px
        assert!(nice_second_interval(0.05) >= 600);
    }

    #[test]
    fn clock_label_format() {
        assert_eq!(format_clock(0, 30), "0:00");
        assert_eq!(format_clock(900, 30), "0:30");
        assert_eq!(format_clock(1800, 30), "1:00");
        assert_eq!(format_clock(3690, 30), "2:03");
    }
}
