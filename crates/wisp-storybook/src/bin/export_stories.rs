//! `cargo run -p wisp-storybook --bin export-stories`
//!
//! Headlessly renders every shipped wisp story to a 256×256 PNG under
//! `_docs/book/src/assets/wisp/<id>.png`. The mdBook site embeds these via
//! standard markdown — without an asset, the chunk's docs page renders empty
//! (that's the convention's gate).
//!
//! Reuses the same code path the integration tests use:
//! `Renderer::render_stage` → `RenderTexture::read_pixels` → `image::save`.

use std::path::Path;

use pollster::block_on;
use wisp::Stage;
use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture};
use wisp_storybook::stories::all_stories;

const SIZE: u32 = 256;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=warn")),
        )
        .init();

    let app = block_on(Application::new(AppConfig::default())).expect("Application::new");
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let renderer = Renderer::new(&app, format).expect("Renderer::new");

    let out_dir = Path::new("_docs/book/src/assets/wisp");
    std::fs::create_dir_all(out_dir).expect("create assets dir");

    let stories = all_stories();
    println!(
        "exporting {} stories → {}",
        stories.len(),
        out_dir.display()
    );

    for story in stories {
        let mut stage = Stage::new();
        (story.build)(&app, &mut stage);
        story.tick(&mut stage, 0.0);

        let target = RenderTexture::with_format(&app, SIZE, SIZE, format);
        let _ = renderer.render_stage(&app, target.view(), Color::BLACK, &stage);

        let bytes = target.read_pixels(&app);
        let img = image::RgbaImage::from_raw(SIZE, SIZE, bytes).expect("dims");
        let path = out_dir.join(format!("{}.png", story.id));
        img.save(&path).expect("save png");
        println!("  ✓ {}", story.id);
    }

    println!("done.");
}
