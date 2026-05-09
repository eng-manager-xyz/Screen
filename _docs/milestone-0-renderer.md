# Milestone 0: `wisp` — Pixi-Equivalent in Rust + WebGPU

> **Goal:** ship a comprehensive 2D scene graph + filter chain library on `wgpu`, with a Pixi-shaped public API, scoped to power the screen recorder app. Library-shaped, in-repo at `crates/wisp`. Not yet published to crates.io — that's a v2 decision after the recorder ships.
>
> **Why M0 first:** the recorder is the consumer; the library is the means. Until the renderer can composite a video texture with rounded corners, drop shadow, motion blur, and a cursor sprite, there's no editor preview to wire UI into. M0 unblocks every meaningful subsequent milestone.

---

## Scope

This milestone delivers the API surface defined in `recorder-features-and-render-api.md` §3, with the **comprehensive cut** (not the MVP cut). That includes:

- Scene graph: `Container`, `Sprite`, `Graphics`, `Text`, `Mesh`
- Transforms with parent-child propagation
- Textures: `Texture`, `VideoTexture`, `RenderTexture`
- Filters: `BlurFilter`, `DropShadowFilter`, `MotionBlurFilter`, `ColorMatrixFilter`
- Graphics primitives: rect, rounded rect, ellipse, line, stroke, gradient fills (linear + radial)
- Bitmap text rendering
- Mesh with custom WGSL shader support (for 3D perspective)
- Headless render + on-screen render
- Four runnable examples that exercise the full surface

Explicitly out of scope (v1+ work, not M0):

- `DisplacementFilter` (needs optical-flow compute pass)
- SDF / advanced text rendering with shaping (only bitmap fonts in M0)
- Particle systems, sprite sheets, asset loader, interaction system, accessibility tree
- Hardware encoder integration (lives in `crates/encode`, not `wisp`)
- WASM target (M0 is native only; WASM target is v2 if ever needed)

---

## Acceptance criteria

- ✅ `cargo build -p wisp` succeeds on stable Rust
- ✅ `cargo test -p wisp` passes (visual regression tests for filters use snapshot images)
- ✅ `cargo run -p wisp --example hello_sprite` opens a window, draws a textured rounded-rect with a drop shadow
- ✅ `cargo run -p wisp --example filter_chain` shows a Container with stacked BlurFilter + DropShadowFilter + MotionBlurFilter
- ✅ `cargo run -p wisp --example video_texture` loops an MP4 as a `VideoTexture` on a moving sprite
- ✅ `cargo run -p wisp --example recorder_mock` builds the recorder scene tree from `recorder-features-and-render-api.md` §4 — the proof point for M1+
- ✅ `cargo run -p wisp --example headless_export` renders 60 frames into a `RenderTexture`, dumps PNGs to disk
- ✅ Public API matches the names from §3 of the design doc

---

## Tech notes

- **wgpu version:** 24.x (latest stable at 2026-05). Pin in workspace `Cargo.toml`.
- **Scene storage:** `slotmap` for `NodeId` indexing. Avoids `Box<dyn Node>` allocation in hot loops; arena-based.
- **Math:** `glam` for `Vec2`/`Vec3`/`Mat3`/`Mat4`, re-exported from `wisp::math`.
- **Image loading:** `image` crate for PNG/JPEG.
- **Text:** `fontdue` for glyph rasterization (faster than `cosmic-text`, no shaping needed for chips/captions).
- **Window/event loop:** examples use `winit` 0.30. Library is window-agnostic — apps pass a `RawWindowHandle`.
- **Shaders:** WGSL files in `crates/wisp/shaders/`, compiled at app init via `wgpu::ShaderModuleDescriptor`. `naga` validates them.
- **Testing:** `insta` for snapshot tests of filter outputs.

---

## Chunks

21 chunks across 9 phases. Each is sized to be completable in 1–4 hours of focused work.

### Phase 1: Workspace + crate (3 chunks)

#### M0.1 — Convert to Cargo workspace
- Edit root `Cargo.toml` to declare `[workspace]` with `members = ["crates/*"]`
- Move existing `src/main.rs` to `crates/app/src/main.rs` (placeholder; M1 fills this in)
- Create `crates/app/Cargo.toml` with `name = "screen-app"`
- **Done when:** `cargo build` succeeds at the workspace root

#### M0.2 — Scaffold `wisp` crate
- `cargo new --lib crates/wisp`
- Set up module skeleton: `lib.rs`, `application.rs`, `scene/`, `texture/`, `filter/`, `render/`, `math/`, `color.rs`, `blend.rs`
- Add deps to `crates/wisp/Cargo.toml`: `wgpu`, `glam`, `slotmap`, `bytemuck`, `image`, `fontdue`, `thiserror`, `tracing`
- Empty stubs for each module, `lib.rs` re-exports the public types
- **Done when:** `cargo build -p wisp` succeeds with the empty surface

#### M0.3 — Math, color, blend primitives
- `wisp::math` re-exports `glam::{Vec2, Vec3, Mat3, Mat4}`, adds `Rect`
- `wisp::color::Color` (RGBA f32, with `rgb()`, `rgba()`, `with_alpha()`, common constants)
- `wisp::blend::BlendMode` enum (NORMAL, MULTIPLY, ADD, SCREEN — only NORMAL implemented in M0; others stubbed)
- **Done when:** types compile, basic unit tests for `Color` and `Rect` pass

### Phase 2: Renderer core (3 chunks)

#### M0.4 — `Application` + wgpu device init
- `Application::new(AppConfig)` initializes `wgpu::Instance`, picks adapter, creates device + queue
- Optional `Surface` when `AppConfig::surface = Some(handle)`; otherwise headless
- `Application::resize(width, height)` reconfigures surface
- **Done when:** running an example creates an Application and prints adapter info

#### M0.5 — Hello triangle
- Build the simplest possible WGSL pipeline: a vertex+fragment shader for a colored triangle
- `Renderer` struct holds the pipeline; `render()` clears the screen and draws the triangle
- `examples/hello_triangle.rs` opens a winit window and draws
- **Done when:** running the example shows a triangle on screen

#### M0.6 — Textured quad pipeline
- Add `quad.wgsl` shader: vertex passes UV, fragment samples texture
- Add `Texture::from_image()` loading a PNG
- Renderer can draw an instanced textured quad with model transform + tint
- **Done when:** running an example shows a PNG textured quad on screen

### Phase 3: Scene graph (3 chunks)

#### M0.7 — Transform with parent-child propagation
- `Transform { position, scale, rotation, pivot, skew }` → composes to `Mat3`
- `WorldTransform` cache with dirty-flag invalidation
- Parent transform multiplied into child's local before world cache
- Unit tests covering: nested rotations, pivot points, scale propagation
- **Done when:** transforms compose correctly across 3+ levels of nesting

#### M0.8 — `Container` + scene graph storage
- `slotmap::SlotMap<NodeId, Node>` storing scene state
- `Container { transform, alpha, visible, blend_mode, filters, clip, children: Vec<NodeId> }`
- `Stage` is the root container, owned by `Application`
- `add_child` / `remove_child` API
- Tree traversal in render order
- **Done when:** an example builds a 3-deep nested container hierarchy and traverses it

#### M0.9 — `Sprite` API
- `Sprite { container, texture, anchor, tint }` with composition (no inheritance)
- Anchor as `Vec2` in 0..=1 normalized coords
- Tint multiplied with sampled texel in fragment shader
- Renderer batches Sprites with same texture + blend mode into one instance buffer
- **Done when:** drawing 100 sprites at varied positions in a single example uses 1 draw call when they share a texture

### Phase 4: Texture types (2 chunks)

#### M0.10 — Image `Texture` loading
- `Texture::from_image(&Application, &DynamicImage) -> Self`
- `Texture::empty(&Application, w, h, format) -> Self`
- Internal: `Arc<TextureInner>` with `wgpu::Texture` + `wgpu::TextureView` + `wgpu::Sampler`
- **Done when:** loading a PNG and assigning to a Sprite renders correctly

#### M0.11 — `VideoTexture` + `RenderTexture`
- `VideoTexture::new(&Application, w, h)` allocates a BGRA texture, exposes `upload_bgra(&[u8])` per-frame update via `queue.write_texture`
- `RenderTexture::new(&Application, w, h)` creates a texture configured as a render target
- `RenderTexture::read_pixels(&Application) -> Vec<u8>` for export readback
- **Done when:** an example uploads synthesized BGRA frames as a flipbook on a sprite, AND another example renders a scene to a RenderTexture and saves a PNG

### Phase 5: Graphics primitives (3 chunks)

#### M0.12 — `Graphics` solid fills
- `Graphics::new()`, `fill(Fill::Solid(color))`, `draw_rect(Rect)`, `draw_rounded_rect(Rect, radius)`
- `graphics_solid.wgsl` for filled quad rendering
- `rounded_quad.wgsl` for SDF-based anti-aliased rounded corners
- **Done when:** an example draws a filled rect and a rounded rect side-by-side

#### M0.13 — `Graphics` ellipse, line, stroke
- `draw_ellipse(center, radii)` via SDF fragment shader
- `draw_line(from, to, width)` via mitered quad strip
- `stroke(Stroke { width, color })` for outline rendering of any primitive
- **Done when:** click-ripple effect (animated stroked ellipse) renders correctly

#### M0.14 — `Graphics` gradient fills
- `Fill::Gradient(LinearGradient::new(start, end, stops))` and `Fill::Gradient(RadialGradient::new(center, radius, stops))`
- `graphics_gradient.wgsl` with linear + radial branches
- **Done when:** an example draws a vertical purple-to-blue gradient as a background

### Phase 6: Text (1 chunk)

#### M0.15 — Bitmap font atlas + `Text` node
- `FontHandle` wraps a `fontdue::Font`
- At app init, rasterize ASCII glyphs into a `Texture` atlas
- `Text { container, content, style }` with `TextStyle { font, size, color, align }`
- Renderer constructs glyph quads per `Text` from the atlas
- **Done when:** an example renders "Hello, world!" and a multi-line keyboard chip label

### Phase 7: Filters (3 chunks)

#### M0.16 — `Filter` trait + `BlurFilter`
- `trait Filter { fn passes(&self) -> u32; fn render_pass(&self, ctx: &mut FilterContext, input: &RenderTexture, output: &RenderTexture, pass: u32); }`
- `FilterContext` provides device, queue, encoder, viewport
- Filter pipeline: render container subtree to RenderTexture A, run filter passes ping-ponging A↔B, composite final output
- `BlurFilter { radius, quality }` separable Gaussian (horizontal pass + vertical pass)
- **Done when:** snapshot test confirms a blurred sprite matches reference image within tolerance

#### M0.17 — `DropShadowFilter`
- Three-pass implementation:
  1. Extract alpha, offset, tint with shadow color → temp RT
  2. Run BlurFilter horizontal+vertical on temp
  3. Composite blurred shadow under original sprite
- `DropShadowFilter { offset, blur, color, alpha }`
- **Done when:** a rounded-rect with drop shadow renders correctly, snapshot matches reference

#### M0.18 — `MotionBlurFilter` + `ColorMatrixFilter`
- `MotionBlurFilter { velocity: Vec2, max_kernel_px, peak_velocity_pps }` — directional 9-tap shader; kernel size scales with `velocity.length() / peak_velocity_pps`
- Lift OpenScreen's constants: `PEAK_VELOCITY_PPS = 1400.0`, `MAX_BLUR_PX = 14.0`
- `ColorMatrixFilter { matrix: [f32; 20] }` — 4×5 matrix multiplication on RGBA
- **Done when:** snapshots of motion-blurred sprite + desaturated sprite match references

### Phase 8: Mesh (1 chunk)

#### M0.19 — `Mesh` with custom WGSL
- `Mesh { container, geometry, shader, uniforms }`
- `Geometry::quad()` helper
- `ShaderHandle` wraps a custom WGSL fragment shader registered with the renderer
- `mesh_perspective.wgsl` example shader: applies a 4×4 perspective matrix
- Used to validate the API is general enough for v1's 3D rotation feature
- **Done when:** an example draws a textured quad rotating around the Y axis with perspective

### Phase 9: Examples (2 chunks)

#### M0.20 — `hello_sprite` + `filter_chain`
- `examples/hello_sprite.rs`: textured sprite, rounded clip, drop shadow, animated rotation
- `examples/filter_chain.rs`: container with three stacked filters (BlurFilter + DropShadowFilter + MotionBlurFilter), live parameter sliders via `egui` (or hardcoded animation)
- **Done when:** both examples run smoothly at 60fps on M-series Mac

#### M0.21 — `video_texture` + `recorder_mock`
- `examples/video_texture.rs`: loops an MP4 (decoded via `ffmpeg-next` — first introduction of the encode crate's dep) and renders it as a `VideoTexture` on a moving Sprite
- `examples/recorder_mock.rs`: builds the full scene tree from `recorder-features-and-render-api.md` §4 with synthetic data — background, recording quad with shadow + motion blur, cursor sprite, click ripples, camera bubble, keyboard chips, captions. **This is the proof point that the library is sufficient for M1+.**
- `examples/headless_export.rs`: renders the recorder_mock scene to a RenderTexture for 60 frames at 1080p, dumps each frame as PNG. Validates headless mode works for the future export pipeline.
- **Done when:** all three examples run; recorder_mock visually resembles a Screen Studio frame; headless_export produces 60 valid PNGs

---

## What this enables for M1+

After M0:
- M1 (drop zone + player) can ship without needing `wisp` (uses HTML5 video), but the renderer is ready to slot in for M2.
- M2 (capture pipeline) integrates `wisp` for the recording HUD overlay and live preview.
- M3 (editor) wires `wisp` into the Leptos UI as the preview canvas (sibling native window strategy from synthesis doc §4).
- M4 (export) reuses the `recorder_mock` scene-tree pattern with real data, rendered to `RenderTexture`, fed to `ffmpeg-next`.

---

## Estimated effort

21 chunks × 1–4 hours each = **~30–80 hours of focused work**. Solo, that's 2–4 calendar weeks at sustainable pace. Includes time for:

- Reading `wgpu` docs and learning curve
- Writing & debugging WGSL shaders
- Snapshot test setup + reference image generation
- API ergonomics iteration (the public surface will get refined as examples reveal pain points)

The *first* chunk is M0.1 (workspace conversion). The *hardest* chunks are likely M0.16/17/18 (filter pipeline orchestration with ping-pong RenderTextures). The *most rewarding* is M0.21 (recorder_mock — the moment the renderer "becomes" a screen recorder compositor).

---

## After M0

Move to M1 (drop zone + video player). M1 doesn't depend on `wisp` directly — it's an HTML5 video player. M1 validates the Tauri+Leptos shell. M2+ then bring `wisp` into the live application.

Tracked as tasks in the task list (M0.1 through M0.21).
