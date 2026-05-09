# Recorder Features → Render Library API

> Comprehensive feature inventory for the screen recorder, mapped to the requirements they place on the in-repo `render` crate. The render library is **Pixi-shaped, Rust-idiomatic, scoped to recorder needs**.
>
> This is the working design doc for both the application feature surface and the library API. Read top-to-bottom on first pass; subsequent reads jump to §3 (API) or §6 (shader inventory) as references.

---

## Tag legend

- **[MVP]** — must work for first usable build (months 0–4 of synthesis doc roadmap).
- **[v1]** — feature parity target with OpenScreen v1.4 (months 4–7).
- **[v2]** — differentiation features SS doesn't have (months 7+).
- **[render]** — the feature has a meaningful requirement on `crates/wisp`.
- **[capture]**, **[encode]**, **[ui]**, **[audio]**, **[ml]** — owned by other crates, listed for completeness.

---

## 1. Recorder feature inventory

### 1.1 Pre-recording

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.1.1 | Source picker (full display, window, custom area) | MVP | ui, capture |
| 1.1.2 | Multi-display support | v1 | capture |
| 1.1.3 | Webcam selector + preview | MVP | ui, capture |
| 1.1.4 | Microphone selector + level meter | MVP | ui, audio |
| 1.1.5 | System audio per-app (macOS) | v1 | capture |
| 1.1.6 | Custom-area picker with grid + numeric inputs | v1 | ui, render (overlay) |
| 1.1.7 | Webcam aspect ratio + format selection (720p/1080p/4K) | v1 | capture |
| 1.1.8 | Recording preset selection | v1 | ui |
| 1.1.9 | Speaker notes / teleprompter window | v2 | ui |
| 1.1.10 | iPhone/iPad capture (USB-C) | v2 | capture |

### 1.2 Recording (active)

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.2.1 | Countdown overlay (3-2-1) | MVP | ui, render |
| 1.2.2 | Recording HUD (floating, frameless, always-on-top) | MVP | ui, render |
| 1.2.3 | Pause / resume | v1 | capture |
| 1.2.4 | Cursor position telemetry (sub-frame timestamps) | MVP | capture |
| 1.2.5 | Click event capture | MVP | capture |
| 1.2.6 | Cursor-type tracking (arrow / I-beam / hand) | v1 | capture |
| 1.2.7 | Keyboard event capture for shortcut overlay | v1 | capture |
| 1.2.8 | Hide-from-dock while recording | v1 | ui |
| 1.2.9 | System tray controls | v1 | ui |
| 1.2.10 | Speaker notes during recording | v2 | ui |

### 1.3 Editor compositing — the render-library-driven section

These are the per-frame composition primitives. **Every item here is owned by `crates/wisp`.**

| # | Feature | Tag | API element required |
|---|---|---|---|
| 1.3.1 | Background: solid color | MVP | `Graphics::rect_fill` |
| 1.3.2 | Background: linear gradient | MVP | `Graphics::gradient_rect` |
| 1.3.3 | Background: image (wallpaper) | MVP | `Sprite` with full-stage texture |
| 1.3.4 | Background: radial gradient | v1 | `Graphics::gradient_rect` (radial mode) |
| 1.3.5 | Background: glassmorphism / frosted | v1 | `BlurFilter` over capture + tint overlay |
| 1.3.6 | Recording quad with rounded corners | MVP | `Sprite` with rounded-rect mask shader |
| 1.3.7 | Recording quad with drop shadow | MVP | `DropShadowFilter` |
| 1.3.8 | Recording quad inset / padding | MVP | Container transform + Stage layout |
| 1.3.9 | Recording quad zoom transform (focus point + scale) | MVP | Container transform animation |
| 1.3.10 | Recording quad pan-follow-cursor inside zoom | MVP | Container transform animation |
| 1.3.11 | Motion blur during zoom/pan transitions | v1 | `MotionBlurFilter` (velocity-driven kernel) |
| 1.3.12 | 3D perspective rotation (X/Y/Z) | v1 | `Mesh` + custom WGSL pass (perspective matrix) |
| 1.3.13 | Cursor sprite (synthetic, hi-res) | MVP | `Sprite` with sprite-set switching |
| 1.3.14 | Cursor smoothing (spring physics) | MVP | computed in app, drives `Sprite.position` |
| 1.3.15 | Cursor velocity-aligned rotation | v1 | `Sprite.rotation` |
| 1.3.16 | Cursor type-aware crossfade (arrow → I-beam) | v1 | `Sprite` texture swap with alpha tween |
| 1.3.17 | Cursor hide-on-idle with fade | v1 | `Sprite.alpha` tween |
| 1.3.18 | Click ripple effect | MVP | `Graphics::ellipse_stroke` with animated radius+alpha |
| 1.3.19 | Click circle/dot ring | v1 | same primitive |
| 1.3.20 | Camera bubble (corner placement, size, roundness) | MVP | `Sprite` (video texture) + rounded-rect mask |
| 1.3.21 | Camera bubble shadow | MVP | `DropShadowFilter` on camera Container |
| 1.3.22 | Camera bubble auto-shrink during zoom | v1 | computed scale, drives Container transform |
| 1.3.23 | Camera mirror toggle | MVP | flip on `Sprite.scale.x` |
| 1.3.24 | Dynamic camera layouts (full-screen / overlay / hidden) | v2 | timeline-driven Container animation |
| 1.3.25 | Camera background blur (segmentation) | v2 | `BlurFilter` masked by ML-generated alpha texture |
| 1.3.26 | Mask/highlight rectangle (blur) | v1 | `BlurFilter` on cropped sub-region |
| 1.3.27 | Mask/highlight rectangle (highlight) | v1 | `Graphics::rect_fill` with alpha tint |
| 1.3.28 | Mask follows scrolling content (optical flow) | v2 | `Mesh` with displacement from compute pass |
| 1.3.29 | Annotations: text overlay | v1 | `Text` |
| 1.3.30 | Annotations: arrow | v1 | `Graphics::arrow` (or `Mesh` with SDF) |
| 1.3.31 | Annotations: rectangle/ellipse outline | v1 | `Graphics` |
| 1.3.32 | Annotations: free-draw | v2 | `Graphics::path` |
| 1.3.33 | Captions (auto-generated, edit-as-text) | v1 | `Text` (per-segment) |
| 1.3.34 | Caption styling (font, size, color, position, background) | v1 | `Text.style`, `Graphics::rounded_rect` background |
| 1.3.35 | Keyboard shortcut chip overlay | v1 | `Container` with `Graphics` chip + `Text` |
| 1.3.36 | Reactions (animated emoji overlay) | v2 | `Sprite` with timeline-driven keyframe animation |
| 1.3.37 | Device frame mockups (iPhone bezels) | v2 | `Sprite` with bezel image, recording quad inside |
| 1.3.38 | Aspect ratio change with re-flow | v1 | Stage layout recompute, animations re-flow |

### 1.4 Editor UI (chrome, not render)

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.4.1 | Timeline (multi-track) | MVP | ui |
| 1.4.2 | Trim / cut / split / ripple-delete | MVP | ui |
| 1.4.3 | Speed up/down per slice | v1 | ui, encode |
| 1.4.4 | Speed-up-typing auto-detection | v1 | ui (heuristic), capture (key events) |
| 1.4.5 | Loop playback | MVP | ui |
| 1.4.6 | Project autosave + recovery | MVP | ui (state) |
| 1.4.7 | Undo/redo (output-affecting state only) | MVP | ui |
| 1.4.8 | Inspector panel (selected element properties) | MVP | ui |
| 1.4.9 | Command menu (⌘K) | v1 | ui |
| 1.4.10 | Per-clip audio volume + waveform | v1 | ui, audio |
| 1.4.11 | Multi-track audio mixer | v2 | ui, audio |
| 1.4.12 | Voiceover re-record over existing video | v2 | ui, capture, audio |
| 1.4.13 | Manual zoom region (drag on canvas + duration on timeline) | MVP | ui, render (selection overlay) |
| 1.4.14 | Auto-zoom suggestions (dwell-based) | v1 | ui (heuristic) |
| 1.4.15 | Zoom level by 0–9 keypress | v1 | ui |
| 1.4.16 | Multi-select zoom ranges | v1 | ui |
| 1.4.17 | Crop tool with grid + numeric | v1 | ui, render (overlay) |

### 1.5 Audio

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.5.1 | Mic capture (mono/stereo) | MVP | audio |
| 1.5.2 | System audio mix | v1 | audio |
| 1.5.3 | Auto noise suppression | v1 | audio |
| 1.5.4 | Noise suppression intensity slider | v2 | audio |
| 1.5.5 | Background music import | v1 | audio |
| 1.5.6 | Built-in royalty-free music library | v2 | audio |
| 1.5.7 | Mouse-click sound effect | v1 | audio |
| 1.5.8 | Audio waveform render on timeline | v1 | ui (uses precomputed peaks) |

### 1.6 Captions / transcription

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.6.1 | On-device Whisper transcription | v1 | ml |
| 1.6.2 | Multi-language detection | v1 | ml |
| 1.6.3 | Edit transcript UI | v1 | ui |
| 1.6.4 | Captions overlay rendering | v1 | render (Text) |
| 1.6.5 | Filler-word removal | v2 | ml |
| 1.6.6 | Silence trimming | v2 | audio |

### 1.7 Export

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.7.1 | MP4 / H.264 software encode | MVP | encode (ffmpeg-next) |
| 1.7.2 | MP4 / H.264 hardware encode (VideoToolbox / MF) | v2 | encode |
| 1.7.3 | GIF export | v1 | encode (gifski) |
| 1.7.4 | Aspect ratio presets (16:9, 9:16, 1:1, 4:5) | v1 | render (stage size) |
| 1.7.5 | Resolution presets (720p / 1080p / 4K) | MVP | render (stage size) + encode |
| 1.7.6 | Framerate presets (24 / 30 / 60 fps) | MVP | encode |
| 1.7.7 | Quality presets | v1 | encode |
| 1.7.8 | Frame-to-clipboard (PNG snapshot) | v1 | render (read RenderTexture) |
| 1.7.9 | Streaming export (encode + upload concurrent) | v2 | encode, cloud |
| 1.7.10 | Multi-project batch export | v2 | ui, encode |
| 1.7.11 | Export raw recording files (separate streams) | v2 | encode |

### 1.8 Sharing / cloud (post-MVP, separate product)

| # | Feature | Tag | Owner |
|---|---|---|---|
| 1.8.1 | Hosted player at `/share/<id>` | v2 | cloud |
| 1.8.2 | Public/private links | v2 | cloud |
| 1.8.3 | Comments | v2 | cloud |
| 1.8.4 | View counter | v2 | cloud |
| 1.8.5 | Embed player | v2 | cloud |
| 1.8.6 | Engagement analytics | v2 | cloud |

---

## 2. Render-library requirements (derived)

Folding §1.3 (and the render-tagged items in 1.4/1.6/1.7) into a single requirements list for `crates/wisp`:

### 2.1 Scene graph

- **Container** with children, transform, alpha, visible, blend mode, optional filters, optional clip mask.
- **Transform** = position + scale + rotation + pivot + skew, composed into `Mat3`. Parent → child propagation with dirty flags.
- **Alpha and tint** propagate down the tree (Pixi-style world-alpha computation).
- **Z-order** by child insertion order; explicit `set_z` for sort key override.

### 2.2 Renderable node types

- **Sprite** — textured quad with anchor, tint, blend mode.
- **Graphics** — vector primitives: filled rect, rounded rect, ellipse, line, polyline, arrow, path. SDF-based for anti-aliased edges. Solid + linear-gradient fill.
- **Text** — bitmap font (MVP) and SDF font (v2). Single style per Text node; no rich text.
- **Mesh** — generic textured mesh with custom WGSL fragment shader; used for video texture quad with zoom transform and 3D perspective.

### 2.3 Textures

- **Texture** — owns or borrows a `wgpu::Texture`. Loaded from PNG/JPEG (image crate), or constructed empty.
- **VideoTexture** — texture whose contents are updated each frame by the app from external decoder output (BGRA bytes, fixed-format for MVP).
- **RenderTexture** — texture configured as a render target; can be sampled by other passes.

### 2.4 Filters

Filters take the bounding region of their target Container's rendered output, run N WGSL passes, and composite the result back. All filters work via `RenderTexture` ping-pong.

- **MotionBlurFilter** — velocity-driven separable directional blur. Velocity is set per-frame from the app (e.g., camera delta). Kernel size scales with velocity magnitude (lift OpenScreen's `PEAK_VELOCITY_PPS=1400`, `MAX_BLUR_PX=14`).
- **DropShadowFilter** — offset alpha mask + Gaussian blur + colored composite. Rounded-corner-aware (it's just blurring an alpha map).
- **BlurFilter** — separable Gaussian. Used standalone (mask blur) and as a building block (drop shadow).
- **ColorMatrixFilter** — 4×5 matrix. Used for tinting, desaturation, brightness, contrast.
- **DisplacementFilter** — sample-offset by displacement map. Used by mask-follows-scroll (v2).

### 2.5 Renderer

- Single `wgpu::Surface` for the editor preview window.
- Headless mode: render to `RenderTexture` with no surface (for export).
- One main render pass + N filter passes (each own pass).
- Draw call batching: sprites with the same texture and blend mode batch into one instance buffer.
- `Renderer::render(stage, target)` takes a target (`Surface | RenderTexture`).
- Internal viewport / scissor management.

### 2.6 Coordinate system

- Top-left origin, +Y down (matches Pixi, screen conventions).
- Logical units in pixels at 1× DPR; render scale handles physical pixels.
- DPR-aware: `Application::set_dpr(2.0)` updates internal viewport scale.

### 2.7 Out of scope (don't build)

- Particle systems
- Sprite sheets / frame animation
- Asset loader / manifest system
- Interaction event system (hit testing, pointer events)
- Accessibility tree
- Sound / ticker / scheduler (host app drives the frame loop)
- Multiple stages / scenes
- Lighting / shadows beyond drop shadow
- Generic post-processing pipeline configurator

---

## 3. Pixi-style Rust API

The API mimics PixiJS naming and structure where Rust idiom permits. Inheritance becomes composition; `extends Container` becomes `Container` as a struct field. `new` becomes `::new` or builder methods. Mutable scene graph is OK — the renderer is single-threaded over the stage.

### 3.1 `Application`

```rust
use render::{Application, AppConfig, Stage};

let app = Application::new(AppConfig {
    width: 1920,
    height: 1080,
    background_color: Color::rgb(0x10, 0x10, 0x14),
    dpr: 2.0,
    surface: Some(window_handle),    // Some = on-screen; None = headless
    ..Default::default()
})?;

let stage: &mut Stage = app.stage_mut();

// Per-frame call:
app.render()?;                       // renders to surface
// or:
app.render_to_texture(&render_tex)?; // headless / export
```

### 3.2 `Container`

```rust
pub struct Container {
    pub transform: Transform,         // position, scale, rotation, pivot, skew
    pub alpha: f32,
    pub visible: bool,
    pub blend_mode: BlendMode,
    pub filters: Vec<Box<dyn Filter>>,
    pub clip: Option<ClipMask>,
    children: Vec<NodeId>,
}

impl Container {
    pub fn new() -> Self;
    pub fn add_child<N: Into<Node>>(&mut self, child: N) -> NodeId;
    pub fn remove_child(&mut self, id: NodeId);
    pub fn set_filters(&mut self, filters: Vec<Box<dyn Filter>>);
}

// Convenience:
container.transform.position = vec2(100.0, 200.0);
container.transform.scale = vec2(1.5, 1.5);
container.transform.rotation = 0.1;
```

### 3.3 `Sprite`

```rust
pub struct Sprite {
    pub container: Container,
    pub texture: Handle<Texture>,
    pub anchor: Vec2,                 // 0.0..=1.0 normalized
    pub tint: Color,
}

let cursor = Sprite::from_texture(cursor_tex);
cursor.anchor = vec2(0.5, 0.5);
cursor.tint = Color::WHITE;
cursor.container.transform.position = vec2(x, y);
stage.add_child(cursor);
```

### 3.4 `Graphics`

```rust
pub struct Graphics {
    pub container: Container,
    primitives: Vec<Primitive>,
}

let mut bg = Graphics::new();
bg.fill(Fill::Gradient(LinearGradient::new(
    vec2(0.0, 0.0), vec2(0.0, 1080.0),
    &[(0.0, color::PURPLE), (1.0, color::BLUE)],
)));
bg.draw_rect(Rect::new(0.0, 0.0, 1920.0, 1080.0));

let mut shadow_bg = Graphics::new();
shadow_bg.fill(Fill::Solid(Color::WHITE));
shadow_bg.draw_rounded_rect(rect, 24.0);
shadow_bg.container.filters.push(Box::new(DropShadowFilter {
    offset: vec2(0.0, 8.0),
    blur: 32.0,
    color: Color::BLACK,
    alpha: 0.45,
}));
```

### 3.5 `Text`

```rust
pub struct Text {
    pub container: Container,
    content: String,
    style: TextStyle,
}

let style = TextStyle::default()
    .font(FontHandle::system_default())
    .size(24.0)
    .color(Color::WHITE)
    .align(TextAlign::Left);

let chip = Text::new("⌘K", style);
stage.add_child(chip);
```

For MVP: bitmap fonts only (precomputed atlas). For v2: SDF fonts via `cosmic-text` integration.

### 3.6 `Mesh`

```rust
pub struct Mesh {
    pub container: Container,
    geometry: Geometry,
    shader: ShaderHandle,
    uniforms: HashMap<String, UniformValue>,
}

// Used for the recording quad with 3D perspective:
let mut recording = Mesh::quad();
recording.set_texture(video_tex);
recording.set_shader(perspective_shader);
recording.set_uniform("u_perspective", Mat4::perspective(...));
```

### 3.7 Textures

```rust
pub struct Texture {
    inner: Arc<TextureInner>,
}

impl Texture {
    pub fn from_image(app: &Application, img: &DynamicImage) -> Self;
    pub fn empty(app: &Application, width: u32, height: u32, format: TextureFormat) -> Self;
}

pub struct VideoTexture { /* impl Deref<Target=Texture> */ }

impl VideoTexture {
    pub fn new(app: &Application, width: u32, height: u32) -> Self;
    pub fn upload_bgra(&mut self, app: &Application, bgra: &[u8]);
    // v2: zero-copy variants for IOSurface / D3D11Texture2D
}

pub struct RenderTexture { /* impl Deref<Target=Texture> */ }

impl RenderTexture {
    pub fn new(app: &Application, width: u32, height: u32) -> Self;
    pub fn read_pixels(&self, app: &Application) -> Vec<u8>;  // for export
}
```

### 3.8 Filters

```rust
pub trait Filter: Send + Sync {
    fn passes(&self) -> u32 { 1 }
    fn render_pass(
        &self,
        ctx: &mut FilterContext,
        input: &RenderTexture,
        output: &RenderTexture,
        pass_index: u32,
    );
}

// Built-in filters:
pub struct MotionBlurFilter {
    pub velocity: Vec2,
    pub max_kernel_px: f32,
    pub peak_velocity_pps: f32,
}

pub struct DropShadowFilter {
    pub offset: Vec2,
    pub blur: f32,
    pub color: Color,
    pub alpha: f32,
}

pub struct BlurFilter {
    pub radius: f32,
    pub quality: u32,    // number of taps
}

pub struct ColorMatrixFilter {
    pub matrix: [f32; 20],  // 4x5
}

pub struct DisplacementFilter {
    pub map: Handle<Texture>,
    pub scale: Vec2,
}
```

### 3.9 Stage layout / "Application root"

```rust
pub struct Stage {
    container: Container,
    size: Vec2,           // logical pixels
}

// Stage acts as the root container; everything else is its descendant.
// The Stage's size is the export resolution / window resolution.
```

---

## 4. Concrete recorder scene tree

What the recorder builds every frame, expressed in the API above:

```rust
let mut stage = app.stage_mut();

// 1. Background layer
let bg = Sprite::from_texture(wallpaper_tex);    // or Graphics::gradient_rect
stage.add_child(bg);

// 2. Recording container (zoomed, padded, rounded, shadowed)
let mut recording = Container::new();
recording.transform.position = padding_offset;
recording.transform.scale = current_zoom_scale;
recording.filters.push(Box::new(DropShadowFilter { /* ... */ }));
recording.filters.push(Box::new(MotionBlurFilter {
    velocity: zoom_pan_delta,
    ..Default::default()
}));
recording.clip = Some(ClipMask::RoundedRect { radius: 24.0 });

let recording_quad = Sprite::from_texture(screen_video_tex);
recording.add_child(recording_quad);
stage.add_child(recording);

// 3. Cursor sprite
let mut cursor = Sprite::from_texture(cursor_set.arrow);
cursor.anchor = vec2(0.0, 0.0);
cursor.container.transform.position = smoothed_cursor_pos;
cursor.container.transform.rotation = velocity_rotation;
stage.add_child(cursor);

// 4. Click effects (transient ripples, animated by app)
for ripple in &app_state.active_ripples {
    let mut ring = Graphics::new();
    ring.stroke(Stroke::new(2.0, Color::WHITE.with_alpha(ripple.alpha)));
    ring.draw_ellipse(ripple.center, vec2(ripple.radius, ripple.radius));
    stage.add_child(ring);
}

// 5. Camera bubble
let mut camera = Container::new();
camera.transform.position = camera_corner_pos;
camera.clip = Some(ClipMask::RoundedRect { radius: camera_radius });
camera.filters.push(Box::new(DropShadowFilter { /* ... */ }));
let cam_quad = Sprite::from_texture(camera_video_tex);
camera.add_child(cam_quad);
stage.add_child(camera);

// 6. Keyboard chips
for chip in &app_state.active_chips {
    let mut chip_container = Container::new();
    chip_container.transform.position = chip.position;

    let mut bg = Graphics::new();
    bg.fill(Fill::Solid(Color::rgba(0, 0, 0, 0.7)));
    bg.draw_rounded_rect(Rect::new(0.0, 0.0, chip.w, chip.h), 8.0);
    chip_container.add_child(bg);

    let label = Text::new(&chip.label, chip_text_style);
    chip_container.add_child(label);

    stage.add_child(chip_container);
}

// 7. Captions
if let Some(caption) = current_caption {
    let mut caption_box = Container::new();
    caption_box.transform.position = caption_position;
    let bg = /* rounded rect */;
    let text = Text::new(&caption.text, caption_style);
    caption_box.add_child(bg);
    caption_box.add_child(text);
    stage.add_child(caption_box);
}

app.render()?;
```

This scene tree is the "single render function" — the same code runs for editor preview AND for export, with the only difference being `app.render()` vs `app.render_to_texture(&export_target)`.

---

## 5. Render-library MVP cut-line

Build only this for the recorder MVP. Everything else is post-MVP.

### 5.1 Scene graph (MVP)

- ✅ `Application` with on-screen surface and headless RenderTexture modes
- ✅ `Container` with transform, alpha, visible, children, single filter
- ✅ `Transform` with position/scale/rotation/pivot, parent-child propagation
- ✅ Z-order by insertion
- ❌ blend modes other than NORMAL (defer)
- ❌ multi-filter chains (defer to v1, single filter is enough for MVP scenes)

### 5.2 Renderable nodes (MVP)

- ✅ `Sprite` with texture, anchor, tint
- ✅ `Graphics` with rect, rounded_rect, ellipse, line — solid fill, solid stroke
- ✅ `Text` with bitmap font (single font, single size — load at app init)
- ❌ `Mesh` (defer to v1 for 3D perspective)
- ❌ Graphics with gradient fill (defer to v1)

### 5.3 Textures (MVP)

- ✅ `Texture::from_image` (PNG, JPEG)
- ✅ `Texture::empty`
- ✅ `VideoTexture::upload_bgra`
- ✅ `RenderTexture` with `read_pixels`

### 5.4 Filters (MVP)

- ✅ `BlurFilter`
- ✅ `DropShadowFilter`
- ❌ `MotionBlurFilter` (defer to v1 — recorder MVP can ship without motion blur)
- ❌ `ColorMatrixFilter` (defer)
- ❌ `DisplacementFilter` (defer to v2)

### 5.5 Clip masks (MVP)

- ✅ `ClipMask::RoundedRect { radius }` (the only one MVP needs)
- ❌ arbitrary path clipping (defer)

### 5.6 What this enables (recorder-side)

With the MVP renderer, the recorder can do:
- Solid + image background
- Recording with rounded corners + drop shadow + zoom transform (no motion blur yet)
- Cursor sprite with smoothing
- Click ripples (animated ellipse stroke)
- Camera bubble with rounded corners + drop shadow
- Keyboard chips
- Captions

Cannot yet do (deferred to v1):
- Motion blur during zoom transitions
- 3D perspective rotation
- Gradient backgrounds
- Annotations beyond simple shapes
- Mask-follows-scroll
- Optical flow effects

That's a usable shipping product.

---

## 6. WGSL shader inventory

Shaders we need to write. Each is small (~50–150 lines). Keep them in `crates/render/shaders/` with naming `<purpose>.wgsl`.

### 6.1 Core shaders

| File | Purpose | Approx LOC |
|---|---|---|
| `quad.wgsl` | Vertex: instanced quad with model+view+proj. Fragment: sample texture + tint | 60 |
| `rounded_quad.wgsl` | Fragment: SDF-rounded-rect alpha clip on top of quad | 30 |
| `graphics_solid.wgsl` | Solid-color fill for rect/rounded_rect/ellipse/line | 40 |
| `graphics_gradient.wgsl` | Linear/radial gradient fill | 60 |
| `graphics_stroke.wgsl` | SDF-based anti-aliased stroke for outlines | 50 |
| `text_bitmap.wgsl` | Sample bitmap font atlas with tint | 30 |
| `composite.wgsl` | Final composite of stage to surface (or RenderTexture to RenderTexture) | 30 |

### 6.2 Filter shaders

| File | Purpose | Approx LOC | Pass count |
|---|---|---|---|
| `filter_blur_h.wgsl` | Horizontal Gaussian, 9-tap | 50 | 1 of 2 |
| `filter_blur_v.wgsl` | Vertical Gaussian, 9-tap | 50 | 2 of 2 |
| `filter_drop_shadow_extract.wgsl` | Extract alpha + shift + tint | 30 | 1 of 3 |
| `filter_drop_shadow_blur.wgsl` | Reuses filter_blur (h+v) | — | 2 of 3 |
| `filter_drop_shadow_composite.wgsl` | Composite shadow-under-source | 30 | 3 of 3 |
| `filter_motion_blur.wgsl` | Velocity-driven directional 9-tap (v1) | 70 | 1 |
| `filter_color_matrix.wgsl` | 4×5 matrix mul (v1) | 30 | 1 |
| `filter_displacement.wgsl` | Sample offset by displacement map (v2) | 40 | 1 |

### 6.3 Mesh / advanced (v1+)

| File | Purpose | Approx LOC |
|---|---|---|
| `mesh_perspective.wgsl` | 3D perspective transform for the recording quad | 70 |

Total MVP shader code: **~370 LOC of WGSL**. Total v1 shader code: **~610 LOC**.

---

## 7. Recorder app MVP cut-line

Mirror of §5 but for the app. Pull only [MVP] tags from §1:

- 1.1.1 Source picker (display / window / area)
- 1.1.3 Webcam selector
- 1.1.4 Mic selector + meter
- 1.2.1 Countdown
- 1.2.2 Recording HUD
- 1.2.4 Cursor telemetry
- 1.2.5 Click capture
- 1.3.1 Solid background
- 1.3.3 Image background
- 1.3.6 Rounded corners
- 1.3.7 Drop shadow
- 1.3.8 Padding
- 1.3.9 Zoom transform
- 1.3.10 Pan-follow-cursor
- 1.3.13 Cursor sprite
- 1.3.14 Cursor smoothing
- 1.3.18 Click ripple
- 1.3.20 Camera bubble
- 1.3.21 Camera shadow
- 1.3.23 Camera mirror
- 1.4.1 Timeline
- 1.4.2 Trim/cut/split
- 1.4.5 Loop playback
- 1.4.6 Project autosave + recovery
- 1.4.7 Undo/redo
- 1.4.8 Inspector panel
- 1.4.13 Manual zoom region
- 1.5.1 Mic capture
- 1.7.1 MP4 software encode
- 1.7.5 Resolution presets
- 1.7.6 Framerate presets

That's ~30 features. With the render library scoped to §5 and the app scoped here, MVP is bounded. Anything else is post-MVP.

---

## 8. Cross-reference: which renderer features unlock which app features

Use this when deciding what to build next inside `crates/wisp`:

| Renderer feature | Unlocks app features |
|---|---|
| `Sprite` + `VideoTexture` | 1.3.13 cursor, 1.3.20 camera, recording quad |
| `Container` + `Transform` propagation | 1.3.8 padding, 1.3.9 zoom, 1.3.10 pan, 1.3.22 camera shrink |
| `Graphics::rect_fill` | 1.3.1 solid bg |
| `Graphics::rounded_rect` | 1.3.6 rounded corners (via clip mask), 1.3.35 keyboard chip bg, 1.3.34 caption bg |
| `Graphics::ellipse` | 1.3.18 click ripple, 1.3.19 click ring |
| `ClipMask::RoundedRect` | 1.3.6 recording rounded corners, 1.3.20 camera bubble shape |
| `DropShadowFilter` | 1.3.7 recording shadow, 1.3.21 camera shadow |
| `BlurFilter` | 1.3.5 glassmorphism bg, 1.3.26 mask blur, 1.3.25 camera bg blur, building block for drop shadow |
| `Text` | 1.3.33 captions, 1.3.34 caption text, 1.3.35 keyboard chip text, 1.3.29 annotations |
| `Graphics::gradient` | 1.3.2 gradient bg, 1.3.4 radial gradient |
| `MotionBlurFilter` | 1.3.11 zoom motion blur |
| `Mesh` + `mesh_perspective.wgsl` | 1.3.12 3D rotation |
| `ColorMatrixFilter` | tone curves, desaturation effects |
| `DisplacementFilter` + optical flow | 1.3.28 mask-follows-scroll (v2) |
| `RenderTexture::read_pixels` | 1.7.1 MP4 export, 1.7.8 frame to clipboard |

---

## 9. What the API does NOT mimic from PixiJS

Capturing this so we don't accidentally drift into building Pixi:

- **No inheritance hierarchy.** Pixi has `Sprite extends Container extends DisplayObject`. We use composition: `Sprite { container: Container, ... }`.
- **No `Application.ticker`.** The host app drives the frame loop. The renderer has no scheduler.
- **No `Loader` / asset pipeline.** Apps load assets directly via `Texture::from_image`.
- **No interaction system.** Hit testing / pointer events live in the app, not the renderer. (Leptos handles UI interaction.)
- **No `ParticleContainer`** or other batched-only optimizations. Plain `Container` batches automatically when same texture + blend mode.
- **No `AnimatedSprite` / sprite sheets.** Recorder doesn't need them.
- **No accessibility tree.**
- **No global `PIXI` namespace.** Each `Application` is its own world.
- **No automatic mipmaps** unless explicitly requested.
- **No `Filter.padding`** auto-management (Pixi expands filter bounds for blur). MVP requires the app to size containers correctly; v1 adds padding heuristics.

---

## 10. Naming and crate organization

```
crates/render/
├─ Cargo.toml                 # name = "render", description = "2D scene graph + filter chain on wgpu"
├─ src/
│  ├─ lib.rs                  # pub re-exports
│  ├─ application.rs          # Application, AppConfig, Stage
│  ├─ scene/
│  │  ├─ mod.rs               # NodeId, Node trait
│  │  ├─ container.rs
│  │  ├─ sprite.rs
│  │  ├─ graphics.rs
│  │  ├─ text.rs
│  │  ├─ mesh.rs
│  │  ├─ transform.rs
│  │  └─ clip.rs
│  ├─ texture/
│  │  ├─ mod.rs
│  │  ├─ texture.rs
│  │  ├─ video_texture.rs
│  │  └─ render_texture.rs
│  ├─ filter/
│  │  ├─ mod.rs               # Filter trait, FilterContext
│  │  ├─ blur.rs
│  │  ├─ drop_shadow.rs
│  │  ├─ motion_blur.rs
│  │  ├─ color_matrix.rs
│  │  └─ displacement.rs
│  ├─ render/
│  │  ├─ mod.rs               # Renderer
│  │  ├─ batcher.rs
│  │  ├─ pipeline.rs          # wgpu RenderPipeline cache
│  │  └─ pass.rs              # FilterPass orchestrator
│  ├─ math/
│  │  ├─ mod.rs               # re-exports glam + extensions
│  │  └─ rect.rs
│  ├─ color.rs
│  └─ blend.rs
├─ shaders/
│  ├─ quad.wgsl
│  ├─ rounded_quad.wgsl
│  ├─ graphics_solid.wgsl
│  ├─ text_bitmap.wgsl
│  ├─ composite.wgsl
│  └─ filter_*.wgsl
└─ examples/
   ├─ hello_sprite.rs
   ├─ filter_chain.rs
   ├─ video_texture.rs
   └─ recorder_mock.rs       # mocks the recorder scene tree from §4
```

Public crate name: just `render` for now. If the library ever extracts, rename to something distinctive (`pixrs`, `tide`, `stage2d`, etc.) at extraction time.

---

## 11. Open API decisions to settle in the spike

Things to decide with code in hand, not in this doc:

1. **Node storage:** `Vec<Box<dyn Node>>` vs slotmap-based `NodeId` indexing. Slotmap is more efficient and avoids `Box<dyn>` for hot loops; pick during week 1.
2. **Filter padding.** Whether the renderer auto-expands the bounding box for blur kernels, or apps size containers correctly. MVP punts to apps; v1 may auto-pad.
3. **Mutable vs immutable scene API.** Pixi is mutable; Rust comfort says mutable too, but ergonomics around children-borrow-checker may push toward an arena.
4. **Text engine.** `cosmic-text` (full shaping, slow) vs `fontdue` (raster only, fast) vs in-house bitmap-only (fastest, ugly). MVP: bitmap-only via `fontdue` for atlas pre-rendering.
5. **WGSL shader compilation:** at app init (slow first frame) vs lazy (slow first use of feature). MVP: at init.
6. **Blend mode coverage.** Pixi has ~15 blend modes; we ship NORMAL and MULTIPLY for MVP. Add as needed.

These shouldn't block the MVP roadmap; lock them down as the code reveals constraints.
