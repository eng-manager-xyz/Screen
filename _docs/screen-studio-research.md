# Screen Studio – Comprehensive Feature & Product Research

> Research compiled for a Rust-based competitor. Sources cited inline at the end of each major section. Date of research: 2026-05-08.

Screen Studio (`screen.studio`) is a macOS-only desktop app from a 3-person Polish indie studio (founder: Adam Pietrasiak / `@pie6k`). Built on Electron + React + TypeScript with a custom GPU-accelerated render/animation engine. Its core value proposition is "beautiful screen recordings, automatically" – the cinematic look (smooth cursor, click-driven auto-zoom, motion blur, padded gradient backgrounds) is generated post-recording from the captured cursor/click stream rather than during capture. The app is post-production-only: you record, then re-render through their animation engine on export.

---

## 1. Recording – Capture Modes & Sources

### Capture sources
- **Full display** capture (multi-display supported; area picker handles external/non-retina/ultrawide displays).
- **Window capture** – focuses recorded window before recording; supports fullscreen apps.
- **Custom area** capture with grid overlay, precise numeric crop inputs, and "Chrome / Safari" suggestion presets that snap to common viewport sizes.
- **iPhone / iPad** capture via USB-C-to-Lightning (USB-A discouraged); Apple Trust prompt required, device must be unlocked, dock not supported. Auto-detects model and applies matching device frame mockup. iPhone Mirroring (macOS 15+) also supported.
- **Webcam** – up to 4K, can also be 720p / 1080p, ideal FPS 30. Full-frame preview before record. Stereo or mono mic input. Custom aspect ratio for webcam.
- **Microphone** – mono-by-default conversion (single mic), stereo supported, auto-gain-control toggle, mute warning if recording with closed lid, "improve mic" enhancement (DSP normalize + noise suppression, runs on-device).
- **System audio** – per-app: pick which apps' audio to capture; other apps are ignored. Built on `ScreenCaptureKit` (macOS 13+). System audio off pre-13.0 is gracefully hidden.
- **Pause / resume** recording (3.2.0+, April 2025) with keyboard shortcut.
- **Speaker notes / teleprompter** during recording (DisplayLink-aware, fullscreen-aware).
- **Recording widget** (floating overlay) shows shortcuts being pressed in real time; can be hidden from output.
- **"Hide from dock while recording"** + system-tray menu for managing ongoing recordings.

### Platform & system requirements
- **macOS only**, recommended macOS 13.1 Ventura+. Some features gated on macOS 14 / 15 / 26 (Apple Speech Recognition for transcripts requires macOS 26).
- Both Apple Silicon and Intel Macs (Intel-specific crash fixes shipped 2024).
- Free trial w/ all features, no card required.
- No Windows/Linux build – mentioned as wanted "if we ever expand," but explicitly absent.

### Capture quality
- **Up to 4K @ 60 fps** export (capture matches display native resolution and refresh).
- Camera high-quality format settings (720p / 1080p / 4K).
- Captured raw files can be extracted ("export raw recording files") separately.

### Sources
- https://screen.studio/
- https://screen.studio/guide/recording-iphone-ipad
- https://screen.studio/changelog
- https://screen.studio/guide/camera

---

## 2. The Cursor Engine (their "killer feature")

This is the most-cited differentiator. Screen Studio re-renders the cursor as an overlay layer; the OS cursor in the captured pixels is hidden/replaced. The cursor stream (positions, click events, cursor-type changes) is captured separately and animated on top during render.

### Smoothing
- "Smooth mouse movement" – converts shaky raw motion into a glide. Spring-physics-based interpolation (mass / stiffness / damping; resembles framer-motion springs). Configurable `Optimize original cursor types` for smooth transitions between system cursors (arrow → I-beam → hand etc.).
- "Remove cursor shakes" advanced toggle to reject jitter from accessibility tools (e.g. macOS shake-to-find).
- "Rotate cursor while moving" – slight rotation aligned to velocity vector for natural feel.
- "Stop cursor movement at the end of the video" – prevents recording-stop motion artifact.

### Visibility
- Hide cursor entirely.
- "Hide cursor if it's not moving" with elegant fade animation when idle.
- Per-slice cursor hiding (hide cursor inside a specific timeline range).
- "Loop cursor position" – at end of recording, cursor smoothly returns to start position so video can be looped seamlessly.

### Appearance
- Adjustable cursor size post-recording (with high-resolution system cursor replacements – they ship their own bitmaps for big sizes since macOS only provides low-res cursors).
- Cursor type: macOS-style or Touch-style (large round indicator – primarily for iPad/iPhone via Bluetooth mouse).
- "Always use default system cursor" override (consistent look across text-select/link/etc).
- Custom cursor library (3.0+): pick from a large set of cursor sets including macOS Tahoe set, Halloween set, Figma cursors, etc.
- Auto-select cursor set based on detected macOS version (Tahoe cursors auto-applied on macOS 15+).

### Click effects
- **Ripple click effect** (toggleable).
- **Circle click effect**.
- **Mouse click sound effect** (3.3.0+, June 2025).
- Auto-zoom triggered on click (see §3).

### Sources
- https://screen.studio/guide/cursor
- https://screen.studio/changelog
- https://hub.screen.studio/

---

## 3. Auto-Zoom / Smart Zoom

The other major differentiator. Auto-zoom is **post-production**, not real-time: the algorithm analyses the captured cursor + click stream and inserts zoom keyframes into a separate "zoom track" on the timeline.

### How it works
- Heuristic detects click positions and creates a zoom range centered on the click.
- Default behaviour: zoom on every click (criticised as creating "drunk cameraman" feel).
- Manual zoom mode for ranges with no clicks – user drags zoom region on canvas + duration on timeline.
- Apply zoom level by typing 0–9 keys.
- "Apply this zoom to all other zooms" batch action.
- "Always keep zoomed in" mode for sustained focus.
- Min zoom duration 0.1 s (was 1 s; reduced 2024).
- Snapping to 50% within 1% tolerance for clean values.
- Zoom percentages shown in manual zoom picker.
- Pinch gesture on timeline to zoom timeline view (separate from content zoom).
- "Remove background move on zoom" – background stays still while content zooms.
- "Following cursor when zooming" – pan tracks cursor inside zoom region.
- Vertical mode automatically optimises zoom levels for portrait aspect ratios; "Disable automatic zoom in vertical mode" toggle.
- Copy/paste zooms across recordings; multi-select zoom ranges (Cmd+click) for batch edits.
- Reset all trims/cuts/zooms.

### Animation
- Custom spring animation engine drives zoom in/out (rewritten in 2.25.2, August 2024).
- "Motion blur" engine renders directional blur during zoom transitions and pans (rewritten 2.25.2). Configurable independently for cursor / zoom / pan motion.

### Sources
- https://screen.studio/guide/auto-zoom
- https://screen.studio/guide
- https://screen.studio/changelog
- https://medium.com/@jhleeroot/i-compared-every-macos-screen-zoom-drawing-tool-

---

## 4. Editing Surface

### Timeline
- Multi-track conceptual model: video track, zoom track, layout track (camera layout), mask track, audio track, captions track, keyboard-shortcut track, background-audio track.
- Trim, cut/split, ripple-delete, speed up/down per slice ("apply to all" speeds).
- Razor / split tool (option key); `C` cuts at playhead, `X` ripple-deletes.
- "Speed up typing" – auto-detects typing segments and accelerates them (3.0+).
- Slow-down ranges (slow part of recording).
- Per-clip audio volume with waveform that scales to volume level.
- Audio waveform preview on timeline; lazy-loaded for huge projects.
- "Always keep one track active", drag/resize without playhead jump, multi-select with Cmd+Click, snap-to-50%, zoom timeline animation, escape to close panels.
- Loop playback mode.
- Picture-perfect static-content playback (avoids exporter stalls).
- Project autosave.
- Project recovery (after crash / missing data).

### Editing tools
- Crop tool with grid overlay + numeric inputs + restore-on-cancel.
- Aspect ratio change (16:9, 9:16, 1:1, 4:5 + custom). Animations re-flow instantly to new ratio.
- Mask & highlight tool (3.1+, March 2025) – blur sensitive data or highlight an area. Fixed rectangle (does not follow scroll content; this is a known limitation). Adjustable opacity for highlights. Hotkey `4`.
- Speed control per slice (≥0.1 s clips, up to >2x with proper audio handling).
- Hide desktop icons during recording.
- Background audio sync to timeline.
- Animation effects (entry/exit – planned per roadmap).
- Reactions (built-in animated reaction effects).
- Command menu (⌘+K) for keyboard-driven actions across the editor (3.0+).

### Sources
- https://screen.studio/guide
- https://screen.studio/guide/adding-a-mask-and-highlight
- https://screen.studio/changelog

---

## 5. Visual Style – Backgrounds, Padding, Branding

- **Wallpaper library** – 100+ wallpapers by Blue Pixel Studio + Raycast wallpapers + Sonoma/Sequoia/Tahoe macOS wallpapers + iPadOS wallpapers + Glassmorphism set (3.4.11). Categories, favourites, random-pick button.
- **Custom solid color** – supports hex with or without `#`.
- **Custom gradient**.
- **Custom image background** (any user image).
- **Padding / outer spacing** adjustable.
- **Inset** around recorded content.
- **Rounded corners** (radius slider).
- **Drop shadow** (size / opacity).
- **Real-time inset color changes** (3.4.10).
- **Device frames** for iPhone/iPad recordings: full lineup of iPhone 11 → 17 Pro Max + iPad Pro 11" / 12.9" / iPad Air 5 + colour-accurate bezels per model. Horizontal iPad mockups. Toggle "disable device mockups". Dynamic Island placement per model.
- **Initial padding/background** preset applied to new projects.
- **Hide all shortcuts from timeline** style toggle.

### Sources
- https://screen.studio/changelog
- https://screen.studio/guide

---

## 6. Camera (Webcam Bubble)

- Position selector (preset corners + free placement).
- Roundness (corner radius from full circle to square).
- Size slider.
- Mirror toggle.
- Shadow under camera (with border-radius aware shadow rendering).
- "Camera size during zoom" – auto-shrinks camera so it doesn't cover zoomed content; can be disabled to keep size constant.
- Hide camera per-segment.
- Background blur – not built in; relies on macOS Portrait Mode (system-level) which Screen Studio passes through. This is a frequently requested missing feature.
- Custom aspect ratio for camera.
- **Dynamic Camera Layouts (3.0+)**: separate Layouts timeline track defines camera mode per segment – Full Screen, Overlay, Hidden. Smooth animated transitions between layouts.
- "Stay zoomed-out" auto behavior to avoid covering cursor.
- Hide camera preview before record (context menu).

### Sources
- https://screen.studio/guide/camera
- https://hub.screen.studio/p/blur-camera-background

---

## 7. Audio

### Microphone
- Stereo & mono support.
- Auto-noise-suppression + normalisation ("improve microphone audio") – on by default; some users report it degrades voice quality and request a toggle/intensity control.
- Auto-gain-control toggleable.
- Per-clip volume with waveform feedback.
- Mute warning when lid closed.
- Robotic-audio-at-1.2x bug history (fixed 3.4.4).

### System audio
- Per-app capture.
- Stereo channels preserved (fix landed 3.2.2 to consistently export 2-channel).

### Background music
- **Built-in royalty-free library** of tracks (3.4.9, Sept 2025) with audio preview.
- **Import** custom MP3 / MP4 audio files.
- Synchronised to timeline with editable position.
- Volume control.
- (Multi-track audio editing & fade in/out are user-requested but not fully exposed; basic volume only.)

### Mouse-click SFX
- Built-in click sound effect track.

### Roadmap / requested
- AI Voiceover (in progress on roadmap).
- Voice audio enhancement (planned).
- Independent mic-vs-system mixer (requested by users).

### Sources
- https://screen.studio/guide/background-music
- https://screen.studio/changelog
- https://screen.studio/roadmap
- https://hub.screen.studio/p/audio-track-on-editor

---

## 8. Captions / Transcription

- **Two engines**, both **on-device** (no cloud upload):
  - **Whisper** – Base / Small / Medium tiers (speed-vs-accuracy).
  - **Apple Speech Recognition** – requires macOS 26+ (added October 2025).
- Multilingual – ~100+ languages (the comparison page cites 106 via Whisper).
- Auto language detection or manual selection.
- Optional prompt field for product-specific terminology ("custom names").
- Edit transcript UI with typo correction.
- Export transcript as separate file.
- Caption size adjustable in preview; show/hide.
- Skips muted/cut sections during transcription.
- Captions are tied to recorded mic audio; no mic audio = no captions.

### Sources
- https://screen.studio/guide/captions
- https://screen.studio/changelog

---

## 9. Keyboard Shortcut Display

- Captures key events during recording and renders them as on-screen labels in the final video.
- Customisable shortcut labels (visual style, position).
- Single-key display toggle.
- Dedicated keyboard-shortcut timeline track (2.22.0+).
- Handles complex modifier combinations, FN keys, F1-F12, space symbol, correct Ctrl ordering.

### Sources
- https://screen.studio/create/screen-recorder-with-keyboard-shortcuts
- https://screen.studio/changelog

---

## 10. Export

### Formats
- **MP4** (H.264 video; codec not officially advertised but it's standard MP4 from VideoToolbox/ffmpeg pipeline). Recommended for anything >1 minute.
- **GIF** – with optimisation pipeline, "high quality" mode, GIF loop count, dedicated settings; explicit warning not to GIF clips longer than 1 min.
- **Frame-to-clipboard** export (single PNG of current frame from preview context menu, 2.25.18+).

### Resolutions / framerate
- Up to **4K (3840×2160) @ 60 fps**.
- 24 / 30 / 60 fps presets.
- Aspect ratios: 16:9 horizontal, 9:16 vertical, 1:1 square, 4:5 portrait social (3.0.0+), custom.

### Compression
- Quality presets (Low/Medium/High style). Notably, the docs say compression level does **not** affect export duration – only FPS, resolution, format do.
- "Reduce export memory" advanced option.

### Multi-export
- **Export multiple projects at once** (3.0+ batch export).
- Quick-export with previous settings.
- New projects inherit previous export settings.
- Quick-share widget with auto-save.
- Export to clipboard (2.6.0).

### Performance
- Multi-threaded export (experimental in 2.22.14, mainstream after).
- New rewritten exporter engine in 3.0.0 (faster).
- 25% export-speed bump in 2.25.18.
- Export speed remains a complaint vs native competitors – ScreenKite (Swift+Metal+VideoToolbox) reports 3-4× faster.

### Raw files
- "Extract separate recording files" – export camera, screen, mic as separate streams for use in Premiere / DaVinci.

### Sources
- https://screen.studio/guide/explanation-of-export-settings
- https://screen.studio/changelog

---

## 11. Sharing / Cloud

### Shareable Links
- Cloud-hosted player at `screen.studio/share/<id>`.
- **30-minute hard cap** (was 15 min). Beyond this you must export locally.
- Public or **private** links (3.4.4+, July 2025) requiring login/access.
- **Comments** on shared videos (3.4.10+).
- **View counter** (3.5.1+, Nov 2025).
- Title editable from quick-export widget.
- Manage all links via in-app dashboard ("Manage Shareable Links").
- Reduced branding on link pages (June 2025).
- Pause without screen obstruction.

### Missing (vs Loom / Tella)
- No team plan / workspace / shared library (in progress on roadmap).
- No embedded player.
- No analytics (only views; no engagement, traffic source).
- No password protection.
- No video CTA / call-to-action overlays.
- No trim-after-share / re-record.

### Sources
- https://screen.studio/guide/shareable-links
- https://www.tella.com/alternatives/screen-studio
- https://screen.studio/changelog

---

## 12. Presets, Project Files, Project Management

- **Preset files** with `.screenstudiopreset` extension (introduced 2.12.0, June 2023). Stored in `~/Documents/Screen Studio Presets`. Shareable via file send / drag-and-drop. Custom background images supported in presets.
- Preset auto-applied to new recordings.
- Custom preset storage location.
- Project autosave.
- Project recovery (handles missing/corrupted data).
- Project files store raw recordings + metadata; can be very large (a 3-hour recording produced a 40 GB project containing tens of thousands of files – hint that captures are likely stored frame-by-frame or chunked).
- Open Recent menu with reliability fixes.
- Project rename from topbar / quick-export widget.
- Project file name validation (#, special chars).
- Project creation from existing video (MP4 / MOV) – limited to those formats.
- Delete project on exit option.
- Custom default project location.

### Sources
- https://screen.studio/guide
- https://screen.studio/changelog
- https://x.com/pie6k/status/1671456814469136385

---

## 13. Hotkeys, Automation, Integrations

- **Command menu (⌘+K)** in 3.0+ for keyboard-driven action access.
- 0–9 number keys to apply zoom.
- C / X / Option for split / ripple-delete / razor.
- CMD+wheel timeline zoom; pinch to zoom.
- Pause/resume recording shortcut.
- Cut at playhead shortcut.
- **Raycast extension** + URL scheme (`screen-studio://...` integration) for triggering recording from Raycast.
- Affiliate program.
- License key system: device-based activation, refunded subscriptions deactivated, lifetime keys honoured.

### Sources
- https://screen.studio/changelog
- https://hub.screen.studio/p/raycast-screen-studio-extension

---

## 14. AI Features (current + roadmap)

Current:
- On-device Whisper transcription.
- Apple Speech Recognition transcription.
- Auto-zoom heuristic (rule-based, not ML).
- Cursor smoothing (spring physics, not ML).
- "Speed up typing" auto-detection.

Roadmap (announced):
- AI Voiceover (in progress).
- Voice audio enhancement (planned).
- Multi-clip merging.
- Annotations.
- Text slides.
- Enter/exit animations.
- Create videos from screenshots.

Notably absent vs competitors (Tella, Descript): AI mistake removal / filler-word removal / silence trimming / AI b-roll / AI captions styling.

### Sources
- https://screen.studio/roadmap
- https://www.tella.com/alternatives/screen-studio

---

## 15. Pricing & Licensing

| Plan | Price | Notes |
|---|---|---|
| Monthly (subscription) | $20/mo (recently raised toward $29/mo per third-party reviewers) | Switchable to yearly |
| Yearly (subscription) | $108/yr (≈ $9/mo) | 70% off vs monthly |
| Lifetime | $229 (legacy / not consistently advertised; reviewers in 2026 say no longer sold to new buyers) | One year of updates included; renew updates at $109/yr |
| Educational | 40% off (~$5.40/mo) with .edu | Self-serve request page |

- Free trial: all features, no card required, no time-limit pressure mentioned.
- No watermark on free trial output (per reviewer reports – they intentionally let people use the full feature set during trial).
- Lemon Squeezy handles billing.
- Founder publicly noted the app's license check is trivially crackable ("return true in one place") and they intentionally don't try to hard-DRM it.

### Sources
- https://screen.studio/#pricing
- https://matte.app/blog/screen-studio-review
- https://screenstudio.coupons/screen-studio-review/
- https://x.com/pie6k/status/1847026857935266283

---

## 16. Tech Stack (confirmed + inferred)

Confirmed:
- **Electron** (founder confirmed: "A bit awkward to say – but Screen Studio is an Electron app made with web technologies").
- **React + TypeScript** UI; multi-window architecture using React portals (founder's blog post on this technique).
- **macOS-native bridges** for capture: `ScreenCaptureKit` (system audio + window/area capture, gated to macOS 13+), `AVFoundation` for camera + mic, `Speech` framework for Apple Speech Recognition.
- **Whisper.cpp**-style local model bundle for transcription (on-device).
- **Lemon Squeezy** for subscriptions.
- **Sentry / crash-reports** integration.

Inferred:
- Custom WebGL/WebGPU compositor for the preview pipeline – necessary to render zoom/pan/cursor/motion-blur in real time at edit time. (OpenScreen's clone uses PixiJS; Screen Studio likely runs a similar GPU pipeline inside its renderer process.)
- Final export likely AVFoundation/VideoToolbox via native side, possibly with ffmpeg fallback for GIF.
- Custom motion-blur shader (rewritten 2.25.2).
- Custom spring-animation engine.
- Project files appear to chunk raw video on disk per recording with a separate JSON-ish manifest of cursor events, click events, audio, and edit graph (40 GB / "tens of thousands of files" for 3 hours implies many small chunk files).

### Sources
- https://x.com/pie6k/status/1624535267401924611
- https://buildwith.app/apps/screenstudio
- https://www.linkedin.com/posts/adampietrasiak_creating-multi-window-electron-apps-using-activity-7062718958289793024
- https://news.ycombinator.com/item?id=47595695

---

## 17. Killer Features (competitors typically lack)

In rough order of how often they're cited as the reason people pay:

1. **Spring-physics cursor smoothing with cursor-type-aware re-rendering** – most clones approximate it; SS feels noticeably more polished. The cursor is fully synthetic (rendered from event stream + sprite), which is what enables the high-res scaling, rotation, and looped-position trick.
2. **Click-driven auto-zoom with timeline editability** – the heuristic is post-production, the zoom keyframes are surfaced as draggable ranges, and they animate with custom motion-blur.
3. **Premium default visual style** – padded gradient/wallpaper background + rounded corners + shadow + inset, all GPU-composited live in preview.
4. **On-device transcription with two engine choices** (Whisper + Apple Speech) and a prompt field for custom vocabulary – privacy story is genuinely strong.
5. **Dynamic camera layouts** track (full-screen / overlay / hidden) with animated transitions across segments.
6. **Speed-up-typing auto-detection** – removes the most boring part of demos automatically.
7. **Loop cursor position** – tiny but extremely loved feature; trivially nice for marketing/social loops.
8. **Custom keyboard-shortcut overlay** – auto-rendered, far better than competitors' manual annotation.
9. **High-fidelity device mockup library** with per-model bezels and Dynamic Island accuracy.
10. **Royalty-free background music library** in-app.
11. **Sensitive-data masks + highlights** with per-frame opacity.
12. **Quick-share widget** for one-click upload-and-link.
13. **Preset files (`.screenstudiopreset`)** that codify an entire visual look and can be sent between teammates.
14. **Raycast deep integration**.

---

## 18. Reported Pain Points / Limitations (from reviews)

These are the gaps your Rust competitor can pick from:

### Performance
- Export speed is the #1 complaint. Native competitors (ScreenKite using Swift+Metal+ScreenCaptureKit+VideoToolbox) advertise 3-4× faster export. Electron's render-process overhead is the bottleneck.
- 4K + 60 fps + long recording = extremely large project files (40 GB for 3 hours) and slow uploads to shareable links.
- Long recordings (>30 min) feel sluggish in the editor; founder himself acknowledged "performance issues when auto-generating 30 minutes long animation."

### Audio
- "Improve microphone audio" can over-process voice; users want a toggle / intensity slider.
- No built-in voiceover recording / re-record audio over existing video.
- No multi-track mixer (system vs mic vs music with independent levels).
- AI features perform poorly on system audio.
- Audio desync after speed changes / cuts (history of bugs in changelog).
- Clipping / metallic export artifacts (multiple changelog fixes).

### Editing
- Masks don't follow scrolling content – they're fixed rectangles on screen coordinates. Big complaint for tutorial recordings of websites that scroll.
- No annotation / text-overlay tools (planned).
- No multi-clip / multi-recording merge into single project (planned).
- No B-roll / picture-in-picture beyond the camera bubble.
- No real keyframing for zoom (just region+duration, plus easing curves are hard-coded spring).
- Auto-zoom-on-every-click can be too aggressive ("drunken cameraman").

### Sharing
- 30-minute hard cap on shareable links.
- No embed.
- No analytics (just view count).
- No password protection.
- No team plan (in progress).
- Slow upload to cloud after long recordings.

### Platform
- macOS only.
- Mac-only sync (no iCloud sync of project files between Macs).
- No mobile companion app.
- Lifetime license discontinued for new buyers in 2026.

### iOS recording
- Cannot record finger taps (Apple privacy restriction). No auto-zoom on iOS recordings.
- iPad mockup library limited (some users report only iPad Air 5).
- USB-A unreliable.

### Sources
- https://www.producthunt.com/products/screen-studio/reviews
- https://efficient.app/apps/screen-studio
- https://matte.app/blog/screen-studio-review
- https://hub.screen.studio/

---

## 19. Release Velocity

From the changelog: ~ 60+ named releases in the last 24 months. Cadence is roughly 1 release every 2-3 weeks. Major arc:

- **2.5–2.7 (May 2023)**: Mouse cursor rotation, click effects, transcript editor, Raycast integration.
- **2.10–2.14 (June 2023)**: Audio waveform, speed ranges, GIF export pipeline, system audio, iPhone/iPad recording, mute mic, hide cursor when static, loop cursor position, motion blur option.
- **2.16–2.17 (July-Aug 2023)**: Wallpaper library overhaul (100+), speaker notes, noise reduction.
- **2.20–2.22 (Feb-Mar 2024)**: Keyboard shortcuts timeline, multi-threaded export.
- **2.25 (Aug-Nov 2024)**: Massive editor overhaul – timeline performance, snapping, copy/paste zooms, batch zoom edits, motion-blur engine rewrite, animations engine rewrite, ripple click effect, custom webcam aspect, MP4-import projects, frame-to-clipboard.
- **3.0 (Dec 2024 – Feb 2025)**: Shareable links, dynamic camera layouts, custom cursors, command menu (⌘K), batch export, quick-share widget, type-acceleration, new exporter engine.
- **3.1 (Mar 2025)**: Masks & highlights.
- **3.2 (Apr-May 2025)**: Pause/resume, iPhone audio, 4:5 ratio, mask polish, audio fixes.
- **3.3 (June 2025)**: Mouse-click sound, longer share links.
- **3.4 (June-Oct 2025)**: Background-music library, private shareable links, comments, glassmorphism wallpapers, iPhone 16/17 mockups, Tahoe cursors, high-quality camera modes.
- **3.5–3.6 (Oct 2025 – Feb 2026)**: Apple Speech Recognition transcripts, iPhone Mirroring, Halloween cursors, view counter, slice-speed apply-to-all.

### Sources
- https://screen.studio/changelog

---

## 20. Implications for a Rust Rebuild

The hard problems decompose roughly into: capture, animation, render preview, export, project storage, and shell/UI. Each pushes a different stack choice.

### Hard Problems (ranked by technical difficulty)

1. **Real-time GPU-accelerated preview compositor** – this is the heart of the product. Every frame in the editor is composited from: raw screen video (decoded), camera video (decoded), background (gradient/image), padding, rounded corners, shadow, optional zoom transform with motion blur, cursor sprite (with smoothing trajectory), click ripple animation, masks (blur), captions, keyboard-shortcut chips, layout track. All of this must hit 60 fps at 4K while scrubbing. → **wgpu** (Rust-native, Vulkan/Metal/DX12) is the right answer. A render graph similar to `bevy_render` would fit.
2. **Smooth cursor synthesis from a click/move stream** – capture timestamps must be sub-frame accurate. Spring-physics interpolation (mass/stiffness/damping) is straightforward but the trajectory smoothing (low-pass + spring + jitter rejection + cursor-type-aware crossfade) needs careful tuning. Easy in pure Rust; the tuning is the hard part. Reference: framer-motion / spring physics literature.
3. **Auto-zoom heuristic** – cluster click events, decide zoom region (extents around interaction), pick a duration, ease in/out, avoid "drunken cameraman" by suppressing rapid consecutive zooms. Good place to outdo SS – you can do better than "zoom on every click" with simple heuristics (DBSCAN clusters of clicks within N seconds + saccade-style smoothing between regions). Pure Rust, no GPU.
4. **System audio capture on macOS** – `ScreenCaptureKit` is the only sanctioned API. You'll need an `objc2` / `swift-bridge` layer. Not trivial but not novel. On Windows you'd use `WASAPI` loopback; on Linux PipeWire.
5. **Screen capture w/ cursor stream** – `ScreenCaptureKit` for pixels (excluding cursor) + `CGEventTap` for cursor positions and clicks at high resolution. Or just `ScreenCaptureKit`'s built-in cursor but you'd have to extract it. Most clones (ScreenKite) draw their own cursor, capturing pixels with cursor hidden + a separate `CGEvent` stream for positions.
6. **Motion blur** – directional blur shader along velocity vector during zoom/pan transitions. wgpu compute shader. Standard postprocess; tunable params.
7. **GPU-accelerated GIF export** – non-trivial. Palette quantisation + dithering. Screen Studio uses ffmpeg's `palettegen` + `paletteuse`; this project uses GStreamer's `gdkpixbufdec` path or the `gifski` crate (no ffmpeg — see [AUT-144](https://linear.app/harwood/issue/AUT-144)).
8. **Motion-blur-aware H.264/H.265 export** – this project uses GStreamer's `vtenc_h264_hw` element on macOS (wraps VideoToolbox), `mfh264enc` on Windows (Media Foundation), `vaapih264enc` / `nvh264enc` on Linux. Push BGRA frames from your wgpu render passes into `appsrc`; the encoder element selection is the only platform-specific bit.
9. **Webcam capture** – `AVCaptureDevice` on macOS via `objc2`. Standard, but needs camera-format negotiation for 720p/1080p/4K.
10. **On-device transcription** – ship Whisper (`whisper-rs`) and bridge Apple Speech via `Speech.framework` for macOS. Whisper bundles add ~150MB+ depending on model.
11. **Robust project file format** – SS's 40GB-for-3hrs is hint they store frames raw. You can do better: chunked H.264 segments + a SQLite or sled-backed event log + JSON edit graph. Goal: random-access scrubbing without re-decoding. mp4 fragments + manifest works.
12. **Multi-window Electron-equivalent UX** – recording widget, picker, dock-attached menu, fullscreen-aware overlays. Native macOS overlay windows need `NSPanel`-level access.
13. **Cloud sharing (`/share/<id>`)** – out of scope for app, but project format must be uploadable in chunks.

### Stack Decision Framework: Leptos+Tauri vs Bevy+Tauri vs Native Cocoa+Rust

Given the above, the bottleneck is **the editor's GPU compositor**. There are three viable paths:

#### A) Tauri 2 + Leptos (web UI) + custom wgpu surface for preview
- **Leptos** drives the chrome: timeline, sidebar, inspector, settings. Renders into Tauri's webview.
- **wgpu** surface is hosted in a sibling native window or as a transparent overlay rendered by a Rust thread, with the Leptos UI drawing chrome on top via DOM.
- Works because: most of the app is form chrome, and the perf-critical surface is one big GPU canvas. Tauri 2 supports multi-window which matches SS's recording-widget / picker / editor split.
- Risks: integrating wgpu with Tauri's webview composition isn't seamless; you'll need a custom `WebviewWindow` + child native window or use `RawWindowHandle` to render under the webview. Workable but fiddly.
- **Best fit if:** you want fast UI iteration with web ergonomics and accept some integration plumbing.

#### B) Bevy + Tauri (or Bevy + egui)
- Bevy handles rendering AND the app shell. Its ECS is overkill for chrome but ideal for the timeline/render-graph layer.
- `bevy_egui` for chrome – pragmatic, ugly by default, but native-feeling and zero-friction with the ECS world.
- **Best fit if:** you treat the app as fundamentally a real-time rendering pipeline with a UI bolted on (which it is). Lower iteration speed on chrome, but the preview window is essentially free.
- Bevy 0.15+ has a usable render graph that can do compositing trees. Custom cursor sprite + zoom transform + motion-blur post-process is straightforward.
- **Risk:** native macOS chrome (menu bar, file picker, system share sheets) requires `objc2` work; Bevy doesn't help here.

#### C) Native Cocoa (objc2) + Rust core + Metal directly
- Highest performance ceiling, matches what ScreenKite did to beat SS by 3-4x.
- Rust core for capture, editing, encode; Cocoa for chrome; Metal for compositor.
- **Risk:** this is essentially writing a Mac app and doubling everything for Windows later. Cross-platform is much harder.

**Recommended:** **Tauri 2 + Leptos for chrome + a dedicated wgpu render thread for the preview canvas + native Rust modules (`objc2`, `windows-rs`) for capture**. This matches the actual SS architecture (Electron + React + native bridges + WebGL canvas) but with a 3-5× perf bump from wgpu vs WebGL and a 2-10× perf bump from Rust vs JS in the encode/decode hot paths. You keep cross-platform optionality, and Leptos's reactivity model maps cleanly onto a timeline / inspector UI.

Bevy is the right choice **only if** you intend to ship effects, particles, complex layered animations, and treat the editor like a game engine – which is overkill for a screen recorder.

### Concrete crates to evaluate
- `wgpu`, `winit`, `tao` (Tauri's window backend).
- `cidre` or `objc2` + `objc2-foundation` for ScreenCaptureKit / AVFoundation bindings.
- `coreaudio-rs` for audio.
- GStreamer for encode/decode/mux: `gstreamer-rs` Rust bindings + `appsrc` for the encode side (wgpu render targets push BGRA → encoder), `gst-launch-1.0` CLI-subprocess for the decode side (already shipped — see `decode::gstreamer_pipe`). Element selection per platform: `vtenc_h264_hw` (macOS) / `mfh264enc` (Windows) / `vaapih264enc` / `nvh264enc` (Linux). **No `ffmpeg-next`** — see [AUT-144](https://linear.app/harwood/issue/AUT-144).
- `whisper-rs` for transcription.
- `dasp` / `cpal` for audio editing graph.
- `rfd` for native file dialogs (cross-platform).
- `serde` + `bincode` for project format; `sled` or `redb` for event log.
- `interpolation` / custom spring crate for cursor / zoom easing.

### Differentiation opportunities (where SS is weak and Rust gives leverage)
- **3-5× faster export** at the same quality (the most-cited complaint).
- **Smaller project files** with smarter chunking (you've seen the 40GB/3h problem).
- **Mask follows DOM/element** – compute optical-flow on a region between frames and translate the mask. SS's #1 missing feature. Rust+wgpu makes this tractable.
- **Auto-zoom that's actually good** – cluster clicks, suppress jitter, predict the next zoom from typing/scroll behaviour.
- **Native cross-platform** (macOS + Windows) from day one – SS still doesn't have Windows after 4 years.
- **Multi-track audio mixer** with independent mic/system/music levels and fade curves.
- **Real keyframable zoom/cam path** for power users while keeping the auto mode on by default.
- **Streaming export** – upload-while-encoding to remove the "wait for export, then wait for upload" double-pause that frustrates SS users.

---

## Appendix: Page Inventory (for further drilling)

Public pages discovered on `screen.studio`:
- `/` – home
- `/#pricing` – pricing anchor
- `/download` – download page
- `/changelog` – changelog
- `/roadmap` – roadmap
- `/guide` – product guide root
  - `/guide/cursor`, `/guide/auto-zoom`, `/guide/camera`, `/guide/captions`, `/guide/background-music`, `/guide/recording-iphone-ipad`, `/guide/adding-a-mask-and-highlight`, `/guide/shareable-links`, `/guide/explanation-of-export-settings`, `/guide/sharing-preset`, `/guide/animations-motion`, `/guide/dynamic-camera-layouts-`
- `/create/*` – marketing landing pages: `/create/record-engaging-courses`, `/create/screen-recorder-with-audio`, `/create/screen-recorder-with-keyboard-shortcuts`, `/create/product-demo-videos`, `/create/loom-vs-screen-studio`, `/create/instagram-tutorials`
- `/license/remind`, `/license/request-educational-discount`
- `/legal/privacy-and-cookie-policy`, `/legal/terms-of-service`
- `/affiliate`, `/brand-materials`
- `/dashboard` – account
- `/beta` – beta builds
- `https://hub.screen.studio/` – feature requests / roadmap voting (Canny-style)
- `https://t.me/screen_studio` – Telegram community
- Twitter: `@screenstudio`, `@pie6k`
- Founder blog: `pietrasiak.com`
