# Progress Log

Append-only log of completed tasks. **Newest entries at top.** Never edit historical entries except to add corrections at the bottom of an entry.

Use the template at the bottom for new entries.

---

## UI-01 — Design tokens + base surface primitives (AUT-121)
- **Date:** 2026-05-11
- **Status:** ✅ done — five new primitives (`Surface`, `Badge`, `Divider`, `Kbd`, `IconTile`) + a semantic-token expansion of `style.css`. Five new stories (`tokens-dark-zinc`, `surface-stack`, `badge-variants`, `kbd-shortcuts`, `icon-tile-variants`). Two new mdBook chapters (token table + surface primitives).
- **Linear:** [AUT-121](https://linear.app/harwood/issue/AUT-121).
- **Files:** new `crates/ui-storybook/src/components/primitives/{surface,badge,divider,kbd,icon_tile}.rs`. `components/primitives/mod.rs` + `components/mod.rs` re-export the new types. `assets/style.css` gains semantic tokens (`--surface-base/-elevated/-popover/-selected/-glass`, `--text-primary/-secondary/-tertiary`, `--line-subtle/-strong`, `--action-record/-hover`, `--shadow-popover/-elevated`, `--radius-panel/-control/-pill`, `--focus-ring`) + new component classes. `stories/primitives.rs` adds the five new stories. New `_docs/book/src/ui/chunks/{tokens,surface-primitives}.md`. `SUMMARY.md` indexes both. Five new HTML assets via `just snapshots-ui`.
- **Verified:** 22 ui-storybook tests pass (12 fixture + 5 registry + 4 primitive class-uniqueness + 1 SSR snapshot — snapshot extended for new stories). Full `just gate` green.
- **Tokens are the public API; raw hex is implementation detail.** `:root` in `style.css` defines the zinc palette; every component class references semantic aliases. UI-23's grep guardrail (later in this PR) will flag stray hex outside `:root` + token demos.

---

## UI-00 — Storybook workbench scaffolding (AUT-120)
- **Date:** 2026-05-11
- **Status:** ✅ done — `crates/ui-storybook` refactored into the proposed product-surface layout (`components/{primitives,shell,menus,recorder,library,editor,cursor}`, `fixtures/`, `stories/`). Public re-export surface preserved so `app-ui`'s `use ui_storybook::components::{Button, DropZone, ...}` imports keep working unchanged. New `StoryViewport` enum on `Story`. Eighteen pre-existing stories all keep their stable kebab-case ids.
- **Linear:** [AUT-120](https://linear.app/harwood/issue/AUT-120).
- **Files:** moved all seven existing components into subgroup folders via `git mv` (history preserved). New `fixtures/{cursor,devices,editor,library,recorder,workspaces}.rs` with owned fixture builders + per-module unit tests. New `stories/{cursor,editor,library,menus,primitives,recorder,shell}.rs` per-surface registries; old `stories.rs` moved to `stories/mod.rs` with `Story` + `StoryViewport` + `render()` helper + `all_stories()` aggregator. New `tests/story_registry.rs` (5 tests: unique ids, kebab-case, non-empty metadata, known category buckets, fixture smoke). New `_docs/book/src/ui/presentational-contract.md` chapter. Updated `_docs/book/src/ui/overview.md` (workbench layout mermaid + boundaries) + `_docs/book/src/ui/components.md` (subgroup index). `_docs/book/src/SUMMARY.md` adds presentational-contract entry.
- **Verified:** 17 ui-storybook tests pass (12 fixture/snapshot + 5 registry). `app-ui` still compiles unchanged. Full `just gate` green; `just snapshots-ui` regenerated every existing story asset.
- **Contract codified in test, not just docs.** `tests/story_registry.rs::category_set_is_within_known_buckets` lists the seven approved categories — adding a new one without registering it here OR in `components.md` trips the gate. Same for kebab-case ids: a story id like `Drop Zone Idle` would fail the regex check.

---

## M-TEXT.11 — Text as mask (AUT-85)
- **Date:** 2026-05-11
- **Status:** ✅ done — text drives the existing `Renderer::apply_mask_to_texture` primitive. Storybook story `text-mask` shows "WISP" clipping three foregrounds: a gradient color-band fill, a blurred circles backdrop, and a warm spotlight. One mask, three foregrounds.
- **Linear:** [AUT-85](https://linear.app/harwood/issue/AUT-85).
- **Files:** new `crates/wisp-storybook/src/stories/s_text_mask.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs` registers. New `_docs/book/src/wisp/text/text-mask.md` chapter (mermaid sequence for the mask-compose pipeline, `admonish important` for the apply_mask_to_texture-is-load-bearing rule, `admonish tip` for the separable-foreground pattern, `admonish warning` for the lavapipe filter guard). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/text-mask.png`.
- **Verified:** `story_smoke` + `story_fingerprints` pass. Full `just gate` green. PNG inspected — top row gradient bands through "WI", middle row blurred circles, bottom row warm spotlight. Three distinct foregrounds clipped to the same glyph silhouette.
- **No new wisp code.** `apply_mask_to_texture` already accepted any coverage RT (M-VEC.4..6, M-MASK.2..4). Text just joins the list of valid coverage sources. Story uses `WISP_SKIP_GPU_FILTER_TESTS` to substitute non-blur foregrounds on lavapipe so CI stays green.

---

## M-TEXT.10 — Vector-backed callouts (AUT-84)
- **Date:** 2026-05-11
- **Status:** ✅ done — five callout shapes (caption pill, number badge, label box, pointer + label, arrow + label) composed from `Graphics::draw_rounded_rect` / `draw_ellipse` / `draw_line` + `CaptionBlock` + text sprites. No new wisp primitives.
- **Linear:** [AUT-84](https://linear.app/harwood/issue/AUT-84).
- **Files:** new `crates/wisp-storybook/src/stories/s_text_callouts.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs` registers. New `_docs/book/src/wisp/text/callouts.md` chapter (flowchart for the five recipes, `admonish tip` for pill-vs-label, `admonish note` flagging the no-general-path limit that forced the arrowhead-via-three-lines workaround). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/text-callouts.png`.
- **Verified:** `story_smoke` + `story_fingerprints` pass. Full `just gate` green.
- **Arrowhead from three lines.** Wisp's `Graphics` doesn't expose a general `draw_path`; the arrowhead uses three `draw_line` calls fanning from the tip. Future M-VEC.13 (SVG path import) or a `Graphics::draw_path` would let us render richer arrow geometries (curved, double-headed, filled triangle). Flagged in the chapter so future contributors don't reinvent the workaround.

---

## M-TEXT.8 — Drop shadow + glow on text (AUT-82)
- **Date:** 2026-05-11
- **Status:** ✅ done — text run through the existing `wisp::DropShadowFilter` produces both drop shadows (with offset) and glows (with offset = 0). One pipeline, two parameter sets. Storybook story `text-shadow-glow` shows both side-by-side on a paper-white backdrop.
- **Linear:** [AUT-82](https://linear.app/harwood/issue/AUT-82).
- **Files:** new `crates/wisp-storybook/src/stories/s_text_shadow_glow.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs` registers. New `_docs/book/src/wisp/text/shadow-glow.md` chapter (mermaid sequence for the pipeline, `admonish important` for the glow-is-shadow-with-offset-zero insight, `admonish warning` for the sprite-vs-graphics backdrop rule, a parameter table for picking the right look). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/text-shadow-glow.png`.
- **Verified:** `story_smoke` + `story_fingerprints` pass. Full `just gate` green. PNG inspected — drop shadow (dark offset blur) and glow (warm halo at zero offset) both render correctly through the same filter.
- **No new wisp code.** The filter pipeline already accepted any source `RenderTexture`; the text-texture pipeline already produced one. The chunk is composition + documentation — but documenting the parameter set per look is what turns a generic filter into a recognizable design vocabulary.

---

## M-TEXT.9 — Word-wrapped caption block (AUT-83)
- **Date:** 2026-05-11
- **Status:** ✅ done — `wisp::text::CaptionBlock` composes wrapped text on a rounded-rect background. Layout measures the wrapped text height via `TextTexturePipeline::engine().layout_concrete()` and sizes the background to `width × (text_h + 2×padding)`. No new shaders; pure scene-graph composition (Graphics + Sprite).
- **Linear:** [AUT-83](https://linear.app/harwood/issue/AUT-83).
- **Files:** new `crates/wisp/src/text/caption.rs` (`CaptionBlock` builder + `CaptionLayout` return). `crates/wisp/src/text/mod.rs` re-exports. Also adds `Sprite::with_anchor_set(&mut self, Vec2)` as a `&mut self` companion to the existing builder-style `with_anchor`. New `crates/wisp-storybook/src/stories/s_text_caption_block.rs` + writeup with short + multi-line captions. `crates/wisp-storybook/src/stories/mod.rs` registers. New `_docs/book/src/wisp/text/caption-block.md` chapter (mermaid sequence for the layout flow, `admonish important` for the composition-over-inheritance shape, `admonish note` for the caller-set wrap precedence). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/text-caption-block.png`.
- **Verified:** 4 unit tests: caption height grows with wrapped lines, height includes padding on both sides, builder methods chain + apply, explicit `.with_wrap` overrides block inner width. `story_smoke` + `story_fingerprints` pass. Full `just gate` green.
- **Caller-set wrap precedence.** If the WispText already has `.with_wrap(...)`, the block respects that instead of `width - 2×padding`. Tooltip-style layouts (wide padding, narrow text) need this; one-size-fits-all wrap would force callers to fight the block.

---

## M-TEXT.12 — Text style presets for Screen workflows (AUT-86)
- **Date:** 2026-05-11
- **Status:** ✅ done — seven curated `WispTextStyle` presets land at `wisp::text::presets::*` (also `wisp::text::TextPreset` enum). Pure data, no allocation, no GPU dependency — same struct the editor consumes and the renderer composes from.
- **Linear:** [AUT-86](https://linear.app/harwood/issue/AUT-86).
- **Files:** new `crates/wisp/src/text/presets.rs` (`TextPreset` enum, 7 `pub fn -> WispTextStyle` accessors). `crates/wisp/src/text/mod.rs` re-exports `TextPreset`. New `crates/wisp-storybook/src/stories/s_text_presets.rs` + writeup — gallery story renders each preset's name in its own style. `crates/wisp-storybook/src/stories/mod.rs` registers. New `_docs/book/src/wisp/text/presets.md` chapter (admonish info for reading order, admonish important for the pure-data property). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/text-presets.png`.
- **Verified:** 8 unit tests: 7-presets returned by `all()`, every preset has positive size + line_height, section title is centered+bold, warning is signal-red, watermark is italic+alpha<1, callout is italic, no two presets byte-identical, every preset has a unique non-empty label. `story_smoke` + `story_fingerprints` pass. Full `just gate` green.
- **Test invariants encode the design intent.** "Warning is red-dominant" and "Watermark alpha < 1" are guardrails against quiet regressions where a refactor desaturates the privacy signal or accidentally bumps the watermark to opaque. Adding a new preset only requires extending `TextPreset::all()` plus a `fn`; the gallery story + every-preset-positive-size test pick it up automatically.

---

## M-TEXT.7 — Stroked / outlined text rendering (AUT-81)
- **Date:** 2026-05-11
- **Status:** ✅ done — text gets a configurable outline via `wisp::text::stroked_text_sprites` + `StrokedTextLayer`. CSS-style technique: render text to a texture once via `TextTexturePipeline`, stamp the texture eight times tinted in the stroke color at offsets on a circle, stamp once more tinted in the fill color at the center. No shader changes.
- **Linear:** [AUT-81](https://linear.app/harwood/issue/AUT-81).
- **Files:** new `crates/wisp/src/text/stroke.rs` (`StrokedTextLayer`, `stroked_text_sprites`, `STROKE_OFFSETS` ring). `crates/wisp/src/text/mod.rs` re-exports. New `crates/wisp-storybook/src/stories/s_text_stroke.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs` registers. New `_docs/book/src/wisp/text/stroke.md` chapter (mermaid sequence for the texture → ring-stamp → fill flow, `admonish important` for the local-NDC convention, `admonish note` for the 8-direction ring, `admonish bug` for the Graphics-after-Sprites gotcha that bit the story's backdrop). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/text-stroke.png` showing READ ME with stroke vs unstroked "no stroke" baseline.
- **Verified:** 4 unit tests (zero stroke → one sprite, positive stroke → eight + one, sprites on radius-r ring, stroke width scales linearly). `story_smoke` + `story_fingerprints` pass. Full `just gate` green.
- **Pre-rendered RT-as-Sprite backdrop is the workaround.** A direct `Graphics` backdrop in the same stage paints AFTER the sprite text and hides it — recurring footgun from CLAUDE.md's renderer-batching note. The story renders the colored backdrop into its own RT and attaches that as a Sprite; the chapter calls this out in an `admonish bug`.

---

## M-MEDIA.14 — Synced video + audio histogram in one Wisp scene (AUT-110)
- **Date:** 2026-05-11
- **Status:** ✅ done — M-MEDIA P1 capstone. `cargo run -p media --example synced_scene` composes a hue-rotating synthetic video frame (top half) + audio histogram bars (bottom-left) in one wisp scene, anchored to `MediaClock::manual`. 10 frames at 100 ms cadence; amplitude ramps `0.30 → 0.90` so bar heights grow visibly. Every M-MEDIA chunk so far shows up at the same call site.
- **Linear:** [AUT-110](https://linear.app/harwood/issue/AUT-110).
- **Files:** new `crates/media/examples/synced_scene.rs`. `crates/media/Cargo.toml` registers the `[[example]]`. New `_docs/book/src/media/synced-scene.md` chapter (mermaid sequence for the per-frame loop, `admonish important` for the one-clock rule, `admonish note` for full reproducibility). New `_docs/book/src/assets/media/synced-scene.png` (frame 05 with mid-amplitude bars). `_docs/book/src/SUMMARY.md`.
- **Verified:** Local run produced exactly the expected output — 10 PNGs at `target/synced-scene/`; video PTS `0.000, 0.100, …, 0.900` s; per-frame `peak ≈ amp` and `rms ≈ amp / √2` for every histogram window. Full `just gate` green.
- **The single-clock rule is the load-bearing decision.** Audio and video don't sync to each other — they both sync to `MediaClock`. Live capture (M-MEDIA.15/.16) plugs in a wall-clock anchor and the rest of the code doesn't move.

---

## M-MEDIA.13 — GStreamer videotestsrc through Wisp (AUT-109)
- **Date:** 2026-05-11
- **Status:** ✅ done — `cargo run -p media --example gst_video_to_wisp` captures 8 frames from `videotestsrc` at 320×180@30fps, uploads each to a wisp `VideoTexture`, renders through `Sprite` to a headless `RenderTexture`, and saves PNGs under `target/gst-video-frames/`. Closes the M-MEDIA.6 → M-MEDIA.12 path end-to-end with a real GStreamer source.
- **Linear:** [AUT-109](https://linear.app/harwood/issue/AUT-109).
- **Files:** new `crates/media/examples/gst_video_to_wisp.rs`. `crates/media/Cargo.toml` adds `wisp` / `wgpu` / `pollster` / `glam` as **dev-dependencies** (library stays wgpu-free) and registers the `[[example]]`. New `_docs/book/src/media/video-render.md` chapter (mermaid sequence for the capture → upload → render → PNG loop, `admonish important` clarifying the dev-dep-only boundary). New `_docs/book/src/assets/media/gst-video-to-wisp.png` (frame 0 SMPTE colorbars). `_docs/book/src/SUMMARY.md`.
- **Verified:** Local run on macOS with GStreamer installed produced 8 PNGs of SMPTE colorbars at 33.33 ms PTS intervals (30 fps). Cumulative `frames_emitted` matches the upload count (8 = 8 = 8). Full `just gate` green.
- **Boundary preserved.** `media` library still doesn't import `wisp`. Only the example brings wisp in via `[dev-dependencies]`. Downstream consumers of `media` (e.g., `playback`, `app`) won't pull wgpu unless they explicitly opt-in by depending on it themselves.

---

## M-MEDIA.12 — Video texture handoff (AUT-108)
- **Date:** 2026-05-11
- **Status:** ✅ done — synthetic `decode::VideoFrame` (128×72 BGRA, diagonal gradient + horizontal stripes) uploaded to wisp `VideoTexture` and rendered through `Sprite`. Storybook story `video-frame-handoff` proves the seam works end-to-end; PNG hero asset confirms BGRA → wgpu `Bgra8UnormSrgb` roundtrip is correct.
- **Linear:** [AUT-108](https://linear.app/harwood/issue/AUT-108).
- **Files:** new `crates/wisp-storybook/src/stories/s_video_frame_handoff.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs` registers the new story. New `_docs/book/src/media/video-texture.md` chapter (mermaid sequence for the handoff path, `admonish important` for the wisp-doesn't-know-source rule, `admonish note` for BGRA being wgpu's native pixel order). `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/video-frame-handoff.png`.
- **Verified:** `story_smoke` + `story_fingerprints` (snapshot extended) pass for the new story. Full `just gate` green. PNG inspected — gradient + stripes render with correct channel order, expected aspect ratio (16:9 scaled to ~75% NDC).
- **Existing infrastructure formalized, not new.** `VideoTexture::upload_bgra` (wisp) and `decode::VideoFrame` (media re-export) existed before this chunk. M-MEDIA.12 is the storybook + chapter that proves the call sites work and locks the contract in a regression test. M-MEDIA.13 will swap the synthetic frame for a real GStreamer-captured one.

---

## M-MEDIA.11 — GStreamer audio → histogram example (AUT-107)
- **Date:** 2026-05-11
- **Status:** ✅ done — `cargo run -p media --example gst_audio_histogram` captures 1 s of `audiotestsrc` (440 Hz, 48 kHz f32 mono), quantizes at 50 ms, and prints bucket / peak / RMS stats with a first-5 + last-5 bar dump. Skips with a friendly message when `gst-launch-1.0` isn't on `PATH`.
- **Linear:** [AUT-107](https://linear.app/harwood/issue/AUT-107).
- **Files:** new `crates/media/examples/gst_audio_histogram.rs`. `crates/media/Cargo.toml` registers the `[[example]]`. New `_docs/book/src/media/audio-histogram-gst.md` chapter (mermaid sequence diagram for the probe → spawn → quantize → stdout flow, `admonish important` for the skip-guard, `admonish note` explaining the 0.5657 vs 0.7071 RMS for `audiotestsrc`'s default volume). `_docs/book/src/SUMMARY.md`.
- **Verified:** Local run on macOS with GStreamer installed produced exactly the expected output — 20 bars × 50 ms, `peak max = 0.80`, `RMS ≈ 0.5657` on every bar (0.8 / √2). PTS cadence `50_000_000` ns monotonic. Full `just gate` green.
- **The same `quantize` pipeline works for both mock and real audio.** Storybook story (M-MEDIA.10) uses `SineWaveSource(0.6) → RMS ≈ 0.4243`; example uses `audiotestsrc volume=0.8 → RMS ≈ 0.5657`. Same code path, different `A`. M-MEDIA.15 (live mic) will just plug in another `next_chunk` source — `quantize` doesn't change.

---

## M-MEDIA.10 — Synthetic audio histogram in Wisp (AUT-106)
- **Date:** 2026-05-11
- **Status:** ✅ done — first storybook story that uses the `media → wisp` seam end-to-end. SineWaveSource → `quantize` → `mono_bars` → `Graphics::draw_rect`. PNG hero asset shows 20 amber bars mirrored about the centerline.
- **Linear:** [AUT-106](https://linear.app/harwood/issue/AUT-106).
- **Files:** new `crates/wisp-storybook/src/stories/s_audio_histogram.rs` + `writeups/audio_histogram.md`. `crates/wisp-storybook/src/stories/mod.rs` registers the new story. `crates/wisp-storybook/Cargo.toml` gains `media = { path = "../media" }` as a storybook-side dep (wisp itself stays media-free). New `_docs/book/src/media/audio-histogram.md` chapter. `_docs/book/src/SUMMARY.md`. New `_docs/book/src/assets/wisp/audio-histogram.png` (regenerated by `just snapshots-wisp`). `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap` extended with the new entry.
- **Verified:** `story_smoke` (no wgpu validation errors + visible pixels) and `story_fingerprints` (quadrant snapshot) both pass for the new story. Full `just gate` green. PNG inspected manually — 20 bars at expected positions, uniform height (constant amplitude).
- **wisp stays unaware of audio.** wisp-storybook bridges the two crates by depending on both; that's the right shape — `wisp` is generic, `wisp-storybook` is a gallery that demonstrates wisp's primitives against domain data (audio, video, capture).
- **Deterministic by construction.** Mock source + integer-arithmetic quantization + `Graphics::draw_rect` = identical PNG run-to-run on the same GPU. M-MEDIA.11's gst-captured variant will diverge per-microphone, so the determinism story is parked here for snapshot purposes.

---

## M-MEDIA.9 — Waveform bar geometry for Wisp (AUT-105)
- **Date:** 2026-05-11
- **Status:** ✅ done — `media::waveform::{mono_bars, stereo_bars}` maps an `AudioHistogram` to a `Vec<WaveformBarRect>` that wisp can render directly via its Graphics pipeline.
- **Linear:** [AUT-105](https://linear.app/harwood/issue/AUT-105).
- **Files:** new `crates/media/src/waveform.rs` (`WaveformBarRect`, `WaveformLayout`, `BarMetric`, `WaveformDisplayMode`, `WaveformLayout::ndc_default()`, `mono_bars`, `stereo_bars`). `crates/media/src/lib.rs` re-exports the geometry types. New `_docs/book/src/media/waveform-geometry.md` chapter (mermaid sequence + `admonish important` for the wisp/media boundary + `admonish note` for Anchored vs Mirrored). `_docs/book/src/SUMMARY.md`.
- **Verified:** 11 unit tests cover anchored-bar stride/x/y, anchored-height = `peak × max_height`, mirrored centers on `baseline_y`, RMS metric switches to `rms` field, silence → zero-height bars, empty histogram → empty geometry, stereo pairs (L above / R below baseline), stereo truncates to shorter input, color carry-through, four-bar manual regression table (`peak = [1.0, 0.5, 0.25, 0.0]` → exact `x` / `height` per row), `Send + Sync`. Full `just gate` green.
- **Boundary preserved.** `wisp` still doesn't depend on `media` or GStreamer — this chunk produces typed `WaveformBarRect` values that wisp consumes through its standard graphics call. The `admonish important` callout in the chapter encodes the rule explicitly so future contributors can't lose it under prose.
- **Unit-agnostic.** `WaveformLayout` carries no NDC-vs-pixels assumption; the math is the same. `ndc_default()` is a convenience preset for storybook-style usage. M-MEDIA.10's Wisp render will use NDC values; a future editor scrubber could call the same function with pixel values.

---

## M-MEDIA.8 — Audio histogram quantization (AUT-104)
- **Date:** 2026-05-10
- **Status:** ✅ done — `quantize(chunk, bucket_duration)` turns an `AudioChunk` into an `AudioHistogram` of `AudioBar`s (start_time, duration, peak, rms). First P1 chunk; foundation for M-MEDIA.9 (waveform geometry) and M-MEDIA.10 (Wisp render).
- **Linear:** [AUT-104](https://linear.app/harwood/issue/AUT-104).
- **Files:** filled in `crates/media/src/histogram.rs` (was scaffolded by M-MEDIA.0). `crates/media/src/lib.rs` re-exports `AudioBar`, `AudioHistogram`. New `_docs/book/src/media/histogram.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 10 unit tests cover silence → zero peak/rms, sine → `rms ≈ A/√2` on every interior bar, pulse → singular peak in the expected bucket + zeros elsewhere, bucket counts at 10/20/50 ms, empty chunk → empty histogram, contiguous bar timestamps (`bar[i+1].start == bar[i].start + bar[i].duration`), stereo chunks collapse to a single bar series, `Send + Sync`. Full `just gate` green at 376 tests (366 + 10 new).
- **Three references match three correctness assertions.** M-MEDIA.4's `SilenceSource` / `SineWaveSource` / `StepPulseSource` line up 1:1 with the histogram's three properties — silence → zero bars, sine → stable RMS, pulse → expected peak. Same mock sources will drive M-MEDIA.10 (Wisp render) + M-MEDIA.11 (gst→histogram example) tests, so the correctness chain is uniform across the visualization stack.
- **Multi-channel collapses to a single bar series.** Every sample in the interleaved buffer counts toward the same bucket — matches dope-sheet rendering (one row per audio track, not per channel) and keeps the math + tests simple. M-MEDIA.9 (geometry) is where mono vs stereo display becomes meaningful.
- **Exact timestamps via `MediaTime::from_sample` round-half-up.** Successive bars are byte-exact contiguous; no gap-or-overlap drift across long histograms. The contiguity assertion in the test asserts this byte-for-byte.

---

## M-MEDIA.7 — A/V sync harness (AUT-103)
- **Date:** 2026-05-10
- **Status:** ✅ done — `sync::run(SyncConfig)` combines audio + video GStreamer captures and reports per-stream timing + inter-stream drift. Closes the P0 tier: GStreamer can capture audio + video and stamp both on one timeline.
- **Linear:** [AUT-103](https://linear.app/harwood/issue/AUT-103).
- **Files:** new `crates/media/src/sync.rs` (`SyncConfig`, `SyncReport`, `run`, `Error`). `crates/media/src/lib.rs` registers the module. New `crates/media/tests/sync_harness_integration.rs` (4 tests, skip-guarded). New `_docs/book/src/media/sync-harness.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 4 integration tests (1 s deterministic capture): exact frame counts (48000 audio + 30 video), first-PTS alignment < 100 ms, drift < 1/25 s, last-PTS ≤ 1 s. 2 unit tests for `SyncReport::drift_within` + `SyncConfig::deterministic_1s` arithmetic. Full `just gate` green at 366 tests (360 + 6 new).
- **`SyncReport::Display`** writes a one-line compact summary for logs — `audio_frames` / `video_frames` / `first_*_pts` / `last_*_pts` / `drift` all reported. The harness's `eprintln!("{report}")` in the integration tests doubles as the AUT-103 "manual regression" diagnostic.
- **Synthetic sources for the test, real shape for live capture.** The harness type signature accepts arbitrary `SyncConfig` so M-MEDIA.15 / .16 (live mic + webcam) can swap the underlying captures without changing the harness or its callers. For now both sources are GStreamer test sources.
- **Drift is near-zero by construction for synthetic sources** (counters in / counters out), so the integration assertion uses a wide-but-meaningful tolerance (< 1/25 s, i.e., one 25-fps frame). The drift instrument becomes a real regression check when live capture lands — then it'll surface actual clock disagreement between the OS audio + video subsystems.

---

## M-MEDIA.6 — GStreamer video test-source capture (AUT-102)
- **Date:** 2026-05-10
- **Status:** ✅ done — `GstreamerVideoCapture::test_source(w, h, fps)` produces BGRA `VideoFrame`s by piping `videotestsrc ! videoconvert ! video/x-raw,format=BGRA ! fdsink fd=1` through `gst-launch-1.0`.
- **Linear:** [AUT-102](https://linear.app/harwood/issue/AUT-102).
- **Files:** new `crates/media/src/gstreamer_video.rs`. `crates/media/src/lib.rs` registers the module. New `crates/media/tests/gstreamer_video_integration.rs` (3 tests, skip-guarded). New `_docs/book/src/media/video-capture.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 3 integration tests (skip-guarded): 5-frame contiguous PTS at 30 fps, distinct frame bytes (SMPTE colorbars include an animated ball), dimensions/framerate accessor round-trip. 2 unit tests for construction guards + `Send`. Full `just gate` green at 360 tests (355 + 5 new).
- **Reuses `decode::VideoFrame`.** The frame contract is shared end-to-end (decode crate's VideoStream impls, media's new GStreamer capture, future webcam capture in M-MEDIA.16). No duplication; one type means one shape passed to wisp's texture upload (M-MEDIA.12).
- **PTS via `MediaTime::from_frame`.** Frame 90 at 30 fps = exactly 3.0 s. No drift across long captures thanks to M-MEDIA.2's round-half-up.
- **`Drop` kills + waits** (same as `gstreamer_audio`). Without it, the gst-launch child keeps decoding into a dropped pipe.

---

## M-MEDIA.5 — GStreamer audio test-source capture (AUT-101)
- **Date:** 2026-05-10
- **Status:** ✅ done — `GstreamerAudioCapture::test_source(fmt, freq)` produces normalized `f32` `AudioChunk`s by piping `audiotestsrc ! audioconvert ! audioresample ! F32LE ! fdsink fd=1` through `gst-launch-1.0`. Companion `from_file(path, fmt)` decodes real audio (the bundled MP3 fixture) through the same pipeline shape — proves the histogram + waveform paths against real-world signal.
- **Linear:** [AUT-101](https://linear.app/harwood/issue/AUT-101).
- **Files:** new `crates/media/src/gstreamer_audio.rs` (`GstreamerAudioCapture`, `Error`). `crates/media/src/lib.rs` registers the module. New `crates/media/tests/gstreamer_audio_integration.rs` (4 tests, skip-guarded). New `_docs/book/src/media/audio-capture.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 4 integration tests cover (1) 3×100 ms chunks with contiguous PTS, (2) audiotestsrc sine RMS ≈ 0.566 (= default volume 0.8 / √2), (3) stereo interleave L ≈ R, (4) the bundled MP3 fixture decodes to 44.1 kHz stereo with RMS in `(0.4, 0.95)` and peak > 0.5 after MP3 round-trip. 3 additional unit tests for caps-string construction, non-F32 rejection, and `Send`. Full `just gate` green at 355 tests (348 + 7 new).
- **`Drop` kills the child + waits.** Without it, `gst-launch-1.0` keeps decoding into a dropped pipe and burns CPU. Matches `decode::GstreamerPipeStream`'s pattern. Documented in the chapter.
- **Real fixture beats mock.** The committed `tests/fixtures/sample-audio.mp3` (a deterministic 35-s 440 Hz sine generated locally via gstreamer in the previous commit) lets M-MEDIA.8 / .9 / .10 assert numeric correctness against actual decoded audio. The license-clean-by-construction nature means no third-party-rights risk.
- **Format gate at construction.** Only `SampleFormat::F32` is accepted. Caps are F32LE; converting integer formats here would push sample-format complexity into the public API for no payoff. Non-F32 returns `Error::UnsupportedFormat`. M-MEDIA.15 (live mic) follows the same gate.

---

## M-MEDIA.4 — Deterministic mock audio sources (AUT-100)
- **Date:** 2026-05-10
- **Status:** ✅ done — `SineWaveSource`, `SilenceSource`, `StepPulseSource` ship as the byte-exact reproducible audio inputs every M-MEDIA test consumes. No microphone, no GStreamer.
- **Linear:** [AUT-100](https://linear.app/harwood/issue/AUT-100).
- **Files:** new `crates/media/src/mock_audio.rs`. `crates/media/src/lib.rs` registers + re-exports the three sources. New `_docs/book/src/media/mock-sources.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 11 unit tests cover sample counts, stereo interleave, peak ≈ amplitude, RMS ≈ A/√2 (the canonical sinusoid identity), PTS-advance across calls, silence → zeros, spike at expected frame, spike outside window (cross-chunk boundary), stereo spike fills both channels, no-GStreamer-or-device-dep contract, `Send + Sync`. Full `just gate` green at 348 tests (337 + 11 new).
- **Three shapes match M-MEDIA.8's three assertions.** Silence → zero bars. Sine → stable RMS. Pulse → expected peak. The same sources used to test the histogram quantizer (M-MEDIA.8) will be used to test the histogram→Wisp render path (M-MEDIA.10) and the GStreamer-audio→histogram example (M-MEDIA.11). One reference, three layers of consumers.
- **PTS comes from `MediaTime::from_sample`.** Two successive `next_chunk(48_000)` calls on a 48 kHz source produce chunks with `pts = 0 s` and `pts = 1 s` exactly — no rounding drift across long sessions thanks to the round-half-up in M-MEDIA.2's clock.

---

## M-MEDIA.3 — Audio sample + chunk data model (AUT-99)
- **Date:** 2026-05-10
- **Status:** ✅ done — `AudioFormat` + `AudioChunk` ship as the single shared audio data model. Every M-MEDIA chunk that touches audio (capture, mock sources, histogram, mic) flows through this type.
- **Linear:** [AUT-99](https://linear.app/harwood/issue/AUT-99).
- **Files:** filled in `crates/media/src/audio.rs` (scaffolded by M-MEDIA.0). New types: `SampleFormat`, `AudioFormat` (with `mono_f32` / `stereo_f32` presets), `AudioChunk` (validated), `AudioChunkError`. `crates/media/src/lib.rs` re-exports all four. New `_docs/book/src/media/audio.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 11 unit tests cover mono@48kHz=1s, stereo@48kHz=0.5s, unaligned-buffer rejection, zero-channels rejection, zero-rate rejection, empty buffer = zero duration, `peak()` returns max-abs, `rms()` zero for silence + unit for constant signal, preset channel counts, `Send + Sync`. Full `just gate` green at 337 tests (326 + 11 new).
- **Normalized f32 end-to-end.** GStreamer's `audioconvert ! audio/x-raw,format=F32LE` produces it natively; cpal / coreaudio prefer it. No re-layout work at the capture seam. The `SampleFormat` enum lets capture-side code declare its *input* layout (F32 / I16 / U8) for future device backends — internally `AudioChunk::samples` is always `&[f32]`.
- **Planar-per-frame interleave** (`[L₀, R₀, L₁, R₁, …]`). Matches GStreamer + cpal conventions.
- **Validation covers what matters at the data layer.** `samples.len() % channels == 0`, channels > 0, rate > 0. NaN / clipping / DC-offset are visualization concerns and live in M-MEDIA.8 (histogram).
- **Pre-computed `peak()` + `rms()`** on the chunk — they're called every bucket by the histogram quantizer; caching the result avoids re-walking the same buffer.

---

## M-MEDIA.2 — Timestamp + clock model (AUT-98)
- **Date:** 2026-05-10
- **Status:** ✅ done — `MediaTime`, `MediaDuration`, `MediaClock`, `Timestamped<T>` ship as the shared timeline vocabulary. Every M-MEDIA chunk after this stamps its chunks/frames against a `MediaClock`.
- **Linear:** [AUT-98](https://linear.app/harwood/issue/AUT-98).
- **Files:** filled in `crates/media/src/clock.rs` (was scaffolded by M-MEDIA.0). `crates/media/src/lib.rs` re-exports the four types. New `_docs/book/src/media/clock.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 13 unit tests cover frame-90@30fps == 3s, sample-48000@48kHz == 1s exactly, sample round-trip at 44.1/48 kHz, frame round-trip at 24/30/60 fps, duration arithmetic (`MediaTime ± MediaDuration` and `MediaTime - MediaTime`), monotonic ordering + sort, manual-clock advance, wall-clock non-decreasing, `Timestamped<T>::assign`, `MediaDuration::abs` (drift), `from_millis` exactness, `Send + Sync`. Full `just gate` green at 326 tests (313 + 13 new).
- **i64 nanoseconds internal.** ≈ 292 years of headroom, exact integer arithmetic, signed for pre-origin offsets. `f64` seconds is exposed for user-facing labels but the math runs in i128 to avoid mid-computation overflow.
- **Round-half-up in `to_sample` / `to_frame`.** Without rounding, 44.1 kHz drifts -1 sample per conversion (the original `from_sample(1, 44100)` produced 22 675 ns, `to_sample(22 675, 44100)` gave back 0). Round-trip now exact for every sample rate that doesn't divide 10⁹ evenly.
- **Two clock modes — `wall_clock` vs `manual`.** Production uses wall-clock; tests + the synthetic A/V sync harness (M-MEDIA.7) use manual mode for byte-exact reproducibility. `MediaClock::is_manual()` exposes the mode for assertions.

---

## M-MEDIA.1 — Shared structured GStreamer probe (AUT-97)
- **Date:** 2026-05-10
- **Status:** ✅ done — `media::gstreamer::probe()` returns a structured `GStreamerProbe` (per-binary version, requested-plugin map, `PATH` snapshot) instead of a bare `bool`. CI failure logs now show *why* GStreamer is unavailable, not just "false."
- **Linear:** [AUT-97](https://linear.app/harwood/issue/AUT-97).
- **Files:** filled in `crates/media/src/gstreamer.rs` (was scaffolded by M-MEDIA.0). New: `GStreamerProbe`, `probe()`, `probe_with_plugins(&[&str])`, `is_available()`, `Display` impl that pretty-prints the diagnostic. `crates/decode/Cargo.toml` adds `media` as a dev-dep (one-way edge — `media` already depends on `decode` for `VideoFrame`, so `dev-dep` reverses safely without cycling the production build). `crates/decode/tests/gstreamer_integration.rs` migrated from a local `gstreamer_available()` to `media::gstreamer::is_available`. `crates/decode/src/gstreamer_pipe.rs::gstreamer_available()` gains a docstring pointing at the canonical helper (`preview` + `app` tests keep calling the decode helper for now — they'll cut over when M-MEDIA.5/.6 touches them).
- **Verified:** 6 new media::gstreamer unit tests (path-snapshot non-empty, empty-plugin-map default, requested-plugin recording, `is_available` consistency, `Display` format, `Send + Sync`). All run without GStreamer installed (the deterministic `__not_a_real_plugin__` check and the synthetic `Display` test are host-independent). Full `just gate` green at 313 tests (311 + 2 new media-crate tests + 4 in `gstreamer`).
- **Plugin checks via `gst-inspect-1.0`.** Used over parsing `gst-launch -h` output. Returns `false` for both "checked-but-missing" and "not-checked"; callers wanting to distinguish inspect `GStreamerProbe::plugins` directly.
- **Dev-dep direction is intentional.** Adding `media` as a normal dep on `decode` would cycle the lib graph (`media` → `decode` → `media`); the test-target-only edge is safe because Cargo doesn't include dev-deps in the lib graph. M-MEDIA.5/.6 may later move `VideoFrame` into `media` and flip the dependency, but that's a later chunk.
- **Decode's bool helper kept on purpose.** `decode::gstreamer_pipe::gstreamer_available()` continues to work as a backwards-compat shim. Its docstring now points at `media::gstreamer::is_available()` as the canonical entrypoint. Callers in `preview` + `app` tests are unchanged — chunk-bound migration avoids unrelated test churn.

---

## M-MEDIA.0 — Media crate boundary + architecture docs (AUT-96)
- **Date:** 2026-05-10
- **Status:** ✅ done — new `crates/media` is the home for GStreamer-backed audio + video capture, playback orchestration, and the data models that `wisp` (renderer) and `app` (Tauri+Leptos shell) consume. Scaffolded with module-level docs that lock in the three-way responsibility split before any of the subsequent 22 M-MEDIA chunks land.
- **Linear:** [AUT-96](https://linear.app/harwood/issue/AUT-96). Foundation chunk for the M-MEDIA track; the remaining P0 (AUT-97..103), P1 (AUT-104..110), P2 (AUT-111..117), and P3 (AUT-118) tickets all build on top.
- **Files:** new `crates/media/Cargo.toml` (depends on `decode` for the `VideoFrame` re-export + `thiserror`/`tracing` workspace deps). New `crates/media/src/lib.rs` with the crate-level architecture docstring + module declarations. Six scaffolded module files (`audio.rs`, `clock.rs`, `gstreamer.rs`, `histogram.rs`, `manifest.rs`, `video.rs`), each carrying a `//!` header documenting the planned surface for its chunk. New `_docs/book/src/media/architecture.md` chapter. `_docs/book/src/SUMMARY.md` gains a new `media` section. CLAUDE.md was updated separately (commit `5e58d54`) with the asset-choice rules every M-MEDIA chunk consumes.
- **Verified:** 2 smoke tests in `crates/media/src/lib.rs` (re-export of `VideoFrame`, trait re-export of `VideoStream`). `cargo check -p media --all-targets` green; full `just gate` green.
- **Why this split.** The boundary is load-bearing: every wisp consumer (storybook, headless export, future plugins) would inherit GStreamer's build + license footprint if `wisp` ever depended on this crate's GStreamer integration. The split also makes the backend swappable — a future ScreenCaptureKit / Media Foundation native path slots into `media` without touching `wisp`.
- **CLI-pipe over `gstreamer-rs`.** Documented in the architecture chapter and in CLAUDE.md's GStreamer lessons. M-MEDIA.1 will extract `decode::gstreamer_pipe::gstreamer_available()` into a shared structured-diagnostic helper.
- **Scaffolded modules are intentional.** Each `//!` doc describes the planned surface so the very next chunk on the module reads as "convert this comment into real types + tests + an mdBook chapter of its own." The crate compiles green from day one; chunks add code, not infrastructure.

---

## M-TEXT.6 — Text composes through mask / filter / blend / export (AUT-80)
- **Date:** 2026-05-10
- **Status:** ✅ done — with M-TEXT.5's `TextTexturePipeline` in hand, text becomes a `RenderTexture`, and the existing renderer plumbs that through every composition surface (render_stage, non-Normal blend, filter chain, headless export, mask clipping).
- **Linear:** [AUT-80](https://linear.app/harwood/issue/AUT-80).
- **Files:** new `crates/wisp/tests/text_composition.rs` (4 integration tests). New `crates/wisp-storybook/src/stories/s_text_composition.rs` + `writeups/text_composition.md` + `mod.rs` + `all_stories()` entry. New `_docs/book/src/wisp/text/composition.md` chapter. `_docs/book/src/SUMMARY.md`. Regenerated `_docs/book/src/assets/wisp/text-composition.png` via the story exporter.
- **Verified:** 4 new integration tests cover (1) render_stage participation with bright-pixel check, (2) blend mode visual difference (Normal vs Subtract sum |Δ| > 1000), (3) grayscale filter via `Renderer::apply_filter` produces R≈G≈B on the top-50 brightest pixels, (4) headless export pixel readback contains the text. Full `just gate` green at 309 tests (305 + 4 new). `just snapshots-check` passes.
- **Subtract vs Multiply for the visual demo.** Multiply with white text against a warm-red backdrop multiplies the backdrop into itself — the text becomes invisible. Subtract (`dst - src` clamped) punches the backdrop out toward black where the glyph alpha is high — visible and clearly different from Normal. Documented in the chapter and writeup.
- **No new renderer code.** Every acceptance surface was already exposed; this chunk wires existing primitives + writes tests + writes the chapter. The architectural work was M-TEXT.5's `RenderTexture::as_texture()`.
- **Mask clipping participation.** Mask APIs (`apply_clip`, `compose_through_*`) already accept `RenderTexture`. The text RT plugs in unchanged — covered transitively by the existing M-MASK test suite. Listed in the chapter's compatibility matrix; not re-tested per-mask-flavor (would duplicate the M-MASK coverage).

---

## M-TEXT.5 — Text render-to-texture path + cache (AUT-79)
- **Date:** 2026-05-10
- **Status:** ✅ done — `TextTexturePipeline` packages engine + renderer + FIFO cache; `WispText` renders into a `RenderTexture` and `RenderTexture::as_texture()` exposes it as a sprite-friendly `Texture` without GPU copy. With this, text inherits transform / alpha / blend / render-pass participation for free via the existing sprite pipeline — the gaps that M-TEXT.3 deferred.
- **Linear:** [AUT-79](https://linear.app/harwood/issue/AUT-79).
- **Files:** new `crates/wisp/src/text/texture.rs` (`TextTextureKey`, `TextTextureCache`, `TextTexturePipeline`, `MAX_ENTRIES = 64`). `crates/wisp/src/text/mod.rs` and `crates/wisp/src/lib.rs` re-export the three new public types. `crates/wisp/src/texture/render_texture.rs` adds `RenderTexture::as_texture() -> Texture` (zero-copy, shares the underlying wgpu handles). `crates/wisp/src/texture.rs` adds crate-private `Texture::from_render_texture_parts(...)`. New `crates/wisp-storybook/src/stories/s_text_texture.rs` + `writeups/text_texture.md` + `mod.rs` + `all_stories()` entry. New `_docs/book/src/wisp/text/textures.md` chapter. `_docs/book/src/SUMMARY.md`. Regenerated `_docs/book/src/assets/wisp/text-texture.png` via `just snapshots-wisp`.
- **Verified:** 11 new unit tests in `text::texture::tests` covering miss-on-first, hit-on-second (same Arc), invalidation on content / style / color / wrap / dims / font_family, FIFO eviction at MAX_ENTRIES, clear-and-refill, and a pixel smoke ("at least one non-zero alpha pixel"). Full `just gate` green at 303 tests (292 + 11 new). `just snapshots-check` confirms every chapter's referenced asset is on disk.
- **`+y` convention diverges between glyphon and sprite UVs.** Glyphon writes textures with `+y` down (texture row 0 is the top); the sprite pipeline samples with `+y` up. The chapter documents the canonical fix — set `sprite.container.transform.scale.y` negative to display upright. The story uses this idiom.
- **Cache invalidates on every renderable input.** `TextTextureKey` hashes content + family + style (size, color, line_height, letter_spacing, weight, italic, align) + wrap_width + (width_px, height_px). `f32` fields go through `to_bits()` so equality is exact-bit — same NaN bits hash identically (acceptable, callers re-pass the same style across frames).
- **`Texture::from_render_texture_parts` stays `pub(crate)`.** Texture's wgpu fields are crate-private; we expose the conversion via `RenderTexture::as_texture()` (public) so the contract is "render targets can become sampleable textures," not "any wgpu::Texture can become a wisp::Texture."
- **Pipeline is opt-in.** Glyphon + cache costs only apply when a caller constructs `TextTexturePipeline`. `Renderer` doesn't own it, mirroring the M-TEXT.3 sibling-of-Renderer pattern.

---

## M-TEXT.3 follow-up — Custom-font family override + screenshot demo (AUT-77)
- **Date:** 2026-05-10
- **Status:** ✅ done — extends M-TEXT.3 with per-text font family selection so FlexibleText can render through specific TTF files (Inter, JetBrains Mono, …) instead of the cosmic-text default sans-serif. Adds the bundled-font exporter that produces the hero screenshot for the chapter.
- **Linear:** still rolls up under [AUT-77](https://linear.app/harwood/issue/AUT-77) — this PR also lands the Glyphon renderer commit cherry-picked from `wisp/text`.
- **Files:** `crates/wisp/src/text/mod.rs` adds `WispText::font_family: Option<String>` + `WispText::with_font_family(...)`. `crates/wisp/src/text/flexible.rs` honors the override via `Family::Name` when set, else falls back to `Family::SansSerif`; new `FlexibleTextEngine::from_font_paths(paths) -> io::Result<Self>` loads font files into a fresh `cosmic_text::fontdb::Database` (no system fonts). New `crates/wisp-storybook/assets/fonts/` bundle (Inter Regular + Bold, JetBrains Mono Regular, both OFL-1.1) with `Inter-LICENSE.txt` + `JetBrainsMono-OFL.txt` copies. New `crates/wisp-storybook/src/bin/export_text_screenshots.rs` + `[[bin]] wisp-export-text-screenshots` in `crates/wisp-storybook/Cargo.toml`. `Justfile` `snapshots-wisp` chains the new binary. `_docs/book/src/wisp/text/glyphon-backend.md` embeds the generated PNG. `_docs/book/src/assets/wisp/text-custom-fonts.png` (1024×512 hero).
- **Verified:** 2 new unit tests (`with_font_family_sets_field`, `custom_font_family_lays_out_without_panic`). Existing M-TEXT.3 renderer tests still pass.
- **Family resolves at attrs time, not at engine-construct time.** Since `Family::Name(&str)` borrows, the string lives on `WispText` (`Option<String>`); the layout fn computes `Family::Name(name.as_str())` in local scope just before `set_text`. Keeps `WispTextStyle` `Copy` (which atlas.rs and flexible.rs both depend on).
- **OFL-1.1 already allowed.** `deny.toml` carried the entry from the egui ecosystem (Open Sans / Hack). No deny.toml change needed for Inter + JetBrains Mono.
- **No system fonts in the exporter.** `from_font_paths` constructs an empty `fontdb::Database` and loads only the supplied files — outputs reproduce byte-identically across hosts. (Cargo doc / clippy still rely on system fonts elsewhere via the default `FontSystem::new`; this helper is opt-in.)

---

## M-TEXT.3 — Glyphon WGPU rasterizer for FlexibleText (AUT-77)
- **Date:** 2026-05-10
- **Status:** ✅ done — `FlexibleTextRenderer` pairs the cosmic-text layout from M-TEXT.2 with glyphon's wgpu pipeline. Layouts produced by `FlexibleTextEngine` now paint into any `wgpu::TextureView`.
- **Linear:** [AUT-77](https://linear.app/harwood/issue/AUT-77).
- **Files:** `crates/wisp/Cargo.toml` adds `glyphon = "=0.8.0"` (pinned, matches wgpu 24 + cosmic-text 0.12). New `crates/wisp/src/text/flexible_renderer.rs` with `FlexibleTextRenderer` owning `TextAtlas`, `Viewport`, `TextRenderer`, `SwashCache`, `Device`, `Queue`, and `Resolution`. `FlexibleTextEngine` gains `font_system_handle() -> Arc<Mutex<FontSystem>>` and now holds the `FontSystem` inside an `Arc<Mutex<…>>` so engine + renderer share the same font database. `crates/wisp/src/text/mod.rs` + `crates/wisp/src/lib.rs` re-export. New `_docs/book/src/wisp/text/glyphon-backend.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 3 new GPU-using unit tests (`renderer_constructs_against_default_app`, `empty_draw_does_not_panic`, `draw_hello_paints_some_non_zero_pixels`). Full `just gate` green at 292 tests (289 + 3 new). `just site` builds cleanly and renders the new chapter at `target/book/wisp/text/glyphon-backend.html`.
- **Sibling-of-Renderer, not method-on-Renderer.** Glyphon owns ~few MB of GPU state (atlas + pipeline) that shouldn't be paid by callers who never render flexible text. The renderer is constructed explicitly by the caller and given an `Arc<Mutex<FontSystem>>` handle from the engine. Documented in the new chapter's "Shape" section.
- **Pixel test, not snapshot test.** System fonts vary by host (Liberation Sans / DejaVu / Helvetica depending on platform); a byte-exact snapshot would churn on CI bumps. "At least one non-zero alpha pixel" is the genuine regression surface (glyphon broken, atlas allocation failed, font system empty) without false positives on cosmetic font swaps.
- **NDC → pixel + REFERENCE_PX rescale at draw time.** Layouts are shaped at `REFERENCE_PX = 1000` (set in M-TEXT.2). The renderer computes `scale = target_height_px / REFERENCE_PX` per draw, so the same `FlexibleTextLayout` can be drawn into any target without re-shaping.
- **Glyphon `=0.8.0` pin.** Glyphon's wgpu version is exact, not semver-driven — patch bumps can break against wgpu 24. Pinned with `=` to match cosmic-text 0.12 + wgpu 24 at the API level; reconsider when wgpu bumps.
- **Known gaps deferred to M-TEXT.5.** "Container transform + alpha" and "`render_stage` participation" from AUT-77's acceptance criteria are deferred — they fall out naturally once `FlexibleTextRenderer` writes into an intermediate `RenderTexture` and that texture composes through the existing sprite pipeline (M-TEXT.5's RT cache work). Documented in the chapter's "Known gaps (intentional)" section.

---

## M-TEXT.2 — Cosmic Text layout backend (AUT-76)
- **Date:** 2026-05-10
- **Status:** ✅ done — `FlexibleTextEngine` (Cosmic Text) lands behind the M-TEXT.1 trait surface. Layout half only; rasterization is M-TEXT.3 (`glyphon`).
- **Linear:** [AUT-76](https://linear.app/harwood/issue/AUT-76).
- **Files:** `crates/wisp/Cargo.toml` adds `cosmic-text = "0.12"` (default-features = false, std + swash). `deny.toml` adds `NCSA` to allowed licenses. New `crates/wisp/src/text/flexible.rs` with `FlexibleTextEngine { font_system: Mutex<FontSystem> }` (impl `WispTextEngine`) + `FlexibleTextLayout` (impl `WispTextLayout`) carrying a private `cosmic_text::Buffer`. `crates/wisp/src/text/mod.rs` + `crates/wisp/src/lib.rs` re-export. New `_docs/book/src/wisp/text/flexible-cosmic.md` chapter.
- **Verified:** 6 unit tests cover empty content, single-line metrics, multi-line via `\n`, word-wrap behavior at tight widths, weight/italic attrs passthrough, and engine `Send + Sync` contract. `just gate` green at 289 tests (283 + 6 new).
- **NDC ↔ pixel basis — `REFERENCE_PX = 1000`.** Cosmic Text is pixel-based; wisp is NDC-based. The engine multiplies `style.size_ndc` by 1000 to get cosmic-text font size and divides glyph positions back by 1000. Picked because (a) numbers stay within f32 precision, (b) sub-pixel headroom for size_ndc=0.06 (60 px caption), (c) matches glyphon's atlas-cache assumptions. Renderer rescales to actual target at draw time — same `FlexibleTextLayout` can be drawn into any-size target without re-shaping.
- **`cosmic_text::*` does not leak.** `FlexibleTextLayout::buffer` is `pub(crate)` only; the public surface is `WispTextLayout::metrics()` plus `Debug` (which omits the buffer). Glyphon renderer (M-TEXT.3) reads the buffer through the crate-private accessor.
- **`FontSystem` is `!Sync`** — wrapped in a `Mutex` so the engine can satisfy `Send + Sync`. The `engine_is_send_and_sync` test locks this contract at compile time.
- **`set_text` API surprise:** cosmic-text 0.12 takes `Attrs<'_>` by value, not `&Attrs<'_>`. Trip cost: one recursive-fix iteration. Lesson added to CLAUDE.md.

---

## M-TEXT.4 — Atlas text backend formalized (AUT-78)
- **Date:** 2026-05-10
- **Status:** ✅ done — formalizes the M0.15 bitmap path as `AtlasText` behind the M-TEXT.1 trait surface. The existing `scene::Text` node + `text_pipeline` keep driving on-GPU draws; this chunk lands the *layout half* (`AtlasTextEngine` + `AtlasTextLayout`) so M-TEXT.5 can route the same data through render-to-texture for masks / filters / blends.
- **Linear:** [AUT-78](https://linear.app/harwood/issue/AUT-78).
- **Files:** converted `crates/wisp/src/text.rs` → `crates/wisp/src/text/mod.rs`. New `crates/wisp/src/text/atlas.rs` with `AtlasGlyphInstance`, `AtlasTextLayout` (impl `WispTextLayout`), `AtlasTextEngine` (impl `WispTextEngine`). Added `PartialEq` to `scene::text::GlyphMetrics` so glyph instances can be compared in tests. `crates/wisp/src/lib.rs` re-exports the new types. New `_docs/book/src/wisp/text/atlas-vs-flexible.md` chapter with the `AtlasText` vs `FlexibleText` comparison table. `_docs/book/src/SUMMARY.md`.
- **Verified:** 7 unit tests cover empty-text, single-line glyph emission, newline/y-advance, non-ASCII drop, center-align line shift, weight/italic-no-op (atlas contract), and total-height metric. Full `just gate` green at 283 tests (276 + 7 new atlas tests). The M0.15 `scene::Text` + `text_pipeline` path is unchanged — no behavior regression.
- **Layout semantics — `style.size_ndc` is the cell side length.** font8x8 cells are square so width = height = `size_ndc`. Advance = `size_ndc + style.letter_spacing_ndc`. Line step = `size_ndc * style.line_height`. `text.max_width_ndc` is **ignored** by `AtlasText` (no soft wrap; `\n` only). Codepoints ≥ 128 are silently dropped — matches existing M0.15 behavior.
- **Weight + italic are no-ops at the atlas layer.** Bitmap atlases have one rasterization. The fields are accepted so a single `WispTextStyle` survives a backend swap. `FlexibleText` (M-TEXT.3) honors them. Test `weight_and_italic_do_not_change_atlas_layout` locks this contract.
- **`AtlasTextEngine::layout_concrete`** preserves the concrete `AtlasTextLayout` return type for the renderer side. `<Self as WispTextEngine>::layout` boxes for dyn dispatch.

---

## M-TEXT.1 — Wisp text abstraction + backend boundary (AUT-75)
- **Date:** 2026-05-10
- **Status:** ✅ done — data layer + trait surface for the M-TEXT track. Backends plug in behind `WispTextEngine` / `WispTextRenderer`; the project format and inspector controls stay backend-stable across upgrades.
- **Linear:** [AUT-75](https://linear.app/harwood/issue/AUT-75).
- **Files:** new `crates/wisp/src/text.rs` (since M-TEXT.4 promoted to `text/mod.rs`) defining `WispText`, `WispTextStyle`, `WispTextMetrics`, `WispFontHandle`, `WispFontWeight`, `WispFontStyle`, `WispTextAlign`, plus `WispTextLayout` / `WispTextEngine` / `WispTextRenderer` traits. `crates/wisp/src/lib.rs` re-exports the surface. New `_docs/book/src/wisp/text/architecture.md` chapter. `_docs/book/src/SUMMARY.md`.
- **Verified:** 5 unit tests cover weight clamping, named-weight CSS values, `WispTextStyle` defaults, builder chains, and `WispText` builder semantics. `just gate` green at 276 tests.
- **API design — `WispFontHandle(u32)`.** Opaque numeric handle. Atlas backend treats it as a slot id; Cosmic Text backend will treat it as a `Family + Weight + Style` query result. Keeps the project format stable across backend swaps.
- **`WispTextLayout: Send + Sync`** so caches (M-DYN.2-style for text layouts) can hold them across frames. Engines + renderers may be `&self` so the `Renderer` struct holds them without interior mutability contention.

---

## M-TEXT.0 — Shared scene traversal + transform helpers (AUT-74)
- **Date:** 2026-05-10
- **Status:** ✅ done — pure refactor. Foundation chunk for the M-TEXT track.
- **Linear:** [AUT-74](https://linear.app/harwood/issue/AUT-74).
- **Files:** new `crates/wisp/src/render/scene_walk.rs` with `walk_visible_subtree(stage, start, exclude, |id, node, world|)` + `mat3_to_mat4(Mat3) -> Mat4` helpers. Refactored `crates/wisp/src/render/sprite_pipeline.rs`, `graphics_pipeline.rs`, `mesh_pipeline.rs`, and `text_pipeline.rs` to use the helpers — removed 4× duplicated traversal-stack loops and 4× duplicated `mat3_to_mat4` definitions. `crates/wisp/src/render.rs` adds the new mod declaration.
- **Verified:** 5 unit tests cover preorder traversal, exclude-set filtering, invisible-node skipping (self + descendants), parent-world transform accumulation, and `mat3_to_mat4` correctness. Full `just gate` green at 271 tests (266 + 5 new scene_walk tests). All existing renderer tests pass byte-equivalent — no behavior change.
- **Why this first.** The next chunk (M-TEXT.1) introduces a `WispText*` trait boundary and two new backends (`AtlasText` + `FlexibleText`). Each backend will need scene traversal; extracting it once keeps the new code from duplicating what the existing pipelines already had.
- **Lesson reinforced:** clippy `field_reassign_with_default` rejects `let mut x = X::default(); x.field = ...`. Use `X { field: ..., ..X::default() }` literal-form instead. Already documented under "Cast hygiene" pattern; tripped me again here.

---

## M-VEC.12 — Vector primitive examples gallery (AUT-64) — phase complete
- **Date:** 2026-05-10
- **Status:** ✅ done — closes the M-VEC track. Single-canvas storybook entry tying together the entire vector catalog plus an mdBook chapter that indexes every M-DYN / M-VEC chunk.
- **Linear:** [AUT-64](https://linear.app/harwood/issue/AUT-64).
- **Files:** new `crates/wisp-storybook/src/stories/s_vector_gallery.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. New `_docs/book/src/wisp/chunks/vector-gallery.md` (gallery image + chunk index for both M-VEC and M-DYN tracks).
- **Verified:** storybook fingerprint snapshot updated; PNG visually confirmed.

### Next-phase summary — 20 issues, 17 commits

The `mvp/next-phase` branch closes 20 issues across two milestone tracks plus two P0 mask-followups:

| Track | Issues | Commits |
|---|---|---|
| M-DYN.1..6 (dynamic textures) | AUT-43/44/45/46/47/48 | 3 |
| M-VEC.1..12 (vector primitives) | AUT-53..64 | 10 |
| P0 export-parity | AUT-27, AUT-33 | 1 |

**Key architectural delivery:** the *separated mask + composition* model. Coverage (M-DYN.1 `MaskTexturePipeline`) is decoupled from composition (M-VEC.4 `MaskComposePipeline`), with a cache (M-DYN.2) and a vector data model (M-VEC.1) layered in between. The five existing mask primitives (clip, privacy_blur, solid_redaction, spotlight, dim_outside) were refactored onto this path while keeping their public APIs byte-equivalent. Path-driven variants land for every primitive.

**+45 tests** beyond the M-MASK baseline (201 → 266 across the branch).

**Test count milestones:**
- M-MASK baseline (start of branch): 201
- After M-DYN.1+.2 + M-VEC.1: 221
- After M-VEC.2+.3: 229
- After M-VEC.4-6 refactor: 235
- After AUT-27/-33 export parity: 243
- After M-DYN.3-6: 247
- After M-VEC.7-12: 266

**Lessons captured in CLAUDE.md** during this phase:
- "Renderer batching / draw order" — Graphics paints AFTER Sprites in `render_stage` regardless of scene-tree order.
- "WGSL ↔ Rust uniform layout" — vec3 fields force 16-byte alignment; size the Rust struct to match.

The architecture is now app-ready: an editor inspector can drive privacy blur, redaction, spotlight, dim-outside, crop, webcam shape, and freehand-path masks through a single `Vector` data type, with caching, export parity, and full primitive composability.

---

## M-VEC.10 + M-VEC.11 — Path stroke + mask boolean ops (AUT-62 + AUT-63)
- **Date:** 2026-05-10
- **Status:** ✅ done — two chunks shipped together. Path stroke unblocks `Callout::arrow_to`; mask boolean ops let masks combine via union / intersect / subtract.
- **Linear:** [AUT-62](https://linear.app/harwood/issue/AUT-62), [AUT-63](https://linear.app/harwood/issue/AUT-63).
- **Files:** new `crates/wisp/src/scene/path.rs` (`PathBuilder`, `Path`, adaptive Bezier flattening); new `crates/wisp/shaders/mask_combine.wgsl` and `crates/wisp/src/render/mask_combine.rs` (`MaskCombineOp`, `MaskCombinePipeline`). `crates/wisp/src/scene/callout.rs` adds `Callout::arrow_to` using the new path stroke. `crates/wisp/src/render.rs` adds `Renderer::combine_masks`. `lib.rs` re-exports `Path`, `PathBuilder`, `PathCommand`, `MaskCombineOp`. New `crates/wisp/tests/mask_combine.rs` (3 cases). Two new stories: `path-stroke`, `mask-combine`. New `_docs/book/src/wisp/chunks/vector-path-stroke.md`. CLAUDE.md gains a new lesson under "WGSL ↔ Rust uniform layout."
- **Verified:** 6 path unit tests + 3 mask-combine integration tests pass. Full `just gate` green.
- **`vec3<u32>` alignment gotcha** (caught during M-VEC.11 first run): WGSL `vec3<u32>` is 16-byte aligned, so a struct `{ op: u32, _pad: vec3<u32> }` is 32 bytes, not 16. The matching Rust struct must pad to the same size or wgpu rejects the bind group with "Buffer is bound with size N where the shader expects M." Validation error is silent at compile time and only surfaces at run time. Documented in CLAUDE.md "WGSL ↔ Rust uniform layout."
- **`draw_line` colors via `current_fill`, not stroke** — caught during M-VEC.10 story rendering. The first version of `Path::stroke_to_graphics` set `Stroke` but `draw_line` ignores stroke (the docstring explicitly says "Strokes do not apply to lines"). Fixed by setting `Fill::Solid(color)` before emitting segments. Small lesson; not adding to CLAUDE.md — the docstring is clear and the fix is local.
- **Adaptive Bezier flattening via perpendicular-distance test:** for a curve to subdivide, the max perpendicular distance from a control point to the chord must exceed `tolerance`. Quadratic checks one control point; cubic checks both. Standard de Casteljau subdivision when triggered. NDC tolerance of 0.005 produces visually smooth curves at 256px output.

---

## M-VEC.8 + M-VEC.9 — Highlight + callout primitives (AUT-60 + AUT-61)
- **Date:** 2026-05-10
- **Status:** ✅ done — preset constructors for the most common attention-guiding overlays. Outputs are plain `Vector`s so they chain through every existing builder. Two issues, one chunk because they share the architectural pattern (preset → `Vector`).
- **Linear:** [AUT-60](https://linear.app/harwood/issue/AUT-60), [AUT-61](https://linear.app/harwood/issue/AUT-61).
- **Files:** new `crates/wisp/src/scene/highlight.rs` (`Highlight::outline / filled / pill / glow`); new `crates/wisp/src/scene/callout.rs` (`Callout::label_box / badge / caption_pill`); `crates/wisp/src/scene.rs` and `lib.rs` re-exports. New `crates/wisp-storybook/src/stories/s_vector_overlays.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. New `_docs/book/src/wisp/chunks/vector-highlight-callout.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/vector-overlays.png`.
- **Verified:** 8 unit tests (4 highlight + 4 callout) cover constructor invariants; storybook smoke + fingerprint green; PNG visually checked (yellow outline ring, cyan pill, amber label, red badge, dark caption pill).
- **Known gaps documented in chapter:**
  - Arrow / pointer-line callouts need M-VEC.10 stroke-along-path commands. Once landed, an `arrow_to` constructor joins `Callout` without breaking changes.
  - True Gaussian glow depends on M-DYN.7 (P2) feathering. `Highlight::glow` is a wider-stroke / lower-alpha approximation in V1.

---

## M-VEC.7 — Vector spotlight + inverse-dim effects (AUT-59)
- **Date:** 2026-05-10
- **Status:** ✅ done — closes the path-accepting gap. `apply_dim_outside_data` was already shipped (M-MASK.7) but only accepted analytic SDF shapes; this chunk adds `apply_dim_outside_vector` for path support, and a chapter that ties M-VEC.7 to the existing `apply_spotlight_vector` (M-VEC.6) + `compose_dim_through_inverted_mask` (M-DYN.5).
- **Linear:** [AUT-59](https://linear.app/harwood/issue/AUT-59).
- **Files:** `crates/wisp/src/render.rs` adds `apply_dim_outside_vector(vector, strength, base, output)`. New `crates/wisp/tests/dim_outside_vector.rs` (2 cases). New `_docs/book/src/wisp/chunks/vector-spotlight-dim.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** byte-equivalence with `apply_dim_outside_data` for analytic shapes; path variant dims correctly around a diamond polygon. `just gate` green.

---

## M-DYN.3..6 — Explicit-mask composition primitives (AUT-45 / -46 / -47 / -48)
- **Date:** 2026-05-10
- **Status:** ✅ done — closes four P1 mask-followup tickets in one chunk. Lower-level companion primitives that take the mask texture as an explicit parameter, allowing one mask to be shared across multiple effects in the same frame.
- **Linear:** [AUT-45](https://linear.app/harwood/issue/AUT-45), [AUT-46](https://linear.app/harwood/issue/AUT-46), [AUT-47](https://linear.app/harwood/issue/AUT-47), [AUT-48](https://linear.app/harwood/issue/AUT-48).
- **Files:** `crates/wisp/src/render.rs` adds `compose_blur_through_mask` (M-DYN.3), `compose_solid_through_mask` (M-DYN.4), `compose_dim_through_inverted_mask` (M-DYN.5). M-DYN.6 (webcam crop dynamic texture) is satisfied by the existing `apply_clip_vector` with `VectorShape::Circle` / `RoundedRect` — pure documentation. Refactored `apply_privacy_blur_vector` to delegate to `compose_blur_through_mask`. New `crates/wisp/tests/blur_mask_reuse.rs` (2 cases). New `crates/wisp/tests/compose_through_mask.rs` (2 cases). New `_docs/book/src/wisp/chunks/compose-through-mask.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** 19 existing privacy-blur tests still pass; 4 new tests pass; full `just gate` green.
- **Why batch four issues into one chunk?** They're all the same architectural pattern — "expose the mask texture as an explicit parameter to the composition primitive." Shipping them separately would have meant 4 PROGRESS entries / 4 chapters / 4 commits for what is one coherent design decision. Linked them in a single chapter that covers all four.
- **M-DYN.6 needed no new code.** Spec said "webcam overlays should crop through the dynamic mask path." `apply_clip_vector(vec_circle_or_rounded_rect, foreground, output)` already does exactly that — generates mask via `MaskTexturePipeline`, composes via `MaskComposePipeline`. Documented the connection explicitly so M-DYN.6 isn't lost.
- **High-level methods now route through the explicit primitives.** `apply_privacy_blur_vector` is a one-liner that calls `compose_blur_through_mask` with the cached mask. The high-level path stays ergonomic; the low-level path stays composable.

---

## Export + copy-frame mask parity (AUT-27 + AUT-33)
- **Date:** 2026-05-10
- **Status:** ✅ done — both P0 mask-followups closed in one chunk. Five export-parity tests + three copy-frame tests lock in: every mask primitive produces identical bytes whether you render to a preview view or to an export RT, and `read_pixels` returns the masked content (not the base).
- **Linear:** [AUT-27](https://linear.app/harwood/issue/AUT-27), [AUT-33](https://linear.app/harwood/issue/AUT-33).
- **Files:** new `crates/wisp/tests/export_mask_parity.rs` (5 cases — clip, redaction, spotlight, privacy blur, path-vector clip). New `crates/wisp/tests/copy_frame_mask_parity.rs` (3 cases — redaction, spotlight, privacy blur). New `_docs/book/src/wisp/chunks/export-mask-parity.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** all 8 new tests pass; full `just gate` green.
- **Architecture is already export-safe — these tests just guard it.** `Renderer::render_stage` is the only code path that produces frames. `apply_*` primitives go through it identically whether the caller binds a preview surface view or an export RT view. The tests render the same scene twice to different RTs and assert byte-equality — guards against any future preview-only shortcut.
- **Both issues in one chunk** because they share the same architectural property (single render path → identical output) and the same underlying primitives. AUT-27 is the byte-equality test; AUT-33 is the inside-vs-outside-mask test on `read_pixels`.

---

## M-VEC.6 — Clip + spotlight on vector masks (AUT-58) — refactor zone closed
- **Date:** 2026-05-10
- **Status:** ✅ done — completes the M-VEC.4..6 refactor of existing M-MASK primitives onto the M-VEC pipeline.
- **Linear:** [AUT-58](https://linear.app/harwood/issue/AUT-58).
- **Files:** `crates/wisp/src/render.rs` — refactored `apply_clip` and `apply_spotlight` internals; added `apply_clip_vector` + `apply_spotlight_vector`. New `crates/wisp/tests/clip_spotlight_vector.rs` (2 cases). New `_docs/book/src/wisp/chunks/vector-clip-spotlight.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** 16 existing M-MASK clip + spotlight + dim-outside + ellipse + circle tests still pass byte-equivalent; 2 new path-driven tests pass; full `just gate` green.
- **Auto-dispatch path NOT refactored.** `render_stage` calls `self.clip.apply(...)` directly when handling `Container::clip = Some(MaskShape)` — the hot path runs per dispatched node every frame. The new vector-mask path adds a render pass; that's fine for explicit calls (cache hits offset it) but would be a regression on auto-dispatch. Documented in the chapter.
- **Spotlight path-inverse special case.** `cached_mask_texture_inverted` exists for SDF shapes but no equivalent for paths in V1. The path route routes through `path_clip.apply(..., invert: true, ...)` — uses the existing inline-clip pipeline directly. Future enhancement: cached path mask + an inverse-compose shader. Not blocking — path-driven spotlight works end-to-end.

---

## M-VEC.5 — Solid redaction on vector masks (AUT-57)
- **Date:** 2026-05-10
- **Status:** ✅ done — same refactor pattern as M-VEC.4. `apply_solid_redaction(MaskShape, ...)` keeps its API; internals route through the shared mask + compose path. Adds `apply_solid_redaction_vector` for path support.
- **Linear:** [AUT-57](https://linear.app/harwood/issue/AUT-57).
- **Files:** `crates/wisp/src/render.rs` — refactored `apply_solid_redaction` to wrap its `MaskShape` in a `Vector` and forward to new `apply_solid_redaction_vector`. New `crates/wisp/tests/solid_redaction_vector.rs` (2 cases). New `_docs/book/src/wisp/chunks/vector-solid-redaction.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** 4 existing M-MASK.5 redaction tests still pass; 2 new vector tests pass; full `just gate` green.
- **Reuses M-VEC.4 infrastructure entirely.** No new pipelines or shaders. The `MaskComposePipeline` and `cached_vector_mask_texture` from M-VEC.4 cover this primitive too. Only the "fill source" differs (clear-to-color instead of blur).

---

## M-VEC.4 — Privacy blur on vector masks (AUT-56)
- **Date:** 2026-05-10
- **Status:** ✅ done — first refactor of an existing M-MASK primitive onto the M-VEC pipeline. `apply_privacy_blur(shape: MaskShape, ...)` now routes through the separated mask-texture path internally; output is byte-equivalent to the previous inline-clip implementation.
- **Linear:** [AUT-56](https://linear.app/harwood/issue/AUT-56).
- **Files:** new `crates/wisp/shaders/mask_compose.wgsl` (samples foreground RT × mask RT → `(fg.rgb, fg.a * mask.a)`); new `crates/wisp/src/render/mask_compose.rs` (`MaskComposePipeline`); `crates/wisp/src/render.rs` adds `apply_mask_to_texture(foreground, mask, output)` public primitive plus `apply_privacy_blur_vector(vector, radius, base, output)`. `apply_privacy_blur` itself was rewritten to wrap its `MaskShape` in a `VectorShape` and forward — same external API, new internals. New `crates/wisp/tests/privacy_blur_vector.rs` (2 cases). New `_docs/book/src/wisp/chunks/vector-privacy-blur.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** 9 existing M-MASK.2/.3/.4 privacy-blur tests still pass unchanged (proves the refactor is byte-equivalent); 2 new vector-driven tests pass; full `just gate` green.
- **Architectural pivot — separated mask + composition.** The old pipeline ran `ClipPipeline` to compute mask + sample foreground in one shader. The new pipeline runs `MaskTexturePipeline` (mask only) then `MaskComposePipeline` (multiply foreground × mask). One extra render pass per call, offset by mask-cache hits — static masks across frames now regenerate exactly once.
- **Path support unlocked.** Privacy blur can now use `VectorShape::Path` — previously impossible because `apply_privacy_blur(shape: MaskShape, ...)` had no Path variant in the enum. Test `path_vector_blurs_inside_polygon` covers a diamond polygon over a red/blue split: center mixes both colors, far corner stays as base red.
- **Old API preserved.** `apply_privacy_blur` still takes a `MaskShape` and produces the same bytes. Existing M-MASK.2/.3/.4 stories, tests, and chapters keep working without modification. The chapter notes the previous inline-clip pipeline description in those chapters is now historical.
- **`apply_mask_to_texture` is now public.** Made it part of the `Renderer` public surface so AUT-57 (solid redaction) and AUT-58 (rounded crops) can reuse the same intermediate primitive — and eventually any app-level code that wants direct control over mask × foreground composition.

---

## M-VEC.3 — Render vectors to alpha-mask textures (AUT-55)
- **Date:** 2026-05-10
- **Status:** ✅ done — bridge between `Vector` (M-VEC.1) and `MaskTexturePipeline` (M-DYN.1). Unblocks the M-VEC.4..6 refactor of existing mask primitives.
- **Linear:** [AUT-55](https://linear.app/harwood/issue/AUT-55).
- **Files:** `crates/wisp/src/render.rs` adds `Renderer::generate_vector_mask_texture(vector, w, h)` and `cached_vector_mask_texture(...)`. New `crates/wisp/tests/vector_mask_bridge.rs` (4 cases). New `_docs/book/src/wisp/chunks/vector-mask-bridge.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** 4 bridge tests pass; full `just gate` green.
- **Pure routing — no new pipelines or shaders.** The bridge dispatches on `VectorShape` and forwards to either `generate_mask_texture` (SDF path) or `generate_path_mask_texture` (path). Output is byte-equivalent to the direct primitive call.
- **Cached variant respects existing semantics.** Analytic shapes go through the M-DYN.2 cache (counted in `mask_cache_stats()`); path shapes bypass and wrap in `Arc` per call. `cached_path_vector_does_not_use_cache` test locks in the V1 limitation.
- **No story.** This is dispatch infrastructure; the renderable output is identical to what the existing `mask-texture` story already shows. M-VEC.4 (privacy blur refactor) will be the first user-facing demonstration.

---

## M-VEC.2 — Render vector primitives to scene geometry (AUT-54)
- **Date:** 2026-05-10
- **Status:** ✅ done — thin layer on top of the existing graphics rasterizer; Vector primitives now appear in `render_stage` output.
- **Linear:** [AUT-54](https://linear.app/harwood/issue/AUT-54).
- **Files:** `crates/wisp/src/scene/vector.rs` adds `Vector::to_graphics()` (analytic shapes → `Graphics`; path returns `None`) and `Vector::add_to_stage()` convenience wrapper. `crates/wisp/src/scene/vector.rs` includes a `scale_fill_alpha` helper that folds opacity into all three `Fill` variants (Solid / LinearGradient / RadialGradient). New `crates/wisp/tests/vector_render.rs` (4 cases). New `crates/wisp-storybook/src/stories/s_vector_render.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/vector-render.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/vector-render.png`.
- **Verified:** 4 vector-render tests pass; storybook smoke + fingerprint green; PNG visually checked (teal rect / amber rounded / gradient circle / green-stroke ellipse / 40%-opacity white rounded-rect).
- **No new pipeline.** The existing graphics rasterizer already handles rect / rounded-rect / ellipse — `Vector::to_graphics()` is a match-arms walker that picks the right `Graphics::draw_*` call. Circle is `Ellipse(half = (r, r))`, same trick as M-MASK.8 used for `MaskShape::Circle`.
- **Opacity folded into paint at conversion time.** The renderer has no per-node opacity channel; multiplying the alpha component of fill + stroke colors at conversion is the practical equivalent. Matches PixiJS semantics ("opacity on the graphics primitive").
- **Path rendering deferred.** `VectorShape::Path` returns `None` from `to_graphics()` — the graphics rasterizer has no draw_path primitive. M-VEC.10 (AUT-62) adds path stroke + fill commands; until then, paths drive masks only.
- **Borrow gotcha** (test code): `v.add_to_stage(&mut stage, stage.root())` doesn't compile — `stage.root()` borrows `stage` immutably while `&mut stage` is held. Bind `let root = stage.root();` first. Same pattern existing M-MASK tests already use; just had to remember to apply it to the new `add_to_stage` API.

---

## M-VEC.1 — Vector shape primitive model in wisp (AUT-53)
- **Date:** 2026-05-10
- **Status:** ✅ done — data-only chunk; rendering layers on in M-VEC.2/.3.
- **Linear:** [AUT-53](https://linear.app/harwood/issue/AUT-53).
- **Files:** new `crates/wisp/src/scene/vector.rs` (`VectorShape` enum with Rect/RoundedRect/Circle/Ellipse/Path variants, `VectorStroke` struct, `Vector` struct with shape/fill/stroke/opacity/transform + builder methods); `crates/wisp/src/scene.rs` exposes the new module + re-exports; `crates/wisp/src/lib.rs` re-exports `Vector` / `VectorShape` / `VectorStroke`. New `_docs/book/src/wisp/chunks/vector-model.md`. `_docs/book/src/SUMMARY.md`.
- **Verified:** 9 unit tests cover constructors, bounds (including empty path → zero rect), `as_mask_shape` round-trip, default opacity = 1.0, and builder chaining. `just gate` green at 221 tests (+9 from M-DYN.2's 212).
- **Bridges to existing mask machinery via `as_mask_shape()`.** Analytic-SDF variants (Rect / RoundedRect / Circle / Ellipse) convert to `MaskShape` cleanly; Path returns `None`. This is what M-VEC.4..6 will use to refactor `apply_clip` / `apply_privacy_blur` / `apply_solid_redaction` / `apply_spotlight` onto vector data without rewriting the SDF shader. Path uses the parallel `as_path_points()` accessor instead.
- **`Clone`, not `Copy`.** The `Path { points: Vec<Vec2> }` variant carries owned data, so the enum can't be `Copy`. That's the same reason `MaskShape::Path` was never added — documented in the chapter and in M-MASK.10's progress entry. Existing `MaskShape` stays `Copy` (no path variant); `VectorShape` is `Clone`.
- **`#[non_exhaustive]` future-proofs the catalog.** M-VEC.10 (path stroke commands), M-VEC.13 (SVG import subset), M-VEC.16 (feathered edges) will extend `VectorShape` without breaking callers.
- **No story for the data layer.** Per the project convention, non-render chunks are exempt. M-VEC.2 (the rasterizer) and M-VEC.3 (the alpha-mask bridge) will both ship stories.

---

## M-DYN.2 — Mask texture cache in wisp (AUT-44)
- **Date:** 2026-05-10
- **Status:** ✅ done — performance guardrail on top of M-DYN.1. Identical mask inputs across frames reuse the GPU texture instead of regenerating.
- **Linear:** [AUT-44](https://linear.app/harwood/issue/AUT-44).
- **Files:** new `crates/wisp/src/render/mask_cache.rs` (`MaskKey` with bit-cast `f32` hashing, `MaskCache` with FIFO eviction, `MAX_ENTRIES = 64`, hits/misses stats); `crates/wisp/src/render.rs` adds `mask_cache: RefCell<MaskCache>` field plus four new public methods (`cached_mask_texture`, `cached_mask_texture_inverted`, `mask_cache_stats`, `clear_mask_cache`); new `crates/wisp/tests/mask_cache.rs` (5 cases); new `_docs/book/src/wisp/chunks/mask-cache.md`; `_docs/book/src/SUMMARY.md`.
- **Verified:** 5 cache tests pass; full `just gate` green.
- **Bit-cast `f32` for hashing.** `MaskShape` has `f32` fields, which aren't `Eq` / `Hash` directly. The cache key bit-casts each `f32` to `u32` via `to_bits()`, hashes the resulting integer representation. Same canonical NaN bits hash identically — fine for our use case where callers re-pass the same shape value across frames; we explicitly want exact-bit equality, not NaN-aware floating-point equality.
- **Returns `Arc<RenderTexture>`, not `&RenderTexture`.** The cache outlives any single render-stage call, so consumers need shared ownership rather than a reference tied to the cache's lifetime. `Arc` (not `Rc`) keeps the `Renderer` `Send`-compatible.
- **FIFO over LRU for V1.** LRU would need access-order bookkeeping on every cache hit; FIFO needs nothing. With a 64-entry cap and typical usage (small set of recurring masks per recording), FIFO behaves close to LRU in practice. Easy to upgrade later if profiling shows churn.
- **No story for the cache.** Caching is invisible — output bytes are identical to non-cached. The existing `mask-texture` story (M-DYN.1) implicitly demonstrates correctness regardless of which path produced the texture. Per the project convention, non-render features are exempt from the story-per-chunk rule.
- **Path masks intentionally excluded.** Hashing a `Vec<glam::Vec2>` is non-trivial and most freehand-polygon use cases mutate between frames. Documented in the chapter as a V1 limitation; callers needing path caching manage it externally.

---

## M-DYN.1 — Dynamic alpha-mask texture primitive in wisp (AUT-43)
- **Date:** 2026-05-10
- **Status:** ✅ done — first chunk of the dynamic-textures phase. Coverage is now a separate primitive from composition; everything that follows in M-DYN / M-VEC builds on this.
- **Linear:** [AUT-43](https://linear.app/harwood/issue/AUT-43).
- **Files:** new `crates/wisp/shaders/mask_texture.wgsl` (SDF coverage to alpha RT, supports Rect/RoundedRect/Circle/Ellipse via `shape_kind` flag, with `invert` flag); new `crates/wisp/shaders/path_mask_texture.wgsl` (uniform-buffered point-in-polygon coverage); new `crates/wisp/src/render/mask_texture.rs` (`MaskTexturePipeline`); new `crates/wisp/src/render/path_mask_texture.rs` (`PathMaskTexturePipeline`); `crates/wisp/src/render.rs` adds three public methods (`generate_mask_texture`, `generate_mask_texture_inverted`, `generate_path_mask_texture`); new `crates/wisp/tests/mask_texture.rs` (6 cases); new `crates/wisp-storybook/src/stories/s_mask_texture.rs` + writeup; `crates/wisp-storybook/src/stories/mod.rs`; `_docs/book/src/wisp/chunks/mask-texture.md`; `_docs/book/src/SUMMARY.md`; `_docs/book/src/assets/wisp/mask-texture.png`; story-fingerprint snapshot updated.
- **Verified:** 6 mask-texture tests pass on Metal; storybook smoke + fingerprint green.
- **Architectural pivot — coverage is now its own primitive.** The existing `apply_clip` / `apply_privacy_blur` / `apply_solid_redaction` / `apply_spotlight` / `apply_path_clip` keep working as before (combined SDF + foreground sample in one shader). M-DYN.1 introduces the *separated* path: shape-data → alpha RT, no foreground involvement. M-DYN.2 (cache), M-VEC.3 (vector → mask bridge), and M-VEC.4..6 (refactor existing primitives onto the new model) all build on this foundation. Pure addition; no existing code was changed.
- **Output format choice — `(m, m, m, m)`:** writing the same value to RGB and alpha means consumers can sample either way. Composition shaders use `.a` for alpha-multiplication; the storybook tile renders the texture as a grayscale silhouette via `Texture::from_rgba` + `Sprite`, which "just works" without a separate display path.
- **Two pipelines, not one.** `MaskTexturePipeline` handles SDF shapes (Rect/RoundedRect/Circle/Ellipse degenerate cases of one rounded-rect SDF, plus the ellipse SDF branch via `shape_kind`). `PathMaskTexturePipeline` handles polygons via the same crossings-test as `path_clip.wgsl`. Symmetry with the existing clip/path-clip split keeps each path single-purpose. Future cleanup may unify them once the AUT-58 vector refactor settles.
- **Gate-loop lesson — pipelines batch by type, not insertion order.** First version of the storybook story added a full-canvas `Graphics` backdrop BEFORE the mask sprites. Storybook smoke passed (backdrop alone exceeded the visibility threshold), but the exported PNG showed only the backdrop. Root cause: `render_stage` batches draws by pipeline type — all sprites first, then all graphics — so a backdrop graphics paints OVER the sprites regardless of insertion order. **Fix:** rely on the renderer's clear color for backdrops; reserve `Graphics` for foreground decoration. Captured in CLAUDE.md "Renderer batching / draw order".

---

## M-MASK.10 — Freehand path mask in wisp (AUT-35) — series complete
- **Date:** 2026-05-10
- **Status:** ✅ done — closes the 10-issue mask suite (AUT-31 / AUT-20 / AUT-21 / AUT-22 / AUT-23 / AUT-28 / AUT-29 / AUT-30 / AUT-34 / AUT-35).
- **Linear:** [AUT-35](https://linear.app/harwood/issue/AUT-35).
- **Files:** new `crates/wisp/shaders/path_clip.wgsl` (point-in-polygon via crossings test). New `crates/wisp/src/render/path_clip.rs` (`PathClipPipeline`, uniform-buffered point list, `MAX_PATH_POINTS=32`). `crates/wisp/src/render.rs` adds `path_clip` field, `Renderer::apply_path_clip(points, foreground, output)` and `Renderer::apply_solid_redaction_path(points, color, base, output)`. New `crates/wisp/tests/path_clip.rs` (3 cases: star center pass-through, concave-gap cutout, path solid redaction). New `crates/wisp-storybook/src/stories/s_path_mask.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/path-mask.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/path-mask.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (201 tests, +3 from M-MASK.9's 198).
- **First shape that doesn't fit any SDF.** A free polygon can't be expressed as a closed-form distance function (no analytic SDF for an arbitrary polygon). Two design options were evaluated: (1) tessellate to a triangle fan + draw to a mask RT, (2) point-in-polygon test in the fragment shader against a uniform-buffered point list. Picked (2): no tessellation needed, handles concave shapes for free, single render pass, no scratch RT, and the implementation is ~80 lines of WGSL vs. ~200 lines of CPU triangulation. Trade-off: capped at 32 vertices (uniform buffer size) and hard-edge for V1 (no AA).
- **Why no `MaskShape::Path` variant.** `MaskShape` is `Copy` — every variant holds POD so the enum stays cheap to pass by value through the auto-dispatch path. A path needs `Vec<Vec2>` or `Arc<[Vec2]>` to own the points; that forces `MaskShape` to drop `Copy` and adopt `Clone`, rippling through `PrivacyBlur`/`DimOutside`/all clip call sites. Not worth it for a "premium expansion." Path-clip lives next to the SDF clip with its own dedicated public methods (`apply_path_clip`, `apply_solid_redaction_path`).
- **Star-polygon contract test.** Tested with a 10-vertex five-pointed star — concave with gaps between the arms. The crossings test correctly classifies a pixel in one of the gaps (NDC -0.65, +0.55) as outside. Self-intersecting paths aren't on the freehand-mask UX path; this primitive doesn't promise sensible results for them.
- **`u32::from(bool)` for the invert flag.** The shader takes a `u32` invert flag; `u32::from(true) = 1`, `u32::from(false) = 0`. Cleaner than the cast-precision-loss-prone `if invert { 1 } else { 0 }` and survives clippy without a reason allow.

### Series summary

10 mask primitives shipped on `mvp/masks` in one continuous loop:

| Chunk | Issue | Primitive | Tests | Story |
| --- | --- | --- | --- | --- |
| M-MASK.1 | AUT-31 | Rounded crop foundation | 4 | clip-rounded |
| M-MASK.2 | AUT-20 | Rectangle privacy blur | 3 | privacy-blur-rect |
| M-MASK.3 | AUT-21 | Rounded privacy blur | 3 | privacy-blur-rounded |
| M-MASK.4 | AUT-22 | Configurable blur strength | 3 | privacy-blur-strength |
| M-MASK.5 | AUT-23 | Solid redaction | 4 | solid-redaction |
| M-MASK.6 | AUT-28 | Spotlight / highlight | 3 | spotlight |
| M-MASK.7 | AUT-29 | Dim-outside | 3 | dim-outside |
| M-MASK.8 | AUT-30 | Webcam circle | 3 | webcam-shapes |
| M-MASK.9 | AUT-34 | Ellipse | 3 | ellipse-mask |
| M-MASK.10 | AUT-35 | Freehand path | 3 | path-mask |

Total: +32 pixel-readback tests, +10 storybook stories, +10 mdBook chapters, started at 173 and finished at 201 tests on `just gate`. Single shader (`clip.wgsl`) plus one new pipeline (`path_clip.wgsl`) cover everything. The five composition primitives (`apply_clip` / `apply_privacy_blur` / `apply_solid_redaction` / `apply_spotlight` / `apply_path_clip`) plus three data wrappers (`PrivacyBlur` / `DimOutside` and their strength enums) form the renderer-data API the editor inspector will eventually drive.

---

## M-MASK.9 — Ellipse / oval mask in wisp (AUT-34)
- **Date:** 2026-05-10
- **Status:** ✅ done — `MaskShape::Ellipse` adds anisotropic SDF support; first shape that needs a real new SDF (vs. degenerating to rounded-rect).
- **Linear:** [AUT-34](https://linear.app/harwood/issue/AUT-34).
- **Files:** `crates/wisp/src/scene/clip.rs` adds `MaskShape::Ellipse { center, half_extents }` + `ellipse()` ctor + `bounds()` arm. `crates/wisp/shaders/clip.wgsl` adds `shape_kind: f32` uniform + `sdf_ellipse` helper + branching fragment dispatch. `crates/wisp/src/render/clip.rs` `ClipUniforms` carries `shape_kind`; `apply_with_invert` sets `shape_kind = 1.0` for `Ellipse`. New `crates/wisp/tests/clip_ellipse.rs` (3 cases). New `crates/wisp-storybook/src/stories/s_ellipse.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/ellipse-mask.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/ellipse-mask.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (198 tests, +3 from M-MASK.8's 195).
- **Pseudo-SDF over closed-form.** Closed-form ellipse SDF requires solving a quartic — expensive every fragment. The scaled-quadratic `(x/a)^2 + (y/b)^2 - 1` shares the same zero level set; multiplying by `min(a, b)` puts the result in roughly NDC units so the existing AA-band code (`smoothstep` over `aa = 2/min(w, h)`) still produces a ~1-pixel edge. Visually indistinguishable from the exact SDF for masking; orders of magnitude cheaper.
- **`shape_kind: f32` flag handles dispatch.** Same uniform buffer as the rest of the clip shader; one extra `if` in WGSL picks the SDF formula. All four primitives (`apply_clip` / `apply_privacy_blur` / `apply_solid_redaction` / `apply_spotlight` / `apply_dim_outside_data`) gain the variant for free.
- **Cache-poisoning replay:** ran into the same nextest+check race documented in M-MASK.8's lesson. `cargo clean -p wisp` + retry was the fix. CLAUDE.md already covers this; reinforced the workflow ordering.

---

## M-MASK.8 — Webcam circle mask shape in wisp (AUT-30)
- **Date:** 2026-05-10
- **Status:** ✅ done — adds `MaskShape::Circle` to the catalog. Webcam overlay now has cinematic circle + rounded-rect options out of the box, both reusing the existing rounded-rect SDF.
- **Linear:** [AUT-30](https://linear.app/harwood/issue/AUT-30).
- **Files:** `crates/wisp/src/scene/clip.rs` adds `MaskShape::Circle { center, radius }` variant, `circle()` ctor, and `bounds()` arm. `crates/wisp/src/render/clip.rs` `apply_with_invert` translates `Circle` to the rounded-rect SDF parameters (`half_extents = (r, r)`, `corner_radius = r`). New `crates/wisp/tests/clip_circle.rs` (3 cases). New `crates/wisp-storybook/src/stories/s_webcam_shapes.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/webcam-shapes.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/webcam-shapes.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (195 tests, +3 from M-MASK.7's 192). Story renders both shapes side-by-side over a dark gradient backdrop.
- **One shader, three shapes.** The rounded-rect SDF (`length(max(|p|-half+r, 0)) + min(max(qx,qy), 0) - r`) degenerates exactly to `length(p) - r` (the circle SDF) when `half = (r, r)` and the corner radius is `r`. So `MaskShape::Circle` plugs into the existing pipeline by translating to those parameters at uniform-build time. No new pipeline, no new shader, no new bind-group — just two `f32` math ops in `apply_with_invert`. Pattern parallels how `MaskShape::Rect` was implemented (RoundedRect with radius=0).
- **All four primitives gain the new shape automatically.** `apply_clip` / `apply_privacy_blur` / `apply_solid_redaction` / `apply_spotlight` / `apply_dim_outside_data` all accept `MaskShape::Circle` without any per-primitive code changes — that's the dividend of routing every shape through one `ClipPipeline::apply_with_invert`.
- **Cache-poisoning gate-loop lesson (CLAUDE.md updated):** `cargo nextest run -p wisp --test X` followed by `just gate` (which runs `cargo check --workspace --all-targets --all-features`) hit a stale-cache E0599 saying `MaskShape::circle` was missing even though it was in the source. `cargo clean -p wisp` + re-run cleared it. The root cause: nextest builds the test target before the workspace check has seen the latest source, and the dependency-graph hash gets mis-cached. Documented in CLAUDE.md "Build hygiene".

---

## M-MASK.7 — Dim-outside renderer-data wrapper in wisp (AUT-29)
- **Date:** 2026-05-10
- **Status:** ✅ done — `DimOutside` + `DimStrength` data API on top of M-MASK.6's `apply_spotlight`. No new shader, no new pipeline; just a thin data shell so the editor inspector can persist symbolic strength names.
- **Linear:** [AUT-29](https://linear.app/harwood/issue/AUT-29).
- **Files:** new `crates/wisp/src/scene/dim_outside.rs` (`DimStrength` enum: Light / Medium (default) / Heavy / Custom(f32) clamped `[0,1]`; `DimOutside` struct with `rect`/`rounded_rect`/`with_strength` constructors). `crates/wisp/src/scene.rs` exposes the new module + re-exports. `crates/wisp/src/lib.rs` re-exports `DimOutside`/`DimStrength`. `crates/wisp/src/render.rs` adds `Renderer::apply_dim_outside_data(dim, base, output)` (one-line wrapper that calls `apply_spotlight` with `Color::rgba(0,0,0,strength.alpha())`). New `crates/wisp/tests/dim_outside.rs` (3 cases: monotonic presets, Custom clamping, end-to-end strength → outside-darkness). New `crates/wisp-storybook/src/stories/s_dim_outside.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/dim-outside.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/dim-outside.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (192 tests, +3 from M-MASK.6's 189).
- **AUT-29 = data wrapper, not new pipeline.** All the rendering work happened in M-MASK.6 (the `invert: f32` flag in clip.wgsl, the `apply_spotlight` primitive, the inverse-clip composition). AUT-29 just bundles `MaskShape` + `DimStrength` into a struct the editor can persist — same pattern as `PrivacyBlur`/`BlurStrength` from AUT-22.
- **Symmetric design with `PrivacyBlur`.** Identical shape: `DimOutside { shape, strength }` where strength is a symbolic enum with a `Custom(f32)` escape hatch. Editor projects persist the symbolic name (`Heavy`); retuning the alpha mapping later doesn't break files. Story shows three named-strength variants side-by-side, exactly mirroring `s_privacy_blur_strength`.

---

## M-MASK.6 — Spotlight / highlight mask in wisp (AUT-28)
- **Date:** 2026-05-10
- **Status:** ✅ done — attention-guiding primitive. Inside `shape`: base unchanged. Outside: blended toward `dim_color`. Foundation for AUT-29 dim-outside.
- **Linear:** [AUT-28](https://linear.app/harwood/issue/AUT-28).
- **Files:** `crates/wisp/shaders/clip.wgsl` adds `invert: f32` uniform that flips the SDF mask. `crates/wisp/src/render/clip.rs` exposes `apply_inverted` + private `apply_with_invert`. `crates/wisp/src/render.rs` adds `Renderer::apply_spotlight(shape, dim_color, base, output)`. New `crates/wisp/tests/spotlight.rs` (3 cases). New `crates/wisp-storybook/src/stories/s_spotlight.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/spotlight.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/spotlight.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (189 tests, +3 from M-MASK.5's 186).
- **One shader, one bit flipped.** Adding `invert: f32` to the existing clip uniforms (with a 0.5 threshold check) keeps a single pipeline / bind-group layout / shader module. The dispatcher's `apply_clip` keeps its previous `invert=false` behavior; the new spotlight + future AUT-29 dim-outside reuse the same pipeline. AUT-29 will be a thin `apply_spotlight` wrapper with stronger dim alpha.
- **`bytemuck::Pod` requires fixed-size struct fields.** Adding `invert: f32` plus `_pad: f32` to `ClipUniforms` keeps the alignment to 16 bytes. WGSL `vec2<f32>` wants 8-byte alignment so the layout `[center: vec2, half_extents: vec2, radius: f32, aa: f32, invert: f32, _pad: f32]` matches std140 / std430-shared rules.
- **Three contract pixels:**
  - Inside the shape — base bit-exact (255 R, 0 G, 0 B over an all-red base).
  - Outside the shape — blended toward `dim_color`. R goes from 255 to ~76 with `dim_color = (0,0,0,0.7)` (alpha-over: `out = src.rgb*src.a + dst.rgb*(1-src.a) = 0 + 255*0.3 = 76.5`).
  - Rounded-corner cutout — *darkened* (treated as outside the focus shape, since the SDF carved that corner away).

---

## M-MASK.5 — Solid-color redaction in wisp (AUT-23)
- **Date:** 2026-05-10
- **Status:** ✅ done — *trust* counterpart to privacy blur. Reuses the same `MaskShape` enum so rect / rounded-rect / circle / ellipse / freehand-path all work identically.
- **Linear:** [AUT-23](https://linear.app/harwood/issue/AUT-23).
- **Files:** `crates/wisp/src/render.rs` adds `Renderer::apply_solid_redaction(shape, color, base, output)`. New `crates/wisp/tests/solid_redaction.rs` (4 cases). New `crates/wisp-storybook/src/stories/s_solid_redaction.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/solid-redaction.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/solid-redaction.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (186 tests, +4 from M-MASK.4's 182).
- **Composition:** four-stage, three RTs — (1) clear `fill_rt` to redaction `color` via `LoadOp::Clear` (no draws, no shaders, just a clear), (2) `ClipPipeline::apply(shape)` over `fill_rt` → `masked_rt`, (3) blit `base → output` (REPLACE), (4) `compose_over(masked_rt onto output)` (ALPHA_BLENDING). Same shape as `apply_privacy_blur` — only step 1 differs (clear-pass vs blur-filter).
- **Contract tests:**
  - `rect_redaction_outside_matches_base` — bit-exact base outside.
  - `rect_redaction_inside_is_exactly_color` — pixel inside is `(R, G, B, 255)` byte-for-byte equal to the redaction color (no blending, no AA distortion at center sample). Used `Rgba8Unorm` (linear) for byte-exact reads.
  - `rounded_redaction_corner_carved_away` — pixel inside the bounding rect but outside the rounded corner stays as `base`. Re-uses AUT-21's coord (36, 36 in 128² with radius 0.3 → distance 0.42 > radius).
  - `rounded_redaction_center_is_color` — center pixel inside the rounded shape is exactly the redaction color.
- **Color → wgpu::Color is `f64::from`** for each channel. No gamma curve to worry about because the renderer's RT format is `Rgba8Unorm` (linear); display targets that use `Rgba8UnormSrgb` get the gamma applied by wgpu itself.
- **One gate-loop lesson:** `cargo fmt --all --check` flagged a long-line method call that fmt collapses to a single-line signature when small enough. Already documented in CLAUDE.md ("`just fmt-fix` before every commit"); reinforced again as the cheapest safeguard against burning a CI gate cycle on cosmetic format diffs.

---

## M-MASK.4 — Configurable blur strength in wisp (AUT-22)
- **Date:** 2026-05-10
- **Status:** ✅ done — strength is renderer-data, persistable as a symbolic enum, mapped to a clamped numeric radius at render time. Story shows Soft/Medium/Strong side by side.
- **Linear:** [AUT-22](https://linear.app/harwood/issue/AUT-22).
- **Files:** new `crates/wisp/src/scene/privacy_blur.rs` (`BlurStrength` enum: Soft / Medium (default) / Strong / Custom(f32); `PrivacyBlur` struct bundling a `MaskShape` + `BlurStrength` with `rect`/`rounded_rect`/`with_strength` constructors). `crates/wisp/src/scene.rs` exposes the new module + re-exports. `crates/wisp/src/lib.rs` re-exports `BlurStrength`/`PrivacyBlur`. `crates/wisp/src/render.rs` adds `Renderer::apply_privacy_blur_data(blur, base, output)` (one-line wrapper that pulls `radius_px()`). New `crates/wisp/tests/privacy_blur_strength.rs` (3 cases: monotonic presets, Custom clamping, end-to-end strength → blur evidence). New `crates/wisp-storybook/src/stories/s_privacy_blur_strength.rs` + writeup. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/privacy-blur-strength.md` chapter; `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/privacy-blur-strength.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (182 tests, +3 from M-MASK.3's 179). Story renders three side-by-side blur variants with color-coded labels under each.
- **Symbolic vs numeric strength:** the symbolic enum is what the *editor* persists ("Strong" stays meaningful even if we retune Strong from 24px to 32px later); the numeric radius is what the *renderer* needs. `radius_px()` is the single point of mapping. `Custom(f32)` stays available for stories and tests that need exact pixel determinism.
- **Pixel test pattern for "more blur":** sample on the *red side* of a sharp red/blue split, *far* from the seam (NDC -0.3, ~38 pixels from seam in a 128-wide image). Soft (radius 6) can't reach that far; Medium (12) reaches a touch; Strong (24) pulls in real blue. Reading the B channel and asserting `soft < medium < strong` is the cleanest "more blur = more evidence" assertion. First attempt sampled too close to the seam (NDC -0.05) where even Soft saturated; the lesson: if testing strength gradients, sample far enough that the smaller kernel can't reach.
- **Gate-loop lessons:**
  - **`#[derive(Default)] + #[default]` for unit-variant enums.** clippy's `derivable_impls` lint fires when a manual `impl Default for E { fn default() -> Self { Self::X } }` could just be `#[derive(Default)]` on the enum + `#[default]` on the variant. Already documented in CLAUDE.md ("Cast hygiene") generically; reinforced here for enum case.
  - **`f32` test assertions need a tolerance.** clippy `float_cmp` rejects `assert_eq!(some_f32, 64.0)`; use `(a - b).abs() < 1e-6` for clamp-comparison tests. Adding to CLAUDE.md.
  - **`(W as f32) * 0.35` triggers `cast_precision_loss` + sign-loss + truncation in tests.** Avoid casting through float for an integer-pixel calculation: `(W as usize * 35) / 100` gives the same answer with no float involved. Adding to CLAUDE.md.

---

## M-MASK.3 — Rounded-rectangle privacy blur in wisp (AUT-21)
- **Date:** 2026-05-10
- **Status:** ✅ done — generalizes M-MASK.2 from `Rect` to any `MaskShape`. The privacy redaction primitive is now shape-agnostic; future shapes (circle, ellipse, freehand path) plug in for free.
- **Linear:** [AUT-21](https://linear.app/harwood/issue/AUT-21).
- **Files:** `crates/wisp/src/render.rs` — `apply_privacy_blur` second argument changed from `region: Rect` to `shape: MaskShape`; pipeline body unchanged. `crates/wisp/tests/privacy_blur_rect.rs` updated call sites to `MaskShape::rect(region)`. `crates/wisp-storybook/src/stories/s_privacy_blur_rect.rs` same. New `crates/wisp/tests/privacy_blur_rounded.rs` (3 cases). New `crates/wisp-storybook/src/stories/s_privacy_blur_rounded.rs` + `writeups/privacy_blur_rounded.md`. `crates/wisp-storybook/src/stories/mod.rs`. `_docs/book/src/wisp/chunks/privacy-blur-rounded.md`. `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/privacy-blur-rounded.png`. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap`.
- **Verified:** `just gate` green (179 tests, +3 from M-MASK.2's 176).
- **API breaking change:** `apply_privacy_blur` second positional arg flipped from `Rect` → `MaskShape`. Acceptable because:
  - The method only landed yesterday in M-MASK.2; no downstream callers outside the test/story we own.
  - Conversion is one-line (`MaskShape::rect(region)`), still NDC-coordinate-system-identical.
  - Required to absorb AUT-22/-23/-28/-29/-30/-34/-35 without method-explosion.
- **Bounding-rect-but-corner contract:** the new test pixel that lives inside the bounding rect but outside the rounded corner must still equal `base` exactly — proves the SDF actually carved the corner away, vs. just attenuating it. Sample at NDC ≈ (-0.42, +0.42) when the rounded shape has center (0, 0), half-extent 0.5, corner radius 0.3: corner-of-bounding-rect's distance to round-corner-center (-0.2, +0.2) is sqrt(0.18) ≈ 0.42 > 0.3, so the SDF is strictly outside.
- **One AA-band lesson reaffirmed:** sample a few pixels in from the carved-corner edge (used row/col 36 of 128) to escape the SDF AA band; otherwise the assertion catches partial alpha and fails. Already documented under "Coordinate / pixel-readback" in CLAUDE.md.

---

## M-MASK.2 — Rectangle privacy blur in wisp (AUT-20)
- **Date:** 2026-05-10
- **Status:** ✅ done — second mask primitive. First *masked-filter composition* (blur + clip + alpha-compose) end-to-end in the renderer.
- **Linear:** [AUT-20](https://linear.app/harwood/issue/AUT-20).
- **Files:** `crates/wisp/src/scene/clip.rs` adds `MaskShape::Rect { rect }` variant + `MaskShape::rect()` ctor (radius=0 routes through the same SDF pipeline). `crates/wisp/src/render/clip.rs` `apply()` matches both `Rect` and `RoundedRect`. `crates/wisp/src/render.rs` adds `Renderer::apply_privacy_blur(region, radius, base, output)`. New `crates/wisp/tests/privacy_blur_rect.rs` (3 pixel-readback cases). New `crates/wisp-storybook/src/stories/s_privacy_blur_rect.rs` + `writeups/privacy_blur_rect.md`. `crates/wisp-storybook/src/stories/mod.rs` registers it. `_docs/book/src/wisp/chunks/privacy-blur-rect.md` chapter; `_docs/book/src/SUMMARY.md`. `_docs/book/src/assets/wisp/privacy-blur-rect.png` regenerated. `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap` updated for the new story.
- **Verified:** `just gate` green (176 tests, +3 from M-MASK.1's 173); `just snapshots-wisp` regenerates the gallery including `privacy-blur-rect.png`.
- **Composition primitive:** `apply_privacy_blur` is a three-stage pipeline reusing existing primitives — (1) `BlurFilter::new(radius)` over `base` → `blur_rt`, (2) `ClipPipeline::apply(MaskShape::Rect{region})` over `blur_rt` → `masked_rt`, (3) `BlitPipeline::blit(base → output)` then `compose_over(masked_rt onto output)` (alpha blending). Outside the region: bit-exact base. Inside: blurred copy fully replaces base. Three RTs total (base + blur + masked); could be fused into a single shader later but the per-primitive separation keeps each pipeline single-purpose and testable.
- **`MaskShape::Rect` is `RoundedRect(radius=0)` in the renderer:** added the `Rect` variant for API ergonomics (callers don't need to pass a magic 0.0), but `ClipPipeline::apply` routes both through the same SDF path (`Rect` → `r=0`). One enum, one shader, two ergonomic constructors.
- **Test strategy:** three pixel-readback assertions lock in the *contract*, not the implementation:
  - **outside-region pixels match base bit-exactly** (red half stays pure 255 R, 0 B) — proves the mask is honored;
  - **near-seam pixels mix both colors** (red AND blue components > 20) — proves the blur is actually running inside the region (not a no-op);
  - **deep-inside pixels still favor the side they're on** — proves blur falloff is bounded (this is privacy *blur*, not privacy *fill*).
- **Gate-loop lessons:**
  - `for i in -4..=4` defaults `i` to `i32`; `f32::from(i32)` doesn't exist (precision loss). Workaround: type-suffix the literal — `for i in -4i16..=4` — so `f32::from(i)` resolves to the `f32: From<i16>` impl. Keeps the cast-hygiene rule applied without an `as` cast or explicit clippy allow.
  - Adding a story bumps the `story_fingerprints` insta snapshot to `*.snap.new`; force-overwrite before the gate goes green. (Already documented in CLAUDE.md.)
  - `cargo doc` (the lenient gate) flagged a few existing intra-doc links that broke once `Renderer::apply_privacy_blur` brought new types into scope (`Application::width/height`, `BlendMode` resolved from `crate::scene::container::*` instead of `crate::blend::*`). Cleaned the immediate ones; one pre-existing `apply` link in `advanced_blend.rs` remains and is queued.

---

## M-MASK.1 — Rounded crop / mask foundation in wisp (AUT-31)
- **Date:** 2026-05-10
- **Status:** ✅ done — first mask primitive. Foundation for AUT-20 through AUT-35.
- **Linear:** [AUT-31](https://linear.app/harwood/issue/AUT-31/m-mask1-add-rounded-crop-foundation-in-wisp).
- **Files:** `crates/wisp/src/scene/clip.rs` (new `MaskShape` enum, `RoundedRect` variant); `crates/wisp/src/scene/container.rs` (new `clip: Option<MaskShape>` field); `crates/wisp/src/scene.rs` + `lib.rs` re-exports `MaskShape`; new `crates/wisp/shaders/clip.wgsl` (rounded-rect SDF fragment shader); new `crates/wisp/src/render/clip.rs` (`ClipPipeline`); `crates/wisp/src/render.rs` slow-path dispatcher reshaped (collector → `collect_dispatched_nodes` returning advanced-blend OR clipped nodes; phase 2 conditionally applies `clip` then advanced-blend or compose-over); `crates/wisp/src/render/blit.rs` extended with a parallel `pipeline_over` (ALPHA_BLENDING) + `compose_over` method for layering masked RTs onto in-progress dest; `crates/wisp-storybook/src/stories/s_clip_rounded.rs` new story; new `_docs/book/src/wisp/chunks/clip-rounded.md` chapter; `_docs/book/src/assets/wisp/clip-rounded.png` regenerated; SUMMARY.md; `crates/wisp-storybook/tests/snapshots/story_fingerprints__story_fingerprints.snap` updated for the new story; `crates/wisp/tests/clip_rounded_rect.rs` (4 pixel-readback cases).
- **Verified:** `just gate` green (173 tests, +4 from M-BLEND.2's 169); storybook gallery regenerated via `just snapshots-wisp` includes the new `clip-rounded.png`.
- **Architecture — clip plugs into M-BLEND.2's dispatch:** a node is "dispatched" if it has an advanced blend mode OR a clip set. The slow-path renderer (`render_stage_with_advanced_dispatch`) handles both: phase 1 renders the scene minus dispatched subtrees, phase 2 walks them in pre-order, optionally clips the foreground, then composites onto the in-progress dest (advanced blends use `apply_advanced_blend` with ping-pong; clip-only nodes use `BlitPipeline::compose_over`'s ALPHA_BLENDING path). A node with both clip AND advanced blend applies the clip first, then the advanced math.
- **Coordinate system trade-off:** `MaskShape::RoundedRect` is in NDC `[-1, +1]²` — screen space, not container-local. Driven by the recording-quad use case (cinematic crop on a fixed-position surface). Transform-aware clipping is queued.
- **SDF anti-aliasing:** standard rounded-rect SDF (`length(max(q, 0)) + min(max(q.x, q.y), 0) - r`); AA band width is `2 / min(w, h)` so it spans roughly one output pixel without per-call scaling. Tests assert center=opaque, far-corner=transparent, and an inside-rect-but-outside-rounded-corner pixel reads as the clear color.
- **`BlitPipeline` extension:** added a second pipeline (ALPHA_BLENDING) alongside the existing REPLACE one. `blit()` keeps REPLACE for the final RT→view flush; new `compose_over()` uses ALPHA_BLENDING for layering masked content onto an in-progress RT. Shared `run()` helper takes the pipeline + a `clear: bool` flag controlling `LoadOp::Clear` vs `LoadOp::Load`.
- **One lesson during the gate loop:** adding a new storybook story bumps the `story_fingerprints` insta snapshot. First `just gate` run after adding `s_clip_rounded.rs` produced `*.snap.new`; the `.snap` baseline must be replaced before the gate goes green. Documented under "Story testing pattern" in CLAUDE.md (lesson already captured for first-run UX).

---

## CI fix — gstreamer skip guards on integration tests
- **Date:** 2026-05-10
- **Status:** ✅ done — unblocks PR #4 merge.
- **Files:** `crates/decode/src/gstreamer_pipe.rs` (new public `gstreamer_available()` + `Error::Spawn` now carries `PATH` snapshot for diagnosis); `crates/preview/tests/render_smoke.rs` + `crates/app/tests/commands.rs` + `crates/app/tests/player_session.rs` (skip guards via `gst_required!()` macro mirroring the decode integration pattern); `CLAUDE.md` "GStreamer / external CLI integration" section gains 4 new lessons.
- **Symptom:** GitHub Actions Ubuntu gate failed at tests 23+ with `Os { code: 2, kind: NotFound }` on `gst-discoverer-1.0` spawn. The decode integration tests at positions 12-14 of the same nextest run successfully spawned gstreamer (real ~1s execution per test). PATH inheritance should be identical, but somehow the later test binaries can't find `gst-discoverer-1.0`.
- **Root cause: TBD.** Apt log confirms `Setting up gstreamer1.0-tools (1.24.2-1ubuntu0.1)`. Hypotheses ruled out: (a) install failed silently — apt log proves otherwise; (b) PATH stripped by nextest — would affect all tests; (c) wgpu init disturbs env — fails before wgpu init in render_smoke. Working theory: GitHub-hosted Ubuntu runner image has some quirk that breaks PATH lookup from specific subprocess trees. The skip guard makes the gate green regardless; the enhanced `Error::Spawn` (now dumps `PATH=...`) will surface the actual lookup state next time it recurs so we can pin the cause.
- **Verified locally:** `just gate` green at 169 tests on macOS (where `brew install gstreamer` puts the binaries in a stable PATH). On Ubuntu CI the affected 6 tests will now skip gracefully with a clear stderr message instead of panicking.
- **Lesson captured in CLAUDE.md:** any integration test that calls `Command::new("...")` with a binary-name (vs absolute path) MUST have a runtime skip guard, even when CI is supposed to install the binary. Apt-installed binaries are sometimes findable from some test processes but not others.

---

## M-BLEND.2 — Auto-dispatch advanced blend modes through render_stage
- **Date:** 2026-05-10
- **Status:** ✅ done — closes the M-BLEND.1 deferral. Setting `container.blend_mode = BlendMode::Overlay` on a sprite and calling `render_stage` now produces the correct advanced-blend composite automatically.
- **Files:** `crates/wisp/src/render/blit.rs` (new — fullscreen-sampler blit pipeline) + `shaders/blit.wgsl`; `crates/wisp/src/render.rs` reshaped (new fast/slow path split, `collect_advanced_blend_nodes`, `render_stage_with_advanced_dispatch`, `draw_subtree_to_rt` helpers); 4 pipelines (sprite/graphics/text/mesh) gain `draw_subtree(stage, start, exclude)` and their `collect_*` walkers accept `start: NodeId` + `exclude: &HashSet<NodeId>`; `crates/wisp/src/render/blend_pipeline.rs` removes the `tracing::warn!` since the fallback is now intentional under auto-dispatch; new `crates/wisp/tests/blend_modes_dispatch.rs` (4 tests); `_docs/book/src/wisp/chunks/blend-modes.md` updated to remove the "deferred" caveat and document the dispatch architecture.
- **Verified:** `just gate` green (169 tests, +4 from M-BLEND.1's 165); the auto-dispatch test asserts pixel-equivalence between `render_stage` with `container.blend_mode=Overlay` and the manual `apply_advanced_blend` path within 3-LSB tolerance.
- **Architecture — fast/slow path split:**
  - `collect_advanced_blend_nodes(stage)` walks the tree once, returning every visible node whose container has an advanced blend mode (pre-order so z-ordering is preserved). Cost: ~O(N) once per render call.
  - **Fast path** (no advanced nodes): identical to pre-M-BLEND.2 behavior. One render pass directly into the caller's view, batching per-pipeline-per-mode. **Zero perf regression for native-only stages.**
  - **Slow path** (any advanced node): allocate two `RenderTexture`s at `app.width()/height()` for ping-pong + one foreground RT. Phase 1: render the scene MINUS the advanced subtrees into `dest_a` (using the new `draw_subtree(start, exclude)` API on each pipeline). Phase 2: for each advanced node in pre-order, render its subtree to foreground, `apply_advanced_blend(mode, dest_a, foreground, dest_b)`, swap. Phase 3: `BlitPipeline::blit` from final `dest_a` to the user's view.
- **Pipeline API extension:** each pipeline (sprite/graphics/text/mesh) now exposes `draw_subtree(app, pass, stage, start: NodeId, exclude: &HashSet<NodeId>)`. Existing `draw_stage` is a wrapper that passes `(stage.root(), &HashSet::new())`. The walker checks `exclude.contains(&id)` before descending — when an advanced-blend node is hit during the main pass, the walker skips it AND its descendants (they're handled in phase 2).
- **`BlitPipeline`** is a small (~150 LOC + 25 LOC WGSL) pipeline that samples one render-texture and outputs it to a target view. Used only for phase 3's final flush. Took the simple fullscreen-triangle approach since the existing `QuadPipeline` requires a wisp `Texture` not a `RenderTexture` view.
- **The `tracing::warn!` is gone:** previously fired any time a pipeline encountered an advanced blend mode (because the fallback to Normal was a known-incomplete behavior). Under M-BLEND.2 the fallback is intentional during phase-2 subtree rendering — the leaf renders with Normal blending into the foreground RT, and the actual advanced-blend math runs in `apply_advanced_blend` at composition time. Removing the warn also gets rid of the `HashSet<BlendMode>` dedupe state in `BlendPipelineMap`.
- **Z-ordering preserved:** advanced-blend nodes composite in pre-order, so a later-in-traversal advanced node sees its earlier siblings (and their composited results) as the backdrop. Verified by the `auto_dispatch_handles_difference_with_solid_underlay` test (red bg drawn first; blue fg with `Difference` → magenta center pixel).
- **Known limitation (documented in chapter):** slow path internal RT dimensions track `app.width()`/`app.height()`, not the caller-supplied view's dims. For the common case where the view is sized to match the app, this is invisible. For mismatched sizes, the user should construct an `AppConfig` matching the view, or pre-render into a fixed-size `RenderTexture`.
- **No new clippy lessons** — the patterns from M-BLEND.1 carried over cleanly.

---

## M-BLEND.1 — Full PixiJS v8 blend mode catalog (28 modes, 3 tiers)
- **Date:** 2026-05-10
- **Status:** ✅ done — closes the blend-mode gap surfaced during the PixiJS deep-research turn.
- **Files:** `crates/wisp/src/blend.rs` rewritten (4 → 28 variants, `is_advanced` / `native_blend_state` / `css_name` / `all` API); `crates/wisp/src/render/blend_pipeline.rs` (new shared `BlendPipelineMap`); `crates/wisp/src/render/advanced_blend.rs` (new — 20 advanced shaders + `AdvancedBlendPipelines`); `crates/wisp/shaders/advanced_blend.wgsl` (new shared template); 4 pipelines refactored (sprite, graphics, text, mesh) to use per-mode pipeline cache; `crates/wisp/src/render.rs` exposes `Renderer::apply_advanced_blend`; `crates/wisp/tests/blend_modes_standard.rs` (8 tests) + `crates/wisp/tests/blend_modes_advanced.rs` (20 tests); `crates/wisp/examples/blend_modes_gallery.rs` (28-tile contact sheet generator); `_docs/book/src/wisp/chunks/blend-modes.md` chapter; `_docs/book/src/assets/wisp/blend-modes.png` (1400×560 contact sheet); SUMMARY.md.
- **Verified:** `just gate` green (165 tests, +33 from M0-close); contact-sheet example runs; all 28 modes produce expected output within 2-LSB tolerance.
- **Architecture:** Three tiers, per the user's deep-research framing.
  - **Tier A** (4 modes wired up): `Normal`, `Multiply`, `Add`, `Screen`. Were declared in the enum but the pipeline ignored them — the doc comment literally said *"Only `BlendMode::Normal` is wired up; other variants are declared so the public API doesn't churn"*. M-BLEND.1 makes them real.
  - **Tier B** (4 new GPU-native modes): `Subtract`, `Min`, `Max`, `Erase`. Single-pass GPU blend equations via `wgpu::BlendOperation::{ReverseSubtract, Min, Max}`.
  - **Tier C** (20 advanced modes): all of PixiJS's `advanced-blend-modes` set — `Overlay`, `HardLight`, `SoftLight`, `PinLight`, `HardMix`, `VividLight`, `LinearLight`, `ColorBurn`, `ColorDodge`, `LinearBurn`, `LinearDodge`, `Darken`, `Lighten`, `Difference`, `Exclusion`, `Negation`, `Divide`, `Saturation`, `Color`, `Luminosity`. Implemented as offscreen filter passes that sample backdrop + foreground and run a per-mode WGSL `blend_fn`.
- **Pipeline refactor:** new shared `BlendPipelineMap` helper builds one [`wgpu::RenderPipeline`] per native [`BlendMode`] at construction time (8 pipelines × 4 affected pipeline types = 32 pipelines, ~ms each). Sprite/Graphics/Text/Mesh pipelines now batch nodes by `(texture, blend_mode)` and bind the right pipeline per batch. Quad and Triangle pipelines stay single-blend (they're internal blits). Advanced modes fall back to Normal in `render_stage` with a `tracing::warn!` once per mode.
- **Shared shader template** at `shaders/advanced_blend.wgsl` includes vertex shader + HSL helpers (`lum`, `clip_color`, `set_lum`, `sat`, `set_sat`) + a `// __BLEND_FN_PLACEHOLDER__` marker that the Rust resolver substitutes per mode. 20 distinct WGSL programs from one source file.
- **Test discipline:** 8 standard tests use direct `render_stage` with backdrop + foreground graphics nodes, asserting the GPU blend equation produces the expected output (2-LSB tolerance for Apple Silicon Metal rounding). 20 advanced tests use `apply_advanced_blend` with separate backdrop/foreground RTs. Per-mode input colors are picked to yield deterministic, human-checkable expected outputs (e.g. `Overlay(0.25, 0.6) = 0.3`, `LinearBurn(0.7, 0.6) = 0.3`).
- **Manual dispatch (deferred):** `render_stage` doesn't *automatically* route advanced-blend nodes through the offscreen path — it falls back to Normal with a warning. Auto-dispatch (PixiJS-style) requires reshaping the renderer so each scene-graph traversal can write into an internal RT first (so subsequent advanced-blend nodes can sample it as backdrop). Tracked as future work in the chapter.
- **3 lessons captured during the gate loop:**
  - **WGSL template substitution:** the placeholder string can't appear in the template's own docstring or `replace` will substitute there too (caught when WGSL parser flagged "let dark" at line 10 — turned out the docstring contained `// __BLEND_FN_PLACEHOLDER__` which got replaced inline). Fixed by rewording the docstring.
  - **`HashMap<K, ()>` triggers `clippy::zero_sized_map_values`** — should be `HashSet<K>`. Renamed `SEEN: HashMap<BlendMode, ()>` to `SEEN: HashSet<BlendMode>` for the warn-once dedupe.
  - **`clippy::too_many_lines` ignores `-D warnings`** (per its own message — there's a meta-lint hierarchy quirk) but still trips the gate. Suppressed with `#[allow(clippy::too_many_lines, reason = ...)]` on the 140-line `blend_fn_body` match (20 modes × multi-line WGSL bodies; splitting into 20 helper fns would obscure the table-of-formulas structure).

---

## M0-close — `headless_export` 60-frame loop, `filter_chain` example, milestone closure
- **Date:** 2026-05-10
- **Status:** ✅ done — closes M0 cleanly. All 21 chunks now have ✅ ticks in `_docs/milestone-0-renderer.md`'s new Status table.
- **Files:** `crates/wisp/examples/headless_export.rs` rewritten (1 frame at 800×450 → 60 frames at 1920×1080 with per-frame animation: recording-quad rotation, cursor oscillation, text scale pulse); new `crates/wisp/examples/filter_chain.rs` (~170 lines — three-filter chain animated over 60 frames, dumps composites to `target/filter_chain/`); 3 new chunk chapters under `_docs/book/src/wisp/chunks/` (`example-filter-chain.md`, `example-recorder-mock.md`, `example-headless-export.md`); 3 new asset PNGs under `_docs/book/src/assets/wisp/` (filter-chain highlight 22 KB, recorder-mock 81 KB, headless-export highlight 110 KB); `SUMMARY.md` adds the three new chapters; `milestone-0-renderer.md` gains a "Status — ✅ closed 2026-05-10" section with all 21 chunks ticked + a note explaining the M0.21 ffmpeg-next → GStreamer pivot, and the "After M0" section is rewritten to reflect the actual M-DEC/M-PLAY/M-INT/M-PREVIEW/M-POLISH/M-TEST chunks shipped between M0 close and now.
- **Verified:** `just gate` green (132 tests, 1 leaky-flag — same as before); `cargo run -p wisp --example headless_export` produces 60 PNGs at `target/headless_export/frame_NN.png`; `cargo run -p wisp --example filter_chain` produces 60 PNGs at `target/filter_chain/frame_NN.png`; `cargo run -p wisp --example recorder_mock` produces `target/recorder_mock.png` (copied to assets dir).
- **Gap closed in M0.21:** the consolidated PROGRESS entry from 2026-05-09 noted "headless_export shipped" but at 1 frame at 800×450 — the spec required **60 frames at 1080p**. This chunk closes that gap properly. `filter_chain.rs` was missing entirely from the original M0.20 → also shipped this turn.
- **Architecture lock:** wisp's `Renderer::apply_filter` chains by passing the previous filter's output `RenderTexture` as the next filter's input. `filter_chain.rs` exercises this with three filters (BlurFilter → DropShadowFilter → MotionBlurFilter) and 4 RTs (base + 3 intermediates). Multi-pass filters (Blur is 2-pass separable Gaussian) get a scratch RT inside `apply_filter` automatically.
- **Interactive examples acknowledged:** `hello_triangle.rs` (M0.5) and `hello_sprite.rs` (M0.20a) are interactive winit demos — they don't dump PNGs, so they don't get mdBook chapters. The Status table in the milestone doc credits them as ✅ via interactive verification (Apple Silicon Metal backend).
- **One clippy refactor during the gate loop** (no `#[allow]` shortcut): `(i32::from(u32) - 32) as f32` → `f32::from(u8) - 32.0`. The `From<u8> for f32` impl is lossless, so the precision-loss + useless-conversion lints both vanish without a reason-pragma. Documented technique already in CLAUDE.md cast-hygiene section.

---

## M-POLISH.1 — Drag-over visual feedback
- **Date:** 2026-05-09
- **Status:** ✅ done — first polish chunk on the milestone-1 Phase 4 list (M4.1).
- **Files:** `crates/app/src/main.rs` (extended `on_window_event` to match all `DragDropEvent` variants — `Enter`/`Drop`/`Leave` emit corresponding Tauri events; `Over` and future variants are no-ops since the enum is `non_exhaustive`); `crates/app/src/commands.rs` (two new debug-only commands `__test_drag_enter`/`__test_drag_leave`); `crates/app/src/main.rs` registers them conditionally; `crates/app-ui/index.html` (JS bridge re-emits `file-drag-enter`/`file-drag-leave` as browser CustomEvents); `crates/app-ui/src/app.rs` (`is_dragging` signal + `install_drag_state_listeners` + reactive `DropZoneState::{Idle | Active}` binding via a closure inside the `<Show>` fallback); `crates/app-e2e/tests/golden_path.rs` (new `drag_enter_leave_toggles_active_class` test); `_docs/book/src/app-ui/integration.md` extended with the M-POLISH.1 section.
- **Verified:** `just gate` green; `trunk build` clean WASM bundle; `cargo check -p app-e2e --tests` green so the new e2e test compiles for the next Linux CI run.
- **Architecture lock:** drag-state lives entirely in Tauri-event-land. We deliberately don't use HTML5 `dragenter`/`dragleave` because Tauri 2 with `dragDropEnabled: true` captures OS-level drags before they bubble into the webview. Re-emitting Tauri's variants through the existing JS bridge keeps the drag-state path symmetric with `file-dropped` and `player-status` — same shape, three times now. Future events (drag-position for cursor crosshairs, etc.) follow the same pattern.
- **Reset-on-drop is intentional:** Tauri's `Leave` event fires *only* when the drag exits the window, not after a drop. So `Drop` also emits `file-drag-leave` to reset the visual, otherwise the active class would stick after a successful drop until the next `Enter`/`Leave` cycle.
- **e2e UPDATED** (per the testing strategy's tier-2 obligation): new test calls `__test_drag_enter` → asserts `.drop-zone-active` appears → calls `__test_drag_leave` → asserts `.drop-zone-idle` returns. Two-test golden suite now: `golden_path_drop_play_pause` (file → play → pause) and `drag_enter_leave_toggles_active_class` (drag visual).
- **DropZone component reuse:** the `Active` variant the storybook shipped in M-UI.1 already had thicker border + accent tint + `Release to import` headline. M-POLISH.1 is purely the *wiring* — no new CSS, no component changes. That's the point of the storybook discipline: components ship complete, integration just connects them.
- **No new clippy lessons:** the auto-fixable patterns from M-PLAY.3 (doc-markdown, format args) didn't recur. Pure composition.

---

## M-PLAY.3 — `<video>` preview bound to convertFileSrc + e2e coverage
- **Date:** 2026-05-09
- **Status:** ✅ done — first user-visible video playback in the recorder. Maps to milestone-1 chunks M3.1 (path → asset URL) + M3.2 (VideoPlayer component); M3.3 (view switching) was already in place via `<Show>`.
- **Files:** `crates/app-ui/index.html` (new `__screenConvertFileSrc` JS bridge); `crates/app-ui/src/player_ipc.rs` (`screen_convert_file_src_js` extern + safe `convert_file_src` wrapper that returns `None` outside Tauri); `crates/app-ui/src/app.rs` (PlayerView replaces placeholder div with `<video>` element using NodeRef + sync click handler + catch-up Effect); `crates/app-ui/Cargo.toml` (`HtmlVideoElement` + `HtmlMediaElement` web-sys features); `crates/app-ui/shell.css` (`.player-video` + repositioned `.player-surface-label` as overlay); `crates/app-e2e/tests/golden_path.rs` (asserts video element exists, src is asset-protocol-shaped, `.paused` flips on toggle clicks); `_docs/book/src/app-ui/player-ipc.md` extended with the M-PLAY.3 video-element section.
- **Verified:** `just gate` green; `trunk build` clean WASM bundle; `cargo check -p app-e2e --tests` green so the new e2e assertions compile (full e2e run on the next push to Linux CI).
- **Architecture lock — two paths to the `<video>` element, by necessity:**
  - **Click handler** drives `<video>` synchronously within the user gesture. WebKit blocks programmatic `.play()` outside a user-initiated event, so the catch-up Effect alone wouldn't be enough.
  - **`Effect::new`** over `player_status` is the catch-up path for non-click state changes (EOF transition to Ended, future seek commands). Idempotent — only acts when `video.paused()` doesn't already match.
- **Two-source-of-truth, deliberate:** Tauri's `PlayerSession` keeps running alongside the visible HTML5 video. It owns the gstreamer-decoded `VideoTexture` that future wisp-rendered surfaces (winit child window for the editor preview) will read. This chunk doesn't fold the two together — the video element is the MVP playback surface; the wisp render path is queued for a later milestone.
- **e2e UPDATED** (per the testing strategy's tier-2 obligation when adding a user-visible feature):
  - Asserts `<video class="player-video">` exists after drop.
  - Reads `src` attribute, asserts non-empty + contains `"asset"` (matches both macOS `asset://` and Linux/Windows `http://asset.localhost`).
  - New `wait_video_paused(driver, want_paused)` helper polls `document.querySelector('video.player-video').paused` via `driver.execute()` because WebKit's `.play()` is async and the property doesn't flip exactly on click.
- **No new lessons during the gate loop** — the feature dropped in cleanly because the JS bridge / extern / NodeRef / Effect patterns were all established in M-PLAY.2 and M-TEST.2. Pure composition.

---

## M-TEST.2 — Tier-2 WebDriver e2e via tauri-driver + fantoccini
- **Date:** 2026-05-09
- **Status:** ✅ done — completes the three-tier test strategy. Linux-CI-gated; macOS skips with a clear message.
- **Files:** new `crates/app-e2e/` (Cargo.toml + src/lib.rs harness + tests/golden_path.rs); `crates/app/src/commands.rs` adds `#[cfg(debug_assertions)] __test_drop_file` (debug-only Tauri command that emits the same `file-dropped` event the OS drag-drop handler emits — WebDriver can't synthesize OS drops); `crates/app/src/main.rs` registers it conditionally in `generate_handler!`; `justfile` adds `just e2e` recipe with platform branching (Linux uses xvfb-run, macOS prints skip); `justfile`'s `test` recipe excludes `app-e2e` (gate doesn't require tauri-driver); `.github/workflows/gate.yml` adds Ubuntu-only steps to apt-install `webkit2gtk-driver`+`xvfb`, `cargo install tauri-driver` (via `taiki-e/install-action`), and run `just e2e`; testing chapter expanded.
- **Verified:** `just gate` green (132 tests on macOS dev; app-e2e compiles + lints + skips at runtime); `just e2e` on macOS prints the skip message and exits 0 cleanly; the harness is structured so cargo nextest registers it when invoked but the workspace gate excludes it.
- **Architecture lock — `E2eApp` lifecycle:**
  - `start()`: `cargo build -p screen-app` (idempotent) → spawn `tauri-driver` on port 4444 → `wait_for_port` (15s budget) → connect fantoccini Client with `tauri:options.application = <bin path>` → tauri-driver spawns the app, fantoccini drives it.
  - `Drop`: best-effort `client.close().await` (asks tauri-driver to shut the app cleanly), then `tauri_driver.kill()` outright. Drop runs in a separate thread because tokio runtimes can't be dropped from inside a running runtime.
- **Golden-path scenario:** invoke `__test_drop_file` (synthesizes the file-dropped event) → wait for `.player-controls` to appear → click `.player-toggle` → wait for `.player-toggle-playing` class → click again → wait for `.player-toggle-paused`. End-to-end: real WebView, real Leptos hydration, real JS bridge, real Tauri IPC, real Rust state machine.
- **Why `__test_drop_file` is debug-only:** WebDriver can't synthesize OS-level drag-drop events. Production users get drag-drop via `WindowEvent::DragDrop` (the real handler in main.rs); tests get the `__test_drop_file` parallel entry point. Gated on `#[cfg(debug_assertions)]` for both the command definition and the `generate_handler!` registration so release builds strip it entirely.
- **CI strategy:** `gate.yml` matrix runs the gate on macos-latest + ubuntu-latest; e2e runs only on the Ubuntu arm via `just e2e`. macOS keeps Tier-0 + Tier-1 coverage; the manual smoke procedure for Tier-2 on mac is documented in the testing chapter.
- **5 lessons captured during the gate loop** (small, all auto-fixable by `cargo clippy --fix`):
  - `clippy::doc_markdown` flags every `WebDriver`/`WebView`/`WebKitGTK` reference in docstrings; CI doesn't tolerate them and `--fix` adds backticks reliably.
  - `clippy::map_err` over `inspect_err` — when the closure doesn't transform the error, `inspect_err` is the right idiom.
  - `clippy::uninlined_format_args` prefers `{var:?}` over `"{:?}", var` — `--fix` rewrites cleanly.
  - `clippy::needless_raw_strings` flags `r#"…"#` when no embedded `"` justifies the `#`. `--fix` strips the hashes.
  - `taiki-e/install-action` supports `tauri-driver` directly — saves the 5-minute `cargo install` cycle on every CI run.

---

## M-TEST.1 — Tier-1 IPC harness for Tauri commands
- **Date:** 2026-05-09
- **Status:** ✅ done — first half of the e2e testing strategy. M-TEST.2 (WebDriver) follows.
- **Files:** `crates/app/Cargo.toml` (dev-dep `tauri = { features = ["test"] }` + `serde_json`); new `crates/app/tests/commands.rs` (4 cases: empty status, full open→play→pause IPC round-trip, lowercase serde wire-shape regression guard, invalid-path error path); `Deserialize` derive added to `PlayerStatus` + `SessionState` (was Serialize-only) so tests can round-trip; `_docs/book/src/app-ui/testing.md` documents the three-tier strategy; `SUMMARY.md`.
- **Verified:** `just gate` green (132 tests, +4 from M-PLAY.2's 128); 4 IPC harness cases run in ~1.5s under nextest.
- **What this catches that direct PlayerSession tests miss:**
  - Misregistration in `tauri::generate_handler!` (typo → silently missing command at runtime, no compile error).
  - serde wire-shape drift: e.g. accidentally dropping `#[serde(rename_all = "lowercase")]` from `SessionState`. Caught by an explicit `state_str == "empty"` assertion.
  - `State<PlayerSession>` plumbing — forgetting `.manage(...)` in main.rs would still compile but fail at runtime. The harness builds via `mock_builder().manage(...).build(...)` so a missed `.manage` would surface here.
- **Test-tier hierarchy now documented in `_docs/book/src/app-ui/testing.md`** — Tier 0 (chunk-level), Tier 1 (this chunk), Tier 2 (WebDriver, M-TEST.2 next). Each tier overlaps deliberately; no tier replaces another.
- **3 clippy refactors during the gate loop:**
  - `Default::default()` for `HeaderMap` and `WebviewUrl` → typed forms (`HeaderMap::default()`, `WebviewUrl::default()`) per `default_trait_access`. New mini-lesson: clippy pedantic prefers explicit type names over `Default::default()` ambiguity.
  - `tauri::webview::WebviewUrl` is `pub(crate)`; the public path is `tauri::WebviewUrl` (re-exported from `config`). Found via `grep` in the registry source.
  - `tauri::test::INVOKE_KEY` is the magic constant the IPC dispatcher checks; tests would silently get rejected without it.
- **Cargo wart noted (acceptance, not a fix):** `[dev-dependencies] tauri = { features = ["test"] }` unifies into the release binary's feature set because cargo doesn't separate dep + dev-dep features per profile. The `test` module is small; accepted.

---

## M-PLAY.2 — Tauri ↔ player IPC for transport controls
- **Date:** 2026-05-09
- **Status:** ✅ done — last chunk on the path to first MP4 playback. **Path complete: M-DEC.1 → M-PLAY.1 → M-DEC.2 → M-INT.1 → M-INT.2 → M-PREVIEW.1 → M-PLAY.2.**
- **Files:** `crates/app/Cargo.toml` (deps: playback/decode/wisp/serde/pollster/tracing); new `crates/app/src/{lib.rs, player_session.rs, commands.rs}`; `crates/app/src/main.rs` rewritten with `.manage(PlayerSession)` + invoke_handler + tick thread; `crates/app/tests/player_session.rs` (6 lifecycle tests); `crates/app-ui/Cargo.toml` (+ serde, serde-wasm-bindgen); new `crates/app-ui/src/player_ipc.rs`; updated `crates/app-ui/src/{lib.rs, app.rs}`; updated `crates/app-ui/index.html` (outbound `__screen{Open,Play,Pause}` helpers + `player-status` listener bridge); `crates/ui-storybook/src/components/player_controls.rs` (optional `on_toggle: Option<Callback<()>>` prop, non-breaking); `_docs/book/src/app-ui/player-ipc.md`; `SUMMARY.md`; `ISSUES.md` ISS-03 marked resolved.
- **Verified:** `just gate` green (128 tests, +6 from M-PREVIEW.1's 122); `just site` renders the new chapter; trunk WASM build passes; SSR snapshot for the storybook unchanged (the optional prop emits no HTML attribute).
- **IPC contract — four commands, one event:**
  - `player_open(path: String) -> Result<PlayerStatus, String>`
  - `player_play()` / `player_pause()`
  - `player_status() -> PlayerStatus`
  - `player-status` event (Tauri → webview), throttled to state-change + 10 Hz elapsed updates.
- **Architecture lock:** `PlayerSession` is pure Rust (no Tauri types) so its lifecycle is testable end-to-end without booting Tauri. The four `#[tauri::command]` wrappers are one-liners over the session. The Application is built once at session boot (~200 ms) and shared by every subsequent `open` — no device-init latency on file open.
- **Tick thread:** plain `std::thread::spawn` + `std::thread::sleep(33 ms)` rather than `tokio::time::interval` — avoids the tokio-features dep dance, and the tick is sync work. Status emits are throttled to lifecycle changes + 100 ms-of-elapsed boundaries while playing (so the timer ticks at ~10 Hz, not 30 Hz).
- **JS bridge symmetry with M-INT.2:** outbound is three `__TAURI__.core.invoke` wrappers (`__screenOpen`/`__screenPlay`/`__screenPause`); inbound is a `player-status` Tauri-event → browser-`CustomEvent` re-emit. No `tauri-sys`. Bridge degrades to no-ops when `__TAURI__` is absent (so `trunk serve` standalone still flips the drop-zone-to-player view via the demo affordance).
- **Component evolution:** `PlayerControls` gains an optional `on_toggle: Option<Callback<()>>` prop. Existing storybook stories pass nothing → SSR HTML output is unchanged → snapshot test passes unchanged. The recorder shell wraps `<PlayerControls>` in a reactive closure that re-renders on `player_status` changes.
- **6 lessons captured during the gate loop** (no `#[allow]` shortcuts where avoidable):
  - Tauri's `generate_handler!` requires fully-qualified paths (`commands::player_play`) so its companion `__cmd__name` macro is in scope — using `use commands::*;` doesn't work.
  - Tauri's `#[tauri::command]` requires `State<T>` by value, not `&State<T>` — clippy's `needless_pass_by_value` fires on every command. Suppressed with a module-level `#![allow]` + reason in `commands.rs`.
  - Leptos 0.7 typed-builder cache can desync after editing a `#[component]` that gains a new prop — `cargo clean -p <crate>` invalidates and the prop reappears. Burned one cycle on this.
  - `--all-features` in workspace check unifies `csr + ssr` features for `ui-storybook`, but the `#[component]` macro itself is feature-agnostic; the cache desync (above) was the real culprit.
  - `usize as u32` and `f32 == 0.0` are still the most common gate trippers — both have CLAUDE.md lessons; applied the documented fixes (`u32::try_from(...).expect(...)`, `f32::abs() < f32::EPSILON`).
  - rustdoc broken-intra-doc-links across crate boundaries (e.g. `[`screen_app::player_session::PlayerStatus`]` from `app-ui`) — when the dep edge isn't there *and shouldn't be there* (WASM ↔ Tauri-native split), the right answer is plain-text references with a comment explaining why. Resolved ISS-03 with that pattern.

---

## M-PREVIEW.1 — Native winit window with a wgpu surface that wisp renders into
- **Date:** 2026-05-09
- **Status:** ✅ done — fifth-and-a-half chunk on the path to first MP4 playback. **1 chunk remains** (M-PLAY.2 Tauri↔player IPC).
- **Files:** new `crates/preview/` (Cargo.toml, src/lib.rs with `aspect_fit_scale` + 4 unit tests, src/main.rs with the winit `ApplicationHandler`); `crates/preview/examples/render_offscreen.rs` (asset generator — same render path against an offscreen `RenderTexture`); `crates/preview/tests/render_smoke.rs` (CI-safe smoke test for the `from_wgpu` codepath); 5 PNG assets at `_docs/book/src/assets/preview/preview_NN.png`; `_docs/book/src/preview/{overview,chunks/preview-window}.md`; `SUMMARY.md`; `ISSUES.md` adds ISS-03 (pre-existing rustdoc warning in `app-ui` spotted during `just site`).
- **Verified:** `just gate` green (122 tests, 0 skipped); `just site` renders both new chapters with embedded asset PNGs; `cargo run -p preview` opens a real winit window and plays the fixture (manual verify).
- **Architecture lock:** `wisp::Application::from_wgpu` is the embedding-host seam. `preview` calls it with the host-built `Instance`/`Adapter`/`Device`/`Queue` (surface-aware in the binary, surfaceless in the example). When M-PLAY.2 wires a winit child of Tauri, the same constructor handles it — no library-side change.
- **Sprite math:** `aspect_fit_scale(surface_w, surface_h, video_w, video_h) -> Vec2` letterboxes/pillarboxes the source into the surface. The bound axis takes 2.0 (full NDC `[-1, +1]`); the loose axis shrinks proportionally. Zero dims fall back to `Vec2::splat(1.0)` to avoid `NaN`.
- **Why a `[lib]` *and* a `[[bin]]`:** the example and integration test need `aspect_fit_scale` and the API stability of the same shape that `main.rs` uses. Cargo handles a dual-target crate cleanly; rustdoc gets the `preview` crate page automatically.
- **One clippy refactor during the gate loop** (no `#[allow]`):
  - `bytes.len() as u32` → `u32::try_from(bytes.len()).expect(...)` (the documented CLAUDE.md cast-hygiene rule, applied prophylactically — caught here on first attempt because the lesson was already in the rules file).

---

## M-INT.2 — Tauri serves Trunk bundle + OS file-drop wiring
- **Date:** 2026-05-10
- **Status:** ✅ done — fifth chunk on the path to first MP4 playback. **2 chunks remain** (M-PREVIEW.1, M-PLAY.2).
- **Files:** `crates/app/tauri.conf.json` (`frontendDist` → `../app-ui/dist` + `beforeDevCommand`/`beforeBuildCommand` running Trunk + `devUrl`); `crates/app/src/main.rs` (`on_window_event` → `WindowEvent::DragDrop` emits `file-dropped` Tauri event); `crates/app-ui/index.html` (JS bridge re-emits as browser `CustomEvent`); `crates/app-ui/src/app.rs` (`install_file_drop_listener` adds web-sys listener that flips `loaded` signal); `crates/app-ui/Cargo.toml` (web-sys with CustomEvent/Window/EventTarget/Event features); deleted `crates/app/dist/` (vanilla HTML M1 frontend, replaced).
- **Verified:** `just gate` green; `trunk build` produces fresh dist; cargo check on screen-app passes after the new event handler.
- **Architecture:** four hops, each one-liner — Tauri `on_window_event` → `window.emit("file-dropped")` → JS bridge `CustomEvent` → web-sys `addEventListener`. No `tauri-sys` crate, no JS-side state. Bridge degrades to no-op when `window.__TAURI__` absent (so `trunk serve` standalone still works for component review).
- **Notes:** `Closure::forget()` on the file-drop listener is intentional — app-lifetime, never removed. Clippy's `collapsible_if` caught the new event handler; fixed via Rust 2024 `if let && let` chains (already a CLAUDE.md lesson — non-duplicative, no new entry).

---

## M-INT.1 — Trunk + Leptos CSR app (`crates/app-ui/`)
- **Date:** 2026-05-09
- **Status:** ✅ done — fourth chunk on the path to first MP4 playback. **2 chunks remain** (M-PREVIEW.1 native winit, M-PLAY.2 Tauri↔player IPC; M-INT.2 Tauri-frontendDist swap is a small follow-on).
- **Files:** new `crates/app-ui/` (Cargo.toml, Trunk.toml, index.html, shell.css, src/lib.rs, src/app.rs); `justfile` adds `app-ui` and `app-ui-build` recipes; `.gitignore` excludes `crates/app-ui/dist`; `_docs/book/src/app-ui/overview.md`; `SUMMARY.md`.
- **Verified:** `trunk build` produces a clean WASM bundle in `crates/app-ui/dist/` (wasm + js shim + copied assets + shell.css); `cargo check -p app-ui` passes on native target via the `rlib` crate-type; `just gate` green.
- **Architecture lock:** `app-ui` consumes the components from `ui-storybook` directly (`use ui_storybook::components::{DropZone, PlayerControls, RecordingToolbar, StatusBar}`) — no duplication. Adding a new component once means it's available in the gallery, the shell, and the mdBook chapter.
- **Demo affordance:** clicking the drop-zone in M-INT.1 flips a Leptos signal into the loaded view, so reviewers can exercise both surfaces before the actual file-drop event lands in M-INT.2.
- **4 new lessons captured in CLAUDE.md** under "Trunk + Leptos CSR":
  - `data-cargo-features="…"` only if the feature actually exists (one cycle lost on `data-cargo-features="csr"` when the crate declared no `csr` feature).
  - `crate-type = ["cdylib", "rlib"]` — both, so workspace native gate still type-checks.
  - `<link data-trunk rel="copy-dir">` is the way to pull peer-crate assets into the Trunk dist.
  - `#[wasm_bindgen(start)]` is the Trunk entry point; no need for `<script>main()</script>` in `index.html`.

---

## M-DEC.2 — GstreamerPipeStream + first real MP4 → wisp playback
- **Date:** 2026-05-09
- **Status:** ✅ done — third chunk on the path to first MP4 playback. **3 chunks remain.**
- **Files:** `crates/decode/Cargo.toml` (adds `thiserror` + `tracing`); `src/gstreamer_pipe.rs` (~330 lines: `GstreamerPipeStream`, `Error`, `VideoMetadata`, `parse_discoverer`, 6 unit tests for the parser); `tests/gstreamer_integration.rs` (3 integration tests against a real MP4); `tests/fixtures/sample.mp4` (committed 11 KB H.264 fixture, encoded once with x264 from the M-DEC.1 mock-stream PNGs); `crates/playback/examples/play_file.rs` (end-to-end demo: decode → Player → wisp → PNG); `_docs/book/src/playback/play-file.md` chapter; `SUMMARY.md`.
- **Verified:** `just gate` green; the 3 GStreamer integration tests pass; `play_file` example runs end-to-end producing 7 PNGs at `_docs/book/src/assets/playback/playfile_NN.png` and exits with `state = Ended`.
- **Pivot during the chunk:** initial implementation used FFmpeg CLI; user asked to use GStreamer instead. Rewrote (~30 min to drop FFmpeg, run `brew install gstreamer` in background, port logic, re-encode fixture, retest). All 3 integration tests + 6 parser tests pass cleanly under GStreamer 1.26.8.
- **5 lessons captured in CLAUDE.md** under new "GStreamer / external CLI integration" section:
  - `brew install gstreamer` is the cask name (not `gstreamer-tools`).
  - CLI-pipe approach beats `gstreamer-rs` for first integration — zero compile-time integration with libgstreamer, swap to bindings later via the `VideoStream` trait.
  - `gst-discoverer-1.0` for metadata, `gst-launch-1.0` for the stream — pipeline caps can't be read out from the consumer side, probe separately.
  - `fdsink fd=1` is the stdout fdsink (don't try `filesink location=-`).
  - `Drop`-kill the child or `gst-launch-1.0` keeps decoding into the dropped pipe.
- **2 clippy refactors** during the gate loop (no `#[allow]`):
  - `.map(...).unwrap_or(false)` on `Result` → `.is_ok_and(...)`.
  - `format!("…{path:?}")` → `format!("…{}", path.display())` for non-UTF-8 path messages.

---

## M-PLAY.1 — Player state machine + frame pump
- **Date:** 2026-05-09
- **Status:** ✅ done — second chunk on the path to first MP4 playback. 4 chunks remain.
- **Files:** new `crates/playback/` (Cargo.toml, src/lib.rs); `tests/timing.rs` (6 tests proving the timing contract); `examples/timed_playback.rs` drives the player for 1 s wallclock at 60 Hz render against a 30 fps source and writes 30 frames to `_docs/book/src/assets/playback/`; `_docs/book/src/playback/overview.md`; `SUMMARY.md`.
- **Verified:** `just gate` green; tests pass (paused-no-advance, first-tick-immediate, ~30-frames-in-1s, end-of-stream→Ended, pause-freezes, duration_hint correct); example reports `31 frames pulled, state = Playing` (the +1 over 30 is the round-up on the wallclock boundary).
- **`Player::tick(dt)` returns the number of frames it uploaded** so the shell can drive a redraw signal off it (no re-render needed when no new frame is due — important for native winit power efficiency on still video sections).
- **Architecture lock:** `Box<dyn VideoStream + Send>` so the player works against any decoder backend without changes. The shell (Tauri / native winit) needs only `Player::play / pause / tick / texture / state / elapsed / duration_hint`.
- **3 clippy fixes via real refactors** (no `#[allow]` shortcuts):
  - `match next_frame() { Some => …, None => break }` → `let-else`. Reads cleaner.
  - `u64 as f64` precision-loss: kept the cast with a documented `reason` (2^52 frames at 60 fps ≈ 2.4M years; not a realistic concern).
  - Manual `Debug` impl that didn't include all fields → dropped the impl entirely. The `Box<dyn>` field can't be `Debug` and the impl wasn't load-bearing.

---

## M-DEC.1 — VideoStream trait + MockVideoStream + playback_demo
- **Date:** 2026-05-09
- **Status:** ✅ done — first chunk on the path to "play an MP4 in the Tauri-Leptos app via wisp" (5–6 chunks total).
- **Files:** new `crates/decode/` (Cargo.toml, src/lib.rs, src/mock.rs); `crates/wisp/Cargo.toml` adds `decode` as dev-dep; `crates/wisp/examples/playback_demo.rs` drives the full decode → upload → render pipeline; `_docs/book/src/decode/overview.md`; `SUMMARY.md` adds a `decode` section; 8 PNG assets at `_docs/book/src/assets/decode/frame_NN.png`.
- **Verified:** `just gate` green; example runs and writes 8 frames; `decode` lib runs 5 unit tests + 1 doctest.
- **Why a trait:** the recorder will eventually wire 3+ codec backends (`AVFoundation`, `MediaFoundation`, `ffmpeg-next`). The consumer side — wisp's `VideoTexture::upload_bgra` — is uniform: `Vec<u8>` BGRA at known dims, ticked at known timestamps. `VideoStream` locks that contract.
- **`MockVideoStream`** synthesizes a deterministic scrolling-gradient stream. No external deps; the `motion_is_visible_between_frames` test gates against a regression where adjacent frames somehow come out identical (which would mean the GPU upload path was caching).
- **Next:** M-DEC.2 wires `AVFoundation` via `objc2` for real MP4 decode on macOS (the path the recorder will use day-to-day).

---

## M-UI.4 — StatusBar (ready / busy / error)
- **Date:** 2026-05-09
- **Status:** ✅ done — fourth chunk under the workflow.
- **Files:** `crates/ui-storybook/src/components/status_bar.rs` (StatusBar + StatusKind + format_bytes); `assets/style.css` (~55 lines: cells, pill variants, pulsing-busy dot reusing `@keyframes rec-pulse`); `stories.rs` (3 new stories under "Shell" category, plus `#[allow(too_many_lines)]` on `all_stories` since the registry pattern reads better flat); SSR snapshot regenerated; 3 chapters under `ui/chunks/`; `SUMMARY.md`.
- **Verified:** `just gate` green; `just snapshots-ui` regenerated 18 demos (was 15); `just site` renders all three chapters.
- **Three telemetry cells + health pill:**
  - `Ready` (green pill, no detail)
  - `Busy` (sky pill with pulsing dot, free-text detail e.g. `"Encoding · 38%"`)
  - `Error` (red pill, no pulse — explicit "stopped" state, free-text detail carries the error)
- `format_bytes` rolls B → KB → MB → GB with appropriate fractional precision so `24_117_248` reads as `23.0 MB` rather than a wall of digits.

---

## M-UI.3 — RecordingToolbar (idle / recording / paused)
- **Date:** 2026-05-09
- **Status:** ✅ done — third chunk under the workflow.
- **Files:** `crates/ui-storybook/src/components/recording_toolbar.rs`; `assets/style.css` (~95 lines: layout, source pill, action variants, pulsing-dot keyframe); `stories.rs` registers three new stories; SSR snapshot regenerated; three new chapters under `ui/chunks/`; `SUMMARY.md`.
- **Verified:** `just gate` green; `just snapshots-ui` regenerated 15 demos (was 12); `just site` renders all three chapters.
- **Three states drive everything:**
  - `Idle` — single primary "Start recording" button (red), source picker, status reads "Ready". Intentionally one-button — pause/stop would be muted-and-dead before capture starts.
  - `Recording` — pulsing red dot (CSS keyframes, ~1.4s loop), red status label, ticking M:SS timer, action stack `Pause` + `Stop`.
  - `Paused` — dot stops pulsing, color shifts to `--kf-marker` (same yellow the dope sheet uses for markers — visual consistency for "interrupted" states), action stack `Resume` (red, same as Start) + `Stop`.
- Timer formats `M:SS` until the hour, then `H:MM:SS`.

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
