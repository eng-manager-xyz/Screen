# Summary

[Introduction](./intro.md)

# Project orientation

- [What this is](./orientation/what.md)
- [Stack](./orientation/stack.md)
- [Theatre metaphor](./orientation/metaphor.md)

# Conventions

- [Workflow](./conventions/workflow.md)
- [Testing](./conventions/testing.md)
- [Documentation gate](./conventions/docs.md)
- [Story / screenshot pipeline](./conventions/screenshots.md)

# `wisp` — the wgpu renderer

- [Overview](./wisp/overview.md)
- [Stories](./wisp/stories.md)
  - [Textured quad — M0.6](./wisp/chunks/hello-quad.md)
  - [Nested transforms — M0.7 / M0.8](./wisp/chunks/transform-nesting.md)
  - [Sprite batcher — M0.9](./wisp/chunks/sprite-batcher.md)
  - [Rounded rect with stroke — M0.12 / M0.13](./wisp/chunks/graphics-rounded.md)
  - [Animated click ripple — M0.13](./wisp/chunks/graphics-ellipse.md)
  - [Gradient fills — M0.14](./wisp/chunks/graphics-gradients.md)
  - [Bitmap text — M0.15](./wisp/chunks/text-bitmap.md)
  - [Blur filter — M0.16](./wisp/chunks/filter-blur.md)
  - [Drop shadow — M0.17](./wisp/chunks/filter-drop-shadow.md)
  - [Motion blur — M0.18](./wisp/chunks/filter-motion-blur.md)
  - [Color matrix — M0.18](./wisp/chunks/filter-color-matrix.md)
  - [Perspective rotation — M0.19](./wisp/chunks/mesh-perspective.md)
  - [Filter chain — M0.20](./wisp/chunks/example-filter-chain.md)
  - [Recorder mock — M0.21](./wisp/chunks/example-recorder-mock.md)
  - [Headless export — M0.21](./wisp/chunks/example-headless-export.md)
  - [Blend modes — M-BLEND.1](./wisp/chunks/blend-modes.md)
  - [Rounded crop foundation — M-MASK.1](./wisp/chunks/clip-rounded.md)
  - [Rectangle privacy blur — M-MASK.2](./wisp/chunks/privacy-blur-rect.md)
  - [Rounded privacy blur — M-MASK.3](./wisp/chunks/privacy-blur-rounded.md)
  - [Privacy blur strengths — M-MASK.4](./wisp/chunks/privacy-blur-strength.md)
  - [Solid redaction — M-MASK.5](./wisp/chunks/solid-redaction.md)
  - [Spotlight / highlight — M-MASK.6](./wisp/chunks/spotlight.md)
  - [Dim outside — M-MASK.7](./wisp/chunks/dim-outside.md)
  - [Ellipse mask — M-MASK.9](./wisp/chunks/ellipse-mask.md)
  - [Freehand path mask — M-MASK.10](./wisp/chunks/path-mask.md)
  - [Dynamic mask textures — M-DYN.1](./wisp/chunks/mask-texture.md)
  - [Mask texture cache — M-DYN.2](./wisp/chunks/mask-cache.md)
  - [Vector shape model — M-VEC.1](./wisp/chunks/vector-model.md)
  - [Render vector primitives — M-VEC.2](./wisp/chunks/vector-render.md)
  - [Vector → mask texture bridge — M-VEC.3](./wisp/chunks/vector-mask-bridge.md)
  - [Privacy blur on vector masks — M-VEC.4](./wisp/chunks/vector-privacy-blur.md)
  - [Solid redaction on vector masks — M-VEC.5](./wisp/chunks/vector-solid-redaction.md)
  - [Clip + spotlight on vector masks — M-VEC.6](./wisp/chunks/vector-clip-spotlight.md)
  - [Export & copy-frame mask parity — AUT-27/-33](./wisp/chunks/export-mask-parity.md)
  - [Compose through explicit mask — M-DYN.3..6](./wisp/chunks/compose-through-mask.md)
  - [Vector spotlight + dim — M-VEC.7](./wisp/chunks/vector-spotlight-dim.md)
  - [Vector highlight + callout — M-VEC.8 + M-VEC.9](./wisp/chunks/vector-highlight-callout.md)
  - [Path stroke + mask boolean ops — M-VEC.10 + M-VEC.11](./wisp/chunks/vector-path-stroke.md)
  - [Vector primitive gallery — M-VEC.12](./wisp/chunks/vector-gallery.md)
  - [Text architecture — M-TEXT.1](./wisp/text/architecture.md)
  - [Atlas vs Flexible text — M-TEXT.4](./wisp/text/atlas-vs-flexible.md)
  - [FlexibleText — Cosmic Text — M-TEXT.2](./wisp/text/flexible-cosmic.md)
  - [FlexibleText — Glyphon WGPU rasterizer — M-TEXT.3](./wisp/text/glyphon-backend.md)
  - [Text render-to-texture — M-TEXT.5](./wisp/text/textures.md)
  - [Text composition — mask / filter / blend / export — M-TEXT.6](./wisp/text/composition.md)
  - [Stroked / outlined text — M-TEXT.7](./wisp/text/stroke.md)
  - [Text style presets — M-TEXT.12](./wisp/text/presets.md)
  - [Word-wrapped caption block — M-TEXT.9](./wisp/text/caption-block.md)
  - [Drop shadow + glow on text — M-TEXT.8](./wisp/text/shadow-glow.md)
  - [Callouts, badges, arrows — M-TEXT.10](./wisp/text/callouts.md)
  - [Text as mask — fill, blur, spotlight — M-TEXT.11](./wisp/text/text-mask.md)

# `decode` — video decode

- [Overview](./decode/overview.md)

# `media` — capture + playback + timing

- [Architecture — M-MEDIA.0](./media/architecture.md)
  - [Clock + timestamp model — M-MEDIA.2](./media/clock.md)
  - [Audio data model — M-MEDIA.3](./media/audio.md)
  - [Mock audio sources — M-MEDIA.4](./media/mock-sources.md)
  - [GStreamer audio capture — M-MEDIA.5](./media/audio-capture.md)
  - [GStreamer video capture — M-MEDIA.6](./media/video-capture.md)
  - [A/V sync harness — M-MEDIA.7](./media/sync-harness.md)
  - [Audio histogram quantization — M-MEDIA.8](./media/histogram.md)
  - [Waveform bar geometry — M-MEDIA.9](./media/waveform-geometry.md)
  - [Audio histogram in Wisp — M-MEDIA.10](./media/audio-histogram.md)
  - [GStreamer audio → histogram example — M-MEDIA.11](./media/audio-histogram-gst.md)
  - [Video texture handoff — M-MEDIA.12](./media/video-texture.md)
  - [GStreamer videotestsrc through Wisp — M-MEDIA.13](./media/video-render.md)
  - [Synced video + audio in one scene — M-MEDIA.14](./media/synced-scene.md)

# `playback` — player state machine

- [Overview](./playback/overview.md)
  - [Real MP4 → wisp playback (M-DEC.2)](./playback/play-file.md)

# `preview` — native window

- [Overview](./preview/overview.md)
  - [Native winit window (M-PREVIEW.1)](./preview/chunks/preview-window.md)

# `app-ui` — recorder shell

- [Overview](./app-ui/overview.md)
  - [Tauri ↔ Leptos integration (M-INT.2)](./app-ui/integration.md)
  - [Player IPC (M-PLAY.2)](./app-ui/player-ipc.md)
  - [Testing tiers (M-TEST.1 / .2)](./app-ui/testing.md)

# `ui-storybook` — Leptos UI

- [Overview](./ui/overview.md)
- [Presentational contract](./ui/presentational-contract.md)
- [Components](./ui/components.md)
  - [Design tokens](./ui/chunks/tokens.md)
  - [Surface primitives](./ui/chunks/surface-primitives.md)
  - [Navigation rail](./ui/chunks/navigation-rail.md)
  - [App shell](./ui/chunks/app-shell.md)
  - [Popover surface](./ui/chunks/popover-surface.md)
  - [Menu row](./ui/chunks/menu-row.md)
  - [Button — variants](./ui/chunks/button-variants.md)
  - [Button — sizes](./ui/chunks/button-sizes.md)
  - [Card — header + body](./ui/chunks/card-basic.md)
  - [Drop zone — idle](./ui/chunks/drop-zone-idle.md)
  - [Drop zone — active](./ui/chunks/drop-zone-active.md)
  - [Player — paused](./ui/chunks/player-controls-paused.md)
  - [Player — playing](./ui/chunks/player-controls-playing.md)
  - [Player — near end](./ui/chunks/player-controls-near-end.md)
  - [Recording toolbar — idle](./ui/chunks/recording-toolbar-idle.md)
  - [Recording toolbar — recording](./ui/chunks/recording-toolbar-recording.md)
  - [Recording toolbar — paused](./ui/chunks/recording-toolbar-paused.md)
  - [Status bar — ready](./ui/chunks/status-bar-ready.md)
  - [Status bar — encoding](./ui/chunks/status-bar-busy.md)
  - [Status bar — error](./ui/chunks/status-bar-error.md)
- [Dope sheet](./ui/dope-sheet.md)
  - [Multi-track](./ui/chunks/dope-sheet-basic.md)
  - [Dense keyframes](./ui/chunks/dope-sheet-dense.md)
  - [Editor panel composition](./ui/chunks/card-with-dope-sheet.md)
  - [Editor mock — full composition](./ui/chunks/editor-mock.md)

# Milestones

- [M0 — wisp renderer](./milestones/m0.md)
- [M1 — Tauri drop-zone](./milestones/m1.md)

# API reference

- [Rustdoc index](./api.md)
