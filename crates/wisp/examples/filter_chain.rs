//! Three-filter chain demo — the M0.20 proof point that wisp's
//! `Filter` trait composes cleanly when multiple post-processing
//! passes need to stack.
//!
//! Pipeline (input → output):
//!
//! ```text
//! Stage  ─Renderer::render_stage─►  RT_base
//!     │
//!     ├─BlurFilter(radius=lerp(0.5..6))──►  RT_a
//!     │
//!     ├─DropShadowFilter(blur=lerp(0..14), offset=(8,8))──►  RT_b
//!     │
//!     └─MotionBlurFilter(velocity=lerp((0,0)..(60,0)))──►   RT_final
//! ```
//!
//! Animates the parameters over **60 frames**, dumps each composite
//! to `target/filter_chain/frame_NN.png`. Frame #30 (peak parameters)
//! also lands at `_docs/book/src/assets/wisp/example-filter-chain.png`
//! so the mdBook chapter can embed a representative composite without
//! committing 60 large PNGs.

use std::path::Path;

use glam::Vec2;
use pollster::block_on;
use wisp::application::{AppConfig, Application};
use wisp::math::Rect;
use wisp::render::Renderer;
use wisp::{
    BlurFilter, Color, DropShadowFilter, Fill, Graphics, MotionBlurFilter, RenderTexture, Sprite,
    Stage, Stroke, Texture,
};

const W: u32 = 800;
const H: u32 = 450;
const FRAMES: u32 = 60;
const HIGHLIGHT_FRAME: u32 = 30;

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=info")),
        )
        .init();

    let app = block_on(Application::new(AppConfig {
        width: W,
        height: H,
        ..Default::default()
    }))?;

    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let renderer = Renderer::new(&app, format).expect("renderer");

    // Reusable render-texture pool — one per chain stage. wgpu textures
    // are Arc-backed, so cloning per frame is cheap; allocating fresh
    // every frame would still work but burns time on `create_texture`.
    let rt_base = RenderTexture::with_format(&app, W, H, format);
    let rt_a = RenderTexture::with_format(&app, W, H, format);
    let rt_b = RenderTexture::with_format(&app, W, H, format);
    let rt_final = RenderTexture::with_format(&app, W, H, format);

    // Reusable sprite texture — a small accent square that picks up the
    // filter chain visibly.
    let mut sprite_bytes = Vec::with_capacity(64 * 64 * 4);
    for y in 0u8..64 {
        for x in 0u8..64 {
            // A radial-ish accent so the blur has something with a sharp
            // edge to fall off from. `f32::from(u8)` is lossless; the
            // 32.0 offset places the center at (32, 32) for the 64×64
            // sprite.
            let dx = f32::from(x) - 32.0;
            let dy = f32::from(y) - 32.0;
            let d = (dx * dx + dy * dy).sqrt();
            let alpha = (1.0 - (d / 32.0)).clamp(0.0, 1.0);
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "alpha clamped [0, 1], multiplied by 255 fits in u8"
            )]
            let a = (alpha * 255.0) as u8;
            sprite_bytes.extend_from_slice(&[255, 200, 100, a]);
        }
    }
    let sprite_tex = Texture::from_rgba(&app, 64, 64, &sprite_bytes);

    let out_dir = Path::new("target/filter_chain");
    std::fs::create_dir_all(out_dir).ok();
    let highlight_dir = Path::new("_docs/book/src/assets/wisp");
    std::fs::create_dir_all(highlight_dir).ok();

    for frame in 0..FRAMES {
        #[allow(clippy::cast_precision_loss, reason = "frame index well below 2^23")]
        let t = frame as f32 / FRAMES as f32;

        // Each filter ramps its strength over 60 frames so the chapter's
        // representative frame (#30) shows all three at meaningful magnitude.
        let blur = BlurFilter::new(0.5 + t * 5.5); // 0.5 .. 6.0
        let drop_shadow = DropShadowFilter {
            offset: Vec2::new(8.0, 8.0),
            blur: t * 14.0, // 0 .. 14
            color: Color::rgba(0.0, 0.0, 0.0, 0.6),
        };
        let motion = MotionBlurFilter {
            velocity: Vec2::new(t * 60.0, 0.0), // px/frame, scaled by Filter
            peak_velocity_pps: 60.0,
            max_kernel_px: 18.0,
        };

        // Build the base scene fresh per frame (cheap; avoids any
        // filter-induced state leaking back into `Stage`).
        let mut stage = Stage::new();
        let root = stage.root();

        // Background.
        let mut bg = Graphics::new();
        bg.fill(Fill::LinearGradient {
            start: Vec2::new(0.0, 1.0),
            end: Vec2::new(0.0, -1.0),
            color_a: Color::rgba_u8(40, 50, 70, 255),
            color_b: Color::rgba_u8(20, 25, 35, 255),
        });
        bg.draw_rect(Rect::new(-1.0, -1.0, 2.0, 2.0));
        let _ = stage.add_child(root, bg);

        // A rounded rectangle that the blur+drop-shadow play against.
        let mut card = Graphics::new();
        card.fill(Fill::Solid(Color::rgba_u8(180, 200, 230, 255)));
        card.stroke(Some(Stroke::new(0.012, Color::rgba_u8(60, 80, 110, 255))));
        card.draw_rounded_rect(Rect::new(-0.5, -0.3, 1.0, 0.6), 0.08);
        let _ = stage.add_child(root, card);

        // The accent sprite — drifts horizontally so motion-blur has
        // a direction-of-motion to lean on.
        let drift = (t * std::f32::consts::TAU).sin() * 0.3;
        let mut accent = Sprite::from_texture(sprite_tex.clone()).with_anchor(Vec2::splat(0.5));
        accent.container.transform.position = Vec2::new(drift, 0.0);
        accent.container.transform.scale = Vec2::splat(0.25);
        let _ = stage.add_child(root, accent);

        // 1. Render base scene to rt_base.
        let _ = renderer.render_stage(&app, rt_base.view(), Color::BLACK, &stage);

        // 2. Filter chain: blur → drop-shadow → motion-blur.
        renderer.apply_filter(&app, &blur, &rt_base, &rt_a);
        renderer.apply_filter(&app, &drop_shadow, &rt_a, &rt_b);
        renderer.apply_filter(&app, &motion, &rt_b, &rt_final);

        // 3. Read pixels and save.
        let bytes = rt_final.read_pixels(&app);
        let buffer = image::RgbaImage::from_raw(W, H, bytes).expect("buffer dims match");
        let path = out_dir.join(format!("frame_{frame:02}.png"));
        buffer.save(&path).expect("save frame png");

        if frame == HIGHLIGHT_FRAME {
            let highlight = highlight_dir.join("example-filter-chain.png");
            buffer.save(&highlight).expect("save highlight png");
            println!("  ✓ highlight frame: {}", highlight.display());
        }
    }

    println!(
        "rendered {FRAMES} composites at {W}×{H} through blur→drop-shadow→motion; \
         dir: {}",
        out_dir.display()
    );

    Ok(())
}
