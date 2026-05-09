//! `wisp-storybook` — interactive feature gallery.
//!
//! Run with: `cargo run -p wisp-storybook` (or `just storybook`).

mod app;
mod stories;
mod story;

fn main() -> Result<(), eframe::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("wisp_storybook=info,wisp=info")
            }),
        )
        .init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("wisp · storybook"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "wisp-storybook",
        native_options,
        Box::new(|cc| Ok(Box::new(app::StorybookApp::new(cc)))),
    )
}
