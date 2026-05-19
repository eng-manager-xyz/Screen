# Milestone 2: Record + Export (M-RECORD-EXPORT)

> **Goal:** press one Record button → cam + screen + mic + sys-audio capture in lockstep → wisp composes them into a per-frame texture → GStreamer encodes to a chosen container/codec → file lands on disk → "Reveal in Finder" works. The end-to-end happy path.
>
> **Why now:** M-RECORDER-V0, M-CAM.3, M-BUBBLE.0, M-AUDIO and the capture-completeness PR (#49) shipped enumeration + per-channel pickers + permissions on macOS. The four input streams exist independently. This milestone is about **coordinating them** and **emitting a real artifact**. Without it the recorder is a permission demo.
>
> **One big PR:** filed as `feat: M-RECORD-EXPORT — coordinated capture + multi-format encode + save to disk (14 chunks)` against `main`. Commits per chunk so individual pieces can be rolled back.
>
> **macOS-first.** Every chunk is required to *compile* on Windows + Linux (cfg-gated stubs where needed), but only macOS is required to *work*. Win/Linux real-encoder ports + portal/manifest permissions land as M-RECORD-EXPORT-PORT in a follow-up milestone.

---

## Acceptance criteria

End-to-end on macOS:

- ✅ Recorder surface shows a big red Record button + elapsed `mm:ss` + per-stream health LEDs
- ✅ Click → all 4 enabled streams start within ~500 ms; the 4 per-channel pickers lock with a tooltip
- ✅ A 10-second recording with cam (circular bubble) + screen (primary or picked) + mic + sys-audio produces a playable `.mp4` (or `.webm`) at `~/Movies/Screen/Screen-YYYY-MM-DD-HHMMSS.<ext>`
- ✅ Format dropdown (MP4-H.264 default, MP4-H.265, WebM-VP9, WebM-AV1) honored by the encode pipeline
- ✅ A same-named `.avif` poster lands next to the video
- ✅ "Reveal in Finder" opens the right Finder window
- ✅ Save-As respects a custom path
- ✅ Lipsync within ~80 ms in QuickTime
- ✅ `just gate` green on macOS / Ubuntu / Windows (Ubuntu/Windows: stubs build, real encoders are macOS-only)

---

## Tech notes

- **GStreamer is the only media library.** No ffmpeg-next. Per-codec pipelines built per-OS via a small string-builder in `crates/media/src/encode.rs`.
- **HW encoders, macOS:** `vtenc_h264_hw` / `vtenc_h265_hw` (H.265 unavailable pre-M3? — gated by runtime probe), `vtenc_av1_hw` (M3+ only — fall back to `svtav1enc` when missing). VP9 has no Apple HW path; use `vp9enc` (libvpx-vp9) software encoder.
- **HW encoders, Windows (scaffold only):** `mfh264enc`, `mfhevcenc`, `mfvp9enc` (Win 11+ Media Foundation), `qsvh264enc` for Intel, `nvh264enc` for NVIDIA. AV1 via `qsvav1enc` (Arc) or `nvav1enc` (RTX 40+).
- **HW encoders, Linux (scaffold only):** `vaapih264enc` / `vaapih265enc` / `vaapivp9enc` / `vaapiav1enc` for Intel/AMD VAAPI; `nvh264enc` / `nvh265enc` / `nvav1enc` for NVIDIA NVENC. Software fallback: `x264enc`, `vp9enc`, `svtav1enc`.
- **Audio:** AAC for MP4 (`avenc_aac` from gst-libav, LGPL-acceptable), Opus for WebM (`opusenc`). Mic + sys-audio mixed via `audiomixer` before encode.
- **A/V sync:** PTS = `Instant::now() - session.started_at` for every video frame + audio chunk. GStreamer's `videorate` + `audiorate` after the sources smooths jitter.
- **Frame readback:** wisp's `RecordingScene` composes to a `wgpu::Texture` per frame; we map a staging buffer + push BGRA bytes into `appsrc`. The texture is single-buffered (latest-frame-wins) — encoder runs at 30 fps independent of source FPS.
- **Save dialog:** `tauri-plugin-dialog` (already in the Tauri 2 plugin family — minimal new surface, just a dep + capability + JS bridge).
- **AVIF poster:** post-stop one-shot pipeline `filesrc location=<mp4> ! decodebin ! videoconvert ! videoscale ! avifenc ! filesink`. Silently skipped (with `tracing::warn`) if `avifenc` element isn't installed.

---

## Chunks

14 chunks total. Numbered M-CAM.4, M-MIC.3, M-SCK.0.1, M-RECORD.0..3, M-EXPORT.0..5, M-RECORD-EXPORT.GATE.

### Phase 1: Routing pre-flight (3 chunks)

#### M-CAM.4 — per-camera routing via `avfvideosrc device-index`
- Thread `camera_id` from the camera picker through `start_preview` → `GstreamerVideoCapture` → `avfvideosrc device-index=<n>` (macOS).
- Map FNV-1a hashed `camera_id` back to its `device-index` via a fresh `list_cameras()` probe on every start (IDs are stable, but indexes may shift; the probe re-resolves).
- Cfg-gate Win/Linux: `mfvideosrc device-path=` / `v4l2src device=/dev/video<n>` — same call site, same resolution pattern.
- **Done when:** clicking a non-default camera in the picker swaps the live preview within ~500 ms on macOS. Unit test for the id → index resolver. Stub builds on Win/Linux.

#### M-MIC.3 — per-mic routing via `osxaudiosrc device-uid`
- `resolve_mic_element` already returns the right prop name (`device-uid` / `device` / `device`); the call site in `from_microphone` ignores it. Wire it through.
- Same id → native-id resolution via `list_microphones()` probe on start.
- **Done when:** clicking a non-default mic in the picker re-routes the level meter to that device within ~500 ms on macOS.

#### M-SCK.0.1 — per-screen-source routing (AUT-291)
- Extend `ScreenCaptureConfig` with `source: ScreenCaptureSource { PrimaryDisplay | Display(String) | Window(String) }`.
- `start_screen_capture(source_id: Option<String>)` — picker passes `Some("display-<id>")` / `Some("window-<id>")` / `None`.
- Use `updateContentFilter_completionHandler` on the live SCStream for swap-in-place (no tear-down + recreate).
- Persist last-used source via LocalStorage; fall back to primary if the persisted id no longer matches a current source.
- **Done when:** clicking a non-primary display or a window row in the screen picker swaps the SCStream within ~500 ms; window capture works.

### Phase 2: Recording orchestrator (4 chunks)

#### M-RECORD.0 — `RecordingSession` state machine + shared clock
- New `crates/app/src/recording.rs`. `RecordingSession { id: Uuid, started_at: Instant, streams: SessionStreams, state: SessionState }`.
- States: `Idle → Starting → Running → Stopping → Idle`. Illegal transitions are a panic in debug, `tracing::error` + no-op in release.
- Per-stream `StreamHealth { lifecycle: Lifecycle, frame_count: u64, last_frame_at: Option<Instant> }`. `Lifecycle` mirrors the per-channel `MicLifecycle` / `ScreenLifecycle`.
- `SessionStreams { camera: Option<CameraHandle>, screen: Option<ScreenHandle>, microphone: Option<MicHandle>, system_audio: Option<SysAudioHandle> }`.
- Failure to start any one stream rolls back the others (best-effort `stop`) and returns `Err(...)`.
- **Done when:** unit tests cover all transitions + the early-return rollback. No Tauri integration yet.

#### M-RECORD.1 — `start_recording` / `stop_recording` / `recording_status` IPC + 500 ms event
- Tauri commands in `crates/app/src/commands.rs`. `RecordingState(Mutex<Option<RecordingSession>>)` registered via `.manage()` in `main.rs`.
- `start_recording(config: RecordingConfig) -> Result<RecordingHandle, String>` — config carries enabled-streams + each stream's picker selection (camera_id, mic native_id, screen source_id, sys-audio app filter) + output path + format.
- `recording-status` event emitted every 500 ms via a `tokio::time::interval` task while session is `Running`.
- **Done when:** macOS smoke — `start_recording(cam+mic config)` runs 5 s, `stop_recording` returns `RecordingSummary` with non-zero per-stream `frame_count`.

#### M-RECORD.2 — `<RecorderControls />` Leptos component
- Replaces the placeholder `RecordingToolbar` in `crates/app-ui/src/app.rs`.
- Big red `<RecordButton />` (filled circle when idle, filled square when recording).
- Elapsed `mm:ss` display driven by the 500 ms `recording-status` event.
- Per-stream health LEDs: green if `last_frame_at < 1 s ago`, yellow if `< 5 s`, red otherwise / when stopped.
- CSS in `crates/app-ui/shell.css`.
- **Done when:** click-to-record-5 s-then-stop works in dev build, elapsed updates live, LEDs flicker green during run.

#### M-RECORD.3 — Lock per-channel pickers during `Running`
- Cam/Mic/SysAudio/Screen picker toggles all subscribe to `recording-status`. When `state == Running`, flip `disabled` attribute + add `title="Recording in progress"` tooltip.
- Re-enabled on Stop.
- **Done when:** opening the picker dropdowns mid-session shows checkboxes disabled with the tooltip.

### Phase 3: Composition + encode (4 chunks)

#### M-EXPORT.0 — wisp `RecordingScene`
- New `crates/wisp/src/recording.rs::RecordingScene`. Composes:
  - Screen frame as fullscreen `Sprite` (BGRA, `wgpu::TextureView` input).
  - Cam frame as a smaller `Sprite` with a circular mask (re-use the M-MASK.* circle SDF), positioned bottom-right by default with 24 px margin.
- `set_screen_frame(&wgpu::Texture)` / `set_camera_frame(&wgpu::Texture)` — latest-frame-wins, no queueing.
- `render(target: &wgpu::TextureView)` — single draw per frame.
- Wisp-storybook story `s_recording_scene_default` renders with two synthetic gradient textures + the bundled Apollo image as "screen".
- PNG snapshot at `_docs/wisp-book/src/assets/wisp/recording-scene-default.png` committed.
- **Done when:** storybook fingerprint test green, chapter shows the composed output.

#### M-EXPORT.1 — `VideoEncoder` trait + `OutputFormat` + pipeline builder
- New `crates/media/src/encode.rs`. `OutputFormat { Mp4H264Aac, Mp4H265Aac, WebmVp9Opus, WebmAv1Opus }` enum.
- `VideoEncoder` trait: `push_video_frame(&self, bgra_bytes: &[u8], pts: Duration)`, `push_audio_chunk(&self, samples: &[f32], pts: Duration)`, `finalize(self) -> Result<PathBuf, EncodeError>`.
- `GstreamerEncoder::new(format, dims, fps, output_path) -> Result<Self, EncodeError>` builds the right pipeline string per-OS + format.
- Pipeline string builder unit-tested for each (format × OS) combo — 4 × 3 = 12 cases.
- Per-OS hot-path:
  - **macOS:** `vtenc_h264_hw` / `vtenc_h265_hw` / `vp9enc` (sw) / `vtenc_av1_hw` (probe; fall back to `svtav1enc` if absent).
  - **Win (scaffold):** `mfh264enc` / `mfhevcenc` / `mfvp9enc` / `qsvav1enc` — pipeline strings present but the worker thread `EncodeError`s out with `Unsupported("encoder not yet wired on Windows")` so the trait + integration code can still ship.
  - **Linux (scaffold):** `vaapih264enc` / `vaapih265enc` / `vaapivp9enc` / `vaapiav1enc` — same scaffold treatment.
- **Done when:** macOS test creates a 30-frame red→black BGRA fade, encodes to `.mp4`, `gst-discoverer-1.0` confirms H.264 30 fps ~1 s duration. Pipeline-string snapshot tests for all 12 (format × OS) combos.

#### M-EXPORT.2 — Audio: mic + sys-audio mix → AAC/Opus → shared mux
- Extend the encoder pipeline with the audio leg. `appsrc` (audio) per source → `audioconvert` → `audioresample` → `audiomixer` → format-specific encoder.
- MP4: `avenc_aac` (preferred) with fallback to `faac`.
- WebM: `opusenc`.
- PTS computed identically to video frames (offset from `session.started_at`).
- **Done when:** 5 s mic-on recording opens in QuickTime with audio visible in the waveform, lipsync within ~80 ms.

#### M-EXPORT.3 — Wire encoder into `RecordingSession`
- `start_recording` constructs the `GstreamerEncoder` from `RecordingConfig.format` + output path; stores it in `SessionStreams`.
- Camera / screen callbacks → wisp's `RecordingScene::set_*_frame` → renderer drains at 30 fps → encoder's `push_video_frame`.
- Mic / sys-audio callbacks → encoder's `push_audio_chunk`.
- `stop_recording` sends EOS down the pipeline, waits for `mp4mux` / `webmmux` to finalize the moov-box / cues, returns the final file path.
- **Done when:** full session start → 5 s run → stop produces a playable file at the configured path.

### Phase 4: File save + format picker + thumbnail (2 chunks)

#### M-EXPORT.4 — Save dialog + format dropdown + Reveal in Finder
- Add `tauri-plugin-dialog` dep + capabilities entry.
- Toolbar gains a "Save as…" button + format dropdown next to the Record button.
- Default path resolved per-OS: `~/Movies/Screen/` (macOS), `~/Videos/Screen/` (Win/Linux). Filename `Screen-YYYY-MM-DD-HHMMSS.<ext>` where `<ext>` matches the format.
- "Reveal in <native explorer>" button appears after `stop_recording` — calls `open -R <path>` (macOS), `explorer /select,<path>` (Windows), `xdg-open $(dirname <path>)` (Linux).
- **Done when:** stopping writes to the default path; Save-As respects the custom path; Reveal opens the right native explorer with the file highlighted.

#### M-EXPORT.5 — AVIF poster-frame thumbnail
- After `stop_recording` succeeds, kick a one-shot post-process: `gst-launch-1.0 -q filesrc location=<mp4> ! decodebin ! videoconvert ! videoscale ! video/x-raw,width=640 ! avifenc ! filesink location=<mp4-without-ext>.avif`.
- Skip silently with `tracing::warn` if the `avifenc` element isn't installed (gst-plugins-bad).
- **Done when:** a 10 s recording produces both the `.mp4` and a same-named `.avif` next to it.

### Phase 5: Polish + gate (1 chunk)

#### M-RECORD-EXPORT.GATE
- Storybook stories for `<RecorderControls />` (idle / recording / stopping states) + `RecordingScene` (default + cam-position variants).
- mdBook chapters for each of the 13 prior chunks at `_docs/book/src/app/chunks/<chunk-id>.md` / `_docs/wisp-book/src/wisp/chunks/recording-scene.md`.
- Add the milestone group to both books' `SUMMARY.md`.
- `just gate` green on all 3 OSes — Win/Linux compile + their tests pass because encoder scaffolds return `Unsupported` gracefully (one cfg-gated integration test per OS verifies the error path).
- macOS manual regression checklist (run + commit `_docs/PROGRESS.md` entry):
  1. Open Recorder surface → all 4 pickers visible + functional.
  2. Click Record → all enabled streams start, elapsed counter runs, LEDs go green.
  3. Pickers are locked mid-record.
  4. Click Stop after ~10 s → toolbar shows "Saved to `<path>`" + Reveal button.
  5. `<path>` is a playable `.mp4` in QuickTime with audio synced.
  6. `<path>.avif` exists next to it and opens in Preview.
  7. Re-record with each format selection → all 4 produce playable files.
- **Done when:** PR opens, CI green, manual checklist all-ticked.

---

## Phase 6 — Real pixel forwarding (M-PIX, 8 chunks) — appended 2026-05-17

The M-RECORD-EXPORT.GATE closeout shipped with a test-pattern encoder feed (solid colour + silence). The recorder produces a real `.mp4` end-to-end but the content isn't the user's actual screen / camera / mic / system audio. **M-PIX replaces the test-pattern feed with real captured frames.**

### Phase 6.1 — Capture-side frame extraction (4 chunks)

#### M-PIX.0 — Shared frame slots + AudioMixer plumbing in `RecordingState`
- Add `camera_frame_slot: Arc<Mutex<Option<Vec<u8>>>>`, `screen_frame_slot: Arc<Mutex<Option<Vec<u8>>>>`, `audio_mixer: Arc<Mutex<AudioMixer>>` to `RecordingState`.
- Slots are latest-frame-wins: capture pipelines overwrite with each new frame; encoder feed thread reads at render time.
- **Done when:** unit tests cover slot semantics + concurrent read/write.

#### M-PIX.1 — Camera worker forwards BGRA to `CameraFrameSlot`
- Extend `CameraPipeline::spawn` worker to clone the BGRA bytes from each `next_frame` into the shared slot.
- Worker takes `Option<Arc<...>>` — None preserves the legacy preview-only behaviour.
- **Done when:** running session sees the slot fill within ~100 ms of preview-up.

#### M-PIX.2 — SCK screen delegate extracts BGRA from `CMSampleBuffer`
- `ScreenOutputHandler::stream_didOutputSampleBuffer_ofType` locks the `CVPixelBuffer`, memcpys out the BGRA bytes (handling row-stride padding), unlocks, writes to `ScreenFrameSlot`.
- Adds `objc2-core-video` dep.
- **Done when:** unit tests cover stride math + a manual macOS smoke shows the slot fills at SCK's framerate.

#### M-PIX.3 — Mic worker pushes F32 samples to shared `AudioMixer`
- Extend mic capture worker to clone F32LE chunks into `AudioMixer.push_mic(samples)`.
- Worker takes `Option<Arc<Mutex<AudioMixer>>>`.
- **Done when:** unit tests verify push_mic alignment; manual smoke confirms `mixer.mic_queued()` grows.

#### M-PIX.4 — SCK audio delegate extracts F32 to `AudioMixer`
- `AudioOutputHandler` extracts F32LE from `AudioBufferList` (handles interleaved vs non-interleaved).
- Pushes to `AudioMixer.push_sys_audio`.
- **Done when:** unit tests cover layout-detection math; manual smoke confirms sys-audio samples reach the mixer.

### Phase 6.2 — Encoder-side composition (3 chunks)

#### M-PIX.5 — wisp render thread with wgpu readback
- New compose worker owns `wisp::Application` + `Renderer` + `RecordingScene` + 1920×1080 `RenderTexture` + wgpu staging buffer.
- 30 fps loop: pull latest cam → `set_camera_frame`; pull latest screen → `set_screen_frame`; render scene; copy RT → staging; map + read BGRA bytes; emit on output channel.
- Handle dimension mismatches via wisp's sprite scaling.
- **Done when:** integration test creates synthetic cam+screen slots, composes 30 frames, asserts the readback bytes are non-zero (real compose happened).

#### M-PIX.6 — Replace test-pattern feed with real-capture feed in `EncoderHandle`
- New `EncoderHandle::start_with_real_capture(config, slots, mixer)`.
- Real-capture feed thread: pull composed BGRA from M-PIX.5 receiver → `push_video_frame`; pull `AudioMixer` → `push_audio_chunk`; pace at framerate.
- `start_recording` picks real-capture path when at least one channel enabled.
- Keep test-pattern path as a debug fallback for "no channels enabled."
- **Done when:** macOS smoke records a real 5-s session, file plays back with actual content.

### Phase 6.3 — Validation (1 chunk)

#### M-PIX.7 — End-to-end macOS smoke + PROGRESS update
- Manual record 10 s with all 4 inputs enabled.
- Verify: real screen content + circular cam overlay; real mic + sys-audio mixed; lipsync ~80 ms; AVIF poster shows real frame.
- PROGRESS.md entry summarising M-PIX phase; PR body refreshed.

### Out of scope for M-PIX (deferred to M-RECORD-EXPORT-PORT)

- **Windows/Linux pixel extraction.** SCK is macOS-only. Windows needs `windows-rs` Graphics.Capture; Linux needs `pipewire-rs`. Both are sizeable separate efforts.
- **Cursor smoothing / zoom effects.** M-CURSOR.
- **Multi-display compose** (record display 1 + display 2 side-by-side). M-SCK.MULTI.

## Out of scope for this milestone

Explicitly punted to follow-up milestones:

- **Windows/Linux real encoders.** Scaffolded only. M-RECORD-EXPORT-PORT.
- **Webcam bubble frame rendering + circle mask in the bubble window itself** (M-BUBBLE.1/.2). The circle mask in the *recorded* output ships here; the live bubble preview stays placeholder.
- **Audio device hot-swap mid-record.** If the user yanks the mic during a session, the existing stream dies; we surface the LED going red but don't auto-reconnect. M-RECORD.4 follow-up.
- **Pause / resume mid-record.** Stop-only for v0.
- **Scene editing post-record** (trim, crop, overlay positioning). M-EDIT.
- **Cursor enhancement** (zoom, magnify, smooth-move). M-CURSOR.
- **Multi-display compose** (record display 1 + display 2 side-by-side). M-SCK.MULTI.
- **Cloud upload / share-link.** M-SHARE.
- **Code-signing / notarization / Windows MSIX / Linux AppImage.** M-DIST.
- **Production-grade permission UX** (first-launch wizard, denied-state recovery, Windows manifest capabilities, Linux portal integration). M-RECP.0..6.
- **Real-time encode preview** (show the encoded output in a corner during record). Optimization, not v0.

---

## Tooling that gets set up along the way

By end of M-RECORD-EXPORT:

- `RecordingSession` orchestrator pattern (1 owner of N stream handles + shared clock) — reusable for any future "coordinate these N things" feature.
- `OutputFormat` + `VideoEncoder` trait — clean seam for swapping encoder backends per-OS.
- wisp `RecordingScene` — the composition primitive any future post-record editor / preview reuses.
- `tauri-plugin-dialog` integrated — unblocks every future "pick a file" / "save a file" UI.
- Per-OS encoder pipeline-string builder pattern — extends to any future "encode this differently per platform" need.

---

## Estimated effort

14 chunks × ~30-60 min each on the macOS hot path + cross-OS scaffold + CI fix loop = **~8-12 hours of focused work**. Single-PR ship gives one big CI run on the matrix rather than 14 mini-runs.

Tracked as tasks in the task list (M-CAM.4 / M-MIC.3 / M-SCK.0.1 / M-RECORD.0..3 / M-EXPORT.0..5 / M-RECORD-EXPORT.GATE).
