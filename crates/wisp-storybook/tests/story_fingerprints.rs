//! Visual-regression fingerprints for every shipped story.
//!
//! For each story we render at 256×256, divide into a 4×4 grid of quadrants,
//! and average each quadrant's RGBA. Values are bucketed to multiples of 8
//! to absorb sub-percent driver-level variation while still catching real
//! visual changes.
//!
//! When a story changes intentionally:
//!   `cargo insta review` (or `INSTA_UPDATE=auto cargo nextest run`).
//! When it changes unintentionally: the snapshot diff shows which quadrants
//! drifted.

use std::collections::BTreeMap;

use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Stage};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 256;
const QUADS_PER_AXIS: u32 = 4;
const BUCKET: u32 = 8;

fn boot() -> Application {
    block_on(Application::new(AppConfig::default())).expect("init wisp")
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "byte sums fit easily in u32 (max 256*256*255 ≈ 16.7M)"
)]
fn fingerprint(bytes: &[u8]) -> Vec<[u32; 4]> {
    let qw = SIZE / QUADS_PER_AXIS;
    let qh = SIZE / QUADS_PER_AXIS;
    let mut out = Vec::with_capacity((QUADS_PER_AXIS * QUADS_PER_AXIS) as usize);
    for qy in 0..QUADS_PER_AXIS {
        for qx in 0..QUADS_PER_AXIS {
            let mut sum = [0u64; 4];
            let mut count = 0u64;
            for y in (qy * qh)..((qy + 1) * qh) {
                for x in (qx * qw)..((qx + 1) * qw) {
                    let idx = ((y * SIZE + x) * 4) as usize;
                    sum[0] += u64::from(bytes[idx]);
                    sum[1] += u64::from(bytes[idx + 1]);
                    sum[2] += u64::from(bytes[idx + 2]);
                    sum[3] += u64::from(bytes[idx + 3]);
                    count += 1;
                }
            }
            let bucket = |s: u64| {
                let avg = (s / count) as u32;
                (avg / BUCKET) * BUCKET
            };
            out.push([
                bucket(sum[0]),
                bucket(sum[1]),
                bucket(sum[2]),
                bucket(sum[3]),
            ]);
        }
    }
    out
}

#[test]
fn story_fingerprints_match_snapshot() {
    // **macOS is the visual-truth runner.** This snapshot captures
    // exact bucketed pixel averages from every story; it's the
    // canonical visual-regression test for the wisp renderer.
    // Skipped on:
    //
    // - **Linux CI** (`WISP_SKIP_GPU_FILTER_TESTS` set) — the filter
    //   stories that lavapipe can't render are filtered upstream, so
    //   the story set differs from macOS and the snapshot mismatches.
    // - **Windows CI** (`target_os = "windows"`) — DX12 produces
    //   text glyphs with slightly different subpixel positioning
    //   than Metal (cosmic-text + glyphon FP precision). Diffs are
    //   8–16 units off (one bucket), not real regressions. macOS
    //   already validates the visual contract; Windows gates the
    //   build path + non-visual correctness.
    //
    // Don't introduce parallel per-OS snapshot files — they drift
    // independently and double the maintenance cost for zero new
    // signal.
    if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some() || cfg!(target_os = "windows") {
        eprintln!(
            "skipping story_fingerprints snapshot — validated on macos-latest \
             with real Metal; non-macOS OSes differ in subpixel text rendering"
        );
        return;
    }
    let app = boot();
    let rt = RenderTexture::with_format(&app, SIZE, SIZE, FORMAT);
    let renderer = Renderer::new(&app, FORMAT).expect("renderer");

    let mut all = BTreeMap::<String, Vec<[u32; 4]>>::new();

    for story in wisp_storybook::stories::all_stories() {
        let mut stage = Stage::new();
        (story.build)(&app, &mut stage);
        story.tick(&mut stage, 0.0);
        let _ = renderer.render_stage(&app, rt.view(), Color::rgba_u8(18, 18, 22, 255), &stage);
        let bytes = rt.read_pixels(&app);
        all.insert(
            format!("{} ({})", story.title, story.milestone),
            fingerprint(&bytes),
        );
    }

    insta::assert_yaml_snapshot!("story_fingerprints", all);
}
