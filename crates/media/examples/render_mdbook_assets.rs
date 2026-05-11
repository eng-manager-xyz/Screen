#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::while_let_loop,
    reason = "single-shot example renderer; dimensions + sample-window widths are validated by construction and the chapter prose explains the math"
)]

//! `cargo run -p media --example render-mdbook-assets`
//!
//! Renders the PNG assets the `media` mdBook chapters embed:
//!
//! - `assets/media/mock-sources-waveform.png` — three-panel line plot of
//!   the M-MEDIA.4 mock audio sources (sine / silence / pulse).
//! - `assets/media/histogram-three-panel.png` — three-panel histogram
//!   bar chart for the same three sources at 50 ms buckets.
//! - `assets/media/audio-capture-waveform.png` — the bundled MP3
//!   fixture's waveform (full duration) after gstreamer decode.
//! - `assets/media/audio-capture-histogram.png` — the same MP3 fixture
//!   quantized at 50 ms buckets.
//!
//! All PNGs use a **white background** per the CLAUDE.md asset rule
//! (audio shape is visible against light; gradient backgrounds bury it).
//! Drawing is pure CPU via the `image` crate — no wgpu, no fonts. Chapter
//! prose carries the labels.

use std::path::{Path, PathBuf};

use image::{Rgba, RgbaImage};
use media::audio::AudioFormat;
use media::gstreamer::is_available;
use media::gstreamer_audio::GstreamerAudioCapture;
use media::histogram::{AudioHistogram, quantize};
use media::mock_audio::{SilenceSource, SineWaveSource, StepPulseSource};
use media::{AudioChunk, MediaDuration, MediaTime};

const BG: Rgba<u8> = Rgba([248, 248, 250, 255]);
const AXIS: Rgba<u8> = Rgba([210, 210, 215, 255]);
const FRAME: Rgba<u8> = Rgba([60, 60, 70, 255]);
const WAVE: Rgba<u8> = Rgba([40, 80, 160, 255]);
const PEAK: Rgba<u8> = Rgba([200, 200, 215, 255]);
const RMS: Rgba<u8> = Rgba([240, 130, 30, 255]);
const PANEL_GAP: u32 = 12;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("media=info")),
        )
        .init();

    let out_dir = PathBuf::from("_docs/book/src/assets/media");
    std::fs::create_dir_all(&out_dir).expect("mkdir media assets");

    // 1. Mock sources — three waveform panels.
    {
        let sine = sine_chunk(440.0, 0.7, 480); // ~10 ms at 48 kHz = many cycles of 440 Hz
        let silence = silence_chunk(480);
        let pulse = pulse_chunk(120, 480);
        let img =
            three_panel_waveform(&[("sine", &sine), ("silence", &silence), ("pulse", &pulse)]);
        let path = out_dir.join("mock-sources-waveform.png");
        img.save(&path).expect("save mock-sources-waveform");
        println!("wrote {}", path.display());
    }

    // 2. Histogram — same three sources, 50 ms buckets, 1 s.
    {
        let fmt = AudioFormat::mono_f32(48_000);
        let sine_chunk = SineWaveSource::new(fmt, 1_000.0, 0.7).next_chunk(48_000);
        let silence_chunk = SilenceSource::new(fmt).next_chunk(48_000);
        let pulse_chunk = StepPulseSource::new(fmt, 6_000).next_chunk(48_000);
        let bucket = MediaDuration::from_millis(50);
        let h_sine = quantize(&sine_chunk, bucket);
        let h_silence = quantize(&silence_chunk, bucket);
        let h_pulse = quantize(&pulse_chunk, bucket);
        let img = three_panel_histogram(&[
            ("sine", &h_sine),
            ("silence", &h_silence),
            ("pulse", &h_pulse),
        ]);
        let path = out_dir.join("histogram-three-panel.png");
        img.save(&path).expect("save histogram-three-panel");
        println!("wrote {}", path.display());
    }

    // 3. + 4. — MP3 fixture waveform + histogram. Requires GStreamer.
    if !is_available() {
        eprintln!("gstreamer not on PATH — skipping MP3-fixture assets");
        return;
    }
    let fixture = Path::new("crates/media/tests/fixtures/sample-audio.mp3");
    if !fixture.exists() {
        eprintln!("MP3 fixture missing — skipping");
        return;
    }
    let mp3_fmt = AudioFormat::mono_f32(44_100);
    let mut cap = GstreamerAudioCapture::from_file(fixture, mp3_fmt).expect("open mp3 fixture");
    let mut all_samples: Vec<f32> = Vec::new();
    loop {
        match cap.next_chunk(4_410) {
            Ok(chunk) => all_samples.extend_from_slice(chunk.samples()),
            Err(_) => break,
        }
    }
    // Wrap as a single AudioChunk for the renderer.
    let captured = AudioChunk::new(mp3_fmt, all_samples, MediaTime::ZERO)
        .expect("build chunk from captured samples");

    {
        // Slice to the first 100 ms so the individual sine cycles are
        // visible. Full-duration envelope of a pure 440 Hz tone is a
        // solid block — accurate but uninformative.
        let first_100ms_samples =
            (captured.format().sample_rate as usize / 10).min(captured.samples().len());
        let slice = captured.samples()[..first_100ms_samples].to_vec();
        let zoomed =
            AudioChunk::new(captured.format(), slice, MediaTime::ZERO).expect("zoomed chunk");
        let img = single_panel_waveform("First 100 ms of MP3 fixture", &zoomed, 1200, 220);
        let path = out_dir.join("audio-capture-waveform.png");
        img.save(&path).expect("save audio-capture-waveform");
        println!("wrote {}", path.display());
    }
    {
        // 200 ms buckets keep the bar count visible (≈ 175 bars over
        // the 35-s fixture); 50 ms crushes them into a solid block.
        let bucket = MediaDuration::from_millis(200);
        let h = quantize(&captured, bucket);
        let img = single_panel_histogram("MP3 fixture @ 200 ms buckets", &h, 1200, 220);
        let path = out_dir.join("audio-capture-histogram.png");
        img.save(&path).expect("save audio-capture-histogram");
        println!("wrote {}", path.display());
    }
}

// ─── Source helpers ──────────────────────────────────────────────────

fn sine_chunk(freq: f32, amplitude: f32, frames: u64) -> AudioChunk {
    let fmt = AudioFormat::mono_f32(48_000);
    SineWaveSource::new(fmt, freq, amplitude).next_chunk(frames)
}

fn silence_chunk(frames: u64) -> AudioChunk {
    let fmt = AudioFormat::mono_f32(48_000);
    SilenceSource::new(fmt).next_chunk(frames)
}

fn pulse_chunk(spike_at: u64, frames: u64) -> AudioChunk {
    let fmt = AudioFormat::mono_f32(48_000);
    StepPulseSource::new(fmt, spike_at).next_chunk(frames)
}

// ─── Three-panel waveform ────────────────────────────────────────────

fn three_panel_waveform(panels: &[(&str, &AudioChunk)]) -> RgbaImage {
    let panel_w = 400;
    let panel_h = 200;
    let _ = panels[0].0; // labels live in chapter prose — keep signature symmetric.
    let total_w = panel_w * 3 + PANEL_GAP * 4;
    let total_h = panel_h + PANEL_GAP * 2;
    let mut img = RgbaImage::from_pixel(total_w, total_h, BG);
    for (i, (_, chunk)) in panels.iter().enumerate() {
        let x = PANEL_GAP + (panel_w + PANEL_GAP) * (i as u32);
        let y = PANEL_GAP;
        draw_waveform_panel(&mut img, x, y, panel_w, panel_h, chunk);
    }
    img
}

fn single_panel_waveform(_label: &str, chunk: &AudioChunk, w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(w, h, BG);
    draw_waveform_panel(&mut img, 0, 0, w, h, chunk);
    img
}

fn draw_waveform_panel(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, chunk: &AudioChunk) {
    draw_panel_frame(img, x, y, w, h);
    let mid_y = y + h / 2;
    // Zero axis.
    for px in x..(x + w) {
        img.put_pixel(px, mid_y, AXIS);
    }
    let samples = chunk.samples();
    if samples.is_empty() {
        return;
    }
    let inner_w = w.saturating_sub(2);
    let amp_half = (h / 2).saturating_sub(8);
    let n = samples.len();
    // For each pixel column, find the min + max sample in the
    // corresponding window of audio. Rendering the envelope (rather
    // than a single sample point) shows the signal's amplitude
    // continuously — at high frequencies, single-sample plots
    // alias into noise-looking dots; the envelope reads as a clean
    // band.
    for col in 0..inner_w {
        let lo = (n as f32 * col as f32 / inner_w as f32) as usize;
        let hi = ((n as f32 * (col + 1) as f32 / inner_w as f32) as usize).min(n);
        let window = if hi > lo {
            &samples[lo..hi]
        } else {
            &samples[lo..(lo + 1).min(n)]
        };
        let (mn, mx) = window
            .iter()
            .copied()
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), x| {
                (a.min(x), b.max(x))
            });
        let mn = mn.clamp(-1.0, 1.0);
        let mx = mx.clamp(-1.0, 1.0);
        let dy_hi = (mx * amp_half as f32) as i32;
        let dy_lo = (mn * amp_half as f32) as i32;
        let px = x + 1 + col;
        let y_top = (mid_y as i32 - dy_hi).max(y as i32) as u32;
        let y_bot = (mid_y as i32 - dy_lo).min((y + h - 1) as i32) as u32;
        for py in y_top..=y_bot {
            if py < img.height() && px < img.width() {
                img.put_pixel(px, py, WAVE);
            }
        }
    }
}

// ─── Three-panel histogram ───────────────────────────────────────────

fn three_panel_histogram(panels: &[(&str, &AudioHistogram)]) -> RgbaImage {
    let panel_w = 400;
    let panel_h = 200;
    let total_w = panel_w * 3 + PANEL_GAP * 4;
    let total_h = panel_h + PANEL_GAP * 2;
    let mut img = RgbaImage::from_pixel(total_w, total_h, BG);
    for (i, (_, h)) in panels.iter().enumerate() {
        let x = PANEL_GAP + (panel_w + PANEL_GAP) * (i as u32);
        let y = PANEL_GAP;
        draw_histogram_panel(&mut img, x, y, panel_w, panel_h, h);
    }
    img
}

fn single_panel_histogram(_label: &str, h: &AudioHistogram, w: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::from_pixel(w, height, BG);
    draw_histogram_panel(&mut img, 0, 0, w, height, h);
    img
}

fn draw_histogram_panel(
    img: &mut RgbaImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    hist: &AudioHistogram,
) {
    draw_panel_frame(img, x, y, w, h);
    let baseline = y + h - 12;
    // Baseline.
    for px in x..(x + w) {
        img.put_pixel(px, baseline, AXIS);
    }
    let bars = &hist.bars;
    if bars.is_empty() {
        return;
    }
    let inner_w = w.saturating_sub(8);
    let amp_max = h.saturating_sub(24);
    let bar_count = bars.len() as u32;
    let bar_w = (inner_w / bar_count).max(1);
    let pad = (inner_w.saturating_sub(bar_w * bar_count)) / 2;
    for (i, bar) in bars.iter().enumerate() {
        let bx = x + 4 + pad + (i as u32) * bar_w;
        let peak_h = (bar.peak * amp_max as f32) as u32;
        let rms_h = (bar.rms * amp_max as f32) as u32;
        // Peak bar (light).
        fill_rect(
            img,
            bx,
            baseline.saturating_sub(peak_h),
            bar_w.saturating_sub(1).max(1),
            peak_h,
            PEAK,
        );
        // RMS bar (bright, narrower so it sits inside peak).
        let rms_w = (bar_w / 2).max(1);
        let rms_x = bx + (bar_w.saturating_sub(rms_w)) / 2;
        fill_rect(
            img,
            rms_x,
            baseline.saturating_sub(rms_h),
            rms_w,
            rms_h,
            RMS,
        );
    }
}

// ─── Drawing primitives ──────────────────────────────────────────────

fn draw_panel_frame(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32) {
    // 1 px frame.
    for px in x..(x + w) {
        if y < img.height() {
            img.put_pixel(px, y, FRAME);
        }
        let py = y + h - 1;
        if py < img.height() {
            img.put_pixel(px, py, FRAME);
        }
    }
    for py in y..(y + h) {
        if x < img.width() && py < img.height() {
            img.put_pixel(x, py, FRAME);
        }
        let px = x + w - 1;
        if px < img.width() && py < img.height() {
            img.put_pixel(px, py, FRAME);
        }
    }
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, w: u32, h: u32, color: Rgba<u8>) {
    for px in x..(x + w).min(img.width()) {
        for py in y..(y + h).min(img.height()) {
            img.put_pixel(px, py, color);
        }
    }
}
