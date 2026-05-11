//! Capture audio from a `GStreamer` `audiotestsrc`, quantize into a
//! histogram, and print bucket / RMS / peak statistics.
//!
//! Skips with an explanatory message if `GStreamer` isn't on `PATH`
//! (per the M-MEDIA.1 probe pattern). Closes the loop on M-MEDIA.11
//! (AUT-107): proves the same `quantize` pipeline that the storybook
//! uses also accepts real-shape `GStreamer` audio chunks.
//!
//! ```sh
//! cargo run -p media --example gst_audio_histogram
//! ```

use media::audio::AudioFormat;
use media::clock::MediaDuration;
use media::gstreamer_audio::GstreamerAudioCapture;
use media::histogram::quantize;
use media::{AudioChunk, AudioHistogram};

const CAPTURE_FRAMES: u64 = 48_000; // 1 s at 48 kHz
const BUCKET_MS: i64 = 50;
const SINE_HZ: f32 = 440.0;

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if !media::gstreamer::is_available() {
        eprintln!(
            "gst-launch-1.0 / gst-discoverer-1.0 not on PATH — install gstreamer \
             (e.g. `brew install gstreamer`) and rerun.\n\nSkipping."
        );
        return;
    }

    let format = AudioFormat::mono_f32(48_000);
    let mut capture = GstreamerAudioCapture::test_source(format, SINE_HZ)
        .expect("spawn gst-launch-1.0 audiotestsrc");

    let chunk = capture
        .next_chunk(CAPTURE_FRAMES)
        .expect("read 1 s of audio from GStreamer");

    let histogram = quantize(&chunk, MediaDuration::from_millis(BUCKET_MS));
    print_report(&chunk, &histogram);
}

fn print_report(chunk: &AudioChunk, histogram: &AudioHistogram) {
    let fmt = chunk.format();
    let pts = chunk.pts();
    println!("GStreamer audiotestsrc → AudioHistogram");
    println!("  source        : audiotestsrc freq={SINE_HZ:.1} Hz");
    println!(
        "  format        : {} ch, {} Hz, f32 LE",
        fmt.channels, fmt.sample_rate
    );
    println!(
        "  chunk         : {} frames, pts = {} ns",
        chunk.frame_count(),
        pts.as_nanos()
    );
    println!(
        "  bucket / bars : {} ms × {} bars",
        BUCKET_MS,
        histogram.len()
    );

    let (peak_max, rms_min, rms_max) = stats(histogram);
    println!("  peak max      : {peak_max:.4}\n  rms  min..max : {rms_min:.4} .. {rms_max:.4}");

    // Compact dump of the first 5 + last 5 bars.
    println!("  first 5 bars  :");
    for bar in histogram.bars.iter().take(5) {
        println!(
            "    pts={:>10} ns  peak={:.4}  rms={:.4}",
            bar.start_time.as_nanos(),
            bar.peak,
            bar.rms
        );
    }
    if histogram.len() > 5 {
        println!("  last 5 bars   :");
        for bar in histogram
            .bars
            .iter()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .iter()
            .rev()
        {
            println!(
                "    pts={:>10} ns  peak={:.4}  rms={:.4}",
                bar.start_time.as_nanos(),
                bar.peak,
                bar.rms
            );
        }
    }

    // audiotestsrc defaults to volume=0.8; RMS = 0.8 / √2 ≈ 0.5657.
    println!(
        "\nSanity check: audiotestsrc defaults to volume=0.8 → RMS ≈ 0.5657\n\
         (= 0.8 / √2). Real-mic captures will vary."
    );
}

fn stats(h: &AudioHistogram) -> (f32, f32, f32) {
    let mut peak_max = 0.0_f32;
    let mut rms_min = f32::INFINITY;
    let mut rms_max = 0.0_f32;
    for bar in &h.bars {
        peak_max = peak_max.max(bar.peak);
        rms_min = rms_min.min(bar.rms);
        rms_max = rms_max.max(bar.rms);
    }
    if h.bars.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        (peak_max, rms_min, rms_max)
    }
}
