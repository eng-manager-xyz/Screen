//! Story: synthetic audio histogram rendered through wisp Graphics
//! (M-MEDIA.10 / AUT-106).
//!
//! Proves the seam — `media` quantizes audio + lays out bar geometry,
//! `wisp` draws the rectangles. wisp does not depend on `media`; this
//! story sits on the storybook side of the boundary.

use media::audio::AudioFormat;
use media::clock::MediaDuration;
use media::histogram::quantize;
use media::mock_audio::SineWaveSource;
use media::waveform::{BarMetric, WaveformDisplayMode, WaveformLayout, mono_bars};
use wisp::application::Application;
use wisp::math::Rect;
use wisp::{Color, Fill, Graphics, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "audio-histogram",
        category: "Media",
        title: "Synthetic audio histogram",
        milestone: "M-MEDIA.10",
        writeup: include_str!("writeups/audio_histogram.md"),
        build,
        tick: None,
    }
}

fn build(_app: &Application, stage: &mut Stage) {
    // 440 Hz sine, amplitude 0.6, mono, 1 second at 48 kHz.
    let fmt = AudioFormat::mono_f32(48_000);
    let mut src = SineWaveSource::new(fmt, 440.0, 0.6);
    let chunk = src.next_chunk(48_000);
    let histogram = quantize(&chunk, MediaDuration::from_millis(50)); // 20 bars

    let layout = WaveformLayout {
        origin_x: -0.85,
        baseline_y: 0.0,
        bar_width: 0.075,
        bar_gap: 0.012,
        max_height: 1.1,
        color: [1.0, 0.74, 0.30, 1.0], // amber
        metric: BarMetric::Peak,
        mode: WaveformDisplayMode::Mirrored,
    };
    let rects = mono_bars(&histogram, &layout);

    // Backdrop — a wide rounded panel under the bars to make the
    // amber + dark contrast carry without an alpha-on-black story.
    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba_u8(28, 32, 40, 255)));
    bg.draw_rounded_rect(Rect::new(-0.95, -0.7, 1.9, 1.4), 0.06);

    // Centerline — a thin horizontal strip at baseline_y.
    bg.fill(Fill::Solid(Color::rgba_u8(64, 72, 88, 255)));
    bg.draw_rect(Rect::new(-0.92, -0.005, 1.84, 0.010));

    let _ = stage.add_child(stage.root(), bg);

    // The bars themselves — pure rectangles, one Graphics node.
    let mut bars = Graphics::new();
    let [r, g, b, a] = layout.color;
    bars.fill(Fill::Solid(Color::rgba(r, g, b, a)));
    for rect in &rects {
        bars.draw_rect(Rect::new(rect.x, rect.y, rect.width, rect.height));
    }
    let _ = stage.add_child(stage.root(), bars);
}
