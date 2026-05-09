# OpenScreen — Deep-Dive Research

> Source repo: <https://github.com/siddharthvaddem/openscreen>
> Version inspected: `v1.4.0` (`main` branch, May 2026)
> License: **MIT**
> Author: Siddharth Vaddem (`svaddem@asu.edu`)
> Stars/forks at time of research: ~35.5k / ~2.4k
> Tagline: "Create stunning demos for free. Open-source, no subscriptions, no watermarks, free for commercial use. An alternative to Screen Studio."

---

## 1. What it does today (implemented features)

Pulled from the README, [release notes](https://github.com/siddharthvaddem/openscreen/releases), and source code:

**Capture**

- Window or full-screen recording via Electron `desktopCapturer` + `getUserMedia` with `chromeMediaSource: "desktop"` constraints (see [`src/hooks/useScreenRecorder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useScreenRecorder.ts)).
- System audio capture (macOS 13+, Windows native, Linux PipeWire only).
- Microphone capture with a `+1.4×` gain boost mixed via Web Audio API.
- Webcam capture stored as a separate file alongside the screen file.
- Cursor telemetry (x/y normalized 0–1 + timestamp) recorded out-of-band by the Electron main process — not encoded into the video — so cursor effects can be re-rendered in the editor.
- Click capture on macOS via [`uiohook-napi`](https://www.npmjs.com/package/uiohook-napi) (the project's only native module) — see [`scripts/rebuild-native.mjs`](https://github.com/siddharthvaddem/openscreen/blob/main/scripts/rebuild-native.mjs).
- Countdown overlay window before recording starts.
- HUD overlay window (transparent, frameless) hosting recording controls.

**Editor**

- React-based timeline with regions for: zoom, trim, speed, annotation, blur (see [`src/components/video-editor/timeline/`](https://github.com/siddharthvaddem/openscreen/tree/main/src/components/video-editor/timeline)).
- Keyframe markers within zoom regions (`KeyframeMarkers.tsx`).
- Auto-zoom suggestions based on **cursor dwell detection** (450–2600 ms stationary windows under a 0.02-unit movement threshold) — see [`src/components/video-editor/timeline/zoomSuggestionUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/zoomSuggestionUtils.ts).
- Cursor-follow camera with adaptive exponential smoothing (deceleration curve) — [`videoPlayback/cursorFollowUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/cursorFollowUtils.ts).
- Cursor highlight ring & mouse-highlighter slider (v1.4.0).
- Annotations: text, arrows, images, and (in PRs) shapes (rect/ellipse).
- Wallpapers / gradients / transparent / native-aspect-ratio backgrounds — [`src/lib/wallpaper.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/wallpaper.ts).
- Border radius, shadow, padding, motion blur, blur regions — [`src/lib/blurEffects.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/blurEffects.ts).
- Webcam masks (circle/rounded/square + presets), PIP/vertical layout — [`src/lib/webcamMaskShapes.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/webcamMaskShapes.ts), [`src/lib/compositeLayout.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/compositeLayout.ts).
- 3D rotation/perspective pass (X/Y/Z axes with perspective projection) — [`src/lib/exporter/threeDPass.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/threeDPass.ts).
- Project save/load (`save-project-file` / `load-project-file` IPC) with autosave snapshots and unsaved-change confirmation — [`projectPersistence.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/projectPersistence.ts).
- Undo/redo via [`useEditorHistory`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useEditorHistory.ts).
- Configurable keyboard shortcuts — [`ShortcutsContext`](https://github.com/siddharthvaddem/openscreen/blob/main/src/contexts/ShortcutsContext.tsx).
- i18n (English, Spanish, Chinese, zh-TW, Polish, Turkish, Korean, Arabic, Japanese in v1.4) — [`electron/i18n.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/i18n.ts), [`src/i18n/`](https://github.com/siddharthvaddem/openscreen/tree/main/src/i18n).

**Export**

- MP4 (H.264, hardware-accelerated when possible) and GIF.
- Multiple aspect ratios (16:9, 9:16, square, native, custom crop).
- Multiple resolution presets and quality levels.
- Diagnostic dump on failure (`save-diagnostic` IPC).

---

## 2. What's planned / on the roadmap

There is **no formal `ROADMAP.md`** and no `roadmap` issue label. Direction is gleaned from open issues, top reactions, and in-flight PRs:

**Top-reaction open feature requests** ([issues sorted by reactions](https://github.com/siddharthvaddem/openscreen/issues?q=is%3Aissue+is%3Aopen+sort%3Areactions-%2B1-desc)):

- `#397` — Music/voiceover audio tracks
- `#220` — Custom cursor sizes & icons
- `#396` — Cursor click animation & sound
- `#227` — More export resolution choices
- `#239` — Optional progress bar burned into output
- `#541` — Arabic localization (shipped in 1.4)
- `#553` — Camera selection from a list
- `#557` — Proper 9:16 export resolutions

**In-flight PRs** ([pulls](https://github.com/siddharthvaddem/openscreen/pulls)) telegraph a major engineering pivot:

- **Zero-copy hardware-accelerated hybrid FFmpeg export pipeline** — strong signal that the all-WebCodecs export is being supplemented by an FFmpeg path to address Windows/Linux export crashes and slow renders.
- "No webcam" layout preset.
- Continuous custom zoom slider (replacing zoom presets).
- Timeline snap guides + copy/paste of components.
- PNG custom background uploads.
- In-app update notifications.
- Russian and Vietnamese localizations.
- Lazy-load the editor bundle (cold-start time fix).

**Top open bugs** that imply planned work:

- `#256` Export broken on Windows
- `#157` Slow rendering on Linux/Windows
- `#269` Video finalization hangs
- `#558` macOS screen-recording permission re-prompts
- `#540` Export crashes when re-opening an existing project
- `#543` Imported video doesn't load in editor

---

## 3. Tech stack

### Application shell
- **Electron 41.x** + **Vite 7** + `vite-plugin-electron` ([`package.json`](https://github.com/siddharthvaddem/openscreen/blob/main/package.json)).
- **TypeScript 5.9**, **React 18.3**, **Tailwind 3.4**, **Radix UI** primitives.
- Biome (lint/format), Vitest, Playwright for tests.
- Husky + lint-staged.
- Nix flake (`flake.nix`) for reproducible Linux builds.

### Multi-window architecture
Four `BrowserWindow` instances created in [`electron/windows.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/windows.ts):
1. **HUD overlay** — transparent, frameless, always-on-top control bar.
2. **Source selector** — picker for screen/window source.
3. **Countdown overlay** — full-screen countdown.
4. **Editor window** — the React video editor.

Main process registered handlers in [`electron/ipc/handlers.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/ipc/handlers.ts) cover ~35 IPC channels: window-switching, source enumeration, recording state, cursor telemetry transport, file pickers, project I/O, exported-file save, shortcuts, diagnostics, and platform queries.

### Capture pipeline
- `desktopCapturer.getSources()` (main process) → `chromeMediaSourceId` constraint on `getUserMedia` (renderer).
- `MediaRecorder` with codec preference: **H.264 first** ("sharp real-time output"), with VP8/VP9/AV1 explicitly avoided because the author considered them "too CPU-intensive at 60 fps live."
- Bitrate scaling: 4K → 45 Mbps base, QHD → 28 Mbps, default 18 Mbps; ≥60 fps gets a 1.7× multiplier.
- Audio: 192 kbps system / 128 kbps mic, mixed via `AudioContext`.
- WebM output is duration-fixed via [`@fix-webm-duration/fix`](https://www.npmjs.com/package/@fix-webm-duration) before being handed to the main process via `store-recorded-session`.

### Render / editor pipeline
Hybrid GPU/CPU compositing in [`src/lib/exporter/frameRenderer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/frameRenderer.ts):
- **PixiJS 8** (WebGL) renders the recording layer with zoom, blur, and motion-blur filters (`pixi-filters/motion-blur`, `@pixi/filter-drop-shadow`).
- **Canvas2D** layers handle background wallpaper, shadows, and final composition.
- **Custom WebGL2 3D pass** (`threeDPass.ts`) applies perspective rotation to the foreground only.
- Linux/Wayland workaround: `gl.readPixels()` rasterization fallback when GPU-to-2D texture sharing fails.
- `dnd-timeline` for the timeline UI; `gsap` and `motion` for animation; `react-rnd` for resizable/draggable elements.

### Export pipeline
Pure browser stack — **no FFmpeg** at runtime today (though a hybrid FFmpeg path is in the open PRs):

| Stage | Library / API | File |
|---|---|---|
| Demux source video | [`web-demuxer`](https://www.npmjs.com/package/web-demuxer) (WASM/FFmpeg-based) | [`streamingDecoder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/streamingDecoder.ts) |
| Decode video frames | **WebCodecs** `VideoDecoder` | [`videoDecoder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/videoDecoder.ts) |
| Render each frame | PixiJS + Canvas2D + 3D pass | [`frameRenderer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/frameRenderer.ts), [`annotationRenderer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/annotationRenderer.ts) |
| Encode video | **WebCodecs** `VideoEncoder` (`avc1.640033`, hw-accel preferred — but `prefer-software` first on Windows) | [`videoExporter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/videoExporter.ts) |
| Encode audio | WebCodecs `AudioEncoder` | [`audioEncoder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/audioEncoder.ts) |
| Mux MP4 | **`mediabunny`** (`Mp4OutputFormat`, `BufferTarget`, `fastStart: 'in-memory'`) | [`muxer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/muxer.ts) |
| GIF export | `gif.js` (worker) | [`gifExporter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/gifExporter.ts) |
| MP4 file written | All in-memory `Blob` → main process saves to disk via `save-exported-video` IPC | — |
| Backpressure | `asyncVideoFrameQueue.ts` | [`asyncVideoFrameQueue.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/asyncVideoFrameQueue.ts) |

The export is essentially: web-demuxer → VideoDecoder → PixiJS render with effects → VideoEncoder → mediabunny mux → Blob → disk. This is why slow exports and Windows/Linux export crashes are repeatedly reported (see issues `#157`, `#256`, `#269`).

### Native APIs touched
- macOS: `systemPreferences.askForMediaAccess("microphone")`, screen-recording authorization (TCC), Continuity Camera, hardened runtime entitlements, `desktopCapturer` (which on macOS uses `SCContentSharing`/`AVFoundation` under the hood inside Chromium).
- Windows: `desktopCapturer` (DXGI Desktop Duplication via Chromium).
- Linux/Wayland: explicit `WaylandWindowDrag` and `WebRTCPipeWireCapturer` Chromium feature flags toggled in [`electron/main.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/main.ts).
- Global click hook: `uiohook-napi` (macOS only; Linux requires X11 dev headers, so it's skipped in `rebuild-native.mjs`).

---

## 4. Key files — where the work happens

### Recording

- [`src/hooks/useScreenRecorder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useScreenRecorder.ts) — full capture state machine, codec selection, bitrate math, audio mixing, ArrayBuffer hand-off.
- [`src/lib/recordingSession.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/recordingSession.ts) — normalization of stored session metadata.
- [`src/lib/cursorTelemetryBuffer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/cursorTelemetryBuffer.ts) — bounded ring-buffer for cursor samples (defaults: 10k active samples, 8 pending batches).
- [`src/hooks/useCameraDevices.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useCameraDevices.ts), [`useMicrophoneDevices.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useMicrophoneDevices.ts), [`useAudioLevelMeter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useAudioLevelMeter.ts) — device enumeration and live VU.
- [`electron/ipc/handlers.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/ipc/handlers.ts) — `set-recording-state`, `get-cursor-telemetry`, `store-recorded-session`, `store-recorded-video`.

### Editing

- [`src/components/video-editor/VideoEditor.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/VideoEditor.tsx) — top-level editor; holds undoable state (regions, wallpaper, shadowIntensity, motionBlurAmount, borderRadius, padding, aspectRatio, cropRegion, webcamLayoutPreset, webcamMaskShape, webcamPosition, cursorHighlight) and non-undoable state (playback, selection, export, project paths, cursor telemetry).
- [`src/components/video-editor/timeline/TimelineEditor.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/TimelineEditor.tsx), [`Item.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/Item.tsx), [`KeyframeMarkers.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/KeyframeMarkers.tsx) — `dnd-timeline`-based region UI.
- [`src/components/video-editor/timeline/zoomSuggestionUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/zoomSuggestionUtils.ts) — auto-zoom dwell detection.
- [`src/components/video-editor/videoPlayback/zoomTransform.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/zoomTransform.ts) — runtime zoom math + motion-blur kernel sizing (`PEAK_VELOCITY_PPS=1400`, `MAX_BLUR_PX=14`).
- [`src/components/video-editor/videoPlayback/cursorFollowUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/cursorFollowUtils.ts) — adaptive exponential smoothing of cursor position.
- [`src/components/video-editor/videoPlayback/cursorHighlight.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/cursorHighlight.ts) — click ring / dot.
- [`src/components/video-editor/AnnotationOverlay.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/AnnotationOverlay.tsx), [`ArrowSvgs.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/ArrowSvgs.tsx) — annotation rendering.
- [`src/components/video-editor/projectPersistence.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/projectPersistence.ts) — JSON project file.

### Export

- [`src/lib/exporter/videoExporter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/videoExporter.ts) — orchestrator (decoder → renderer → encoder → muxer).
- [`src/lib/exporter/frameRenderer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/frameRenderer.ts) — PixiJS + Canvas2D compositor.
- [`src/lib/exporter/threeDPass.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/threeDPass.ts) — WebGL2 perspective shader.
- [`src/lib/exporter/streamingDecoder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/streamingDecoder.ts) — `web-demuxer` driver.
- [`src/lib/exporter/muxer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/muxer.ts) — `mediabunny` MP4 container.
- [`src/lib/exporter/gifExporter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/gifExporter.ts) — `gif.js` worker pipeline.
- [`src/lib/exporter/audioEncoder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/audioEncoder.ts), [`asyncVideoFrameQueue.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/asyncVideoFrameQueue.ts).

---

## 5. Heavy dependencies a Rust port would need to replace

| Dependency | Role today | Rust replacement |
|---|---|---|
| **Electron** | Shell, capture APIs (`desktopCapturer`), file dialogs, multi-window | **Tauri** (multi-window supported in v2) — Tauri exposes none of the capture APIs Chromium gives Electron, so this is the *biggest* substitution to plan for. |
| **MediaRecorder + WebCodecs (capture)** | Real-time encoding of the screen stream | Native: `scap`/`screencapturekit-rs` (macOS), `windows-capture` crate (Windows), `pipewire-rs` / `wayland-protocols-screencopy` (Linux). Encoding via `ffmpeg-next` / `gstreamer-rs` / `cpal` for audio, or hardware encoders (VideoToolbox / Media Foundation / VAAPI) directly. |
| **PixiJS** | WebGL render of zoom/blur/shadow on the editor canvas | **`wgpu`** (with `bevy_render` if going Bevy) or **Skia/skia-safe** for 2D-style compositing. |
| **`mediabunny`** | MP4 muxing | `mp4` crate, `matroska-rs`, or `ffmpeg-next` mux. |
| **`web-demuxer`** | Demuxing source MP4 in browser | `symphonia` (no video codecs yet) + `ffmpeg-next`, or `gstreamer-rs`. |
| **`gif.js`** | GIF encoding in worker | `image::codecs::gif` or `gifski`. |
| **`pixi-filters` / `@pixi/filter-drop-shadow`** | Motion blur, drop shadow on canvas | wgpu shaders (compute or fragment); Bevy's built-in post-processing if Bevy. |
| **`gsap`, `motion`** | Animation / easing curves | Pure Rust: hand-rolled tweens or `bevy_tweening`. |
| **`dnd-timeline`** | Timeline drag/drop | Hand-rolled in Leptos/Bevy — no equivalent crate; this is the highest-effort UI piece. |
| **`react-rnd`** | Resizable/draggable widgets | Hand-rolled in Leptos. |
| **Radix UI / Tailwind** | Component primitives | Leptos: `leptos-use`, `daisyui` via Tailwind (works with Leptos), or hand-rolled. Bevy: `bevy_egui` or custom UI. |
| **`uiohook-napi`** | Global click capture (macOS) | `rdev` crate (cross-platform global hooks) or platform-native (`CGEventTap` via `core-graphics`, `SetWindowsHookEx`, `evdev`). |
| **`fix-webm-duration`** | Fixes MediaRecorder WebM headers | Not needed — direct H.264/MP4 encoding from native APIs avoids this class of problem entirely. |

---

## 6. Architecture characteristics

- **Monolithic Electron app**, *not* plugin-based.
- Strict main/renderer split via Electron IPC; no Node API exposed in renderer (preload bridge in [`electron/preload.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/preload.ts) → `window.electronAPI`).
- Four windows (HUD, source selector, countdown, editor) share state through IPC `set-current-recording-session` / `get-current-recording-session` channels rather than a shared store.
- Project file is plain JSON written via [`save-project-file`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/projectPersistence.ts) — recordings live next to it as `recording-{timestamp}.webm` files in `app.getPath('userData')/recordings/`.
- Renderer-side state is split between **undoable** (everything that affects output) and **non-undoable** (UI/playback/dialog) inside `VideoEditor.tsx`. Undo/redo is a snapshot stack in `useEditorHistory`.
- No central state library (no Redux/Zustand). State is drilled via React props.
- `electronAPI` is the only bridge — no MessagePort, no shared workers, no SharedArrayBuffer.

---

## 7. Cursor / zoom / effects — real-time vs post-render

**Both, but the same code in two contexts.** The renderer pipeline in `frameRenderer.ts` is used for **(a) live editor preview** and **(b) the export render loop** — driven by either the playback `currentTime` or the decoder's per-frame timestamp. Specifically:

- **Zoom transform**: `computeZoomTransform` in [`zoomTransform.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/zoomTransform.ts) is computed per-frame from `zoomScale`, `zoomProgress`, `focusX`, `focusY`. Real-time. The same function is called in the export loop.
- **Cursor follow**: `interpolateCursorAt` (binary search in cursor telemetry) + `smoothCursorFocus` (exponential) + `adaptiveSmoothFactor` (distance-weighted lerp). Real-time, deterministic at any timestamp — meaning seeking is correct.
- **Motion blur**: stateful — depends on inter-frame camera delta. Has a `MotionBlurState` (prevCamX/Y/Scale, lastFrameTimeMs) so it must be reset on seek and is naturally re-initialized when export starts. Filters are PixiJS `MotionBlurFilter` with kernel-size auto-tuned by velocity.
- **Auto-zoom suggestions**: precomputed once, post-recording, by analyzing cursor telemetry for dwell intervals (450–2600 ms, < 0.02-unit movement). Suggestions are then user-confirmed before becoming actual zoom regions.
- **3D rotation**: applied real-time per frame via the WebGL2 pass.
- **Annotations**: rendered each frame from the timeline's region list — real-time.

This deterministic-from-state design is one of OpenScreen's strongest decisions: **the export pipeline reuses the editor's per-frame renderer**, so what you see is what you get. A Rust port should preserve this.

---

## 8. Limitations the project itself acknowledges (or is bug-tracking)

From the README, releases, and [open issues](https://github.com/siddharthvaddem/openscreen/issues):

- **Export is the weakest part**: slow on Linux/Windows (#157), crashes on Windows (#256), hangs at finalization (#269), crashes when exporting a re-opened project (#540). The whole pipeline runs in JS/WebCodecs in a renderer process — no isolated worker, no native fallback. This is *the* reason a "hybrid FFmpeg" PR exists.
- **Linux/Wayland needs hacks**: `gl.readPixels()` rasterization fallback in `frameRenderer.ts` because GPU-to-2D texture sharing doesn't work reliably on Wayland.
- **System audio is platform-gated**: macOS ≥13, Windows native, Linux PipeWire-only.
- **macOS permission prompts re-trigger** (#558) — a recurring pain.
- **Click capture is macOS-only** (`uiohook-napi` is skipped on Linux/Windows in `rebuild-native.mjs`). Cursor *position* is fine cross-platform; *click events* are not.
- **No animated GIF size cap warnings** and large GIFs OOM on long clips.
- **Cold start is heavy** — the editor bundle is large; "lazy load editor" is in PRs.
- **Author's own README disclaimer**: *"I'm new to open source, idk what I'm doing lol."* The project is one person + drive-by contributors; bus-factor risk.
- **No formal roadmap** — direction is implicit.
- **macOS Apple Silicon "damaged" warning** (`#88`) — Gatekeeper/notarization friction. v1.4 finally ships notarized builds.
- **Codec choice is browser-limited**: no AV1 encode path that's practical, no ProRes, no DNxHD; everything is H.264/AVC.

---

## 9. Community reception (HN, blogs)

- [Hacker News thread](https://news.ycombinator.com/item?id=47595695): praised for ease of use ("zoom effect from first attempt, learning curve roughly zero") and Linux support (a Screen Studio gap). Criticized for: aggressive auto-zoom feeling "dizzying," and one vocal commenter calling open-source alternatives a "trojan horse" against paid tools.
- Requested in HN: cursor click highlighting (now shipped), slider zoom (in PR), arrows/callouts (shipped), freeze-frame (not yet), preset effects for video-to-video consistency.
- [emelia.io review](https://emelia.io/hub/openscreen-screen-recorder-review): confirms feature parity on the basics with Screen Studio; flags export reliability and integrated-GPU smoothness.
- [thepixelspulse.com](https://thepixelspulse.com/posts/openscreen-alternative-screen-studio-caveats/) and [screenkite.com](https://www.screenkite.com/blog/screenkite-vs-openscreen-native-vs-open-source-electron) both contrast OpenScreen's Electron base unfavorably with native macOS recorders on RAM/CPU/cold-start.

---

## 10. License

**MIT** — the entire repo, including all rendering, capture, and export code. A hard fork or wholesale Rust translation is legally trivial. Attribution is required in the binary distribution.

---

## 11. Lessons for a Rust port (Leptos+Tauri or Bevy+Tauri)

### What OpenScreen got right — preserve these

1. **Out-of-band cursor telemetry**: capturing cursor position as a separate stream (not burned into video) lets the editor re-render cursor effects deterministically at any zoom/scale/aspect ratio. **Copy this design pattern verbatim.** Native APIs (`CGEventTap` on macOS, `SetWinEventHook` on Windows, libinput/wayland on Linux) make this *easier* in Rust than it was in Electron.
2. **Single render function used for both preview and export.** Don't build a separate "export renderer" — that's where bugs hide. Make the per-frame pure function the source of truth. wgpu makes this clean.
3. **Cursor dwell-based auto-zoom suggestions** (450–2600 ms, 0.02-unit threshold) — these constants are well-tuned and worth lifting directly. They're in [`zoomSuggestionUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/zoomSuggestionUtils.ts).
4. **Adaptive-smoothing cursor follow** with deceleration curve — far better than naive lerp; the formula in [`cursorFollowUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/cursorFollowUtils.ts) is a good starting point.
5. **Velocity-driven motion blur** with kernel-size auto-tuning. The `PEAK_VELOCITY_PPS = 1400`, `MAX_BLUR_PX = 14` constants in [`zoomTransform.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/zoomTransform.ts) are tuned values worth copying as starting defaults.
6. **Project file is plain JSON** — easy to migrate, diff, share, version. Stick with this. Use `serde_json` and a versioned schema.
7. **Bitrate ladder** scaled to resolution × framerate. The math in `useScreenRecorder.ts` (4K=45M, QHD=28M, default=18M, ×1.7 for ≥60fps) is directly transferable.
8. **Codec preference H.264 first**: matches reality of consumer machines. AV1/VP9 *should* be optional. In Rust, hardware H.264 via VideoToolbox/Media Foundation/VAAPI gives huge wins over software encoders.
9. **Multi-window separation**: HUD overlay, source picker, countdown, editor are different windows. Tauri 2 supports this. Don't try to cram into one window.
10. **Undoable vs non-undoable state distinction** in the editor reducer — undo only the things that affect output, not UI selection state. Saves enormous design time.

### What they struggle with — Rust+Tauri can do better

1. **Export reliability is the #1 user pain.** Doing decode → render → encode → mux in a renderer process via WebCodecs is fragile. A native Rust pipeline using `ffmpeg-next` or `gstreamer-rs` (or directly: VideoToolbox/MediaFoundation/VAAPI + the `mp4` crate) sidesteps the entire class of bugs in issues #157, #256, #269, #540. **This is the strongest single argument for a Rust port.**
2. **Cold start / bundle size.** Electron + a 100+ MB JS bundle vs Tauri's ~10 MB. Editor lazy-loading wouldn't even be needed.
3. **Memory.** Electron + Chromium for a video editor with PixiJS easily hits 1+ GB. wgpu + Tauri can sit under 200 MB.
4. **Wayland/Linux compositing hacks** (`gl.readPixels()` fallback) disappear when you control the GPU pipeline directly with wgpu.
5. **Permissions on macOS re-prompt** (#558) — usually because TCC database entries get stale per-bundle-id. A Tauri app with stable bundle ID + proper `Info.plist` `NSScreenCaptureDescription` and `NSMicrophoneUsageDescription` handles this cleanly.
6. **Click capture cross-platform**. The `rdev` crate or platform-native event taps work on all three OSes — no need to skip Linux/Windows like `uiohook-napi` does today.
7. **Codec breadth**. A native ffmpeg link gets you ProRes, DNxHD, HEVC, AV1, VP9 — useful for power users who want to feed editing pipelines downstream.
8. **Real "zero-copy" capture**. ScreenCaptureKit (macOS 13+) and Windows Graphics Capture both support `IOSurface`/`ID3D11Texture2D` zero-copy paths into the encoder. Electron's `desktopCapturer` round-trips through `MediaRecorder` and a JS ArrayBuffer. Native Rust can keep frames on GPU end-to-end.

### What Rust+Tauri will do *worse* — be honest about these

1. **No `desktopCapturer` API.** You re-implement source enumeration per platform: ScreenCaptureKit (`screencapturekit-rs` crate), DXGI/Windows.Graphics.Capture, PipeWire screencast portal. ~3 weeks of platform-specific code that Electron gives you free.
2. **No `MediaRecorder`.** You wire encoder + muxer yourself.
3. **No browser DevTools magic** for the editor preview canvas — debugging wgpu shaders is harder than debugging Pixi.
4. **No npm ecosystem of UI components.** Leptos is younger; you'll re-implement `react-rnd`, `dnd-timeline`, drag-drop on canvas, color picker UX. **The timeline editor alone is 4–6 weeks of UI work**, no shortcuts.
5. **WebCodecs hardware-accel was free.** Rust hardware-encoder bindings (`videotoolbox-sys`, `mfx-sys`) are crusty and platform-locked.
6. **Hot-reload story** is worse than Vite + React for the renderer-heavy parts.
7. **Bevy in particular** is overkill for a 2D compositor and brings ECS overhead that doesn't pay off here. Bevy makes sense if you want game-like real-time previews with many simultaneous animated elements; it's heavyweight if you're really just doing "video frame + overlays + zoom transform." **Lean toward Leptos+Tauri+wgpu rather than Bevy+Tauri** unless you're already committed to Bevy for other reasons.

### Fork vs. start fresh

**Start fresh.** Reasons:

- The repo is **96% TypeScript** (per GitHub stats); essentially nothing is salvageable as Rust source. There are no shared protocols, no native crates, no FFI bindings to lift.
- A fork of an Electron app to "convert to Tauri" is a misnomer — you'd keep maybe the project-file JSON schema and a few tuning constants, and rewrite the rest.
- The tuning constants and algorithmic ideas (dwell detection, adaptive smoothing, motion-blur kernel sizing, bitrate ladder) are easy to read and reimplement in a few hundred lines of Rust. Lift those as inspiration and credit MIT.
- Forking adds the obligation of tracking upstream Electron-specific changes that don't apply to your stack — pure friction.

**What to lift verbatim (with MIT attribution):**

- The auto-zoom dwell-detection thresholds and algorithm (`zoomSuggestionUtils.ts`).
- The cursor-follow adaptive smoothing math (`cursorFollowUtils.ts`).
- The motion-blur velocity → kernel-size mapping (`zoomTransform.ts`).
- The bitrate ladder (`useScreenRecorder.ts`).
- The project-file JSON schema (`projectPersistence.ts`, `types.ts`).
- The UI/UX flow: HUD → source picker → countdown → recording → editor.

**What to rewrite from scratch:**

- All capture (native per platform, not Chromium).
- All encoding/muxing (ffmpeg/gstreamer/native HW).
- All rendering (wgpu, not PixiJS).
- All UI (Leptos, not React).
- All IPC (Tauri commands, not Electron `ipcMain`).

### A reasonable build order for the Rust port

1. **Capture** on one platform (recommend macOS first via `screencapturekit-rs` — best APIs, biggest user base for demo-makers). Pipe to file via VideoToolbox H.264 hardware encoder. Cursor telemetry as separate JSON stream.
2. **Headless export pipeline**: take a recording + a project JSON → produce MP4. wgpu compositor + ffmpeg encode. This validates the renderer before any UI.
3. **Editor UI** in Leptos with Canvas/wgpu preview. Timeline last (it's the hardest UI).
4. **Cross-platform**: Windows next (`windows-capture`), Linux last (PipeWire portal — most fiddly).
5. **Tauri shell** with multi-window (HUD/picker/countdown/editor) integrated last.

Plan **6–9 months** for feature parity with v1.4 by a competent solo Rust developer. The win is reliability and resource use; the cost is months of platform-specific capture code that Electron gives you in an afternoon.

---

## Appendix: notable file paths to bookmark

| Concern | Path |
|---|---|
| App entrypoint | [`electron/main.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/main.ts) |
| Window definitions | [`electron/windows.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/windows.ts) |
| All IPC channels | [`electron/ipc/handlers.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/ipc/handlers.ts) |
| Preload bridge | [`electron/preload.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/electron/preload.ts) |
| Recording state machine | [`src/hooks/useScreenRecorder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/hooks/useScreenRecorder.ts) |
| Cursor telemetry buffer | [`src/lib/cursorTelemetryBuffer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/cursorTelemetryBuffer.ts) |
| Editor root | [`src/components/video-editor/VideoEditor.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/VideoEditor.tsx) |
| Timeline UI | [`src/components/video-editor/timeline/TimelineEditor.tsx`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/TimelineEditor.tsx) |
| Auto-zoom algorithm | [`src/components/video-editor/timeline/zoomSuggestionUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/timeline/zoomSuggestionUtils.ts) |
| Zoom transform math | [`src/components/video-editor/videoPlayback/zoomTransform.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/zoomTransform.ts) |
| Cursor follow smoothing | [`src/components/video-editor/videoPlayback/cursorFollowUtils.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/videoPlayback/cursorFollowUtils.ts) |
| Frame compositor | [`src/lib/exporter/frameRenderer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/frameRenderer.ts) |
| 3D perspective shader | [`src/lib/exporter/threeDPass.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/threeDPass.ts) |
| Export orchestrator | [`src/lib/exporter/videoExporter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/videoExporter.ts) |
| MP4 muxer wrapper | [`src/lib/exporter/muxer.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/muxer.ts) |
| GIF exporter | [`src/lib/exporter/gifExporter.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/gifExporter.ts) |
| Demuxer | [`src/lib/exporter/streamingDecoder.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/lib/exporter/streamingDecoder.ts) |
| Project file format | [`src/components/video-editor/projectPersistence.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/projectPersistence.ts) |
| Editor type definitions | [`src/components/video-editor/types.ts`](https://github.com/siddharthvaddem/openscreen/blob/main/src/components/video-editor/types.ts) |
| Native rebuild script | [`scripts/rebuild-native.mjs`](https://github.com/siddharthvaddem/openscreen/blob/main/scripts/rebuild-native.mjs) |
| macOS entitlements | [`macos.entitlements`](https://github.com/siddharthvaddem/openscreen/blob/main/macos.entitlements) |
| Build config | [`electron-builder.json5`](https://github.com/siddharthvaddem/openscreen/blob/main/electron-builder.json5) |

## Sources

- [GitHub: siddharthvaddem/openscreen](https://github.com/siddharthvaddem/openscreen)
- [Releases](https://github.com/siddharthvaddem/openscreen/releases)
- [Open issues sorted by reactions](https://github.com/siddharthvaddem/openscreen/issues?q=is%3Aissue+is%3Aopen+sort%3Areactions-%2B1-desc)
- [Pull requests](https://github.com/siddharthvaddem/openscreen/pulls)
- [Hacker News discussion](https://news.ycombinator.com/item?id=47595695)
- [Emelia.io OpenScreen review](https://emelia.io/hub/openscreen-screen-recorder-review)
- [Pixels and Pulse review with caveats](https://thepixelspulse.com/posts/openscreen-alternative-screen-studio-caveats/)
- [ScreenKite vs OpenScreen comparison](https://www.screenkite.com/blog/screenkite-vs-openscreen-native-vs-open-source-electron)
