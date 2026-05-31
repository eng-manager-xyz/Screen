# M-EDIT export pipeline — build-ready plan (ED.20 / ED.21)

> Synthesized from a 5-agent read-only research sweep of the compose /
> encode / decode-seek / golden-frame subsystems (2026-05-30). This is the
> implementation contract for the export chunks; revise as code lands.

## Crate placement (load-bearing)

- **The generator lives in `crates/app/src/editor_export.rs`** (new). `app`
  is the only crate depending on `edit` + `decode` + `media` + `wisp`.
  **`edit` MUST stay wasm-clean** (app-ui depends on it), so the generator
  cannot live there.
- The compose extension (zoom/crop applied to the screen sprite) **extends
  `app::editor_preview::EditorPreview`**, which already wraps
  `app::recording_compose::RecordingCompose`. So **preview and export share
  one code path by construction** — the founding "never cut the negative"
  rule.

## Compose internals (what ED.20 builds on)

- `RecordingCompose::compose_frame(&mut self, camera_slot, screen_slot) -> Option<ComposedFrame>`
  — pulls BGRA from slots, `set_screen_frame` / `set_camera_frame`, calls
  `Renderer::render_stage(app, target.view(), Color::BLACK, scene.stage())`,
  then `RenderTexture::read_pixels` → packed BGRA `ComposedFrame`.
- `EditorPreview::render_frame(bgra) -> Option<ComposedFrame>` (ED.6) wraps
  the decoded frame into `screen_slot` (cam slot empty) and delegates.
- `wisp` scene: `RecordingScene` builds a fullscreen screen `Sprite`
  (anchor 0.5, scale 2.0 filling NDC) + a cam `Sprite` in an
  `Ellipse`-clipped `Container`. `Sprite { container, texture, anchor, tint }`;
  `Container { transform, alpha, visible, blend_mode, clip: Option<MaskShape>, … }`;
  `Transform::to_mat3` composes `T(pos)·S(scale)·R·K·T(-pivot)`.
- `set_screen_frame` flips top-down→bottom-up internally (wisp +y
  convention); producers pass standard top-down BGRA.
- Clip path: a clipped subtree renders to an offscreen RT, SDF mask
  multiplied into alpha, composited back — this is how the cinematic
  rounded-corner canvas (ED.18) would apply.

## ED.20 — deferred frame generator

New on `EditorPreview`: `render_framed(bgra, zoom: ZoomTransform, crop: CropRect) -> Option<ComposedFrame>`:
writes a crop-then-zoom transform into the screen sprite's
`Container.transform` (via `scene.stage_mut().get_mut(scene.screen_sprite_id()).container_mut().transform`),
then calls `compose_frame`.

**Transform math** (base sprite anchor 0.5 / scale 2.0 fills NDC):
1. **Crop** selects sub-rect `[x,y,w,h]` of source: pre-scale sprite by
   `(1/crop.width, 1/crop.height)`, offset position by the crop-center
   delta in NDC.
2. **Zoom** multiplies scale by `zoom.scale` and offsets position so the
   focal NDC point `(2*fx-1, -(2*fy-1))` stays fixed:
   `position += focal_ndc * (1 - zoom.scale)`.
   Compose **crop then zoom**, in that order.

**Generator loop** — for each project frame `f` in `0..project.project_duration()`:
1. `src = project.source_time(f)` (`None` ⇒ stop).
2. `vframe = stream.frame(src)` on `EditorVideoStream` (forward walk never
   re-spawns; LRU cache absorbs 2×/slow-mo repeats).
3. `zoom = edit::zoom_anim::active_zoom_at(project, f)`;
   `crop = project.crop.unwrap_or(CropRect::full())`.
4. `composed = preview.render_framed(vframe.bgra, zoom, crop)`.
5. `pts = Duration::from_micros(f * (1_000_000 / project.project_fps))`
   — **identical formula to `feed_real_capture` (recording.rs:677)** so
   encoder timestamps match the live path.

Compose dims = `project.aspect.canvas_dims(long_edge)` clamped via
`media::encode::fit_within_encoder_limits` (vtenc 4096/edge); construct
`EditorPreview::new` at those clamped dims so source frames reframe (not
stretch).

**Golden-frame test** — `crates/app/tests/editor_export_golden.rs` (new),
gst-guarded (`if !media::gstreamer::is_available() { return; }`), mirrors
`wisp-chart-web/tests/render_gantt.rs` + `editor_pipeline.rs`. Assert the
composed frame for a fixed project-frame is deterministic.

## ED.21 — end-to-end edited export

- **Reuse `media::encode::LiveGstreamerEncoder` UNCHANGED** via the
  `VideoEncoder` trait + `build_live_video_args`
  (`fdsrc fd=0 ! rawvideoparse format=bgra ! vtenc_h264_hw ! h264parse ! mp4mux ! filesink`)
  and `build_remux_args` at finalize.
- **Reuse `app::recording::EncoderHandle`**: add
  `EncoderHandle::start_with_edited_project(encoder_config, project, source_path)`
  mirroring `start_with_real_capture` (recording.rs:498). It spawns a feed
  thread running a new `feed_edited_export(encoder, cancel, project, source_path, framerate)`
  modeled on `feed_real_capture` (recording.rs:644): **build the generator
  INSIDE the thread** (wgpu `Application` isn't `Send` — same reason
  `RecordingCompose` is built in-thread). Loop
  `while !cancel { match gen.next_frame() { Some((bgra,pts)) => enc.push_video_frame(&bgra,pts), None => break } }`.
- **Audio = a second gst-launch pass, NOT `push_audio_chunk`.** The source
  is a single muxed MP4 (no `.f32` scratch at edit time). Finalize the
  **video-only** edited stream first (`has_audio=false` ⇒ finalize just
  moves the intermediate), then a second `gst-launch` builds the edited
  audio and muxes it onto the video. Per `TimelineSegment`: seek-trim
  `[source_start, source_end)` of the source audio, apply `timescale` via
  `speed speed=<ts>` (or `pitch tempo=<ts>` to preserve pitch), feed a
  single `concat name=acat ! audioconvert ! audioresample ! avenc_aac/opusenc ! mux.`;
  ~2 ms crossfade at joins via `audiomixer` to mask resample clicks. Skip
  the whole pass if the source has no audio track (probe via
  `media::encode::scratch_has_audio`-style `gst-discoverer`).
- Add **`build_edited_audio_args(project, source, video_in, output) -> Vec<String>`**
  next to `build_remux_args` in `media/src/encode.rs` — split out so it's
  **unit-testable without spawning gst** (matches the existing
  `build_*_args` test convention). `generate_poster(final)` at the end like
  `export_recording`.
- Spawn under `tauri::async_runtime::spawn_blocking` like `export_recording`
  (commands.rs:2248).

**E2E test** — `crates/app/tests/edited_export_e2e.rs` (new), gst-guarded,
mirrors `editor_pipeline.rs`. Open `decode/tests/fixtures/sample.mp4`; build
an `EditProject` with trim + 2×-speed segment + zoom + crop +
`AspectRatio::Vertical`. `EncoderConfig::for_output(tmp.mp4, Mp4H264Aac)`
with dims = `fit_within_encoder_limits(project.aspect.canvas_dims(720), …)`.
`start_with_edited_project` → finalize → assert: (1) output exists,
non-empty; (2) `gst-discoverer-1.0` reports video at the clamped dims + a
duration consistent with `project_duration()/project_fps` (the 2× segment
must have **halved** its source span — proves retiming); (3) audio track
present if the fixture has one. Plus a pure unit test of
`build_edited_audio_args`. **Assert `stream.spawn_count()==1`** after the
full generate to lock the forward-only (no re-spawn) invariant.

## Risks / sequence

1. ED.15 (crop + aspect ops) and ED.18 (background framing) feed the
   compose transform — land crop before wiring `render_framed`'s crop arm,
   or stub `CropRect::full()`.
2. wgpu `Application` not `Send` forces the in-thread generator build.
3. Sequence: `render_framed` + transform math → generator + golden test
   (ED.20) → `feed_edited_export` + `start_with_edited_project` + video-only
   e2e → `build_edited_audio_args` + audio mux pass → progress/cancel
   (ED.22).
