//! Print wgpu adapter info. M0.4 verification example.
//!
//! Run with: `cargo run -p wisp --example adapter_info`

fn main() -> wisp::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("wisp=debug,info")),
        )
        .init();

    let app = pollster::block_on(wisp::application::Application::new(
        wisp::application::AppConfig::default(),
    ))?;

    let info = app.adapter_info();
    println!("Adapter:      {}", info.name);
    println!("Backend:      {:?}", info.backend);
    println!("Device type:  {:?}", info.device_type);
    println!("Driver:       {} {}", info.driver, info.driver_info);
    println!("Stage size:   {}x{}", app.width(), app.height());

    Ok(())
}
