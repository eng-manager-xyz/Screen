//! Headless-browser test for the `BROWSER_WEBGPU` **surface
//! presentation** path used by `wisp-chart-web`.
//!
//! Local-only; **not wired into `just gate`**. The native readback
//! at `tests/render_gantt.rs` already covers the render-logic
//! layer. This test exists for the bug class the chunk-3 demo
//! hit: `cargo check` was green, the wasm binary loaded, no JS
//! errors were thrown, yet the canvas stayed grey because the
//! surface configuration silently failed to commit pixels.
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
//! 4. After `render_gantt` + submit, the canvas-backed surface
//!    texture's centre pixel for `sample_gantt()`'s `bar[0]`
//!    reads as Matt's Wong-navy `#0072b2` (format-aware Bgra ↔
//!    Rgba byte swap). That's the same assertion the native
//!    readback test makes, against the real `BROWSER_WEBGPU` path.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "viewport dims and pixel coords are bounded by W=256, H=256 — \
              well below the f32 precision boundary, and `round()` plus the \
              non-negative pixel-rect outputs make sign loss / truncation safe."
)]

use glam::Vec2;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::{wasm_bindgen_test, wasm_bindgen_test_configure};
use web_sys::HtmlCanvasElement;
use wisp::application::{AppConfig, Application};
use wisp_chart::Theme;
use wisp_chart::gantt::layout::bar_pixel_rect;
use wisp_chart_web::{render_gantt, sample_gantt};

wasm_bindgen_test_configure!(run_in_browser);

/// 256-px wide so the texture→buffer copy's row stride
/// (W × 4 bytes/px = 1024) is naturally 256-aligned.
const W: u32 = 256;
const H: u32 = 256;

#[wasm_bindgen_test]
#[allow(
    clippy::too_many_lines,
    reason = "single-test linear sequence — wgpu bring-up + render + readback + assertion"
)]
async fn surface_renders_gantt_bar_in_browser() {
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

    // 4. Render the Gantt via the same code path the demo uses.
    let frame = surface.get_current_texture().expect("get_current_texture");
    let view = frame
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut app = Application::from_wgpu(
        instance.clone(),
        adapter.clone(),
        device.clone(),
        queue.clone(),
        AppConfig {
            width: W,
            height: H,
            ..Default::default()
        },
    );
    let gantt = sample_gantt();
    let theme = Theme::light();
    let viewport = Vec2::new(W as f32, H as f32);
    render_gantt(&mut app, &view, surface_format, viewport, &gantt, &theme).expect("render_gantt");

    // Compute the centre of bar[0] in pixel space — same math
    // the native test uses.
    let bar0 = &gantt.bars[0];
    let rect = bar_pixel_rect(bar0, &gantt, &theme, viewport.x).expect("bar0 resolves");
    let cx = (rect.x + rect.w * 0.5).round() as u32;
    let cy = (rect.y + rect.h * 0.5).round() as u32;

    // 5. Copy the surface texture into a CPU-mappable buffer for
    //    pixel readback.
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
    frame.present();

    let (tx, rx) = futures_channel::oneshot::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
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
    let offset = ((cy * bytes_per_row) + cx * 4) as usize;
    let centre = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];

    // Format-aware expected pixel. Matt's `#0072b2`:
    // - Rgba8Unorm bytes: [0x00, 0x72, 0xb2, 0xff]
    // - Bgra8Unorm bytes: [0xb2, 0x72, 0x00, 0xff]
    let expected = match surface_format {
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            [0xb2, 0x72, 0x00, 0xff]
        }
        _ => [0x00, 0x72, 0xb2, 0xff],
    };

    // ±2 tolerance absorbs SDF anti-alias noise; the rounded
    // rect's centre should still be well inside the solid
    // region.
    for (i, (a, e)) in centre.iter().zip(expected.iter()).enumerate() {
        let diff = a.abs_diff(*e);
        assert!(
            diff <= 2,
            "channel {i} got {a}, expected {e} ± 2 (full pixel got {centre:?}, expected {expected:?}); \
             surface_format={surface_format:?}, alpha_mode={alpha_mode:?}, bar0 centre @ ({cx}, {cy})"
        );
    }
}
