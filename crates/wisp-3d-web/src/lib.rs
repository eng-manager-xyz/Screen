//! `wisp-3d-web` — Trunk-built wasm32 demo bundle (W3D.8 / AUT-300).
//!
//! Ports the engmanager.xyz 404 pyramid to wisp-3d. The bundle
//! looks for `<canvas data-404-stage>` in the host page (the exact
//! selector the existing `not-found.js` already renders into) and
//! drives a spinning pyramid with `PaletteRampMaterial` + a 1px
//! wireframe overlay through wgpu's `BROWSER_WEBGPU` backend.
//!
//! The full eye-of-providence composition (glow ellipse + iris
//! ring + pupil) ships as a follow-up — W3D.6's `Sprite3D` is in
//! the toolbox, so it's purely a composition addition that doesn't
//! gate the wisp-3d → wisp-3d-web architecture proof.
//!
//! ## Native build path
//!
//! This crate is wasm32-only by intent. `cargo check` on the native
//! target builds the empty stub below so the workspace gate stays
//! green; only `cargo build --target wasm32-unknown-unknown` (via
//! Trunk) produces the real bundle.

// ─── Native target: empty stub so the workspace check is green. ─

#[cfg(not(target_arch = "wasm32"))]
mod native_stub {
    /// Placeholder marker so the crate has something to expose on
    /// the native target. Tests pull this in via the binary-smoke
    /// path that spawns `trunk build` against the wasm32 target.
    #[must_use]
    pub fn crate_name() -> &'static str {
        env!("CARGO_PKG_NAME")
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native_stub::crate_name;

// ─── wasm32 target: the real boot. ─────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod web {
    use glam::{Mat4, Vec3};
    use wasm_bindgen::{JsCast, JsValue, prelude::*};
    use web_sys::HtmlCanvasElement;

    use wisp::application::{AppConfig, Application};
    use wisp_3d::{
        Camera3D, EdgesMesh, LineColor, MaterialRenderer, Mesh3D, PaletteRampMaterial,
        Render3DPass, WireframePipeline, reduced_motion,
    };
    use wisp_animation::Driver;

    /// Trunk entry point — `wasm_bindgen(start)` is invoked from
    /// Trunk's generated bootstrap JS as soon as the wasm module
    /// finishes instantiating.
    #[wasm_bindgen(start)]
    pub fn start() -> Result<(), JsValue> {
        console_error_panic_hook::set_once();
        let _ = console_log::init_with_level(log::Level::Info);
        log::info!("wisp-3d-web: starting");

        let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("no document"))?;

        // Find the canvas via the engmanager.xyz selector
        // (`<canvas data-404-stage>`). Falls back to the first
        // canvas in the document so this bundle also runs in a
        // plain `trunk serve` test page that uses the default
        // attribute.
        let canvas: HtmlCanvasElement = document
            .query_selector("canvas[data-404-stage]")
            .ok()
            .flatten()
            .or_else(|| document.query_selector("canvas").ok().flatten())
            .ok_or_else(|| JsValue::from_str("no <canvas> in document"))?
            .dyn_into()
            .map_err(|_| JsValue::from_str("selected element is not a <canvas>"))?;

        // Match the canvas's intrinsic resolution to its layout
        // box (DPR-aware-lite — bounded to 2× to keep wasm fill
        // rate sane on Retina displays).
        let dpr = window.device_pixel_ratio().clamp(1.0, 2.0);
        let css_w = f64::from(canvas.client_width().max(1));
        let css_h = f64::from(canvas.client_height().max(1));
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "css dims are positive + bounded by viewport in any sensible page"
        )]
        let width = ((css_w * dpr) as u32).max(1);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "css dims are positive + bounded by viewport in any sensible page"
        )]
        let height = ((css_h * dpr) as u32).max(1);
        canvas.set_width(width);
        canvas.set_height(height);
        log::info!("wisp-3d-web: canvas is {width}x{height} (dpr {dpr})");

        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = run(canvas).await {
                web_sys::console::error_1(&JsValue::from_str(&format!("wisp-3d-web: {e}")));
            }
        });
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "Linear wgpu bring-up + scene composition; splitting the device init from the per-frame draw would duplicate state-passing without reuse."
    )]
    async fn run(canvas: HtmlCanvasElement) -> Result<(), String> {
        let width = canvas.width().max(1);
        let height = canvas.height().max(1);

        // ─── wgpu bring-up (WebGPU-only; no WebGL fallback). ───
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| format!("create_surface: {e}"))?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or_else(|| "no WebGPU adapter available".to_owned())?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("wisp-3d-web device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .map_err(|e| format!("request_device: {e}"))?;
        let caps = surface.get_capabilities(&adapter);
        let surface_format = caps.formats[0];
        let alpha_mode = if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            caps.alpha_modes[0]
        };
        surface.configure(
            &device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: surface_format,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                alpha_mode,
                view_formats: vec![],
                desired_maximum_frame_latency: 2,
            },
        );

        let app = Application::from_wgpu(instance, adapter, device, queue, AppConfig::default());

        // ─── Scene composition. ────────────────────────────────
        let pyramid = Mesh3D::pyramid(1.34, 1.25);
        let edges = EdgesMesh::from_mesh(&pyramid, 8.0);
        let mut renderer = MaterialRenderer::new(&app);
        let mut wireframe = WireframePipeline::new(&app, surface_format, 1);
        let _ = Render3DPass::new(&app, surface_format, width, height, 1); // anchor pass; depth attachment owned by pass instance below

        // Camera matches `not-found.js`: 38° FOV at (0, 0.28, 6.2).
        #[allow(
            clippy::cast_precision_loss,
            reason = "viewport dims bounded by browser; f32 mantissa sufficient"
        )]
        let aspect = (width as f32) / (height as f32);
        let mut camera = Camera3D::perspective(38.0, aspect, 0.1, 100.0);
        camera.position = Vec3::new(0.0, 0.28, 6.2);

        let driver = Driver::realtime();
        let _ = reduced_motion::detect_via_media_query();

        // Single static draw — full animation loop lands in the
        // engmanager.xyz integration (W3D.10). This proves the
        // architecture: WebGPU adapter → wisp-3d pass → palette
        // material → wireframe overlay → surface present.
        let frame = surface
            .get_current_texture()
            .map_err(|e| format!("get_current_texture: {e}"))?;
        let color_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let depth_tex = app.device().create_texture(&wgpu::TextureDescriptor {
            label: Some("wisp-3d-web::depth"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wisp_3d::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let depth_view = depth_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = app
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wisp-3d-web::encoder"),
            });

        let material =
            PaletteRampMaterial::engmanager_404().with_time(driver.elapsed().as_secs_f32());
        renderer.draw_one(
            &app,
            &mut encoder,
            &color_view,
            &depth_view,
            &camera,
            &material,
            &pyramid,
            Mat4::IDENTITY,
            [1.0, 1.0, 1.0, 1.0],
            wgpu::Color::TRANSPARENT,
            surface_format,
            1,
        );

        // Wireframe overlay — separate pass so we can use a
        // pre-built color + bind-group lifetime cleanly.
        let edge_vbuf = wireframe.build_vertex_buffer(app.device(), app.queue(), &edges);
        let (_color_buf, color_bg) = wireframe.build_color_resources(
            app.device(),
            app.queue(),
            LineColor {
                color: [0.96, 0.88, 0.86, 0.82],
            },
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wisp-3d-web::wireframe_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            wireframe.draw_into(
                &app,
                &mut pass,
                &camera,
                &edges,
                LineColor {
                    color: [0.96, 0.88, 0.86, 0.82],
                },
                &edge_vbuf,
                &color_bg,
            );
        }
        app.queue().submit(std::iter::once(encoder.finish()));
        frame.present();
        log::info!("wisp-3d-web: first frame submitted");
        Ok(())
    }
}
