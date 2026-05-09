# Synthesis: Unified Feature Catalog + Rust Stack Decision

> Cross-references `screen-studio-research.md` (commercial reference) and `openscreen-research.md` (open-source reference). Read those first for citations; this doc focuses on **what to build** and **what to build it with**.

---

## 1. The two-source picture in one paragraph

**Screen Studio** is the commercial north star: a polished macOS-only Electron app whose moat is the *cinematic look* — synthetic cursor with spring-physics smoothing, click-driven auto-zoom with motion blur, GPU-composited padded backgrounds, dynamic camera layouts, on-device transcription. It's been iterating ~1 release every 2-3 weeks for ~3 years and its top user pain is **export speed** (Electron-bound).

**OpenScreen** is the open-source proof-of-architecture: an MIT-licensed Electron + React + PixiJS clone hitting feature parity on the basics, but suffering the same export-reliability problems plus several Electron-specific Wayland/Linux/Windows pain points (issues #157, #256, #269, #540 are all export crashes/hangs). Its architectural patterns are *correct* (out-of-band cursor telemetry, deterministic per-frame render function shared between editor and exporter) and worth lifting; its implementation is the wrong stack for those patterns.

**The thesis for a Rust port:** keep the architecture, replace the runtime. The wins are export reliability, memory footprint, cold start, and zero-copy native capture. The costs are 3+ months of platform-specific capture code that Electron gives away for free, and a hand-rolled timeline UI (4–6 weeks).

---

## 2. Unified feature catalog

Marked `[SS]` if Screen Studio has it, `[OS]` if OpenScreen has it, `[—]` if neither has it (i.e., differentiation opportunity).

### 2.1 Capture

| Feature | SS | OS | Notes |
|---|---|---|---|
| Full display capture | ✅ | ✅ | Multi-display on SS |
| Window capture | ✅ | ✅ |  |
| Custom area capture | ✅ | — | SS has grid + numeric crop + viewport presets |
| Webcam capture (up to 4K) | ✅ | ✅ | OS captures separate file; SS allows custom aspect |
| Microphone with gain/AGC | ✅ | ✅ | SS has on-device "improve mic" (DSP) |
| System audio (per-app on macOS) | ✅ | partial | SS does *per-app*; OS captures all system audio |
| Cursor position telemetry (out-of-band) | ✅ | ✅ | The single most important architectural decision |
| Click event capture | ✅ | macOS only | OS uses `uiohook-napi`, no Linux/Win |
| Pause / resume | ✅ | — | Diff opportunity |
| Speaker notes / teleprompter | ✅ | — | Diff opportunity |
| iPhone / iPad capture (USB-C) | ✅ | — | Big SS feature, low priority for v1 |
| iPhone Mirroring (macOS 15+) | ✅ | — |  |
| Recording widget (floating overlay) | ✅ | partial | OS has HUD window but no shortcut display |
| Hide cursor / hide-on-idle / loop position | ✅ | partial |  |
| Countdown overlay | ✅ | ✅ |  |
| Free-screen drawing during recording | — | — | **Real differentiation** (Loom has this) |

### 2.2 Cursor engine

The single biggest differentiator. Implementation pattern is identical between SS and OS:

1. Capture pixels with native cursor *hidden*.
2. Stream cursor positions, click events, and cursor-type changes separately (sub-frame timestamped).
3. Re-render cursor as a sprite overlay during compositing — both in editor preview and on export.

| Feature | SS | OS | Notes |
|---|---|---|---|
| Spring-physics smoothing | ✅ | adaptive exponential | OS uses simpler curve; SS feels smoother |
| Cursor-type-aware crossfade (arrow → I-beam → hand) | ✅ | — | Diff opportunity |
| Velocity-based rotation | ✅ | — |  |
| Adaptive smoothing factor (distance-weighted lerp) | — | ✅ | Lift OS's `cursorFollowUtils.ts` math |
| Hi-res cursor sprites (replace low-res macOS bitmaps) | ✅ | — | SS ships its own bitmap sets |
| Click effect: ripple | ✅ | ✅ |  |
| Click effect: circle / dot ring | ✅ | ✅ |  |
| Click sound effect | ✅ | — | One-line addition |
| Hide cursor when idle (with fade) | ✅ | — |  |
| Hide cursor inside specific timeline range | ✅ | — |  |
| "Loop cursor position" (return to start at end) | ✅ | — | Tiny but loved; trivial to implement |
| Custom cursor library (theme sets) | ✅ | — | Low-effort polish |
| Auto-pick cursor set per OS version | ✅ | — |  |
| Touch-style cursor for iPad/iPhone | ✅ | — |  |
| Cursor "shake rejection" (accessibility tool jitter) | ✅ | — |  |

### 2.3 Auto-zoom

| Feature | SS | OS | Notes |
|---|---|---|---|
| Click-driven zoom suggestion | ✅ | — | SS zooms on every click (over-aggressive per users) |
| **Dwell-based** zoom suggestion (cursor stationary 450–2600 ms) | — | ✅ | **OS's signature insight — copy this verbatim** |
| Manual zoom region (drag on canvas + duration on timeline) | ✅ | ✅ |  |
| Zoom level by 0–9 keypress | ✅ | — |  |
| "Apply this zoom to all" batch action | ✅ | — |  |
| "Always keep zoomed in" | ✅ | — |  |
| Zoom snap to 50% within 1% | ✅ | — |  |
| Pan-follow-cursor inside zoom region | ✅ | ✅ |  |
| Vertical-mode zoom optimisation | ✅ | — |  |
| Multi-select zoom ranges | ✅ | — |  |
| Copy/paste zooms across recordings | ✅ | — |  |
| Custom spring easing per zoom | ✅ | partial | OS uses fixed easing |
| Motion blur during zoom transitions | ✅ | ✅ | Velocity → kernel size; OS constants `PEAK=1400 PPS, MAX=14 px` |
| Real keyframable zoom path (advanced) | — | partial | OS has keyframe markers in zoom regions; SS doesn't |
| Cluster-based zoom suggestion (DBSCAN over click bursts) | — | — | **Real differentiation** — fixes "drunken cameraman" |

### 2.4 Editing surface

| Feature | SS | OS | Notes |
|---|---|---|---|
| Multi-track timeline (video / zoom / camera / mask / audio / captions / shortcuts) | ✅ | partial | OS has fewer tracks |
| Trim, cut, ripple-delete | ✅ | ✅ |  |
| Razor / split (`C`/`X`/Option) | ✅ | partial |  |
| Speed up/down per slice (with proper audio) | ✅ | ✅ |  |
| "Speed up typing" auto-detection | ✅ | — | High-leverage feature |
| Slow-down ranges | ✅ | ✅ |  |
| Per-clip audio volume + waveform | ✅ | ✅ |  |
| Loop playback | ✅ | — |  |
| Project autosave + recovery | ✅ | ✅ |  |
| Aspect ratio change with re-flow (16:9, 9:16, 1:1, 4:5, custom) | ✅ | ✅ |  |
| Crop tool | ✅ | ✅ |  |
| Mask & highlight (blur sensitive area) | ✅ | ✅ | SS limit: doesn't follow scroll content |
| **Mask follows scrolling content** (optical-flow tracking) | — | — | **#1 SS user complaint — real differentiation** |
| Reactions (animated emoji overlays) | ✅ | — |  |
| Annotations (text, arrows, shapes) | — | ✅ | OS has these; SS has them on roadmap |
| Command menu (⌘K) | ✅ | — |  |
| Undo/redo (output-affecting state only) | ✅ | ✅ | OS's split is the right pattern |

### 2.5 Visual style / branding

| Feature | SS | OS | Notes |
|---|---|---|---|
| Wallpaper library (100+) | ✅ | ✅ | OS has a smaller set |
| Custom solid color / gradient / image background | ✅ | ✅ |  |
| Padding / inset / rounded corners / shadow | ✅ | ✅ |  |
| Drop shadow with corner-radius-aware rendering | ✅ | ✅ |  |
| Real-time inset color changes | ✅ | partial |  |
| Device frames (iPhone 11→17 Pro Max, iPad lineup) | ✅ | — | Large undertaking; nice-to-have for v2 |
| Initial preset applied to new projects | ✅ | partial |  |
| Glassmorphism wallpapers | ✅ | — |  |
| 3D rotation/perspective | ✅ | ✅ | OS has X/Y/Z perspective |

### 2.6 Camera bubble

| Feature | SS | OS | Notes |
|---|---|---|---|
| Position selector (corners + free placement) | ✅ | ✅ |  |
| Roundness (circle ↔ square) | ✅ | ✅ |  |
| Size slider | ✅ | ✅ |  |
| Mirror toggle | ✅ | ✅ |  |
| Shadow under camera | ✅ | ✅ |  |
| Auto-shrink camera during zoom | ✅ | — |  |
| Hide camera per-segment | ✅ | partial |  |
| Background blur (built-in, not OS-level) | — | — | **Real differentiation** — SS punts to macOS Portrait Mode |
| Custom aspect ratio for camera | ✅ | — |  |
| **Dynamic camera layouts** (full-screen / overlay / hidden timeline track) | ✅ | — | One of SS's killer features |

### 2.7 Audio

| Feature | SS | OS | Notes |
|---|---|---|---|
| Mic stereo + mono | ✅ | ✅ |  |
| AGC / noise suppression toggle | ✅ | partial | SS's "improve mic" is auto-on; users want intensity control |
| **Noise-suppression intensity slider** | — | — | **Real differentiation** — SS over-processes |
| Per-clip volume with waveform | ✅ | ✅ |  |
| System audio per-app | ✅ | — | macOS only with ScreenCaptureKit |
| Background music library (royalty-free) | ✅ | — | Curated set in-app |
| Import custom audio file | ✅ | partial |  |
| Mouse-click sound effect | ✅ | — |  |
| **Multi-track mixer** (mic / system / music with independent levels and fades) | — | — | **Real differentiation** — SS users complain |
| **Voiceover re-record** over existing video | — | — | **Real differentiation** |
| AI voiceover (TTS) | roadmap | — |  |

### 2.8 Captions / transcription

| Feature | SS | OS | Notes |
|---|---|---|---|
| On-device Whisper transcription | ✅ | — | `whisper-rs` in Rust |
| Apple Speech Recognition (macOS 26+) | ✅ | — | `Speech.framework` via `objc2` |
| Multi-language (~100+) | ✅ | — |  |
| Auto-detect language | ✅ | — |  |
| Custom-vocabulary prompt | ✅ | — |  |
| Edit transcript UI | ✅ | — |  |
| Export transcript file | ✅ | — |  |
| Captions style (size, position, font) | ✅ | — |  |
| Filler-word / "um/uh" removal | — | — | **Real differentiation** (Descript has this) |
| Silence trimming | — | — | **Real differentiation** |

### 2.9 Keyboard shortcut overlay

| Feature | SS | OS | Notes |
|---|---|---|---|
| Capture key events during recording | ✅ | — |  |
| Render shortcut chips on output | ✅ | — |  |
| Customizable shortcut chip style | ✅ | — |  |
| Shortcut timeline track | ✅ | — |  |
| Modifier handling (FN, F-keys, space symbol, Ctrl ordering) | ✅ | — |  |

### 2.10 Export

| Feature | SS | OS | Notes |
|---|---|---|---|
| MP4 (H.264) | ✅ | ✅ | Both prefer H.264; OS via WebCodecs, SS via VideoToolbox/ffmpeg |
| GIF | ✅ | ✅ |  |
| WebM / VP9 / AV1 | partial | — |  |
| ProRes / DNxHD | — | — | **Differentiation for power users** |
| 4K @ 60 fps | ✅ | ✅ |  |
| 24 / 30 / 60 fps | ✅ | ✅ |  |
| Aspect ratio presets (16:9, 9:16, 1:1, 4:5, custom) | ✅ | ✅ |  |
| Quality presets | ✅ | ✅ |  |
| Multi-project batch export | ✅ | — |  |
| Quick-export with previous settings | ✅ | — |  |
| Frame-to-clipboard (PNG snapshot) | ✅ | — |  |
| Export to clipboard (whole video) | ✅ | — |  |
| Multi-threaded export | ✅ | partial |  |
| Extract raw recording files (separate streams) | ✅ | — |  |
| **Streaming export (encode while uploading)** | — | — | **Real differentiation** — eliminates the SS "wait twice" pain |
| **Zero-copy GPU encode** (IOSurface → VideoToolbox) | — | partial | OS's PR pivot is exactly this |

### 2.11 Sharing / cloud

| Feature | SS | OS | Notes |
|---|---|---|---|
| Hosted player (`/share/<id>`) | ✅ (30-min cap) | — |  |
| Public/private links | ✅ | — |  |
| Comments on shared video | ✅ | — |  |
| View counter | ✅ | — |  |
| **Embed player** | — | — | **Real differentiation** — SS doesn't have it |
| **Engagement analytics** (watch time, drop-off) | — | — | **Real differentiation** |
| **Password protection** | — | — | **Real differentiation** |
| **CTA overlays** | — | — | **Real differentiation** (Loom has) |
| Team plan / workspace | roadmap | — |  |

Cloud is a separate product from the app; out of MVP scope.

### 2.12 Project format

| Feature | SS | OS | Notes |
|---|---|---|---|
| Versioned schema | ✅ | ✅ | OS uses plain JSON — copy this |
| Project file = recording + edit graph + cursor stream + click events + audio | ✅ | ✅ |  |
| Preset files (`.screenstudiopreset`) | ✅ | partial |  |
| Project from existing video (MP4/MOV import) | ✅ | partial |  |
| **Smaller project files** (chunked H.264 segments + sled/redb event log) | — | — | SS hits 40 GB / 3 hrs — easy win |
| iCloud / cross-device sync | — | — | **Differentiation** |

### 2.13 Hotkeys / integrations

| Feature | SS | OS | Notes |
|---|---|---|---|
| Configurable shortcuts | ✅ | ✅ |  |
| Raycast extension + URL scheme | ✅ | — |  |
| CLI / scripting interface | — | — | **Differentiation for dev audience** |
| Plugin system | — | — | Probably out of scope but worth flagging |

---

## 3. Prioritized roadmap (suggested)

### MVP (months 0–4) — "you can record, edit, export"

The bare minimum to validate the stack and the architecture. Cross-platform from day one is non-negotiable for the "vs SS" pitch — if you launch macOS-only you have no story.

1. **Capture** (one platform — macOS first):
   - ScreenCaptureKit pixel capture w/ cursor hidden.
   - `CGEventTap` cursor positions + clicks, sub-frame timestamps, ring-buffered to JSON.
   - `AVCaptureSession` webcam + mic.
   - VideoToolbox H.264 hardware encode → `.mp4` chunks on disk.
2. **Project format**: lift OS's JSON schema verbatim, change names if you must, version it from v1.
3. **`crates/wisp` library — MVP API surface** (Rust + WGSL): scene graph (`Container`, `Sprite`, `Mesh`, `Graphics`, `Text`), transform tree, render-to-texture, video texture binding, filter chain (motion blur, drop shadow, gaussian blur, color matrix). Ship a `cargo run -p render --example screen-mock` standalone demo before integrating with the recorder. This is the foundation; everything else depends on it.
4. **Headless export pipeline**: `cargo run -- render project.json out.mp4` opens a headless wgpu surface, runs the same scene graph for each frame timestamp, hands `RenderTexture` output to `ffmpeg-next` for encode. Validates per-frame determinism before any UI exists.
5. **Editor UI** in Leptos (Tauri webview chrome) + native preview window (winit, rendered by `render` crate):
   - Leptos: timeline (budget 6 weeks, the hardest UI), inspector, settings sidebar, source picker, HUD.
   - Preview: native winit window positioned under/alongside the Leptos chrome. Cursor + project-state changes flow from Leptos via Tauri events into the render thread.
5. **Core effects**:
   - Spring-physics cursor smoothing (lift OS adaptive smoothing as starting point).
   - Click ripple + click sound.
   - Padded gradient/wallpaper background, rounded corners, shadow.
   - Camera bubble (corner + size + roundness + shadow).
   - Click-driven auto-zoom *with dwell suppression* (lift OS's 450–2600 ms / 0.02-unit thresholds).
   - Velocity-driven motion blur (lift OS's `PEAK_VELOCITY_PPS=1400`, `MAX_BLUR_PX=14`).
6. **Export**: MP4 only. GIF, ProRes, WebM in v1.
7. **Tauri shell** with multi-window (HUD / source picker / countdown / editor). Multi-window from day one — don't try to retrofit it.

### v1 (months 4–7) — "feature parity with OpenScreen, exceeding it on reliability"

8. **Cross-platform capture**: Windows (`windows-capture` crate), Linux (PipeWire portal). Click capture via `rdev`.
9. **Cursor library**: ship 2-3 cursor sprite sets (default macOS, minimal, large), cursor-type-aware crossfade (arrow → I-beam → hand).
10. Loop cursor position, hide-on-idle, hide-in-range.
11. Aspect ratio change with re-flow.
12. Mask & highlight tool.
13. Annotations: text, arrows, shapes (rect/ellipse).
14. Speed-up-typing auto-detection.
15. On-device transcription (`whisper-rs`).
16. Captions track + edit transcript UI.
17. Keyboard shortcut chip overlay (capture + render).
18. GIF export with `gifski`.
19. Background music library (start with 5–10 curated CC-0 tracks).
20. Mic noise-suppression with intensity slider.
21. Project autosave + recovery.
22. Preset files.

### v2 (months 7–10) — "things SS doesn't have"

These are where you actually win, not just match:

23. **Multi-track audio mixer** (mic / system / music with independent levels, fades, ducking).
24. **Voiceover re-record** over existing video.
25. **Mask follows scrolling content** (optical-flow tracking — SS's #1 user complaint).
26. **Cluster-based auto-zoom** (DBSCAN over click bursts to suppress "drunken cameraman").
27. **Filler-word removal** + silence trimming (Descript-style).
28. **Streaming export** (encode-while-upload).
29. **Camera background blur** (segmentation model on-device).
30. **Real keyframable zoom path** for power users.
31. **CLI / headless render** for CI use cases.
32. ProRes / DNxHD export for downstream NLE workflows.
33. Dynamic camera layouts (timeline track for full-screen / overlay / hidden).

### v3+ (10+ months)

- Cloud sharing surface (separate product, separate stack — probably a Rust API + Cloudflare R2).
- Embed player, analytics, password, CTA, team plan.
- iPhone/iPad capture.
- Device frames library.
- Plugin system.
- AI voiceover (Coqui TTS / on-device).

---

## 4. Stack decision: all-Rust, in-repo Pixi-equivalent — LOCKED 2026-05-09

**Pivot from earlier revisions:** PixiJS+Bun sidecar is dropped. The project is now a **Rust+WebGPU 2D scene graph + filter chain library** with the screen recorder as its first consumer. The library fills the gap identified earlier (no Rust crate has scene graph + filter chain + video textures + WebGPU). It's shaped like a standalone library but lives in-repo, scoped to what the recorder needs.

**Locked architecture:**
- **Shell:** Tauri 2 (multi-window).
- **UI / chrome:** Leptos (Rust → WASM, in the Tauri webview).
- **Editor preview canvas:** native `winit` sibling window (NOT in the webview), rendered by the in-repo `render` crate via `wgpu`. Tauri 2 supports child/sibling native windows; chrome is webview, preview is native.
- **Renderer (`crates/wisp`):** in-repo, library-shaped, Rust + `wgpu` + WGSL. Pixi-style API surface (Container, Sprite, Filter, RenderTexture). Scoped to what the recorder uses — not full Pixi parity.
- **Capture:** native Rust per OS — `objc2` / ScreenCaptureKit (macOS), `windows-rs` / Windows.Graphics.Capture (Windows), `pipewire-rs` / portal (Linux).
- **Encode / mux:** native Rust — `ffmpeg-next` for MVP; VideoToolbox / Media Foundation HW paths in v2.
- **Click telemetry:** native Rust per OS.
- **Audio capture:** `cpal` + `coreaudio-rs`.
- **Project format:** plain JSON via `serde_json`, schema lifted from OpenScreen.

**Why native preview window instead of webview canvas:**
- Video texture path is direct: ffmpeg-next decode → `wgpu::Queue::write_texture`. No `VideoFrame`, no wasm-bindgen, no `importExternalTexture` lifetime dance.
- No WASM compilation needed for the render crate at MVP — pure native.
- No frame-blit IPC bottleneck for export — same render path writes to `RenderTexture`, encoder consumes from same process.
- Cost: positioning a native winit window inside/under Tauri's webview takes platform-specific care, especially on Linux/Wayland. Spike validates this in week 1.

**Architectural invariant (preserved from OpenScreen):** ONE scene graph drives both editor preview AND export. Export is the same scene rendered into a `RenderTexture` per frame, then handed to the encoder.

**Render library scope (MVP, "what the recorder actually needs"):**
- Scene graph: `Container`, `Sprite`, `Mesh`, `Graphics` (filled rects/rounded rects), `Text` (bitmap font).
- Transform tree (parent→child `Mat3`/`Mat4` with dirty flags).
- Texture types: `Texture` (image), `VideoTexture` (driven by external decoded frames), `RenderTexture` (offscreen target).
- Filter chain: `MotionBlur`, `DropShadow`, `Blur` (separable Gaussian), `ColorMatrix`, `Displacement`. Render-to-texture handled internally.
- Renderer: 1 draw-call batcher, 1 filter pass orchestrator. No instancing for v1.
- Coordinate system: top-left origin, DPR-aware, viewport math.
- Optional v1+: hit testing, clipping masks, sprite atlases.

Out of scope for the in-repo library: particle systems, sprite sheets / animation, accessibility, asset loading pipeline, sound, complex text shaping (use bitmap fonts only), interaction event system. If the library ever gets extracted as a real OSS project, those land in v2.

### Why all-Rust + in-repo render library, not PixiJS+Bun

- **Toolchain coherence.** One language, one build system, one CI, one mental model. Cargo, clippy, fmt, machete, deny, audit — uniform quality gates across UI, render, capture, encode.
- **Fills a real ecosystem gap.** Pixi-style 2D scene graphs with filter chains don't exist in Rust. Building it is shipping the missing crate. Even scoped to MVP needs (~3–5k LOC Rust + ~1k LOC WGSL), it's a meaningful contribution that could be extracted later.
- **WGSL shaders work everywhere.** Same shader source runs in browser WebGPU (if we ever target web) and native wgpu. No port cost.
- **No JS in the hot path.** Capture → render → encode is all native Rust. No `<canvas>`, no `VideoFrame`, no `importExternalTexture`, no IPC frame blits.
- **Library-shaped from day one.** Clean public API (`Container::new()`, `Sprite::from_texture()`, `MotionBlur::default()`), but no `cargo publish`, no semver burden, no contributor model. We get most of the benefits of library hygiene with none of the OSS overhead.
- **The "video texture battle-tested" gap is reduced** — when you control native ffmpeg decode and the wgpu texture upload directly, the unproven part is *just* "does my filter chain look right at 4K@60." That's a tractable engineering problem, not an unknown system.

### What we lose vs Pixi+Bun

- **~6–10 weeks of renderer-engineering work** that Pixi would have given for free (scene graph, filter chain, video texture binding, render-to-texture).
- **No Pixi DevTools** for debugging the scene graph. We add `tracing` instrumentation and a debug overlay.
- **No mature filter library to crib from.** We write motion blur, drop shadow, gaussian blur, color matrix, displacement in WGSL. Each is ~50–150 lines of focused shader work.
- **No sprite atlas tooling.** We don't need it for MVP (only ~5 sprite types — cursor, click ring, etc.).

### Why not Bevy

Bevy is a *game engine*. You'd be using ECS to model:
- one video texture
- one camera bubble
- one cursor sprite
- one background quad
- a handful of overlay quads (annotations, masks, captions, keyboard chips)

That's 6–10 entities at any time. The ECS overhead doesn't pay off; you'd write at least as much code wiring `bevy_ui` or `bevy_egui` for chrome as you would writing Leptos directly. Bevy also doesn't help with native macOS chrome (menu bar, file picker, share sheet, dock interactions, native overlay windows) — you'd still be calling `objc2`.

Bevy is the right answer if you treat the editor as a real-time particle/animation playground (think: 50 reactions on screen at once, complex animated transitions, custom per-pixel effects with GPU compute). For a screen recorder, that's overkill.

The one place Bevy *might* win is the timeline if you treat regions as ECS entities with components for `{TimeRange, Effect, Selection}` — but that's a 6-week timeline UI vs the 6-week timeline UI you'd write in Leptos either way.

### Why not pure native Cocoa + Metal (the ScreenKite approach)

You get max performance and the cleanest macOS experience, but you double every feature when you eventually port to Windows. Both SS and OS pay the cross-platform tax in different ways; you should choose to pay it *once*, in Rust, with a single render path.

Caveat: if you're macOS-first and accept "Windows is a stretch goal", native Cocoa is genuinely viable and 3–4× faster on export per ScreenKite's own benchmarks. But the threading-of-the-needle that Tauri+Leptos+wgpu offers — *one render path, three OSes, near-native perf* — is hard to give up.

### Why Leptos over Tauri's other options (Vue, Svelte, React-via-Tauri)

- **All-Rust:** the renderer (wgpu), the capture layer (objc2/windows-rs), the export pipeline (ffmpeg-next), the project format (serde), and the UI all share types. No FFI between UI state and engine state.
- **Reactivity model maps cleanly onto a timeline + inspector:** signals/effects are exactly what you need for "scrubbing the playhead causes the inspector to re-read X".
- **Hot-reload via `cargo-leptos`** is workable (not Vite-fast, but acceptable).
- **Bundle size** is excellent (compiled WASM for SSR/CSR depending on choice).
- **You don't lose the JS UI ecosystem entirely** — Tauri lets you embed `tailwindcss`, and Leptos works with most CSS-in-JS-free libraries.

### Why PixiJS WebGPU (and where wgpu still lives)

The render layer is PixiJS in the webview. Native `wgpu` is **not** in the MVP path — but it's a v2 hardening target if PixiJS+WebGPU hits a wall on Linux (WebKitGTK) or if export perf demands zero-copy GPU encode.

What stays native Rust:
- All capture (ScreenCaptureKit, Windows.Graphics.Capture, PipeWire).
- All encode/decode/mux (ffmpeg-next; later VideoToolbox/Media Foundation HW paths).
- All cursor telemetry capture (CGEventTap / SetWinEventHook / libinput).
- All audio capture and mixing (cpal).
- All file I/O, project format, autosave, recovery.
- Tauri shell, multi-window orchestration, IPC.

What runs in the webview (Leptos + PixiJS):
- All UI chrome (Leptos): timeline, inspector, settings, source picker, HUD.
- All compositing (PixiJS+WebGPU): zoom transform, motion blur, drop shadow, padding, rounded corners, cursor sprite, click effects, camera bubble, annotations, captions overlay, keyboard chips, 3D perspective.
- Editor preview = same scene graph used for export.

### Why this preserves OpenScreen's "single render function" architecture

The deterministic-from-state per-frame render is the architectural insight. With Option A:
- **Editor preview**: PixiJS renders to the on-screen canvas at the playhead time.
- **Export**: PixiJS renders to an offscreen WebGPU texture for each frame timestamp, posted to native Rust for encoding.

Same PixiJS scene graph, same render code, same effects. The only change is the destination of the rendered texture (screen vs Uint8Array → Rust). This is exactly OpenScreen's pattern, just with the encoder moved out of the renderer process where it belongs.

### Why Tauri 2 specifically

- Multi-window is first-class (you need 4 windows: HUD, source picker, countdown, editor) — matches both SS and OS architecture.
- `tauri::WebviewWindow` + a sibling native window with a wgpu `Surface` is a known integration pattern; alternative is a transparent overlay rendered by a Rust thread under the webview.
- File system access, autostart, single-instance, system tray, global shortcuts — all there without re-implementation.
- ~10 MB bundle size vs Electron's 100+ MB.

---

## 5. Concrete crate inventory

| Layer | Crates | Why |
|---|---|---|
| **Shell** | `tauri`, `tauri-plugin-fs`, `tauri-plugin-dialog`, `tauri-plugin-global-shortcut`, `tauri-plugin-single-instance`, `tauri-plugin-autostart` | Multi-window app shell |
| **UI** | `leptos`, `leptos_meta`, `leptos_router`, `leptos-use` | Reactive UI |
| **GPU render** | `wgpu`, `bytemuck`, `naga`, `glam` (math), `cosmic-text` or `fontdue` (text) — all wrapped by in-repo `crates/wisp` | The Pixi-equivalent library |
| **Native preview window** | `winit` (sibling window under Tauri), `raw-window-handle` | Editor preview canvas |
| **Telemetry transport (native ↔ webview chrome)** | Tauri `invoke` / `Event`, `serde_json` | Cursor stream, timeline state, project file ops |
| **macOS capture** | `objc2`, `objc2-foundation`, `objc2-app-kit`, `cidre` (alt), `screencapturekit-rs` (when stabilised), `core-graphics`, `core-foundation` | ScreenCaptureKit, AVFoundation, CGEventTap |
| **Windows capture** | `windows-rs`, `windows-capture` | Windows.Graphics.Capture / DXGI Desktop Duplication |
| **Linux capture** | `pipewire-rs`, `ashpd` (XDG portal), `wayland-protocols` | PipeWire screencast portal |
| **Click hooks** | `rdev` (cross-platform) or platform-native (`CGEventTap`, `SetWindowsHookEx`, `evdev`) | Global mouse click capture |
| **Audio capture** | `cpal`, `coreaudio-rs` | Mic + system audio fallback |
| **Encode/Decode/Mux** | `ffmpeg-next` (or `ac-ffmpeg`), `mp4` (mux only), `symphonia` (audio decode) | The reliability story |
| **HW encode (later)** | direct `objc2` bindings to `VTCompressionSession`, `windows-rs` Media Foundation | Beat SS on export speed |
| **GIF** | `gifski`, `image::codecs::gif` | High-quality GIF |
| **Transcription** | `whisper-rs` + bundled GGML model; `objc2` Speech framework on macOS | On-device |
| **Math / animation** | `glam`, `interpolation`, hand-rolled spring-physics | Cursor smoothing, zoom easing |
| **Project file** | `serde`, `serde_json`, `bincode` (for binary streams), `redb` or `sled` (event log) | Smaller project files than SS |
| **Async** | `tokio`, `flume` or `crossbeam-channel` | Capture/encode pipeline |
| **Logging / errors** | `tracing`, `tracing-subscriber`, `anyhow`, `thiserror` | |
| **Update / telemetry** | `tauri-plugin-updater`, `sentry-rust` | |

---

## 6. Things to lift from OpenScreen (with MIT attribution)

In ascending order of "how worth it":

1. **Project file JSON schema** (`projectPersistence.ts`, `types.ts`) — versioned, plain, well-thought-out.
2. **Bitrate ladder** (`useScreenRecorder.ts`): 4K=45M, QHD=28M, default=18M, ×1.7 for ≥60fps.
3. **Auto-zoom dwell-detection thresholds** (`zoomSuggestionUtils.ts`): 450–2600 ms windows, 0.02-unit movement threshold.
4. **Cursor-follow adaptive smoothing math** (`cursorFollowUtils.ts`): exponential-decay with distance-weighted lerp factor.
5. **Motion-blur velocity → kernel-size mapping** (`zoomTransform.ts`): `PEAK_VELOCITY_PPS=1400`, `MAX_BLUR_PX=14`.
6. **UX flow**: HUD → source picker → countdown → recording → editor (4-window architecture).
7. **Undoable vs non-undoable state distinction** in the editor reducer.
8. **The deterministic-from-state per-frame render function** used for both editor preview and export.

Cite MIT in `LICENSES/` and a `CREDITS.md`.

---

## 7. Risks & open questions

1. **Building a renderer is the new biggest line item.** The `render` crate is genuine library work — scene graph, transform tree, filter chain, video texture binding, render-to-texture. Budget **6–10 weeks** before the recorder UI starts integrating it. Mitigation: scope tightly to recorder needs; resist gold-plating.
2. **Timeline UI in Leptos** is the second biggest UI unknown. No `dnd-timeline` equivalent in Leptos. Budget 6 weeks. Consider porting `dnd-timeline`'s drag-region math as a small Rust crate that doubles as portfolio work.
3. **Native preview window inside Tauri.** Sibling/child native windows under Tauri 2 webview chrome work, but compositing and z-order are platform-specific:
   - macOS: `NSPanel` or child `NSWindow` under the webview's `NSView` — works but takes care.
   - Windows: child HWND under WebView2 — well-supported.
   - Linux/Wayland: most fragile path; subsurface compositing varies by compositor. Validate this in the spike.
   Fallback: render to an offscreen surface + stream RGBA frames into a `<canvas>` in the webview via shared memory. Slower, but works everywhere.
4. **Video texture battle-testing burden is now ours.** The "video frame uploads work" is API-level; the system-level "real-time scrub + filter + export" is the unproven part. Mitigation: build a synthetic stress test early — generate 30 minutes of dummy ScreenCaptureKit-shaped frames, scrub them at random with all filters enabled, log frame drops.
5. **Filter correctness.** Motion blur, drop shadow, gaussian blur in WGSL each have subtle bugs (premultiplied alpha, edge clamping, separable-pass kernel sizes). Plan for visual regression tests against reference images per filter.
6. **`screencapturekit-rs` maturity.** As of 2026-05, the wrapper crates are usable but evolving. You may write your own `objc2` bindings for the `SCStream` lifecycle. Plan for that.
7. **VideoToolbox bindings for HW H.264 encode.** No mature Rust crate; you'll write `objc2` glue. MVP can use software libx264 via `ffmpeg-next` and still beat OpenScreen on reliability; HW encode is a v2 perf milestone.
8. **Whisper bundle size.** Base model = 75 MB, Small = 460 MB. Bundle Base, download larger models on demand.
9. **Cross-platform click capture latency.** `rdev` is fine for desktop apps but check Windows kernel-hook latency with a click-burst test before committing.
10. **Export reliability is the differentiation pitch.** Make the headless export pipeline (`screen render in.json out.mp4`) the first integrated milestone after the renderer works. If it can render 1000 random project files without hangs/crashes/desync, you've already beaten OpenScreen.
11. **The "single render function" invariant**: editor preview and export use the *same* `render` crate scene graph. Don't fork them.
12. **Solo time-to-v1 estimate: 6–9 months.** Roughly: 6–10 weeks renderer + 6 weeks timeline UI + 4–6 weeks capture/encode + 4–6 weeks integration + 4–6 weeks polish & cross-platform. Two devs split well: one on render+capture+encode, one on UI+timeline+integration. Both estimates assume strong Rust comfort.

---

## 8. Validation spike plan (first 2 weeks)

Before committing to the full architecture, prove the three load-bearing assumptions:

**Week 1 — Renderer skeleton:**
- Cargo workspace converted; `crates/wisp` created with `wgpu` dependency.
- Render a textured quad in a `winit` window — basic `Sprite` API working.
- Add `Container` + transform tree with parent-child propagation.
- Render-to-texture (`RenderTexture`) round-trip: render scene to texture, sample texture, render to surface.
- Native window-in-Tauri spike: open a Tauri 2 window with Leptos chrome and a sibling `winit` window rendering the wgpu surface. Validate compositing on macOS, Windows, Linux/Wayland — flag platforms where it's painful.

**Week 2 — Filter chain + video texture:**
- Implement `MotionBlur` filter in WGSL (velocity-driven kernel; lift OpenScreen's `PEAK_VELOCITY_PPS=1400`, `MAX_BLUR_PX=14` constants). Test against a reference frame.
- Implement `DropShadow` filter (separable Gaussian + offset composite).
- `objc2` + ScreenCaptureKit: record 5 seconds of screen → `CMSampleBuffer` → `ffmpeg-next` decode → `wgpu::Queue::write_texture` → render textured quad with motion blur applied.
- `CGEventTap` cursor stream → JSON file with sub-frame timestamps.
- **Deterministic-from-state demo:** load the recorded video as a `VideoTexture`, render with a moving cursor `Sprite` re-derived from the JSON stream, with motion blur on the cursor and drop shadow on the recording quad. This is the smallest end-to-end proof.
- Headless export benchmark: render 60 frames of 1080p into `RenderTexture`, hand to `ffmpeg-next` H.264 encoder. Target: ≥30 fps export on M-series Mac.

If those work, the architecture is sound. The hardest unknown is the **native preview window under Tauri webview** on Linux/Wayland — if that fights us, fall back to rendering offscreen and streaming RGBA frames into a `<canvas>` in the webview via shared memory (slower but ships everywhere).

---

## 9. One-paragraph elevator pitch for the product

> *Cinematic screen recordings, native everywhere.* A Rust-based screen recorder that produces the polished, motion-blurred, click-zoomed, cursor-smoothed look of Screen Studio — at one-fifth the memory, with reliable exports on Windows and Linux from day one, and with the things SS users keep asking for: noise-suppression intensity, multi-track audio mixer, voiceover re-record, scroll-following masks, embed-able share links, and a CLI for headless rendering. Open-core: free desktop app, paid hosted sharing.

---

## 10. Recommended next step

Convert the current `screen` binary scaffold into a Cargo workspace, with `crates/wisp` as the first member. Run the 2-week validation spike (§8) starting with the renderer.

Workspace layout (target):

```
screen/
├─ Cargo.toml                # [workspace] members = ["crates/*"]
├─ rust-toolchain.toml       # nightly (already present)
├─ _docs/
├─ crates/
│  ├─ render/                # the Pixi-equivalent library — first to build
│  │  ├─ Cargo.toml
│  │  ├─ src/
│  │  │  ├─ lib.rs
│  │  │  ├─ scene/           # Container, Sprite, Mesh, Graphics, Text
│  │  │  ├─ filter/          # MotionBlur, DropShadow, Blur, ColorMatrix
│  │  │  ├─ texture/         # Texture, VideoTexture, RenderTexture
│  │  │  ├─ render/          # Renderer, batcher, filter pass orchestrator
│  │  │  └─ math/            # Mat3/Mat4 transforms, viewport
│  │  ├─ shaders/            # WGSL files
│  │  └─ examples/
│  │     ├─ hello_sprite.rs
│  │     ├─ filter_chain.rs
│  │     └─ video_texture.rs
│  ├─ capture-macos/         # ScreenCaptureKit + AVFoundation bindings
│  ├─ capture-windows/       # Windows.Graphics.Capture
│  ├─ capture-linux/         # PipeWire portal
│  ├─ encode/                # ffmpeg-next wrapper
│  ├─ telemetry/             # cursor + click event capture
│  └─ app/                   # Tauri 2 + Leptos shell, integrates everything
└─ examples/                 # cross-crate examples (e.g., end-to-end record→render→export)
```

Build order (critical path):
1. Convert workspace, scaffold `crates/wisp` skeleton.
2. **Weeks 1–2 spike** (§8): renderer + filter chain + video texture + capture probe.
3. Render library MVP (weeks 3–8): finish all required filters, scene graph polish, text rendering, RenderTexture orchestration. Ship 3–4 standalone examples in `crates/render/examples/`.
4. Native capture for macOS (weeks 9–10): cleanly wrap ScreenCaptureKit + AVFoundation behind a `capture` crate trait.
5. Encode pipeline (week 11): ffmpeg-next wrapper crate, headless `screen render` CLI works end-to-end.
6. Tauri+Leptos shell (weeks 12+): timeline UI, inspector, integration with render+capture+encode.

If the spike reveals a blocker (most likely: native preview window compositing under Tauri webview on Wayland), the fallback is rendering offscreen and streaming RGBA frames into a webview `<canvas>` via shared memory — slower but ships.
