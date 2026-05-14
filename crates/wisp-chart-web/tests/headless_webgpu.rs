//! Headless-browser test for the `BROWSER_WEBGPU` **surface
//! presentation** path used by `wisp-chart-web`.
//!
//! Local-only; **not wired into `just gate`**. The native readback
//! at `tests/clear_pass.rs` already covers the render-logic layer.
//! This test exists for the bug class the chunk-3 demo hit:
//! `cargo check` was green, the wasm binary loaded, no JS errors
//! were thrown, yet the canvas stayed grey because the surface
//! configuration silently failed to commit pixels. The distinctive
//! demo purple (`[153, 51, 204, 255]`) is what makes the
//! assertion unambiguous — no default canvas backdrop or
//! mishandled-alpha path lands on those exact bytes by accident.
//!
//! # How to run locally
//!
//! ```bash
//! brew install chromedriver  # one-time
//! # Chrome 113+ (124+ recommended for headless WebGPU on macOS).
//! WASM_BINDGEN_TEST_TIMEOUT=60 \
//!   cargo test --target wasm32-unknown-unknown -p wisp-chart-web
//! ```
//!
//! `cargo test` against the host target naturally skips this file
//! (the whole module is gated on `cfg(target_arch = "wasm32")`),
//! which is why `just gate` doesn't pull `chromedriver` in.
//!
//! # What this asserts
//!
//! 1. The page sees a usable `navigator.gpu` adapter.
//! 2. `Surface::create_surface(SurfaceTarget::Canvas)` returns Ok
//!    against a freshly-created `<canvas>` element.
//! 3. `Surface::configure` with `RENDER_ATTACHMENT | COPY_SRC`
//!    succeeds — Chrome allows `COPY_SRC` on canvas surfaces.
//! 4. After `clear_with_color(_, _, _, DEMO_CLEAR_COLOR)` + submit,
//!    the canvas-backed surface texture's centre pixel matches the
//!    demo purple — `[153, 51, 204, 255]` on `Rgba8Unorm`,
//!    `[204, 51, 153, 255]` on `Bgra8Unorm` (R↔B swap).

#![cfg(target_arch = "wasm32")]

use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlCanvasElement;
use wisp_chart_web::{DEMO_CLEAR_COLOR, DEMO_CLEAR_RGBA8, clear_with_color};

wasm_bindgen_test_configure!(run_in_browser);

/// Width/height chosen so the row stride is naturally
/// 256-aligned: 64 px × 4 bytes/px = 256 bytes, no padding needed
/// for the texture→buffer copy.
const W: u32 = 64;
const H: u32 = 64;

#[wasm_bindgen_test]
#[allow(
    clippy::too_many_lines,
    reason = "single-test linear sequence — wgpu bring-up + readback + assertion"
)]
async fn surface_clear_paints_demo_purple_in_browser() {
    console_error_panic_hook::set_once();

    // 1. Create a fresh <canvas> for the test. wasm-bindgen-test
    //    starts from a minimal document; the demo's index.html
    //    canvas isn't here.
    let window = web_sys::window().expect("no window");
    let document = window.document().expect("no document");
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .expect("create_element canvas")
        .dyn_into()
        .expect("dyn_into HtmlCanvasElement");
    canvas.set_width(W);
    canvas.set_height(H);
    document
        .body()
        .expect("no body")
        .append_child(&canvas)
        .expect("append canvas");

    // 2. Same wgpu instance configuration as the production demo
    //    — `Backends::BROWSER_WEBGPU` only.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
        .expect("create_surface");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: Some(&surface),
        })
        .await
        .expect("request_adapter (is WebGPU enabled in this headless Chrome?)");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("headless test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::downlevel_defaults(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        )
        .await
        .expect("request_device");

    // 3. Configure with the same alpha-mode picker as the demo
    //    plus `COPY_SRC` so we can read the canvas texture back
    //    for the assertion.
    let caps = surface.get_capabilities(&adapter);
    let surface_format = caps.formats[0];
    let alpha_mode = if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        caps.alpha_modes[0]
    };

    device.push_error_scope(wgpu::ErrorFilter::Validation);
    surface.configure(
        &device,
        &wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            format: surface_format,
            width: W,
            height: H,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        },
    );

    // 4. Render to the surface texture + copy it into a
    //    CPU-mappable buffer.
    let frame = surface.get_current_texture().expect("get_current_texture");
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
    clear_with_color(&device, &queue, &view, DEMO_CLEAR_COLOR);

    // Read-back buffer. Row stride must be a multiple of 256;
    // W=64 × 4 bytes/px = 256 exactly so no padding is needed.
    let bytes_per_row = W * 4;
    assert_eq!(bytes_per_row % 256, 0, "row stride must be 256-aligned");
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size: u64::from(bytes_per_row * H),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &frame.texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(H),
            },
        },
        wgpu::Extent3d {
            width: W,
            height: H,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    // Present is cosmetic for the test but mirrors the demo's
    // full path so any present-time validation error surfaces in
    // the error scope below.
    frame.present();

    // 5. Map the buffer (async on browser) + assert the demo
    //    purple.
    let (tx, rx) = futures_channel::oneshot::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    // On wasm `device.poll` is a no-op; the JS event loop drives
    // the map callback. Awaiting the channel yields back so the
    // browser can dispatch it.
    let _ = device.poll(wgpu::Maintain::Poll);
    rx.await
        .expect("oneshot dropped")
        .expect("map_async failed");

    let validation = device.pop_error_scope().await;
    assert!(
        validation.is_none(),
        "wgpu validation error on BROWSER_WEBGPU surface path: {validation:?}"
    );

    let data = buffer.slice(..).get_mapped_range();
    let pixel_centre_offset = ((H / 2) * bytes_per_row + (W / 2) * 4) as usize;
    let centre = [
        data[pixel_centre_offset],
        data[pixel_centre_offset + 1],
        data[pixel_centre_offset + 2],
        data[pixel_centre_offset + 3],
    ];

    // Format-aware expectation. Chrome's preferred canvas formats
    // are `Bgra8Unorm` (Apple Silicon) and `Rgba8Unorm` (others) —
    // we accept both. The R↔B swap is the *only* difference
    // between the two; the demo purple was chosen with G ≠ R, B
    // so the swap is observable.
    let expected = match surface_format {
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => [
            DEMO_CLEAR_RGBA8[2],
            DEMO_CLEAR_RGBA8[1],
            DEMO_CLEAR_RGBA8[0],
            DEMO_CLEAR_RGBA8[3],
        ],
        _ => DEMO_CLEAR_RGBA8,
    };

    assert_eq!(
        centre, expected,
        "BROWSER_WEBGPU surface centre pixel expected demo purple {expected:?}, got {centre:?}; \
         surface_format={surface_format:?}, alpha_mode={alpha_mode:?}"
    );
}
