# Progress Log

Append-only log of completed tasks. **Newest entries at top.** Never edit historical entries except to add corrections at the bottom of an entry.

Use the template at the bottom for new entries.

---

## M-UI.2 — PlayerControls + editor-mock composition
- **Date:** 2026-05-09
- **Status:** ✅ done — second chunk under the locked workflow.
- **Files:** `crates/ui-storybook/src/components/player_controls.rs` (PlayerControls + PlayState); `assets/style.css` (~95 new lines for transport bar + editor-mock surface); `stories.rs` registers four new stories (paused / playing / near-end / editor-mock); SSR snapshot regenerated; four new chapters under `ui/chunks/`; `SUMMARY.md`.
- **Verified:** `just gate` green; `just snapshots-ui` regenerated 12 HTML demos (was 8); `just site` renders all four chapters.
- **Three player stories:**
  - `paused` (position 0.0) — round-trip the resting state.
  - `playing` (position 0.32) — the typical state, also covers the toggle-glyph swap.
  - `near-end` (position 0.94) — anti-regression for handle clipping at the right edge of the scrub track. Without an explicit story for this case, an SSR snapshot can't catch a careless dimension tweak that pushes the dot off the track.
- **Editor mock** composition: full editor preview as a single story — `Card`(metadata) → preview surface → `PlayerControls` → `Card`(Timeline) → `DopeSheet`. Becomes the reference layout the Tauri shell mounts when M-INT.1 lands.

---

## M-UI.1 — DropZone Leptos component (idle + active)
- **Date:** 2026-05-09
- **Status:** ✅ done — first chunk shipped under the new full workflow (story → asset → chapter → gate → site).
- **Files:** `crates/ui-storybook/src/components/drop_zone.rs`; `assets/style.css` (60 new lines); `stories.rs` registers two new stories; SSR snapshot updated; `_docs/book/src/ui/chunks/drop-zone-{idle,active}.md`; `SUMMARY.md`.
- **Verified:** `just gate` green; `just snapshots-ui` regenerated 8 HTML demos (was 6); `just site` renders both chunk pages with embedded `<iframe>` previews.
- **Notes:**
  - Two states: `Idle` (dashed outline, neutral copy, kbd hint) and `Active` (sky-blue solid border, transform-scale, distinct glyph + copy).
  - Glyphs differ between states (`⤓` idle, `⇣` active) — clippy's `match_same_arms` caught the initial duplicate and forced real visual differentiation.
  - Pure presentational: takes `state: DropZoneState` prop. Tauri shell will drive it from `WindowEvent::DragDrop` (next chunk M-UI.2).

---

## Side quest — Per-chunk mdBook chapters + workflow lock
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files:** `_docs/book/src/{wisp,ui}/chunks/<id>.md` × 18; `SUMMARY.md`; `CLAUDE.md` per-task workflow expanded to 7 steps with explicit ASSET + CHAPTER between STORY and CHECK.
- **Verified:** `just gate` green; `just docs-strict` passes (after fixing two broken intra-doc links in `wisp::filter::motion_blur`); `just site` renders all 18 chapters.
- **Four mdBook lessons** in CLAUDE.md: `--dest-dir` is source-relative not cwd-relative; 0.5 dropped `multilingual` and `copy-fonts`; duplicate bin names collide in `cargo doc`; rustdoc paths use underscored crate names.

---

## Side quest — Docs infrastructure (rustdoc + mdBook + screenshot pipeline)
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files:** workspace lints (`missing_docs = "warn"` + rustdoc broken-link enforcement); `justfile` (`docs`, `docs-strict`, `site`, `snapshots`); `_docs/book/` mdBook scaffold; `crates/{wisp,ui}-storybook/src/bin/export_stories.rs`; per-feature assets at `_docs/book/src/assets/`.
- **Verified:** `just gate` (now includes `docs`); `just site` produces `target/book/` with prose + rustdoc API ref + 18 per-feature assets.
- **80+ missing-docs warnings backfilled** across wisp (34), wisp-storybook (2), screen-app (1), ui-storybook (42 via module-level allow because UI components are documented in mdBook stories).

---

## Side quest — Leptos UI (Button + Card + DopeSheet + SSR snapshots)
- **Date:** 2026-05-09
- **Status:** ✅ done — `crates/ui-storybook/` parallels `wisp-storybook` for the HTML/CSS layer.
- **Files:** new crate with `src/{lib.rs, components/{button,card,dope_sheet}.rs, stories.rs}`; SSR snapshot `tests/snapshots.rs`; plain-CSS stylesheet in rust-ui's dark zinc aesthetic.
- **3 new lessons:** `#[component]` macro rewrites function shape (use module-level allow); `leptos::prelude::*` brings `RenderHtml::to_html()` into scope; `<Show when=…>` requires `'static` closure.

---

## Side quest — Story integration tests + deferred examples
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files:** wisp-storybook split into `[lib]` + `[[bin]]`; `tests/story_smoke.rs` (2 tests: validation-scope + visibility); `tests/story_fingerprints.rs` (insta YAML quadrant snapshot); `tests/snapshots/` baseline; 2 new stories (s_motion_blur + s_color_matrix); `crates/wisp/examples/recorder_mock.rs` and `examples/video_texture.rs`.
- **Verified:** `just gate` passes (95 tests, was 92); `just security` clean. Both new examples actually run and produce PNGs (`target/recorder_mock.png` 1280×720; `target/video_texture/frame_NN.png` × 8).
- **Three layers of story testing:**
  1. **Smoke** — wgpu validation error scope catches "console errors at runtime"
  2. **Visibility** — at least 50 pixels diverge from clear color (story actually drew)
  3. **Fingerprint** — insta YAML snapshot of 4×4 quadrant averages, bucketed for ~3% driver tolerance
- **4 new CLAUDE.md lessons** captured under "Story testing pattern": insta first-run UX, wgpu error scope as console-error gate, quadrant fingerprint pattern, `tick(0.0)` for animated stories.
- **All 12 storybook stories now under regression testing.** Future story changes show structured snapshot diffs.

---

## M1.1 → M4.3 — Tauri shell + drop-zone player (consolidated)
- **Date:** 2026-05-09
- **Status:** ✅ done — all 11 chunks delivered as one Tauri shell with vanilla HTML/CSS/JS frontend.
- **Files:** `crates/app/{Cargo.toml, build.rs, tauri.conf.json, src/main.rs, dist/{index.html, styles.css, app.js}, icons/icon.png}`
- **Verified:** `just gate` passes (92 tests); `just security` passes after adding 16 gtk-rs unmaintained-advisory exemptions (ISS-02). `cargo run -p screen-app` awaits manual verification.
- **Notes:**
  - **Pivoted Leptos → vanilla HTML+JS.** Tauri+Leptos+Trunk has dual-target ceremony; the M1 product is small enough that 150 lines of vanilla frontend works. Leptos can be re-introduced as the editor UI grows.
  - **Three new lessons** in CLAUDE.md (Tauri 2 specifics): icon.png required at compile time even with bundle disabled; `protocol-asset` feature required for `convertFileSrc`; `cargo machete` needs `ignored = ["tauri"]` because `generate_context!` is a macro.
  - **ISS-02 filed:** 16 gtk-rs unmaintained-only advisories exempted in `deny.toml`. Linux-only, none exploits.

---

## M0.17 → M0.21 — Filters, Mesh, examples (consolidated)
- **Date:** 2026-05-09
- **Status:** ✅ done (M0.17 / M0.18 / M0.19 / M0.20 fully; M0.21 partial — `headless_export` shipped, `recorder_mock` + `video_texture` examples deferred)
- **Commits:** `feat(wisp): DropShadowFilter`, `feat(wisp): MotionBlurFilter + ColorMatrixFilter`, `feat(wisp): Mesh node with perspective rotation`
- **Tests:** 92 → ?? (M0.20/M0.21 add hello_sprite + headless_export examples). `just gate` green; `headless_export` actually wrote a 800×450 PNG with `draw_calls=3 sprites=1 graphics=2 glyphs=20`.
- **Notes:**
  - **DropShadow architecture:** all four passes (extract → blur h → blur v → composite) inside one `Filter::render_pass` call so it can manage two scratch RTs without changing the Filter trait. Reuses `blur::run_blur_pass`.
  - **`replace_all` disaster, again:** clobbered `scratch_a`/`scratch_b` identifier code while trying to add backticks to doc comments. Fixed by reverting and using targeted `Edit`. Lesson already in CLAUDE.md from M0.11 — applied this time after the fact.
  - **MotionBlur** = single-direction blur via existing `run_blur_pass`, with kernel size driven by `velocity.length() / peak_velocity_pps` (constants 1400 / 14 lifted from OpenScreen).
  - **ColorMatrix** = generic 4×5 matrix in shader, with named constructors (identity / grayscale / brightness).
  - **Mesh** = textured quad with Y-axis perspective rotation. Generic per-instance custom WGSL deferred — current shape covers the recorder's needs (camera-bubble tilt, 3D card flip).
  - **`headless_export.rs`** — full proof point: builds a scene with Sprite + Graphics + Text, renders to RenderTexture, writes PNG via the `image` crate. Validated `target/headless_export.png` is a real 800×450 image.
  - **Deferred to follow-up chunk:** `recorder_mock.rs` (full recorder scene tree) and `video_texture.rs` (per-frame BGRA upload demo). The infra exists and is exercised by tests; only the dedicated example files are missing.

---

## M0.16 — Filter trait + BlurFilter
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files:** `filter.rs` (trait + FilterContext), `filter/blur.rs`, `shaders/filter_blur.wgsl`, `render.rs::apply_filter` orchestrator, tests, storybook story
- **Verified:** `just gate` (87 tests, was 85), `just security` clean, storybook "Blur filter"
- **Notes:** Two-pass separable Gaussian. Filter::apply_filter orchestrator allocates a scratch RenderTexture and ping-pongs. Test caught a real geometry surprise: blur with `radius=4` and offset multiplier `i*radius` spans 16 texels — too wide for a 12-px input square; centers darken under wide blur. Test now uses `radius=1.0` for the brightness assertion + a sum-of-diffs check that proves the blur ran.
- **Issues filed:** none

---

## M0.15 — Bitmap font atlas + Text node
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:** `Cargo.toml` (font8x8), `scene/text.rs`, `scene/node.rs`, `shaders/text.wgsl`, `render/text_pipeline.rs`, `render.rs` (+ glyphs_drawn stat), tests, storybook story
- **Verified:** `just gate` (85 tests, was 78), `just security` clean, storybook shows "Bitmap text"
- **Notes:**
  - **Decision:** font8x8 (bitmap, embedded) over fontdue (vector, needs binary). Avoids checking in font files; vector swap is a future chunk if needed.
  - **New lesson recorded:** empty wgpu buffer panics on `buffer.slice(..)` — always skip empty batches before the draw path. Documented in CLAUDE.md.
  - **Recursive-fix loop:** 4 iterations (fmt, clippy similar_names + cast_precision in atlas math, doc backticks, empty-buffer panic).
- **Issues filed:** none

---

## M0.14 — Graphics gradient fills (linear + radial)
- **Date:** 2026-05-09
- **Status:** ✅ done — first chunk under the locked storybook convention
- **Files changed:**
  - `crates/wisp/src/scene/graphics.rs` — `Fill` extended with `LinearGradient { start, end, color_a, color_b }` and `RadialGradient { center, radius, color_a, color_b }`. Endpoints in primitive-local coords.
  - `crates/wisp/shaders/graphics_solid.wgsl` — added `color_b`, `grad_a`, `grad_b`, `fill_kind` instance fields. New `evaluate_fill` function: solid (kind=0), linear gradient via projection (kind=1), radial gradient via distance (kind=2).
  - `crates/wisp/src/render/graphics_pipeline.rs` — `GraphicsInstance` now 144 bytes with 14 vertex attributes. New `ResolvedFill` struct decouples Fill enum from instance encoding.
  - `crates/wisp/tests/render_graphics.rs` — 2 new integration tests: `linear_gradient_visually_red_at_top_blue_at_bottom`, `radial_gradient_center_to_edge`.
  - `crates/wisp-storybook/src/stories/s_graphics_gradients.rs` + writeup — story showing linear + radial side-by-side.
- **Verified:**
  - `just gate` — passes (78 tests, was 76)
  - `just security` — clean
  - `just storybook` — `Graphics → Gradient fills` story renders correctly
- **Notes:**
  - **Recursive-fix loop fired 5 iterations:**
    1. fmt collapse
    2. test failure: bottom row blue=0 → I had picked row 24 which is OUTSIDE the rect (NDC y=-0.53, rect spans -0.5..+0.5). Fixed by picking row 21 (NDC y≈-0.34, well inside).
    3. test failure on first run: gradient direction swapped — I'd put red at -Y, but +Y is up in NDC and "top" visually. Reordered start/end so red lives at the visual top.
    4. another fmt collapse from the assertion changes
    5. green
  - **Coordinate sanity-check learned:** for a 32-row image rendering an NDC rect from y=-0.5 to y=+0.5, valid interior rows are 8..23. Row 8 has NDC y≈+0.47 (just inside top), row 23 has NDC y≈-0.47 (just inside bottom), row 24 has NDC y≈-0.53 (outside, below).
  - **Gradient coordinates are primitive-local.** `(0, +0.5)` is the top of a unit-half-extent rect; `(0, -0.5)` is the bottom. Same axes the SDF uses; gradients transform with the primitive.
  - **Solid fill backward compatible.** `Fill::Solid` resolves to `fill_kind=0` with `color_b == color`. Existing M0.12/M0.13 tests pass unchanged.
- **Issues filed:** none

---

## Side quest — wisp-storybook + locked convention
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp-storybook/` — new crate. eframe (egui+winit+wgpu) shell, `Story` trait, registry, app with top-bar story picker + 4/5 canvas + 1/5 right-sidebar write-up. wisp renders to a `RenderTexture` registered with egui via `register_native_texture` — zero-copy GPU-side display.
  - `crates/wisp-storybook/src/stories/s_sprite_batcher.rs` + writeup — animated 100-sprite Lissajous demo (M0.9)
  - `crates/wisp-storybook/src/stories/s_graphics_rounded.rs` + writeup — 3 rectangles with fill/stroke/sharp variants (M0.12 + M0.13)
  - `crates/wisp-storybook/src/stories/s_graphics_ellipse.rs` + writeup — animated 3-ripple click effect (M0.13)
  - `crates/wisp/src/application.rs` — added `Application::from_wgpu(instance, adapter, device, queue, config)` for embedding hosts that already own a wgpu context
  - `Cargo.toml` (workspace) — added `[workspace.dependencies]` for shared deps
  - `Justfile` — `just storybook` recipe
  - `deny.toml` — added `OFL-1.1` and `Ubuntu-font-1.0` to license allowlist (egui's bundled fonts)
  - `CLAUDE.md`, `_docs/WORKFLOW.md`, `_docs/PROGRESS.md` template — locked the storybook-entry convention into the workflow
- **Verified:**
  - `just gate` — passes (76 tests; storybook builds clean)
  - `just security` — passes after license allowlist update
  - `just storybook` — opens window with 3 stories, top-bar nav, sidebar write-up, live render
- **Notes:**
  - **Locked convention:** every renderable feature ships with a story in `crates/wisp-storybook/src/stories/`. The CLAUDE.md non-negotiable loop is now `TEST → STORY → CHECK → UPDATE → STATUS`. Non-render features (math, capture, encode) are exempt.
  - **eframe + wisp share one wgpu device.** Storybook accepts eframe's wgpu device via `Application::from_wgpu`. wisp renders to a `RenderTexture`; the texture view is registered with egui via `register_native_texture`. egui samples it as `egui::Image` — zero CPU readback.
  - **egui 0.31** (latest as of 2026-05) is the version aligned with wgpu 24. Earlier 0.29 used wgpu 22 and caused type-mismatch errors.
  - **Recursive-fix loop fired 8 iterations** — every category we've trained on:
    1. wgpu version mismatch (egui 0.29 vs wgpu 24) → bumped to egui 0.31
    2. fmt collapse (×2)
    3. clippy: dead `id` field, f32→u32 casts (×4), collapsible_if, i32→f32 cast precision
    4. cargo-deny: wildcard wisp dep → set explicit version
    5. cargo-deny: license OFL-1.1 not allowed → added
    6. cargo-deny: Ubuntu-font-1.0 not allowed → added
    7. cargo-machete: pollster + egui-wgpu unused (eframe transitively re-exports)
    8. cargo-machete: tracing unused
  - **Backfilled 3 stories of 9.** M0.5 hello_triangle, M0.6 hello_quad, M0.7 transform, M0.8 scene graph, M0.10 image, M0.11 video texture, M0.12 sharp rect remain. Will land per-chunk in subsequent passes.
- **Issues filed:** none

---

## M0.13 — Graphics ellipse, line, stroke
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/scene/graphics.rs` — added `Stroke { width, color }`, `Primitive::Ellipse { center, radii, fill, stroke }`, `Primitive::Line { from, to, width, fill }`. Builder API gains `stroke(Option<Stroke>)`, `draw_ellipse(center, radii)`, `draw_line(from, to, width)`. RoundedRect primitive now also carries optional stroke.
  - `crates/wisp/shaders/graphics_solid.wgsl` — extended with `kind: u32` (0=rect, 1=ellipse) and `mode: u32` (0=fill, 1=outline). Vertex shader expands quad by `stroke_width/2` in outline mode. Fragment branches on kind for SDF (rounded-rect or ellipse), branches on mode for fill or outline-band alpha.
  - `crates/wisp/src/render/graphics_pipeline.rs` — `GraphicsInstance` now 104 bytes with `radius`, `stroke_width`, `kind`, `mode`. Vertex layout grew to 10 attributes (added two `Float32` + two `Uint32`). Stroked primitives emit a second outline instance with the stroke color.
  - `crates/wisp/src/scene.rs` / `lib.rs` — re-export `Stroke`
  - `crates/wisp/tests/render_graphics.rs` — 3 new integration tests: `ellipse_fills_center_clears_corner`, `line_renders_along_diagonal`, `stroked_rect_emits_two_instances_one_draw_call`
- **Verified:**
  - `just gate` — passes (76 tests, was 73)
  - `just security` — clean
- **Notes:**
  - **Stroke rendering via second instance.** Each stroked primitive emits two instances: one fill (mode=0), one outline (mode=1). Both batch into the same draw call thanks to the unified pipeline. The shader expands the bounding quad by `stroke_width/2` in outline mode so the band has room to render.
  - **Ellipse SDF is the standard scaled-circle approximation** (`(length(p / r) - 1) * min(r.x, r.y)`). Visually correct for moderate eccentricities; exact SDF would need iteration. Good enough for click ripples, camera bubble masks, etc.
  - **Lines render as rotated thin rects.** `delta.y.atan2(delta.x)` for the rotation angle, `(length, width)` for the half-extents, midpoint for the position. No special line code path in the shader.
  - **Recursive-fix loop fired 2 iterations:** fmt collapse, then green.
  - **Primitives_drawn counts logical primitives.** A stroked rect = 1 `graphics_drawn`, but emits 2 instances. Tests verify the distinction.
- **Issues filed:** none

---

## M0.12 — Graphics solid fills (rect + rounded rect)
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/scene/graphics.rs` — `Graphics`, `Fill::Solid(Color)`, internal `Primitive::RoundedRect { rect, radius, fill }`. Builder API: `new()`, `fill(Fill)`, `draw_rect(Rect)`, `draw_rounded_rect(Rect, radius)`. 6 unit tests.
  - `crates/wisp/src/scene/node.rs` — added `Node::Graphics(Graphics)` variant + `From<Graphics> for Node`
  - `crates/wisp/src/scene.rs` / `lib.rs` — re-export `Fill`, `Graphics`
  - `crates/wisp/shaders/graphics_solid.wgsl` — instanced SDF rounded-rect shader (radius=0 ⇒ sharp); `fwidth`-based AA
  - `crates/wisp/src/render/graphics_pipeline.rs` — instance vertex layout (`mat4×4 + vec4 + vec2 + f32 + pad`), `draw_stage` collects primitives across all `Graphics` nodes into one batch
  - `crates/wisp/src/render.rs` — wire `GraphicsPipeline` into `Renderer`, add `RenderStats::graphics_drawn`
  - `crates/wisp/tests/render_graphics.rs` — 4 integration tests including `solid_rect_fills_specified_region` (pixel verification at center + corner) and `rounded_rect_clears_corners` (corner pixel cut by SDF) + `many_primitives_batch_into_one_draw_call` (50 rects = 1 draw call)
- **Verified:**
  - `just gate` — passes (73 tests, was 63)
  - `just security` — clean
- **Notes:**
  - **One unified shader for rect + rounded rect.** The milestone doc suggested separate `graphics_solid.wgsl` and `rounded_quad.wgsl`; collapsed into one because `radius == 0` already produces a sharp rect under the SDF. Less code, one pipeline, primitives across all `Graphics` batch into one draw call.
  - **Pixel-readback verification works.** The center-of-rect / corner-of-rounded-rect tests confirm the SDF + AA produce visually-correct output, not just non-panicking submissions.
  - **Recursive-fix loop fired 4 iterations:**
    1. `fmt` collapses
    2. `clippy::derivable_impls` — replaced manual `Default for Graphics` with `#[derive(Default)]`
    3. `clippy::identity_op` — `1 * 32 + 1` → `32 + 1`
    4. green
  - **All Graphics primitives across the entire scene batch into 1 draw call.** Stress test: 50 primitives in one Graphics, all in one draw call — verified by `many_primitives_batch_into_one_draw_call`.
  - **Sprite + Graphics together = 2 draw calls** (one per pipeline). Render order: sprites first, then graphics. M0.13/M0.14 keep adding to graphics; render order stays sprite→graphics for now.
- **Issues filed:** none

---

## M0.11 — VideoTexture + RenderTexture
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/texture.rs` — renamed internal field `_texture` → `texture` (now used by `wgpu_texture()`); added `pub(crate) wgpu_texture()` accessor
  - `crates/wisp/src/texture/video_texture.rs` — `VideoTexture { texture: Texture }` with `new`, `upload_bgra` (BGRA8 per-frame upload via `queue.write_texture`), `width`, `height`, `texture` accessor; panics on byte-length mismatch
  - `crates/wisp/src/texture/render_texture.rs` — `RenderTexture` with own `Arc<RenderTextureInner>` storage (RENDER_ATTACHMENT | COPY_SRC | TEXTURE_BINDING), `new`/`with_format` constructors, `view`/`sampler`/`format`/`width`/`height` accessors, **`read_pixels(app) -> Vec<u8>`** with proper `COPY_BYTES_PER_ROW_ALIGNMENT` padding strip
  - `crates/wisp/src/lib.rs` — re-export `VideoTexture` and `RenderTexture`
  - `crates/wisp/tests/render_texture.rs` — 5 integration tests including the **first real pixel-readback verification**: clear-color round-trip + a sprite captured at the texture's center pixel
- **Verified:**
  - `just gate` — passes (63 tests, was 58)
  - `just security` — clean
- **Notes:**
  - **`read_pixels` is the unlock.** Subsequent chunks (Graphics M0.12+, filters M0.16+) get real pixel-level assertions instead of just "validation passed without panic." Pattern: render to a `RenderTexture`, `read_pixels`, assert specific pixel positions.
  - **sRGB sting:** the first run of `render_texture_read_pixels_round_trips_clear_color` failed because the clear color (linear f32) gets gamma-encoded when written to an `Rgba8UnormSrgb` target — `0.251 → 0.537 = 137` not the expected 64. Fix: use `RenderTexture::with_format(Rgba8Unorm)` (non-sRGB) for the round-trip test so bytes match.
  - **`COPY_BYTES_PER_ROW_ALIGNMENT` (256-byte rows):** `read_pixels` allocates a padded staging buffer and strips padding row-by-row before returning a tightly-packed buffer. Verified by tests asserting `bytes.len() == w*h*4`.
  - **Recursive-fix loop fired 4 iterations:**
    1. `clippy used_underscore_binding` — the `_texture` field is now read by `wgpu_texture()`; rename
    2. `fmt` failed because my over-eager `replace_all _texture → texture` clobbered `pub mod render_texture`/`video_texture`/`create_texture`/`write_texture`/`wgpu_texture` (each contained `_texture` substring). **Lesson: never `replace_all` on a substring without auditing matches.** Recovered by rewriting `texture.rs` cleanly.
    3. test failure on sRGB conversion (above)
    4. green
  - **`VideoTexture` defaults to `Bgra8UnormSrgb`** — matches macOS ScreenCaptureKit output. Other backends (Windows/Linux) may need `Rgba8UnormSrgb`; revisit when we wire the recorder.
- **Issues filed:** none

---

## M0.10 — Image Texture loading
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/texture.rs` — added `Texture::empty(app, w, h, format)` (default usage: `TEXTURE_BINDING | COPY_DST`)
  - `crates/wisp/tests/texture_loading.rs` — 4 tests: empty in sRGB + BGRA, grayscale `from_image` conversion, full PNG round-trip via `image::load_from_memory`
- **Verified:**
  - `just gate` — passes (58 tests, was 54)
  - `just security` — clean
- **Notes:**
  - M0.6 already shipped `Texture::from_rgba` and `Texture::from_image` along with the `Arc<TextureInner>` storage. M0.10's residual scope was `Texture::empty` plus explicit PNG-byte verification.
  - **No recursive-fix loop iterations needed** — first run was green.
  - **PNG round-trip without checked-in binaries:** the test encodes a synthetic `RgbaImage` to PNG bytes via `DynamicImage::write_to`, decodes via `image::load_from_memory`, and feeds the result to `Texture::from_image`. Repo stays binary-free.
  - **`Texture::empty` accepts BGRA format** — verified for the 1920×1080 surface that `VideoTexture` (M0.11) will use.
  - **`Texture::empty` usage flags are fixed** at `TEXTURE_BINDING | COPY_DST`. A `with_usage` constructor will land in M0.11 to support `RenderTexture`'s `RENDER_ATTACHMENT | COPY_SRC | TEXTURE_BINDING`.
- **Issues filed:** none

---

## M0.9 — Sprite API + instanced batcher
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/scene/sprite.rs` — `Sprite { container, texture, anchor, tint }` via composition; `from_texture`/`with_anchor`/`with_tint` builders; 3 unit tests
  - `crates/wisp/src/scene/node.rs` — added `Node::Sprite(Sprite)` variant + `From<Sprite> for Node`
  - `crates/wisp/src/scene.rs` — `pub use sprite::Sprite`
  - `crates/wisp/src/lib.rs` — re-export `Sprite`
  - `crates/wisp/src/scene/container.rs` — `children()` now returns `impl DoubleEndedIterator + ExactSizeIterator` (the bare `impl Iterator` opaque type drops `rev()` capability)
  - `crates/wisp/src/texture.rs` — manual `impl Debug for Texture` (wgpu types don't all derive Debug); `id()` accessor for batching identity
  - `crates/wisp/shaders/sprite.wgsl` — instanced sprite shader: 6-vert quad, per-instance `(model: mat4x4f, tint: vec4f, anchor: vec2f)`
  - `crates/wisp/src/render/sprite_pipeline.rs` — `SpritePipeline`, `SpriteInstance` (88 bytes, `Pod`), `Float32x4×4 + Float32x4 + Float32x2` instance vertex buffer layout, `collect_batches` traversal
  - `crates/wisp/src/render.rs` — `RenderStats { draw_calls, sprites_drawn }`, `Renderer::render_stage` + `SpritePipeline` field
  - `crates/wisp/tests/render_sprite.rs` — 5 integration tests including the **headline `one_hundred_sprites_share_texture_one_draw_call`**
- **Verified:**
  - `just gate` — passes (54 tests, was 46)
  - `just security` — passes
- **Notes:**
  - **The headline test passes.** 100 sprites sharing a texture batch into exactly 1 draw call, 100 sprites drawn — the M0.9 anti-regression contract.
  - **Pre-order batching with stable order:** the batcher uses an `order: Vec<Key>` alongside the `HashMap` so batches drain in first-encounter order. Texture-pointer-equality (`Arc::as_ptr`) is the batch key; clones of the same `Texture` collapse into one batch.
  - **Hidden parents skip descendants** (`hidden_container_skips_sprite_descendants` test): the traversal `continue`s on `!container.visible` and never pushes its children.
  - **Scene-graph traversal lives in the renderer** (`sprite_pipeline::collect_batches`), not in `Stage`. Stage owns the data; rendering owns the policy of how to walk it. This keeps Stage focused on storage.
  - **Recursive-fix loop fired 5 iterations:**
    1. `fmt` — rustfmt collapsed long expressions in scene/sprite.rs and sprite_pipeline.rs
    2. `check` — `Texture` lacked `Debug`; `impl Iterator` opaque type lacked `DoubleEndedIterator`
    3. `clippy` — `len() as u32` cast warnings (×2), `must_use` candidates on `render_stage` and `children` (×2)
    4. `clippy` (test) — `i as f32` cast precision warning; switched loop var to `u16` for lossless `f32::from`
    5. green
  - **Coordinate system: NDC for now.** Each sprite's transform.position is in NDC space `[-1, 1]`. Pixel-space projection (orthographic camera) lands in a later chunk. The 100-sprite test scatters across NDC for visual variety; doesn't depend on this.
  - **Anchor handling:** local quad is `[0, 1]²`; vertex shader subtracts `anchor` so that point lands at local origin before the model is applied. `anchor = (0.5, 0.5)` centers the sprite at its position.
  - **Per-batch allocation** (instance buffer + bind group per draw call) is acceptable for M0.9. Persistent per-frame buffers + arena allocator are post-M0 hardening if needed.
- **Issues filed:** none

---

## M0.8 — Container + scene graph storage
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/Cargo.toml` — re-added `slotmap = "1"`
  - `crates/wisp/src/scene/container.rs` — `Container { transform, alpha, visible, blend_mode, children, parent }`. `new()`/`default()`, `children()` iterator, `child_count`, `parent` accessors. (`filters`/`clip` deferred to M0.16/M0.12 per YAGNI.)
  - `crates/wisp/src/scene/node.rs` — `NodeId` (slotmap key via `new_key_type!`), `Node` enum (just `Container` for now, grows with M0.9/M0.12/M0.15/M0.19), `Node::container()` / `container_mut()` uniform accessors, `From<Container> for Node`
  - `crates/wisp/src/scene.rs` — `Stage` struct: `SlotMap<NodeId, Node>` + root id; `new`, `root`, `len`, `is_empty`, `get`/`get_mut`, `add_child`, `detach`, `destroy` (cascading), `traverse_pre_order`. 7 unit tests.
  - `crates/wisp/src/application.rs` — `Application` now owns a `Stage`; `stage()` / `stage_mut()` accessors
  - `crates/wisp/src/lib.rs` — re-exports `Container`, `Node`, `NodeId`, `Stage`
- **Verified:**
  - `just gate` — passes (46 tests, was 37)
  - `just security` — passes (slotmap MIT/Apache, machete clean)
- **Notes:**
  - **Recursive-fix loop fired 4 iterations:**
    1. `fmt` — rustfmt collapsed long expressions to single lines (`fmt-fix`)
    2. `clippy` — `unused_mut` on a `let mut other = Stage::new()` that wasn't mutated
    3. `test` — `add_child_to_nonexistent_parent_returns_none` made a wrong assumption: NodeIds from two fresh `Stage`s can collide (both have a slot-0/gen-1 root). Replaced with `add_child_to_destroyed_parent_returns_none`, which exercises the actual slotmap stale-key guard
    4. green
  - **Stage owns the SlotMap.** Application owns the Stage. Tree relationships: each Container holds `children: Vec<NodeId>` and `parent: Option<NodeId>`; `Stage::add_child` keeps both sides in sync.
  - **`destroy` cascades to descendants** — collects subtree first then removes. `destroy(root)` is rejected (would leave the stage in an invalid state).
  - **Pre-order traversal**: stack-based with reverse-push so children pop in insertion order. Tested against a 3-deep + sibling tree and produced the expected `[root, a, b, c, d]` sequence.
  - **`filters` / `clip` fields deferred:** `Filter` trait is M0.16; `ClipMask` is M0.12. Adding the fields now would have nothing to put in them — YAGNI.
  - **`Node` is a tagged union** (not `Box<dyn>`). M0.9 adds `Sprite(Sprite)`, M0.12 adds `Graphics`, etc. Cache-friendly, no dyn dispatch in the render loop.
- **Issues filed:** none

---

## M0.7 — Transform with parent-child propagation
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/scene/transform.rs` — `Transform { position, scale, rotation, pivot, skew }`, `IDENTITY` const, `from_position`/`from_scale`/`from_rotation` constructors, `to_mat3` (Pixi-order: `T·S·R·K·T(-pivot)`), `transform_point`. Free function `compose(world_parent, &local) -> Mat3` for parent→child math.
  - `crates/wisp/src/scene.rs` — `pub use transform::Transform;`
  - `crates/wisp/src/lib.rs` — re-export `Transform`
- **Verified:**
  - `just gate` — passes (37 tests, was 24)
  - `cargo fmt` auto-applied (rustfmt collapsed multi-line asserts)
  - 8 unit tests + 4 property tests (proptest, 64 cases each) for: identity, default, translation, scale, rotation quarter/full turn, pivot under rotation, compose with identity parent, compose adds translations
- **Notes:**
  - **Property test coverage:** translation-adds, scale-multiplies, compose-associates-with-application, rotation-inverse-round-trips. All 4 properties green across 64 generated cases each.
  - **WorldTransform cache deferred to M0.8** — that lives with whatever owns children, which is `Container` (next chunk). M0.7 is just the math layer.
  - **Composition order matches Pixi:** `M = T(position) · S(scale) · R(rotation) · K(skew) · T(-pivot)`. Pivot is the local-space anchor that maps to `position` in parent space.
  - **Recursive-fix loop fired once:** rustfmt wanted multi-line asserts collapsed to single lines. Iteration 1 = `just fmt-fix`; gate green on iteration 2.
  - **Why `compose` is a free function**, not a method: it operates on a parent `Mat3` (no `Transform` available — the parent's world transform may already be cached). Method form would force constructing a temporary `Transform`. Free function is more honest.
- **Issues filed:** none

---

## M0.6 — Textured quad pipeline
- **Date:** 2026-05-09
- **Status:** ✅ done (visual confirmation pending user run)
- **Files changed:**
  - `crates/wisp/Cargo.toml` — re-added `image = "0.25"` (PNG+JPEG features) for `Texture::from_image`
  - `crates/wisp/shaders/quad.wgsl` — new shader: 6-vert quad, model+tint uniforms, texture sample
  - `crates/wisp/src/texture.rs` — promoted to module file; `Texture` struct with `from_rgba`, `from_image`, `width`, `height`, internal `view`/`sampler` accessors. Cheaply cloneable via `Arc<TextureInner>`.
  - `crates/wisp/src/render/quad_pipeline.rs` — `QuadPipeline` (private to crate), bind-group layouts for uniforms + texture/sampler, `BlendState::ALPHA_BLENDING`
  - `crates/wisp/src/render.rs` — added `Renderer::render_quad`; refactored to share clear+pass via `with_clearing_pass` helper (associated fn, not method — clippy `unused_self`)
  - `crates/wisp/src/lib.rs` — re-export `Texture`
  - `crates/wisp/examples/hello_quad.rs` — winit window rendering a 64×64 procedural checker pattern at 50% NDC scale
  - `crates/wisp/tests/render_quad.rs` — **first integration test file**: 4 tests covering dimensions, image round-trip, byte-length panic guard, and offscreen-target wgpu validation via `device.poll(Maintain::Wait)`
- **Verified:**
  - `just gate` — passes (24 tests, was 20)
  - `just security` — passes (advisories/bans/licenses/sources ok; machete clean with `image` now in use)
  - **Manual check pending:** `cargo run -p wisp --example hello_quad` — should show a grey-checker square at 50% scale on a dark-purple background. Esc or close to exit.
- **Notes:**
  - **Integration testing pattern established.** `tests/render_quad.rs` is the first member of the integration-test layer (TESTING.md layer 2). Pattern: boot `Application`, build offscreen `wgpu::Texture`, call renderer entry, `device.poll(Maintain::Wait)` to surface validation errors. Pixel-readback comes in M0.11 with `RenderTexture::read_pixels`.
  - **wgpu 24 API surprise:** `ImageCopyTexture` / `ImageDataLayout` are renamed to `TexelCopyTextureInfo` / `TexelCopyBufferLayout`. Used the new names successfully.
  - **Recursive-fix loop:** clippy fired `unused_self` on `with_clearing_pass` (the closure carries `&self.triangle` / `&self.quad`, so the helper itself doesn't need self). Iteration 1: changed to associated function (`Self::with_clearing_pass`). Gate green on iteration 2.
  - **Per-draw allocation:** `QuadPipeline::draw` allocates a uniform buffer + 2 bind groups per call. Acceptable for M0.6 (1 quad per frame). M0.9 batcher will share buffers.
  - **`Renderer::render` retained** as M0.5 triangle entry alongside new `render_quad`. M0.7+ replaces both with scene-graph traversal.
  - **License allowance hint:** `cargo deny` notes `Unicode-DFS-2016` in our allow list is unused. Non-blocking; keep until we know we don't need it for future deps.
- **Issues filed:** none

---

## Side quest — Testing spine + recursive-fix loop
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `_docs/TESTING.md` — new doc: brutally honest assessment of lead engineer's testing recommendation, our adapted 5-layer pyramid, per-chunk testing minimum, recursive-fix loop, folder layout, coverage policy
  - `CLAUDE.md` — promoted ⚠️ banner from "check → update" to "test → check → update → loop"; added recursive-fix loop block; added test-first hard rule; added `_docs/TESTING.md` to critical-load list
  - `_docs/WORKFLOW.md` — § 4 rewritten with explicit recursive-fix loop pseudocode; § 3 step 6 added: "Add at least one test"
  - `crates/wisp/Cargo.toml` — added `rstest`, `insta`, `proptest` as dev-dependencies
  - `crates/wisp/src/color.rs` — refactored two test groups to `#[rstest]` table-driven form (rgba_u8_matches_constants × 6 cases, premultiplied_cases × 3 cases); test count went from 15 → 20
- **Verified:**
  - `just gate` — passes (20 tests, was 15)
  - `just security` — passes (advisories, bans, licenses, sources all ok; machete clean with new dev-deps)
- **Notes:**
  - **Brutally honest verdict:** ~half the lead engineer's recommendation was for AI-runtime products (CLI tools, agent frameworks, services with HTTP). Skipped: `assert_cmd` (no CLI), `wiremock` (no HTTP), `testcontainers` (no DB), AI eval tests (we use AI to write code, not as runtime), `cargo-fuzz` (no parsers handling untrusted input — yet), 80% coverage threshold (premature; ratchet later).
  - **Locked in:** `cargo-nextest` (already installed), `insta`, `rstest`, `proptest`, `cargo-llvm-cov`, the recursive-fix loop convention.
  - **Anti-regression gravity rule:** every meaningful chunk ships with at least one test (unit/integration/snapshot/property/regression). Pure scaffolding chunks (M0.2-style stubs) are exempt.
  - **The recursive-fix loop:** explicit. When `just gate` fails, you must loop until green — never `#[allow]`/`#[ignore]`/comment-out/bypass. Documented in CLAUDE.md, WORKFLOW.md § 4, and TESTING.md.
- **Issues filed:** none

---

## Side quest — QA toolchain + Justfile
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `Justfile` — 28 recipes across 4 tiers (`gate`/`pr`/`release`/`full`) plus individual tools
  - `rustfmt.toml` — `edition = "2024"`, `max_width = 100`
  - `deny.toml` — license allowlist + advisories + bans + sources policy; one documented exemption (RUSTSEC-2024-0436)
  - `_docs/QA.md` — full QA tier reference, tool-specific notes, bootstrap instructions
  - `_docs/WORKFLOW.md` — § 4 rewritten: every code drop runs `just gate` until green
  - `_docs/CONVENTIONS.md` — added § QA toolchain
  - `CLAUDE.md` — workflow + hard rules updated to reference `just gate`
  - `_docs/ISSUES.md` — filed ISS-01 (paste unmaintained, transitive via wgpu)
  - `crates/wisp/Cargo.toml` — removed `slotmap`, `image`, `fontdue` (caught by machete; will be added back in their respective milestones)
- **Verified:**
  - `just gate` — passes (fmt, check, lint, nextest 15 tests, doctest 0 tests)
  - `just security` — passes (deny + unused-deps)
- **Notes:**
  - **Tools installed:** `just` (1.43.1) and `cargo-nextest` (0.9.114) via Homebrew; `cargo-deny`, `cargo-audit`, `cargo-machete` via cargo install.
  - **`cargo-audit` excluded from `security` chain** — collides with cargo-deny on `~/.cargo/advisory-db`, and cargo-deny already runs RustSec checks. Available standalone via `just audit`.
  - **YAGNI enforced by tooling:** `cargo machete` failed because we'd pre-added `slotmap`, `image`, `fontdue` ahead of need. Removed them; they'll come back in M0.8/M0.10/M0.15. This is exactly the convention working as designed.
  - **`paste` advisory:** transitive via wgpu's metal backend; documented exemption in `deny.toml` + ISS-01 to track upstream fix.
  - Tier 2/3/4 tools (llvm-cov, semver-checks, public-api, msrv, bloat, geiger, mutants, miri) not auto-installed — see `just bootstrap` output for install commands.
- **Issues filed:** ISS-01

---

## M0.5 — Hello triangle
- **Date:** 2026-05-09
- **Status:** ✅ done (visual confirmation pending user run)
- **Files changed:**
  - `crates/wisp/shaders/triangle.wgsl` — created; vertex_index-based equilateral triangle, RGB-tinted per vertex
  - `crates/wisp/src/render/triangle_pipeline.rs` — `TrianglePipeline` (private to crate), wraps the WGSL shader + `wgpu::RenderPipeline`
  - `crates/wisp/src/render.rs` — promoted to module file with `Renderer` struct: takes a `TextureView`, clears, draws triangle
  - `crates/wisp/Cargo.toml` — added `winit = "0.30"` as dev-dependency
  - `crates/wisp/examples/hello_triangle.rs` — winit 0.30 `ApplicationHandler`, surface configured to first sRGB format, render-on-redraw
- **Verified:**
  - `cargo build -p wisp --examples` — passes
  - `cargo fmt --all --check` — passes
  - `cargo clippy --workspace --all-targets -- -D warnings` — passes (after merging `CloseRequested | KeyboardInput` arms)
  - `cargo test --workspace` — passes (15 tests; no new tests since the verification is the example)
  - **Manual check pending:** `cargo run -p wisp --example hello_triangle` should show an RGB-vertex triangle on a black background. Esc or close to exit.
- **Notes:**
  - Renderer constructed per output format. Surface picks first sRGB format; falls back to first format if none srgb.
  - `Renderer::render` body is the M0.5 smoke path; M0.6 will introduce the textured-quad pipeline as a sibling and the body grows to traverse a scene graph by M0.7+.
  - WGSL uses `let positions = array<...>(...)` rather than `var` for the const arrays — naga validates fine.
- **Issues filed:** none

---

## M0.4 — `Application` + wgpu device init
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/application.rs` — `AppConfig` (defaults to 1280x720, HighPerformance), `Application` with `Instance`/`Adapter`/`Device`/`Queue`, async `new`, getters for each
  - `crates/wisp/src/error.rs` — added `AdapterUnavailable(String)` and `DeviceRequest(String)` variants
  - `crates/wisp/Cargo.toml` — added `pollster` and `tracing-subscriber` as dev-dependencies
  - `crates/wisp/examples/adapter_info.rs` — first wisp example: prints adapter name, backend, device type
- **Verified:**
  - `cargo build -p wisp --examples` — passes
  - `cargo run -p wisp --example adapter_info` — prints `Apple M1 / Metal / IntegratedGpu` on this machine
  - `cargo fmt --all --check` — passes (after auto-fix)
  - `cargo clippy --workspace --all-targets -- -D warnings` — passes
  - `cargo test --workspace` — passes (15 tests)
- **Notes:**
  - **wgpu 24 surprises:** `request_adapter` returns `Option<Adapter>` (not `Result`); `request_device` takes `(descriptor, trace_path)` (not just descriptor). Both fixed during compile.
  - `Application::new` is async; examples call `pollster::block_on` to drive it.
  - `tracing` `info!` fires on adapter selection — visible when subscriber is initialized.
- **Issues filed:** none

---

## M0.3 — Math, color, blend primitives
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/src/color.rs` — `Color` (RGBA f32, `Pod`/`Zeroable`, `repr(C)`); 6 named constants; `rgb`/`rgba`/`rgb_u8`/`rgba_u8`/`with_alpha`/`premultiplied`
  - `crates/wisp/src/blend.rs` — `BlendMode` enum (`Normal` default, `Multiply`/`Add`/`Screen` declared)
  - `crates/wisp/src/math.rs` — re-exports `glam::{Mat3,Mat4,Vec2,Vec3,Vec4}` + `Rect`
  - `crates/wisp/src/math/rect.rs` — `Rect { min, size }` with constructors and queries
  - `crates/wisp/src/lib.rs` — re-exports `Color`, `BlendMode`, `Rect`, `Vec2`, `Vec3`, `Vec4`, `Mat3`, `Mat4`
- **Verified:**
  - `cargo fmt --all --check` — passes
  - `cargo clippy --workspace --all-targets -- -D warnings` — passes
  - `cargo test --workspace` — passes (15 tests)
- **Notes:**
  - `Pod`/`Zeroable` derive on `Color` confirmed working with bytemuck `derive` feature.
  - `Vec2::new` is const in glam 0.29 → `Rect::new` is const-fn.
  - Tests use `(a - b).abs() < f32::EPSILON` for f32 comparisons to satisfy clippy.
- **Issues filed:** none

---

## M0.2 — Scaffold `wisp` crate
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `crates/wisp/Cargo.toml` — created with deps (wgpu 24, glam 0.29, slotmap, bytemuck, image, fontdue, thiserror, tracing)
  - `crates/wisp/src/lib.rs` — module declarations, `Error` re-export, `Result<T>` alias
  - `crates/wisp/src/error.rs` — `Error` enum with placeholder `NotImplemented` variant
  - `crates/wisp/src/{application,blend,color,texture,filter,scene,math,render}.rs` — module stubs (parent.rs pattern)
  - `crates/wisp/src/scene/{container,sprite,graphics,text,mesh,transform,clip}.rs` — node stubs
  - `crates/wisp/src/texture/{video_texture,render_texture}.rs` — texture stubs
  - `crates/wisp/src/filter/{blur,drop_shadow,motion_blur,color_matrix}.rs` — filter stubs
  - `crates/wisp/src/render/{batcher,pipeline,pass}.rs` — internal stubs
  - `crates/wisp/src/math/rect.rs` — Rect stub
- **Verified:**
  - `cargo build -p wisp` — passes (with one transitive future-incompat warning on `block v0.1.6` via `metal`)
  - `cargo fmt --all --check` — passes
  - `cargo clippy --workspace --all-targets -- -D warnings` — passes after fixing doc_markdown + module_inception
  - `cargo test --workspace` — passes (0 tests)
- **Notes:**
  - **Decision:** switched from `mod.rs` to Rust 2018+ `parent.rs + parent/` pattern. Eliminates `clippy::module_inception` triggered by `texture/texture.rs`. CONVENTIONS.md updated to bless this pattern.
  - Future-incompat warning on `block v0.1.6` is upstream in `metal` (wgpu's macOS backend); not actionable on our side.
- **Issues filed:** none

---

## M0.1 — Convert to Cargo workspace
- **Date:** 2026-05-09
- **Status:** ✅ done
- **Files changed:**
  - `Cargo.toml` — converted to `[workspace]`, resolver=3, members=`["crates/*"]`, shared `[workspace.package]` and `[workspace.lints]` (clippy::pedantic)
  - `crates/app/Cargo.toml` — created, `name = "screen-app"`
  - `crates/app/src/main.rs` — moved from `src/main.rs`
  - `src/` — removed (empty after move)
- **Verified:**
  - `cargo build` — passes
  - `cargo run -p screen-app` — prints "Hello, world!"
- **Notes:**
  - `screen-app` is currently a placeholder hello-world; M1 fills it in
  - Workspace lints inherit to all members via `[lints] workspace = true`

---

## Template for new entries

Copy this block to the top of the log. Replace placeholders. Keep entries terse.

```
## M<X>.<Y> — <chunk title from milestone doc>
- **Date:** YYYY-MM-DD
- **Status:** ✅ done | 🚧 partial | ❌ blocked
- **Files changed:**
  - `path/to/file` — brief reason
- **Verified:**
  - `just gate` — passes (fmt, check, lint, nextest, doctest)
  - `cargo run -p <crate> --example <name>` — visual confirm OK (if applicable)
  - `just storybook` — story renders correctly (if chunk is renderable)
  - `just security` — passes (if dep tree changed)
- **Notes:**
  - any non-obvious decision or surprise
- **Issues filed:** ISS-NN (if any)
```
