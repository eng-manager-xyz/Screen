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
- [Dev loop — local](./conventions/dev-loop.md)
- [Remote dev — phone over Tailscale](./conventions/remote-dev.md)

# `wisp` — the wgpu renderer

- [Wisp at a glance](./wisp-overview.md)

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
  - [Tray → AppShell → NavRail routing (M-TRAY.0..4)](./app-ui/chunks/tray-to-appshell.md)
  - [Webcam bubble overlay (M-BUBBLE.0 + .3)](./app-ui/chunks/webcam-bubble.md)
  - [macOS permissions — embedded Info.plist](./app-ui/chunks/macos-permissions.md)
  - [Camera pipeline worker (M-CAM.3 gst layer)](./app-ui/chunks/camera-pipeline.md)
  - [Audio capture — microphone + system audio (M-AUDIO)](./app-ui/chunks/audio-capture.md)
  - [Recorder Page — live composition](./app-ui/chunks/recorder-page.md)

# `ui-storybook` — Leptos UI

- [Overview](./ui/overview.md)
- [Presentational contract](./ui/presentational-contract.md)
- [State boundaries](./ui/state-boundaries.md)
- [Review checklist](./ui/review-checklist.md)
- [Shared fixture library](./ui/fixtures.md)
- [Components](./ui/components.md)
  - [Design tokens](./ui/chunks/tokens.md)
  - [Surface primitives](./ui/chunks/surface-primitives.md)
  - [Navigation rail](./ui/chunks/navigation-rail.md)
  - [App shell](./ui/chunks/app-shell.md)
  - [Popover surface](./ui/chunks/popover-surface.md)
  - [Menu row](./ui/chunks/menu-row.md)
  - [Workspace switcher](./ui/chunks/workspace-switcher.md)
  - [Controls](./ui/chunks/controls.md)
  - [Capture mode tabs](./ui/chunks/capture-mode-tabs.md)
  - [Display source card](./ui/chunks/display-source-card.md)
  - [Capture source row](./ui/chunks/capture-source-row.md)
  - [Device picker menu](./ui/chunks/device-picker-menu.md)
  - [System audio picker](./ui/chunks/system-audio-picker.md)
  - [On-screen options](./ui/chunks/on-screen-options.md)
  - [Recording controls footer](./ui/chunks/recording-controls-footer.md)
  - [Save panel](./ui/chunks/save-panel.md)
  - [Tray record popover](./ui/chunks/tray-record-popover.md)
  - [Recording status button](./ui/chunks/recording-status-button.md)
  - [Library sidebar](./ui/chunks/library-sidebar.md)
  - [Recording card + grid](./ui/chunks/recording-card.md)
  - [Editor shell](./ui/chunks/editor-shell.md)
  - [Editor drop zone + canvas](./ui/chunks/editor-drop-zone-canvas.md)
  - [Inspector panel](./ui/chunks/inspector-panel.md)
  - [Timeline skeleton](./ui/chunks/timeline-skeleton.md)
  - [Cursor style picker](./ui/chunks/cursor-style-picker.md)
  - [Cursor preview canvas](./ui/chunks/cursor-preview-canvas.md)
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
