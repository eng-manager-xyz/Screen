//! `cargo run -p wisp-storybook --bin wisp-export-text-screenshots`
//!
//! Headlessly renders a single hero image demonstrating `FlexibleText`
//! with custom OFL-licensed fonts (Inter Regular + Inter Bold +
//! `JetBrains` Mono Regular). Output:
//!
//! ```text
//! _docs/wisp-book/src/assets/wisp/text-custom-fonts.png
//! ```
//!
//! Sibling to `wisp-export-stories`. We don't piggyback on the Story
//! abstraction because `FlexibleTextRenderer` writes directly to a
//! `TextureView`, not through `Stage` / `Renderer::render_stage`.
//!
//! Fonts live under `crates/wisp-storybook/assets/fonts/`. The exporter
//! is run from the workspace root by `just snapshots-wisp`, so the
//! relative paths are anchored there.

use std::path::PathBuf;

use glam::Vec2;
use pollster::block_on;
use wisp::Stage;
use wisp::application::{AppConfig, Application};
use wisp::color::Color;
use wisp::render::Renderer;
use wisp::text::{FlexibleTextEngine, FlexibleTextRenderer, WispText, WispTextStyle};
use wisp::texture::render_texture::RenderTexture;
use wisp::{WispFontWeight, WispTextAlign};

const WIDTH: u32 = 1024;
const HEIGHT: u32 = 512;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=warn")),
        )
        .init();

    let fonts_dir = PathBuf::from("crates/wisp-storybook/assets/fonts");
    let font_paths = [
        fonts_dir.join("Inter-Regular.ttf"),
        fonts_dir.join("Inter-Bold.ttf"),
        fonts_dir.join("JetBrainsMono-Regular.ttf"),
    ];

    let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
    let format = wgpu::TextureFormat::Rgba8Unorm;

    let engine =
        FlexibleTextEngine::from_font_paths(&font_paths).expect("load Inter + JetBrains Mono");

    let mut text_renderer = FlexibleTextRenderer::new(&app, format, engine.font_system_handle());
    text_renderer.set_resolution(WIDTH, HEIGHT);

    // Dark editor-aesthetic background: render an empty stage with a
    // clear color, then text composes on top with clear = false.
    let bg = Color::rgba(0.08, 0.09, 0.12, 1.0);
    let renderer = Renderer::new(&app, format).expect("Renderer::new");
    let stage = Stage::new();
    let target = RenderTexture::with_format(&app, WIDTH, HEIGHT, format);
    let _ = renderer.render_stage(&app, target.view(), bg, &stage);

    let inter_regular = WispText::new("Inter — the body face")
        .with_style(
            WispTextStyle::default()
                .with_size(0.10)
                .with_color(Color::WHITE),
        )
        .with_font_family("Inter");

    let inter_bold = WispText::new("Inter Bold — emphasis & headings")
        .with_style(
            WispTextStyle::default()
                .with_size(0.10)
                .with_weight(WispFontWeight::Bold)
                .with_color(Color::rgba(1.0, 0.85, 0.35, 1.0)),
        )
        .with_font_family("Inter");

    let jbmono = WispText::new("JetBrains Mono — fn render(text: &str)")
        .with_style(
            WispTextStyle::default()
                .with_size(0.08)
                .with_color(Color::rgba(0.55, 0.85, 1.0, 1.0))
                .with_align(WispTextAlign::Left),
        )
        .with_font_family("JetBrains Mono");

    let layout_regular = engine.layout_concrete(&inter_regular);
    let layout_bold = engine.layout_concrete(&inter_bold);
    let layout_mono = engine.layout_concrete(&jbmono);

    text_renderer.draw(
        target.view(),
        &[
            (&layout_regular, Vec2::new(-0.92, 0.65), Color::WHITE),
            (
                &layout_bold,
                Vec2::new(-0.92, 0.15),
                Color::rgba(1.0, 0.85, 0.35, 1.0),
            ),
            (
                &layout_mono,
                Vec2::new(-0.92, -0.35),
                Color::rgba(0.55, 0.85, 1.0, 1.0),
            ),
        ],
        /* clear = */ false,
    );

    let bytes = target.read_pixels(&app);
    let img = image::RgbaImage::from_raw(WIDTH, HEIGHT, bytes).expect("dims");

    let out_dir = PathBuf::from("_docs/wisp-book/src/assets/wisp");
    std::fs::create_dir_all(&out_dir).expect("create out dir");
    let out_path = out_dir.join("text-custom-fonts.png");
    img.save(&out_path).expect("save png");

    println!("wrote {}", out_path.display());
}
