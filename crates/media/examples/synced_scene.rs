//! Capstone for the M-MEDIA P1 tier: video frame + audio histogram
//! rendered in one Wisp scene, driven by a single `MediaClock`
//! (M-MEDIA.14 / AUT-110).
//!
//! Synthetic sources only — deterministic + reproducible:
//! - Video: 320×180 BGRA frames, hue rotates with the clock.
//! - Audio: 48 kHz mono `SineWaveSource` whose amplitude ramps from
//!   0.3 → 0.9 over the run — bar heights grow visibly per frame.
//! - Clock: `MediaClock::manual`, advanced by 100 ms per frame, 10
//!   frames over 1 s.
//!
//! For each frame, the example prints the video PTS and the histogram
//! window currently active (start time + bar count + peak / RMS
//! summary), and saves the rendered scene to
//! `target/synced-scene/frame_NN.png`.
//!
//! ```sh
//! cargo run -p media --example synced_scene
//! ```

use glam::Vec2;
use pollster::block_on;
use std::path::Path;

use media::audio::AudioFormat;
use media::clock::{MediaClock, MediaDuration, MediaTime};
use media::histogram::{AudioHistogram, quantize};
use media::mock_audio::SineWaveSource;
use media::waveform::{BarMetric, WaveformDisplayMode, WaveformLayout, mono_bars};
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::texture::video_texture::VideoTexture;
use wisp::{Color, Fill, Graphics, RenderTexture, Sprite, Stage};

const WIDTH: u32 = 640;
const HEIGHT: u32 = 360;
const VIDEO_W: u32 = 320;
const VIDEO_H: u32 = 180;
const FRAME_COUNT: usize = 10;
const FRAME_PERIOD: MediaDuration = MediaDuration::from_millis(100);
const BUCKET: MediaDuration = MediaDuration::from_millis(20);
const SAMPLE_RATE: u32 = 48_000;
const OUT_DIR: &str = "target/synced-scene";

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("Renderer::new");
    let render_target = RenderTexture::with_format(&app, WIDTH, HEIGHT, format);
    let video_tex = VideoTexture::new(&app, VIDEO_W, VIDEO_H);

    std::fs::create_dir_all(OUT_DIR).expect("create output dir");

    // One timeline. We advance it by FRAME_PERIOD each iteration; the
    // video PTS and the audio histogram both anchor against `now()`.
    // `advance_by` takes `&self` (interior mutability), so the binding
    // doesn't need `mut`.
    let clock = MediaClock::manual(MediaTime::ZERO);

    println!(
        "Synced scene: {FRAME_COUNT} frames × {} ms,\n  video {VIDEO_W}×{VIDEO_H}, audio {SAMPLE_RATE} Hz mono, bucket {} ms\n",
        FRAME_PERIOD.as_nanos() / 1_000_000,
        BUCKET.as_nanos() / 1_000_000,
    );

    for i in 0..FRAME_COUNT {
        let t = clock.now();

        // Audio for THIS frame only — one short chunk anchored at `t`.
        // Amplitude ramps so bar heights are visibly different between
        // frames, proving the scene reflects the active timeline window.
        let amp = ramp_amplitude(i);
        // SineWaveSource::next_chunk emits chunks anchored at the
        // source's own monotonic frame counter; for the scene we
        // splice a fresh chunk at the clock's `t` by reconstructing
        // with the right PTS. Mock source is cheap to recreate.
        let mut audio: SineWaveSource =
            SineWaveSource::new(AudioFormat::mono_f32(SAMPLE_RATE), 440.0, amp);
        let frames_per_window: u64 = (FRAME_PERIOD.as_nanos() / 1_000_000_000_i64
            * i64::from(SAMPLE_RATE)
            + FRAME_PERIOD.as_nanos() % 1_000_000_000_i64 * i64::from(SAMPLE_RATE) / 1_000_000_000)
            .try_into()
            .expect("positive frame count");
        let raw_chunk = audio.next_chunk(frames_per_window);
        let chunk =
            media::audio::AudioChunk::new(raw_chunk.format(), raw_chunk.samples().to_vec(), t)
                .expect("re-anchored chunk is valid");
        let histogram = quantize(&chunk, BUCKET);

        // Synthetic video frame for this PTS — hue rotates over time.
        let bgra = synth_frame_bgra(i);
        video_tex.upload_bgra(&app, &bgra);

        // Compose the scene: video sprite on top, histogram bars on
        // bottom, with a thin divider.
        let mut stage = Stage::new();
        let root = stage.root();

        // Video sprite — NDC y in [0.05, 0.95] (top half of screen).
        let mut sprite =
            Sprite::from_texture(video_tex.texture().clone()).with_anchor(Vec2::splat(0.5));
        sprite.container.transform.position = Vec2::new(0.0, 0.45);
        sprite.container.transform.scale = Vec2::new(1.5, 0.85);
        let _ = stage.add_child(root, sprite);

        // Histogram bar layout — bottom 40% of NDC space.
        let layout = WaveformLayout {
            origin_x: -0.85,
            baseline_y: -0.85,
            bar_width: 0.06,
            bar_gap: 0.014,
            max_height: 0.40,
            color: [1.0, 0.74, 0.30, 1.0],
            metric: BarMetric::Peak,
            mode: WaveformDisplayMode::Anchored,
        };
        let rects = mono_bars(&histogram, &layout);

        // Divider + bars + a level indicator line.
        let mut chrome = Graphics::new();
        chrome.fill(Fill::Solid(Color::rgba_u8(48, 52, 64, 255)));
        chrome.draw_rect(Rect::new(-0.95, -0.04, 1.9, 0.008));
        let _ = stage.add_child(root, chrome);

        let mut bars = Graphics::new();
        let [br, bg, bb, ba] = layout.color;
        bars.fill(Fill::Solid(Color::rgba(br, bg, bb, ba)));
        for rect in &rects {
            bars.draw_rect(Rect::new(rect.x, rect.y, rect.width, rect.height));
        }
        let _ = stage.add_child(root, bars);

        let _ = renderer.render_stage(
            &app,
            render_target.view(),
            Color::rgba_u8(12, 14, 18, 255),
            &stage,
        );

        let pixels = render_target.read_pixels(&app);
        let path = format!("{OUT_DIR}/frame_{i:02}.png");
        save_rgba_png(&path, WIDTH, HEIGHT, &pixels);

        let (peak_max, rms_max) = peak_rms(&histogram);
        println!(
            "  frame {i:02}: video.pts = {:>7.3} s | hist.window = [{:>5.3} s, +{} ms × {:>2} bars] amp={amp:.2} peak={peak_max:.3} rms={rms_max:.3}",
            t.as_seconds(),
            t.as_seconds(),
            BUCKET.as_nanos() / 1_000_000,
            histogram.len(),
        );

        clock.advance_by(FRAME_PERIOD);
    }

    println!("\n{FRAME_COUNT} frames written to {OUT_DIR}/.");
}

fn ramp_amplitude(i: usize) -> f32 {
    // i ∈ [0, FRAME_COUNT - 1] → amplitude ∈ [0.30, 0.90].
    let denom = u16::try_from((FRAME_COUNT - 1).max(1)).unwrap_or(1);
    let idx = u16::try_from(i).unwrap_or(0);
    let frac = f32::from(idx) / f32::from(denom);
    0.30 + 0.60 * frac
}

fn synth_frame_bgra(frame_idx: usize) -> Vec<u8> {
    let pixels_w = VIDEO_W as usize;
    let pixels_h = VIDEO_H as usize;
    let mut bgra = vec![0u8; pixels_w * pixels_h * 4];
    // Hue rotates across frames — every frame visibly different.
    // Use u8 throughout to keep arithmetic in lint-friendly territory.
    let hue_shift: u8 = u8::try_from(frame_idx).unwrap_or(0).wrapping_mul(24);
    for row in 0..pixels_h {
        for col in 0..pixels_w {
            let idx = (row * pixels_w + col) * 4;
            let red = u8::try_from((col * 255) / pixels_w).unwrap_or(255);
            let green = u8::try_from((row * 255) / pixels_h).unwrap_or(255);
            let blue = u8::try_from(((row + col) * 255) / (pixels_w + pixels_h)).unwrap_or(255);
            // Cyclic shift of each channel by an offset multiple.
            let red = red.wrapping_add(hue_shift);
            let green = green.wrapping_add(hue_shift.wrapping_mul(2));
            let blue = blue.wrapping_add(hue_shift.wrapping_mul(3));
            bgra[idx] = blue;
            bgra[idx + 1] = green;
            bgra[idx + 2] = red;
            bgra[idx + 3] = 255;
        }
    }
    bgra
}

fn peak_rms(h: &AudioHistogram) -> (f32, f32) {
    let mut peak_max = 0.0_f32;
    let mut rms_max = 0.0_f32;
    for bar in &h.bars {
        peak_max = peak_max.max(bar.peak);
        rms_max = rms_max.max(bar.rms);
    }
    (peak_max, rms_max)
}

fn save_rgba_png(path: &str, width: u32, height: u32, rgba: &[u8]) {
    let img: image::RgbaImage =
        image::ImageBuffer::from_raw(width, height, rgba.to_vec()).expect("rgba size matches");
    img.save(Path::new(path)).expect("save png");
}
