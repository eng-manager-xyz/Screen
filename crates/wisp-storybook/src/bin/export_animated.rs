//! `cargo run -p wisp-storybook --bin wisp-export-animated`
//!
//! Renders every animated story (`tick: Some(...)`) to an MP4 by
//! piping raw RGBA frames into a `gst-launch-1.0` subprocess that
//! encodes via `x264enc + mp4mux`. Outputs land at
//! `_docs/wisp-book/src/assets/wisp/<id>.mp4` next to the existing PNGs.
//!
//! Local-only by design — gstreamer 1.x must be on PATH (`brew
//! install gstreamer`). CI does not run this binary; mdBook chapters
//! that embed an MP4 stay green via the static `<video>` element +
//! the committed `.mp4` file. Run before commit when an animated
//! story changes.
//!
//! Pipeline:
//! ```text
//! fdsrc fd=0
//!   ! rawvideoparse width=W height=H format=rgba framerate=30/1
//!   ! videoconvert
//!   ! x264enc tune=zerolatency speed-preset=fast bitrate=2000
//!   ! mp4mux
//!   ! filesink location=<path>
//! ```
//!
//! `rawvideoparse` generates PTS timestamps from the framerate caps —
//! the plain `video/x-raw,...` caps blob upstream of `mp4mux` produces
//! "Buffer has no PTS" failures.
//!
//! Allow-list: stories listed in `ANIMATED` are exported. The rest
//! still go through `wisp-export-stories` for their static PNGs.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use pollster::block_on;
use wisp::Stage;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture};
use wisp_storybook::stories::all_stories;

const W: u32 = 256;
const H: u32 = 256;
const FPS: u32 = 30;
const DURATION_SECS: f32 = 3.0;

/// Story IDs we render to MP4. Each id must correspond to a story
/// with `tick: Some(...)`; otherwise the loop renders a static
/// 3-second clip of the same frame (still valid, just boring).
const ANIMATED: &[&str] = &["mesh-perspective"];

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=warn")),
        )
        .init();

    if !gstreamer_available() {
        eprintln!(
            "ERROR: gst-launch-1.0 not found on PATH.\n\
             Install with `brew install gstreamer` (macOS) or apt's gstreamer1.0-tools.\n\
             This binary is local-only — CI does NOT run it; the .mp4s are committed."
        );
        std::process::exit(2);
    }

    let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("Renderer::new");

    let out_dir = PathBuf::from("_docs/wisp-book/src/assets/wisp");
    std::fs::create_dir_all(&out_dir).expect("create out dir");

    let stories = all_stories();
    let mut exported = 0;
    for story in &stories {
        if !ANIMATED.contains(&story.id) {
            continue;
        }
        let out_path = out_dir.join(format!("{}.mp4", story.id));
        println!("rendering {} -> {}", story.id, out_path.display());

        // Spawn gst-launch. `rawvideoparse` synthesizes PTS timestamps
        // from the explicit framerate; without it mp4mux fails with
        // "Buffer has no PTS".
        let mut child = Command::new("gst-launch-1.0")
            .args([
                "-q",
                "-e", // send EOS on close so mp4mux finalizes the file
                "fdsrc",
                "fd=0",
                "!",
                "rawvideoparse",
                &format!("width={W}"),
                &format!("height={H}"),
                "format=rgba",
                &format!("framerate={FPS}/1"),
                "!",
                "videoconvert",
                "!",
                "x264enc",
                "tune=zerolatency",
                "speed-preset=fast",
                "bitrate=2000",
                "!",
                "mp4mux",
                "!",
                "filesink",
            ])
            .arg(format!("location={}", out_path.display()))
            .stdin(Stdio::piped())
            .spawn()
            .expect("spawn gst-launch-1.0");

        let mut stdin = child.stdin.take().expect("child stdin");

        let mut stage = Stage::new();
        (story.build)(&app, &mut stage);

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "FPS * DURATION (e.g. 30 * 3.0 = 90) is small and positive"
        )]
        let total_frames = (f32::from(u16::try_from(FPS).unwrap()) * DURATION_SECS) as u32;
        let dt = 1.0_f32 / f32::from(u16::try_from(FPS).unwrap());
        for frame in 0..total_frames {
            // Frame index fits in u16 for any reasonable duration —
            // f32::from(u16) is lossless.
            let t = f32::from(u16::try_from(frame).unwrap_or(0)) * dt;
            story.tick(&mut stage, t);

            let target = RenderTexture::with_format(&app, W, H, format);
            let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);
            let bytes = target.read_pixels(&app);
            stdin
                .write_all(&bytes)
                .expect("write frame to gst-launch stdin");
        }

        drop(stdin); // signal EOS
        let status = child.wait().expect("wait gst-launch");
        if status.success() {
            println!("  ✓ {}", story.id);
            exported += 1;
        } else {
            eprintln!("WARN: gst-launch exited with {status:?} for {}", story.id);
        }
    }

    if exported == 0 {
        eprintln!("no animated stories rendered (allow-list empty?)");
    } else {
        println!("done. exported {exported} animated stories.");
    }
}

fn gstreamer_available() -> bool {
    Command::new("gst-launch-1.0")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
