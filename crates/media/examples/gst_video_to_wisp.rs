//! Capture frames from `videotestsrc`, upload each to a wisp
//! `VideoTexture`, render through the standard `Sprite` pipeline, and
//! save the rendered output as PNGs (M-MEDIA.13 / AUT-109).
//!
//! Skips with an explanatory message if `gst-launch-1.0` isn't on
//! `PATH`. Output goes to `target/gst-video-frames/frame_NN.png` —
//! eight frames at 320×180, SMPTE colorbar default pattern from
//! `videotestsrc`.
//!
//! ```sh
//! cargo run -p media --example gst_video_to_wisp
//! ```

use std::path::Path;

use glam::Vec2;
use media::gstreamer_video::GstreamerVideoCapture;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::texture::video_texture::VideoTexture;
use wisp::{Color, RenderTexture, Sprite, Stage};

const WIDTH: u32 = 320;
const HEIGHT: u32 = 180;
const FRAMERATE: u32 = 30;
const FRAME_COUNT: usize = 8;
const OUT_DIR: &str = "target/gst-video-frames";

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    if !media::gstreamer::is_available() {
        eprintln!(
            "gst-launch-1.0 / gst-discoverer-1.0 not on PATH — install gstreamer\n\
             (e.g. `brew install gstreamer`) and rerun.\n\nSkipping."
        );
        return;
    }

    let mut capture = GstreamerVideoCapture::test_source(WIDTH, HEIGHT, FRAMERATE)
        .expect("spawn gst-launch-1.0 videotestsrc");

    let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("Renderer::new");
    let render_target = RenderTexture::with_format(&app, WIDTH, HEIGHT, format);

    let video_tex = VideoTexture::new(&app, WIDTH, HEIGHT);

    std::fs::create_dir_all(OUT_DIR).expect("create output dir");

    println!(
        "Capturing {FRAME_COUNT} frames from videotestsrc at {WIDTH}×{HEIGHT}@{FRAMERATE} fps\n\
         and rendering each through wisp → PNG.\n"
    );

    for i in 0..FRAME_COUNT {
        let frame = capture.next_frame().expect("read frame from GStreamer");
        video_tex.upload_bgra(&app, &frame.bgra);

        let mut stage = Stage::new();
        let sprite =
            Sprite::from_texture(video_tex.texture().clone()).with_anchor(Vec2::splat(0.5));
        let _ = stage.add_child(stage.root(), sprite);

        let _ = renderer.render_stage(
            &app,
            render_target.view(),
            Color::rgba_u8(12, 12, 16, 255),
            &stage,
        );

        let bytes = render_target.read_pixels(&app);
        let path = format!("{OUT_DIR}/frame_{i:02}.png");
        save_rgba_png(&path, WIDTH, HEIGHT, &bytes);

        #[expect(
            clippy::cast_possible_truncation,
            reason = "PTS seconds × 1e9 is well within i64 for any realistic capture duration"
        )]
        let pts_ns = (frame.pts_seconds * 1e9).round() as i64;
        println!(
            "  frame {i:02}: pts = {:.3} s ({pts_ns} ns), index = {}, png = {path}",
            frame.pts_seconds, frame.frame_index,
        );
    }

    println!(
        "\nSummary: {FRAME_COUNT} frames captured, {FRAME_COUNT} VideoTexture uploads,\n\
         {} cumulative gst frames emitted.",
        capture.frames_emitted()
    );
}

fn save_rgba_png(path: &str, width: u32, height: u32, rgba: &[u8]) {
    let img: image::RgbaImage =
        image::ImageBuffer::from_raw(width, height, rgba.to_vec()).expect("rgba size matches");
    img.save(Path::new(path)).expect("save png");
}
