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
    use std::cell::RefCell;
    use std::rc::Rc;

    use glam::{Mat4, Vec2, Vec3};
    use wasm_bindgen::{JsCast, JsValue, prelude::*};
    use web_sys::HtmlCanvasElement;

    use wisp::application::{AppConfig, Application};
    use wisp_3d::{
        Camera3D as Wisp3dCamera, EdgesMesh, LineColor, MaterialRenderer, Mesh3D,
        PaletteRampMaterial, Render3DPass, WireframePipeline, reduced_motion,
    };
    use wisp_animation::Driver;
    use wisp_interaction::{Camera3D, OrbitController};

    /// Adapter newtype: wisp-interaction's `Camera3D` trait wrapping
    /// `wisp_3d::Camera3D`. Lives here (not in `wisp-3d` itself) so
    /// neither crate gains an interaction dep — the host where both
    /// meet is the natural place for the glue.
    struct OrbitCam<'a> {
        inner: &'a mut Wisp3dCamera,
    }

    impl Camera3D for OrbitCam<'_> {
        fn position(&self) -> Vec3 {
            self.inner.position
        }
        fn target(&self) -> Vec3 {
            self.inner.target
        }
        fn up(&self) -> Vec3 {
            self.inner.up
        }
        fn set_position(&mut self, p: Vec3) {
            self.inner.position = p;
        }
        fn set_target(&mut self, t: Vec3) {
            self.inner.target = t;
        }
        fn fov_y(&self) -> f32 {
            self.inner.fov_y
        }
    }

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
        let renderer = MaterialRenderer::new(&app);
        let wireframe = WireframePipeline::new(&app, surface_format, 1);
        let _ = Render3DPass::new(&app, surface_format, width, height, 1); // anchor pass; depth attachment owned by pass instance below

        // Camera matches `not-found.js`: 38° FOV at (0, 0.28, 6.2).
        #[allow(
            clippy::cast_precision_loss,
            reason = "viewport dims bounded by browser; f32 mantissa sufficient"
        )]
        let aspect = (width as f32) / (height as f32);
        let mut camera = Wisp3dCamera::perspective(38.0, aspect, 0.1, 100.0);
        camera.position = Vec3::new(0.0, 0.28, 6.2);

        let driver = Driver::realtime();
        let _ = reduced_motion::detect_via_media_query();

        // ─── Long-lived render state (WI.11: now lives across
        // frames so the rAF loop can re-render after pointer input).

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
        let edge_vbuf = wireframe.build_vertex_buffer(app.device(), app.queue(), &edges);
        let (_color_buf, color_bg) = wireframe.build_color_resources(
            app.device(),
            app.queue(),
            LineColor {
                color: [0.96, 0.88, 0.86, 0.82],
            },
        );

        // ─── OrbitController (WI.11): auto-rotates while idle, drags
        // and wheel-zooms when the user interacts with the canvas.

        let mut orbit = OrbitController::new();
        orbit.enable_damping = true;
        orbit.auto_rotate = true;
        orbit.auto_rotate_speed = 0.4; // slow ambient spin
        orbit.min_distance = 3.5;
        orbit.max_distance = 12.0;
        orbit.min_polar_angle = 0.15;
        orbit.max_polar_angle = std::f32::consts::PI - 0.15;

        // Capture the long-lived state in `Rc<RefCell<...>>` so the
        // pointer-event closures and the rAF tick can all mutate it
        // through interior mutability.
        let state = Rc::new(RefCell::new(RenderState {
            surface,
            surface_format,
            app,
            renderer,
            wireframe,
            edge_vbuf,
            color_bg,
            pyramid,
            edges,
            camera,
            depth_view,
            driver,
            orbit,
            #[allow(
                clippy::cast_precision_loss,
                reason = "viewport dims bounded by browser; f32 fits"
            )]
            viewport_size: Vec2::new(width as f32, height as f32),
            last_pointer: None,
        }));

        // ─── Wire DOM pointer + wheel listeners. Each handler
        // mutates the controller via `state` then drops the borrow.

        let canvas_target_el: web_sys::EventTarget = canvas_target();
        attach_pointer_listeners(&canvas_target_el, &state)?;

        // ─── rAF loop. ─────────────────────────────────────────
        let raf_state = state.clone();
        let f = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
        let g = f.clone();
        let mut last_t: Option<f64> = None;
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move |t: f64| {
            let dt = match last_t {
                Some(prev) => {
                    #[allow(
                        clippy::cast_possible_truncation,
                        reason = "ms→sec dt fits in f32 for any plausible frame"
                    )]
                    let v = ((t - prev) / 1000.0) as f32;
                    v
                }
                None => 0.0,
            };
            last_t = Some(t);
            if let Err(e) = render_one_frame(&raf_state, dt) {
                web_sys::console::error_1(&JsValue::from_str(&format!("wisp-3d-web rAF: {e}")));
            }
            // Schedule the next frame.
            let window = web_sys::window().unwrap();
            let _ = window.request_animation_frame(
                f.borrow()
                    .as_ref()
                    .expect("rAF closure")
                    .as_ref()
                    .unchecked_ref(),
            );
        }) as Box<dyn FnMut(f64)>));
        let window = web_sys::window().ok_or_else(|| "no window".to_owned())?;
        window
            .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .map_err(|e| format!("request_animation_frame: {e:?}"))?;

        log::info!("wisp-3d-web: rAF loop started with OrbitController");
        Ok(())
    }

    struct RenderState {
        surface: wgpu::Surface<'static>,
        surface_format: wgpu::TextureFormat,
        app: Application,
        renderer: MaterialRenderer,
        wireframe: WireframePipeline,
        edge_vbuf: wgpu::Buffer,
        color_bg: wgpu::BindGroup,
        pyramid: Mesh3D,
        edges: EdgesMesh,
        camera: Wisp3dCamera,
        depth_view: wgpu::TextureView,
        driver: Driver,
        orbit: OrbitController,
        viewport_size: Vec2,
        last_pointer: Option<Vec2>,
    }

    /// Recover the canvas from the document so pointer listeners
    /// know what to bind to. Falls back to the same selector chain
    /// used in `start()`.
    fn canvas_target() -> web_sys::EventTarget {
        let document = web_sys::window()
            .and_then(|w| w.document())
            .expect("document");
        let canvas: HtmlCanvasElement = document
            .query_selector("canvas[data-404-stage]")
            .ok()
            .flatten()
            .or_else(|| document.query_selector("canvas").ok().flatten())
            .expect("canvas")
            .dyn_into()
            .expect("canvas");
        canvas.into()
    }

    /// Cast helper: i32 → f32, hiding the clippy `cast_precision_loss`
    /// lint behind a documented reason. `client_x`/`y` always fit.
    #[allow(
        clippy::cast_precision_loss,
        reason = "browser client_x/y values are within f32 mantissa for any plausible viewport"
    )]
    fn i32_to_f32(v: i32) -> f32 {
        v as f32
    }

    fn attach_pointer_listeners(
        target: &web_sys::EventTarget,
        state: &Rc<RefCell<RenderState>>,
    ) -> Result<(), String> {
        // pointerdown — start a rotate drag on left, pan on middle,
        // dolly on right. PointerEvent.button matches the W3C indices
        // 0/1/2.
        {
            let s = state.clone();
            let cb = Closure::wrap(Box::new(move |e: web_sys::PointerEvent| {
                let pos = Vec2::new(i32_to_f32(e.client_x()), i32_to_f32(e.client_y()));
                let mut state = s.borrow_mut();
                state.last_pointer = Some(pos);
                match e.button() {
                    0 => state.orbit.pointer_down_rotate(pos),
                    1 => state.orbit.pointer_down_pan(pos),
                    2 => state.orbit.pointer_down_dolly(pos),
                    _ => {}
                }
                e.prevent_default();
            }) as Box<dyn FnMut(web_sys::PointerEvent)>);
            target
                .add_event_listener_with_callback("pointerdown", cb.as_ref().unchecked_ref())
                .map_err(|e| format!("addEventListener pointerdown: {e:?}"))?;
            cb.forget();
        }

        // pointermove — drag the controller.
        {
            let s = state.clone();
            let cb = Closure::wrap(Box::new(move |e: web_sys::PointerEvent| {
                let mut state = s.borrow_mut();
                let pos = Vec2::new(i32_to_f32(e.client_x()), i32_to_f32(e.client_y()));
                state.last_pointer = Some(pos);
                let viewport = state.viewport_size;
                let distance = (state.camera.position - state.camera.target).length();
                let fov = state.camera.fov_y;
                // Compute right + up in world space from view matrix.
                let view = state.camera.view_matrix();
                let right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
                let up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);
                state
                    .orbit
                    .pointer_drag(pos, viewport, distance, right, up, fov);
            }) as Box<dyn FnMut(web_sys::PointerEvent)>);
            target
                .add_event_listener_with_callback("pointermove", cb.as_ref().unchecked_ref())
                .map_err(|e| format!("addEventListener pointermove: {e:?}"))?;
            cb.forget();
        }

        // pointerup — release.
        {
            let s = state.clone();
            let cb = Closure::wrap(Box::new(move |_e: web_sys::PointerEvent| {
                s.borrow_mut().orbit.pointer_up();
            }) as Box<dyn FnMut(web_sys::PointerEvent)>);
            target
                .add_event_listener_with_callback("pointerup", cb.as_ref().unchecked_ref())
                .map_err(|e| format!("addEventListener pointerup: {e:?}"))?;
            cb.forget();
        }

        // wheel — zoom.
        {
            let s = state.clone();
            let cb = Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
                let mut state = s.borrow_mut();
                #[allow(
                    clippy::cast_possible_truncation,
                    reason = "wheel delta fits in f32 with plenty of headroom"
                )]
                let dy = e.delta_y() as f32;
                state.orbit.wheel(dy);
                e.prevent_default();
            }) as Box<dyn FnMut(web_sys::WheelEvent)>);
            target
                .add_event_listener_with_callback("wheel", cb.as_ref().unchecked_ref())
                .map_err(|e| format!("addEventListener wheel: {e:?}"))?;
            cb.forget();
        }

        Ok(())
    }

    fn render_one_frame(state: &Rc<RefCell<RenderState>>, dt: f32) -> Result<(), String> {
        let mut state = state.borrow_mut();

        // Apply orbit deltas to the camera. `update` returns true iff
        // anything moved — we render anyway because auto_rotate fires
        // most frames.
        {
            let RenderState { orbit, camera, .. } = &mut *state;
            let mut cam = OrbitCam { inner: camera };
            let _ = orbit.update(&mut cam, dt);
        }

        let frame = state
            .surface
            .get_current_texture()
            .map_err(|e| format!("get_current_texture: {e}"))?;
        let color_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            state
                .app
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("wisp-3d-web::encoder"),
                });

        let material =
            PaletteRampMaterial::engmanager_404().with_time(state.driver.elapsed().as_secs_f32());
        let surface_format = state.surface_format;
        // Pull immutable references for the draw call (release the
        // mutable borrows of `app` + `camera` + `renderer` are
        // already in scope as &mut state fields).
        {
            let RenderState {
                app,
                renderer,
                pyramid,
                camera,
                depth_view,
                ..
            } = &mut *state;
            renderer.draw_one(
                app,
                &mut encoder,
                &color_view,
                depth_view,
                camera,
                &material,
                pyramid,
                Mat4::IDENTITY,
                [1.0, 1.0, 1.0, 1.0],
                wgpu::Color::TRANSPARENT,
                surface_format,
                1,
            );
        }

        {
            let RenderState {
                app,
                wireframe,
                camera,
                edges,
                depth_view,
                edge_vbuf,
                color_bg,
                ..
            } = &mut *state;
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
                    view: depth_view,
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
                app,
                &mut pass,
                camera,
                edges,
                LineColor {
                    color: [0.96, 0.88, 0.86, 0.82],
                },
                edge_vbuf,
                color_bg,
            );
        }
        state.app.queue().submit(std::iter::once(encoder.finish()));
        frame.present();
        Ok(())
    }
}
