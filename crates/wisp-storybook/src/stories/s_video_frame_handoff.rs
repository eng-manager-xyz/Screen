//! Story: synthetic `decode::VideoFrame` → wisp `VideoTexture` → Sprite
//! (M-MEDIA.12 / AUT-108).
//!
//! Proves the second seam — the video-side counterpart to the audio
//! `WaveformBarRect` handoff. The frame is constructed by hand
//! (decode crate, BGRA bytes) and uploaded to wisp's pre-existing
//! `VideoTexture`. wisp doesn't know where the bytes came from.

use glam::Vec2;
use media::VideoFrame;
use wisp::application::Application;
use wisp::texture::video_texture::VideoTexture;
use wisp::{Sprite, Stage};

use crate::story::Story;

pub fn story() -> Story {
    Story {
        id: "video-frame-handoff",
        category: "Media",
        title: "VideoFrame → VideoTexture",
        milestone: "M-MEDIA.12",
        writeup: include_str!("writeups/video_frame_handoff.md"),
        build,
        tick: None,
    }
}

fn build(app: &Application, stage: &mut Stage) {
    // 128×72 synthetic BGRA frame — a smooth diagonal gradient with
    // a few colored stripes. Deterministic.
    let frame = synthetic_frame(128, 72);

    let video_tex = VideoTexture::new(app, frame.width, frame.height);
    video_tex.upload_bgra(app, &frame.bgra);

    let mut sprite =
        Sprite::from_texture(video_tex.texture().clone()).with_anchor(Vec2::splat(0.5));
    // Fill ~75% of the NDC viewport, preserving the 128:72 aspect.
    sprite.container.transform.scale = Vec2::new(1.5, 1.5 * 72.0 / 128.0);
    let _ = stage.add_child(stage.root(), sprite);
}

fn synthetic_frame(width: u32, height: u32) -> VideoFrame {
    let pixels_w = width as usize;
    let pixels_h = height as usize;
    let mut bgra = vec![0u8; pixels_w * pixels_h * 4];
    for row in 0..pixels_h {
        for col in 0..pixels_w {
            let idx = (row * pixels_w + col) * 4;
            // Diagonal gradient — interesting per-pixel content.
            // Each numerator stays ≤ 255 by construction.
            let red = u8::try_from((col * 255) / pixels_w).unwrap_or(255);
            let green = u8::try_from((row * 255) / pixels_h).unwrap_or(255);
            let blue = u8::try_from(((col + row) * 255) / (pixels_w + pixels_h)).unwrap_or(255);
            // Horizontal stripes every 12 rows give the snapshot fingerprint
            // enough cross-quadrant variance to detect orientation regressions.
            let stripe = if row % 12 < 3 { 60 } else { 0 };
            bgra[idx] = blue.saturating_add(stripe); // B
            bgra[idx + 1] = green; // G
            bgra[idx + 2] = red.saturating_add(stripe); // R
            bgra[idx + 3] = 255; // A
        }
    }
    VideoFrame {
        width,
        height,
        bgra,
        pts_seconds: 0.0,
        frame_index: 0,
    }
}
