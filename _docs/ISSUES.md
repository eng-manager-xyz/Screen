# Issues, Deferrals, and Open Questions

Append entries during development for: bugs, deferred features, technical debt, decisions to revisit, open questions.

**Format:** newest at top. Issue IDs `ISS-NN` are sequential, never reused.

---

## Template

Copy and fill when filing a new issue.

```
## ISS-NN: <short title>
- **Filed:** YYYY-MM-DD
- **By:** <task ID like M0.4 — or "user">
- **Severity:** bug | deferral | question | tech-debt
- **Affects:** <crate / file / feature area>
- **Status:** open | resolved | closed-wontfix
- **Description:**
  <what's the issue, what was expected, what was observed, why it matters>
- **Resolution:** (fill in when closing)
  <how it was fixed, or why it was closed without fixing, with date>
```

---

## ISS-22: path-boolean De Morgan proptest is seed-flaky at boundary points
- **Filed:** 2026-06-01
- **By:** M-EDIT closeout (surfaced on a macOS gate run, unrelated to the branch)
- **Severity:** bug (test reliability) — pre-existing, NOT introduced by M-EDIT
- **Affects:** `crates/wisp/tests/path_boolean_proptest.rs` (`de_morgan_on_samples`), `crates/wisp` path-boolean point classification
- **Status:** open
- **Description:** `de_morgan_on_samples` intermittently fails in CI: `De Morgan failed at Vec2(0.0, 0.0): in_union=false, in_a=true, in_b=false`. The property checks `in_union == in_a || in_b`; it broke at exactly `(0,0)` — almost certainly a sampled test point landing **exactly on a path vertex/edge**, where point-in-region classification is ambiguous (boundary points are implementation-dependent under even-odd / winding). The same code passed on the immediately-prior commit and on Ubuntu+Windows; the failure is **seed-dependent**. Worse, CI logs `proptest: FileFailurePersistence::SourceParallel set, but failed to find lib.rs or main.rs` — proptest can't write the failing seed to a `.proptest-regressions` file from the CI cwd, so failing cases are never pinned and the test stays silently flaky. Last touched in `9caadbb` (M-BOOL); untouched by the M-EDIT branch — confirmed via `git diff main...HEAD`.
- **Resolution:** When fixed (its own focused PR, not the M-EDIT closeout): make the proptest robust by **not sampling within an epsilon of any path boundary** (boundary-sensitive equality must avoid boundary points), or assert the property with a boundary-exclusion guard; and fix the regression-file persistence (set an explicit `FileFailurePersistence::Direct(path)` or a `PROPTEST_REGRESSIONS` location that exists in CI) so failing seeds are pinned and reproducible. Optionally make boundary-point classification deterministic in the path-boolean impl. (open)

## ISS-21: live editor preview is a placeholder — cinematic compose is export-only
- **Filed:** 2026-06-01
- **By:** M-EDIT closeout (verifying AUT-351/353/354 "reflected in preview")
- **Severity:** tech-debt / deferral (affects ticket closeability)
- **Affects:** `crates/app-ui/src/editor_surface.rs` (the canvas placeholder), `crates/app/src/editor_preview.rs` (`EditorPreview` is constructed only by `editor_export.rs`), AUT-341 (ED.6) / AUT-351 / AUT-353 / AUT-354
- **Status:** ✅ resolved 2026-06-01 (by AUT-510)
- **Description:** The render-integration features (zoom ED.16, crop ED.15, background ED.18, cursor ED.19) are wired into the **export** path (`ExportFrameGenerator` → `EditorPreview::render_framed[_with_cursor]`) and verified there. But the **live on-screen editor preview** did not render the composed frame: `editor_surface.rs` showed a literal `"Preview renders here."` placeholder, `EditorPreview::render_at` had **no production caller**, and there was **no winit sibling window** for the editor. So zoom/crop/background/cursor were **export-only**, and the "reflected in preview" half of AUT-351/353/354's "Done when" was not visually true in committed code. This is the domain of **AUT-341 (ED.6)**, which was marked *Done* but had shipped the compose pipeline + a manually-verified prototype rather than a committed production render loop.
- **Resolution:** AUT-510 wires the live preview via the **offscreen-RT-to-webview** path (not a winit window): the export's per-frame compose was extracted to a shared `editor_export::compose_project_frame` (preview/export parity by construction); a `Mutex`-guarded `EditorPreviewState` holds the wgpu compose pipeline + a seekable decode stream; `editor_preview_open` / `editor_preview_frame` commands open the session + return composed BGRA at the playhead; and the editor canvas polls + `putImageData`s it (mirroring `<CameraPreview />`). The placeholder is replaced by a `<canvas>` sized to the clip. Backend headless-tested (`compose_project_frame_produces_full_bgra`); the live canvas display is GUI-verified on a real Mac (can't run headless). (resolved)

## ISS-20: ED.17 — active-window telemetry for window-aware auto-zoom (enhancement)
- **Filed:** 2026-06-01
- **By:** ISS-16 closeout (AUT-352)
- **Severity:** deferral
- **Affects:** `crates/app/src/click_capture.rs` / `cursor_capture.rs`, `crates/edit/src/telemetry.rs`
- **Status:** open
- **Description:** AUT-352's "Done when" lists three telemetry signals — cursor-move, click, **active-window** events. The first two ship (cursor poll + `CGEventTap` click log → `auto_zoom_segments` → `project.zooms` on import). The **active-window** signal (which app/window has focus, for window-framed auto-zoom and tighter cluster targeting) is not captured. The click-cluster auto-zoom is the headline and is delivered; active-window is a secondary refinement.
- **Resolution:** When implemented: observe focus changes (macOS `NSWorkspace.didActivateApplicationNotification` or the AX API) on the same recording clock, persist as a window-event track, and feed it into the auto-zoom generator to bias cluster centroids / windows toward the focused window's frame. Pure-test the generator extension; the capture is runtime-only like the click tap. (open)

## ISS-18: ED.21 — pitch-preserving audio retime (enhancement)
- **Filed:** 2026-06-01
- **By:** ED.21 closeout
- **Severity:** deferral / enhancement
- **Affects:** `crates/app/src/editor_export.rs` (`retime_audio`)
- **Status:** open (deliberate v1 decision, not a bug)
- **Description:** `retime_audio` resamples per segment, so a sped-up segment rises in pitch (and a slowed one drops) — the **industry-default** for editor speed ramps (Premiere / FCP / DaVinci / Loom all default to speed-with-pitch). This is the intended v1 behavior, pinned by the `retime_audio_double_speed_shifts_pitch_with_tempo` contract test. A pitch-*preserving* retime (time-stretch) is an enhancement, not a fix.
- **Resolution:** When implemented: a pure-Rust WSOLA / overlap-add time-stretch (~100–200 lines, stays CI-testable) slotted in behind the *same* pure `(samples, segments, source_fps, sample_rate, channels) -> Vec<f32>` contract — callers + the existing tests don't change. (Rejected: per-segment gst `scaletempo`, which reintroduces the per-segment gst-trim complexity the team deliberately avoided and isn't unit-testable.) (open)

## ISS-17: ED.17 — cursor capture assumes the main display (multi-display refinement)
- **Filed:** 2026-05-31
- **By:** ED.17 (cursor capture)
- **Severity:** deferral / tech-debt
- **Affects:** `crates/app/src/cursor_capture.rs` (`parse_display_id` / `display_bounds_for_source`)
- **Status:** ✅ resolved 2026-06-01 (window-source framing still a refinement)
- **Description:** The cursor poller normalized the global cursor against the **main** display, so recording a non-primary display put the overlay in the wrong place.
- **Resolution:** `cursor_capture::display_bounds_for_source(source_id)` parses the recording's screen-source id (`parse_display_id`: `"display-<CGDirectDisplayID>"` → the id, pure + unit-tested) and passes that display's `CGDisplayBounds` to the poller; `start_cursor_capture` now takes the `screen_source_id` (threaded from `RecordingConfig` in `start_recording`). Primary / window / malformed ids fall back to the main display. **Remaining refinement:** a *window* source should normalize against the captured window's frame, not its display — out of scope here (multi-display *display* capture is the common case + now correct). (resolved)

## ISS-16: ED.17 — click capture (auto-zoom + ripples) needs a CGEventTap
- **Filed:** 2026-05-31
- **By:** ED.17 (cursor capture)
- **Severity:** deferral
- **Affects:** `crates/app/src/click_capture.rs`, `crates/app/src/recording.rs`, `crates/app/src/commands.rs`, `crates/app/src/editor_command.rs`
- **Status:** ✅ resolved 2026-06-01
- **Description:** ED.17's **position** track ships via no-permission `CGEvent` polling, but the **click log** — which feeds the already-tested `auto_zoom_segments` and ED.19's click ripples — needs a `CGEventTap` (`objc2_core_graphics::CGEvent::tap_create`, listen-only). A tap requires the **Input-Monitoring** permission (prompts the user, cannot be granted in CI/headless) and a `CFRunLoop` callback on a worker thread.
- **Resolution:** New `crates/app/src/click_capture.rs`: a listen-only `CGEventTap` for `LeftMouseDown | RightMouseDown`, its `CFMachPort` source pumped on a dedicated worker thread's `CFRunLoop` (added `objc2-core-foundation` with `CFRunLoop`/`CFMachPort`). The C `extern "C-unwind"` callback normalizes each click against the captured display (reusing `cursor_capture::normalize_cursor_to_frame`) and accumulates `(elapsed, x, y)`; at stop, the pure `samples_to_clicks` maps them onto the project frame grid. Wired through `RecordingState` (`start_click_capture` / `finish_click_capture` / `take_clicks` / `clear_clicks`) on the **same consume-once Record→Edit handoff** as the cursor track, and attached to `project.clicks` in `open_in_editor`. **Graceful degrade**: a null tap (permission not granted) logs + leaves an empty log; the recording is never blocked. **Key FFI subtlety:** `CFRetained<CFRunLoop>` is `!Send`, so the run loop is stored as a `usize` *pointer* (`AtomicUsize`) — the `CFRetained` lives only on the worker thread; `stop()` calls the thread-safe `CFRunLoopStop` through the address. This keeps `ClickTap` (and thus the shared `RecordingState`) `Send + Sync`. The live tap is **runtime-only** (no CI); the pure `samples_to_clicks` + the `RecordingState` handoff + a `samples_to_clicks → auto_zoom_segments` contract test cover the wiring on every OS. **Verify on a real macOS session** with Input-Monitoring granted: clicks → auto-zoom regions + ED.19 ripples in the editor. (resolved)

## ISS-15: ED.18 — `Wallpaper` background source has no asset pipeline
- **Filed:** 2026-05-31
- **By:** ED.18 (background-framing render integration)
- **Severity:** deferral
- **Affects:** `crates/app/src/editor_preview.rs` (`wallpaper_rgba` / `set_background`), `crates/wisp/src/recording.rs` (`set_background_wallpaper`)
- **Status:** ✅ resolved 2026-06-01
- **Description:** `BackgroundSource::Wallpaper { name }` was authored but not rendered (fell back to the default gradient).
- **Resolution:** Render a **procedural** wallpaper — no bundled asset, license-clean, deterministic. `app::editor_preview::wallpaper_rgba(name, w, h)` generates a soft diagonal three-stop gradient + faint aurora band, keyed on `name`'s palette (aurora / sunset / ocean / forest), as RGBA8 (pure + unit-tested). `wisp::RecordingScene::set_background_wallpaper(app, w, h, rgba)` uploads it as a full-NDC `Sprite` — the backmost layer (a `Sprite` paints before the gradient/shadow `Graphics` per the renderer's batch order). The wallpaper and the gradient/color backdrop are **mutually exclusive** (a Sprite and a Graphics backdrop fight the batch order): the app shows one and hides the other (`set_background_visible` / `set_background_wallpaper_visible`). Isolation assert keeps recorder output bit-identical. Verified: wisp node test, app pure + GPU (blue-dominant "ocean" margin) tests. Real bundled/aspect-correct wallpapers are a follow-up — see ISS-19. (resolved)

## ISS-19: ED.18 — wallpaper is procedural + stretch-scaled (real-asset / aspect-correct follow-up)
- **Filed:** 2026-06-01
- **By:** ISS-15 closeout
- **Severity:** enhancement
- **Affects:** `crates/app/src/editor_preview.rs` (`wallpaper_rgba`), `crates/wisp/src/recording.rs` (`set_background_wallpaper`)
- **Status:** open
- **Description:** v1 wallpapers are *procedurally generated* (four palettes) and the Sprite *stretches* to fill NDC (scale 2), so a non-16:9 wallpaper would distort — acceptable because the procedural gradients are aspect-agnostic. Two enhancements: (a) bundle real license-clean wallpaper images + a `name → bytes` resolver (decode app-side via the `image` crate, pass RGBA to `set_background_wallpaper` unchanged); (b) aspect-correct cover-scaling (scale the Sprite per-axis from the image aspect vs the canvas aspect) so real photos don't distort.
- **Resolution:** (open)

## ISS-14: ED.18 — drop-shadow + inset border are authored but not rendered
- **Filed:** 2026-05-31
- **By:** ED.18 (background-framing render integration)
- **Severity:** deferral
- **Affects:** `crates/app/src/editor_preview.rs` (`set_background` / `apply_shadow_and_border`), `crates/wisp/src/recording.rs` (`set_frame_shadow` / `set_frame_border`)
- **Status:** ✅ resolved 2026-06-01
- **Description:** `BackgroundConfig.shadow` (0..=100) and `.inset` (px) were authored + persisted but not rendered. The blur path was deferred because `DropShadowFilter` runs `filter::blur::run_blur_pass` → lavapipe-incompatible (would force a `WISP_SKIP_GPU_FILTER_TESTS` guard + a skipped Ubuntu snapshot).
- **Resolution:** Rendered **without** blur, so it stays single-bind-group + fully CI-verifiable. The shadow is `RecordingScene::set_frame_shadow` — a `color` rounded-rect the shape of the frame window, offset down-right, as a Phase-1 unclipped `Graphics` node behind the screen (the offset sliver reads as a shadow). The inset border is `set_frame_border` — a rounded-rect stroke tracing the window, as a Phase-2 full-NDC-clipped node over the screen. App-side `EditorPreview::apply_shadow_and_border` derives them from `BackgroundConfig` (alpha/offset scale with `shadow`; stroke width = `2·inset/W` NDC). Isolation asserts (`shadow`/`border` `None` on a fresh scene) keep recorder output bit-identical. Verified: wisp node/dispatch test, app GPU diff-with/without test, storybook PNG (shadow + white border read clearly), no lavapipe guard needed. (resolved)

## ISS-13: review — assorted deferred polish/robustness items
- **Filed:** 2026-05-31
- **By:** user (aggressive editor-track review)
- **Severity:** deferral / tech-debt
- **Affects:** `app-ui` (editor surface, inspectors, library, export bar), `wisp` render-texture, `decode` editor stream
- **Status:** open
- **Description:** Verified review findings not yet actioned, each low-to-medium value and best done with the user's eyes / a profiler:
  (1) `use_context().unwrap_or_else(|| RwSignal::new(...))` silently mints orphan signals on a missing provider — prefer `expect_context` for always-provided editor signals (team call: fail-loud vs degrade).
  (2) Editor keyboard shortcuts require the surface `<section>` to hold focus — move to a window/document listener that early-returns on input/select/textarea targets.
  (3) Editor canvas + recordings-library lack loading / load-error states (failed/slow open looks like a hang / empty-forever).
  (4) Export cancel and a real encoder error surface the same string + are styled as failure — add a distinct `Cancelled` state.
  (5) `wisp` `read_pixels` allocates a fresh staging buffer + Vec every composed frame — cache the MAP_READ staging buffer (needs the wisp test+story+snapshot+chapter gate).
  (6) Backward scrub past the 300-frame cache re-spawns gst + re-decodes from 0 (latent — scrub-preview not yet wired to a command); the real fix is the planned `gstreamer-rs` ACCURATE seek.
  (7) `VideoFrame.bgra` could be `Arc<Vec<u8>>` to share one allocation between cache store + return (~20 call sites).
- **Resolution:** (open)

## ISS-12: review — editor/recorder shell ignores the theme system (undefined design tokens)
- **Filed:** 2026-05-31
- **By:** user (aggressive editor-track review)
- **Severity:** tech-debt (visual)
- **Affects:** `crates/app-ui/shell.css`
- **Status:** open
- **Description:** `shell.css` references 8 design tokens that don't exist in `:root` (`--surface-1` ×16, `--surface-2` ×24, `--accent-soft`, `--bg-elev-1/2/3`, `--border-subtle`, `--danger-strong`), each with a hardcoded hex fallback — so the whole editor + recorder shell renders off hardcoded dark hex and ignores the theme system in `ui-storybook/assets/style.css`. The fix is to map each onto a real theme token (e.g. `--surface-1` → `--surface-elevated`), but several target values differ from the current hex, so it **needs a human visual check** — deferred to do with the user rather than guess values blind.
- **Resolution:** (open)

## ISS-11: review — editor UX-perf re-architectures (re-render scope + tick storm)
- **Filed:** 2026-05-31
- **By:** user (aggressive editor-track review)
- **Severity:** deferral (perf) / tech-debt
- **Affects:** `crates/app-ui/{app_shell_mount,editor_surface,timeline_view,filmstrip,zoom_lane}.rs`
- **Status:** open
- **Description:** Four UX-perf wins that change runtime render/timing behavior and so need a running-app check (can't verify in a headless gate) — deliberately not shipped blind:
  (1) the whole `EditorShell` (5 inspectors + 6 lanes) re-mounts on every project edit — hoist it out of the project-subscribing closure and drive only the chrome via a `Memo` keyed on stable clip identity;
  (2) a 30 Hz Tauri tick IPC storm runs during playback even when idle — gate the `gloo` interval to editor-active-and-playing;
  (3) ruler ticks regenerate + rebuild all tick DOM ~30×/s — key a `Memo` on `(duration, fps)`;
  (4) filmstrip / zoom lanes rebuild the whole lane DOM on every edit — keyed `<For>` over a `Memo<Vec<...>>` (will shift SSR baselines).
- **Resolution:** (open)

## ISS-10: `resolve_history` compares source paths exactly (not canonically)
- **Filed:** 2026-05-30
- **By:** ED.20 (adversarial review of ED.11–19)
- **Severity:** closed-wontfix
- **Affects:** `crates/app-ui/src/editor_edits.rs` (`resolve_history`)
- **Description:**
  `resolve_history` decides whether to reuse the running undo stack by
  comparing `history.project().source.path == current.source.path` with plain
  `PathBuf` equality. Two differently-spelled paths to the same file
  (`/x/a.mp4` vs `/x/./a.mp4`) compare unequal, so reopening the clip via a
  different spelling would start a fresh undo stack. Surfaced by the ED.11–19
  adversarial review.
- **Resolution:** closed-wontfix (2026-05-30). `std::fs::canonicalize` is
  unavailable on `wasm32` (app-ui is wasm), and the Tauri backend passes a
  single stable path per clip, so the divergent-spelling case doesn't arise
  in practice. The fallback (a fresh, empty undo stack) is safe — no data
  loss, no crash. Documented inline in `resolve_history`. Revisit only if a
  real reopen path proves non-stable.

---

## ISS-09: lift-delete (leave-gap) needs a timeline gap item
- **Filed:** 2026-05-30
- **By:** ED.2 (AUT-337)
- **Severity:** deferral
- **Affects:** `crates/edit` (ops / segment), ED.11 timeline editing
- **Status:** open
- **Description:**
  ED.2 ships ripple-delete (close the gap) as the primary delete. The "lift" variant — delete a project range but leave a black gap in place — needs the timeline to represent gaps, which the current `Vec<TimelineSegment>` model can't (every segment references source media). Adding a `TimelineItem { Clip(TimelineSegment), Gap { project_len } }` (or a gap flag) would let `EditCompose` render black for gap spans and the export generator emit black frames. Ripple is the far more common screen-recorder delete, so lift-gap is deferred. ED.11 maps both `Delete` and `Shift+Delete` to ripple until then.
- **Resolution:** (open)

---

## ISS-08: increase camera + screen recording quality
- **Filed:** 2026-05-26
- **By:** user (next feature after `feat/export`)
- **Severity:** deferral → in progress
- **Affects:** the capture + encode path — `crates/media` (GStreamer encode pipeline + SCK config) + `crates/app` (recording orchestration); **not** the UI layer.
- **Description:**
  Goal: raise the quality of recorded camera + screen output. Three independent axes, each touching a different part of the pipeline:
  1. **Encoder settings** — bitrate, H.264/H.265 profile + level, keyframe interval, rate-control mode. Lives in the GStreamer encode element config (HW encoders: `vtenc_h264_hw` macOS). All currently at GStreamer defaults.
  2. **Resolution + framerate** — capture at native Retina (no downscale) and/or higher fps; capture was fixed at 1920×1080 (squishing non-16:9 Retina panels).
  3. **Color** — HDR / wide-gamut / 10-bit (biggest pipeline change; a later pass).
- **Resolution:** (partial — 2026-05-26) v1 target chosen = **Axis 2 (resolution)**. Shipped on `feat/recording-quality`:
  - **M-QUAL.1** — live (streaming) video encode: `LiveGstreamerEncoder` streams BGRA into `gst-launch-1.0`'s stdin so only compressed video lands on disk, removing the raw-scratch firehose (≈250 MB/s at 1080p, >1 GB/s at Retina) that made native res untenable. CLI-pipe, not `gstreamer-rs`.
  - **M-QUAL.2** — native-resolution screen capture: `resolve_native_screen_dims` reads the display's true backing pixels (`CGDisplayMode::pixel_width/height`) and threads them through the SCK caps + encoder + compose canvas (3024×1964 on the dev MBP, vs the old squished 1920×1080).
  - **M-QUAL.3** — webcam bubble 480×480 → 720×720 + de-squish (`aspectratiocrop aspect-ratio=1/1` before `videoscale`), so the circular bubble is undistorted + sharper instead of a 16:9-squished, upscaled 480².

  **Axis 2 (resolution) is complete** for both screen + camera. **Axis 1** (encoder bitrate / keyframe / profile / rate-control) and **Axis 3** (HDR / 10-bit / wide-gamut) remain **open** for a later pass. See PROGRESS.md M-QUAL.1/.2/.3 and milestone-2 "Phase 7 — M-QUAL".

---

## ISS-07: stale rustdoc deep-links in existing `ui/chunks/*.md` chapters
- **Filed:** 2026-05-25
- **By:** M-SAVE.GATE (verified rustdoc paths while authoring `save-panel.md`)
- **Severity:** bug (docs only)
- **Affects:** `_docs/book/src/ui/chunks/*.md` — at least `status-bar-*.md`, `button-sizes.md`, `card-basic.md`, `card-with-dope-sheet.md`, `drop-zone-idle.md`, `recording-toolbar-recording.md`, `dope-sheet-basic.md`, `player-controls-near-end.md`
- **Status:** open (not in the `just gate` CI path — markdown `[](…)` hrefs aren't intra-doc links, so `cargo doc` doesn't validate them; only the deploy-time smoke in `docs.yml` would, and it only spot-checks a few well-known files)
- **Description:**
  These chapters link into the published rustdoc with a path that omits the component subgroup, e.g. `[`StatusBar`](../../api/ui_storybook/components/status_bar/fn.StatusBar.html)`. The real generated path includes the subgroup: `components/shell/status_bar/fn.StatusBar.html` (confirmed via `cargo doc -p ui-storybook --no-deps` → `target/doc/ui_storybook/components/shell/status_bar/fn.StatusBar.html`). Same class of error for `button` (→ `components/primitives/button/`), `card` (→ `primitives/card/`), `dope_sheet` (→ `editor/dope_sheet/`), `recording_toolbar` (→ `recorder/recording_toolbar/`), `drop_zone` (→ `shell/drop_zone/`). Every one of these deep-links 404s on the deployed site. The new `save-panel.md` uses the correct `components/recorder/save_panel/…` path, so it's not affected.
- **Resolution:** (open) Mechanical fix — for each chapter, re-point the `api/` href to the subgroup-qualified path (grep `target/doc/ui_storybook` for the real location of each item). Worth a dedicated `docs: fix stale rustdoc deep-links in ui chapters` pass; consider a `doc-gates` check that resolves every `api/…` href in the book against `target/doc` so this can't regress.

---

## ISS-06: `cargo deny` / `cargo machete` fail on pre-existing repo state with current tool versions
- **Filed:** 2026-05-25
- **By:** M-SAVE.0 (ran deny/machete after adding `tauri-plugin-dialog`)
- **Severity:** tech-debt
- **Affects:** workspace-wide (`deny.toml`, `crates/app-ui/Cargo.toml`, `crates/app-e2e/Cargo.toml`, `tools/doc-gates/Cargo.toml`, `crates/playback/Cargo.toml`) — tooling only, not the build
- **Status:** open (not in the `just gate` CI path; surfaces only on manual `just deny` / `just unused-deps`)
- **Description:**
  Installing the latest `cargo-deny` (0.19.7) + `cargo-machete` and running `cargo deny check` / `cargo machete` against the workspace produces failures that are **all pre-existing** (reproduce on `main`, unrelated to the M-SAVE.0 dep addition — `tauri-plugin-dialog`/`tauri-plugin-fs`/`rfd` introduced no rejected license and no new ban):

  - **`bans FAILED` — wildcard path deps.** `deny.toml` sets `[bans] wildcards = "deny"`, and cargo-deny 0.19.x flags workspace-internal path deps that omit a `version` field (`ui-storybook = { path = "../ui-storybook", ... }` in `app-ui`; `screen-app = { path = "../app" }` in `app-e2e`). Older cargo-deny defaulted `allow-wildcard-paths` on for path deps. **Fix:** add `allow-wildcard-paths = true` under `[bans]` in `deny.toml`.
  - **`licenses FAILED` — `doc-gates` unlicensed.** `tools/doc-gates/Cargo.toml` has no `license` field, so cargo-deny rejects it. It's a workspace-internal tool (`publish = false`). **Fix:** add `license = "MIT"` to its `Cargo.toml` (or configure `[licenses] private = { ignore = true }` in deny.toml to skip non-published crates).
  - **`cargo machete` — `playback → tracing`.** Flagged as unused; likely a false positive (macro-only usage that machete's static pass misses — it suggests `--with-metadata`). Needs a one-line check: either remove the dep if truly unused, or add `[package.metadata.cargo-machete] ignored = ["tracing"]`.

  Why it doesn't block CI today: the gate workflow (`gate.yml`) runs `just gate`, and `just gate` is the 7-step fmt→check→lint→nextest→doctest→docs→snapshots-check — `cargo deny` is **not** in it (it's a separate `just deny` / `just security` recipe). So `main` is green despite these.
- **Resolution:** (open) Three small, independent fixes above. Deferred out of M-SAVE.0 to avoid scope creep (the chunk is the output-dir picker; none of these crates are touched by it). Worth a dedicated `chore: modernize cargo-deny config` pass.

---

## ISS-05: Camera toggle ↔ webcam-bubble visibility is one phase out of sync from page mount
- **Filed:** 2026-05-22
- **By:** user (caught during the recorder-redesign visual refactor — pre-existing bug, not introduced by the refactor)
- **Severity:** bug
- **Affects:** `screen-app` + `app-ui` (`crates/app-ui/src/recorder_page.rs::on_camera_toggle`, `crates/app/src/commands.rs::toggle_webcam_bubble`, `crates/app/src/tray/bubble_toggle.rs`)
- **Status:** ✅ resolved 2026-05-22 — setter-shaped IPC landed in same session as the filing
- **Description:**
  `RecorderPage`'s `camera_enabled` `RwSignal` defaults to `true` but `BubbleVisibility::default()` is `Hidden`. The two start out of sync on every page mount. `on_camera_toggle` unconditionally calls `bubble_ipc::toggle_webcam_bubble()` (always-flip), so every click thereafter does the opposite of what the toggle pill implies:
  - Mount: camera pill ON, bubble window HIDDEN
  - 1st click → camera pill OFF, bubble SHOWS (Hidden→Visible)
  - 2nd click → camera pill ON, bubble HIDES (Visible→Hidden)
  
  Visible only after the redesign because the new pill-toggle CSS makes the on/off state legible — before it rendered as an unstyled button + literal "true" / "false" text and the mismatch was invisible.

  **Proposed fix (≈30 LOC, scope expanded beyond "UI only" so deferred):**
  - Add `BubbleVisibility::set(&mut self, visible: bool) -> Option<BubbleAction>` — idempotent setter returning `Some(action)` only on state transition.
  - Add `#[tauri::command] set_webcam_bubble_visibility(visible: bool, ...)` calling the setter + `apply_bubble_action` (factor out of `toggle_webcam_bubble`).
  - Register in `main.rs` `generate_handler!` (both debug + release arms).
  - Add `__screenSetBubbleVisibility(visible)` JS bridge in `crates/app-ui/index.html` + matching wasm extern + Rust wrapper in `crates/app-ui/src/bubble_ipc.rs`.
  - In `on_camera_toggle`, replace `toggle_webcam_bubble()` with `set_webcam_bubble_visibility(next)`.
  - Mount-time sync call so the page also aligns the bubble on first paint + on rail-surface navigation back into the recorder.
  
  Alternative narrower fix (CSS / state-only, no IPC change): flip the default of `camera_enabled` to `false`. Drift goes away on initial mount; still drifts on rail-surface navigation if the user enabled the bubble before navigating away.
- **Resolution:** 2026-05-22 — implemented the proposed setter-shaped IPC.
  `BubbleVisibility::set(visible)` returns `Option<BubbleAction>` (None when already in the requested state). New `set_webcam_bubble_visibility` Tauri command + JS bridge + wasm wrapper. `on_camera_toggle` now calls `set_webcam_bubble_visibility(next)` instead of the always-flip `toggle_webcam_bubble()`. Mount-time sync added so the bubble aligns with `camera_enabled` on every page mount (including rail-surface navigation back into the recorder). Three new state-machine unit tests cover the transitions + the no-op case.

---

## ISS-04: `block` + `proc-macro-error2` future-incompat warnings (transitive)
- **Filed:** 2026-05-13
- **By:** user (post-CI investigation on `Gantt` branch)
- **Severity:** tech-debt
- **Affects:** workspace-wide (cosmetic — surfaced on macOS most prominently because `block` only compiles there)
- **Status:** open, **accepted** (not actionable from our code without forking)
- **Description:**
  `cargo build --workspace --all-features` emits a note:
  > warning: the following packages contain code that will be rejected by a future version of Rust: block v0.1.6, proc-macro-error2 v2.0.1
  Neither warning fails CI — they are future-incompat *informational* notes, not errors. Root causes both live upstream and we don't write either crate.

  **block v0.1.6** — `static _NSConcreteStackBlock: Class;` is an uninhabited static (lint rust-lang/rust#74840). The `block` crate is unmaintained (last release Sep 2024) and `block2` is the modern replacement. **`metal-rs` master still pins `block 0.1.6` directly** (verified upstream) — no migration issue or PR exists. So even upgrading wgpu 24 → latest does NOT clear this warning until metal-rs adopts `block2`. macOS-only because `metal` only compiles there.

  **proc-macro-error2 v2.0.1** — `pub use proc_macro;` re-exports the private `extern crate proc_macro` (lint rust-lang/rust#127909). Upstream issue [GnomedDev/proc-macro-error-2#13](https://github.com/GnomedDev/proc-macro-error-2/issues/13) is open with PR [#14](https://github.com/GnomedDev/proc-macro-error-2/pull/14) (2-char fix, unmerged as of 2026-05-13). Pulled in by every Leptos macro crate; we stay on Leptos, so the only way to excise it is to wait for upstream to publish a fixed `2.0.2` (or `2.1`) and for Leptos to bump.

  Discarded alternatives (each is a worse trade-off than accepting the note):
  - Fork `block` into `third_party/` — permanent maintenance burden for an Objective-C interop layer we don't author.
  - Fork Leptos to drop the `proc-macro-error2` dep — 100k-LOC permanent fork.
  - `[patch.crates-io]` to PR #14's unmerged fork commit — depends on a contributor branch that could be force-pushed or deleted; not a stable pin.
  - Coordinated `wgpu 24 → 29` ecosystem bump — doesn't even fix `block` (metal-rs main still uses it) and the migration is wildly out of scope.

  These warnings are informational and CI-green; the prior policy noted in CLAUDE.md ("we can't fix those upstream") remains correct.
- **Resolution:** (open)
  Re-check when (a) `proc-macro-error2 2.0.2+` ships with the PR #14 fix, or (b) `metal-rs` migrates to `block2`. At that point the lockfile bump should make the warning go away with no code change on our side.

---

## ISS-03: `app-ui` rustdoc has an unresolved intra-doc link to `playback::Player`
- **Filed:** 2026-05-09
- **By:** M-PREVIEW.1 (spotted during `just site`)
- **Severity:** tech-debt
- **Affects:** `app-ui` (`crates/app-ui/src/lib.rs:16` — `//! [`playback::Player`]`)
- **Status:** ✅ resolved 2026-05-09 by M-PLAY.2
- **Description:**
  The crate-level docstring references `[`playback::Player`]`, but `app-ui`
  doesn't depend on the `playback` crate so rustdoc can't resolve the path.
- **Resolution:**
  M-PLAY.2 rewrote the lib.rs docstring to describe the actual IPC wiring
  and replaced the `playback::Player` reference with a `[`player_ipc`]`
  link to the new in-crate module. Cross-crate references in
  `player_ipc.rs` to `screen_app::player_session` types are intentionally
  plain text (with a comment explaining why) — `app-ui` is a WASM crate
  that can't depend on `screen-app` (Tauri-native). Verified by `just gate`
  (no remaining rustdoc warnings).

---

## ISS-02: Tauri 2 Linux backend pulls gtk-rs unmaintained crates
- **Filed:** 2026-05-09
- **By:** M1.1 (Tauri foundation setup)
- **Severity:** tech-debt
- **Affects:** `screen-app` (transitive — Linux-only)
- **Status:** open (16 advisories exempted in `deny.toml`)
- **Description:**
  Tauri 2's Linux WebView backend depends on `gtk-rs` GTK3 bindings (atk, gdk, gtk, gio, etc.) that have been archived upstream. RustSec emits ~16 unmaintained advisories. None are exploits — all are "no longer actively maintained." macOS/Windows backends don't pull these.
- **Resolution:**
  Exempted in `deny.toml` `[advisories].ignore` with reason. Re-evaluate when Tauri migrates to GTK4 (tracked in tauri-apps issues).

---

## ISS-01: `paste` crate unmaintained (transitive via wgpu)
- **Filed:** 2026-05-09
- **By:** side quest (QA toolchain setup)
- **Severity:** tech-debt
- **Affects:** `wisp` (transitive dep tree) — `paste 1.0.15` reaches us through `metal → wgpu-hal → wgpu`.
- **Status:** open (exempted in `deny.toml`)
- **Description:**
  RustSec advisory `RUSTSEC-2024-0436` flags `paste` as unmaintained (not vulnerable). Author archived the repo. Suggested alternatives: `pastey` (drop-in fork) or `with_builtin_macros`. We can't fix this directly — wgpu's `metal` backend depends on it.
- **Resolution:**
  Exempted in `deny.toml` `[advisories].ignore` with documented reason. Re-evaluate when wgpu releases a version that drops the dep, or when `metal` migrates to `pastey`. Track at https://github.com/gfx-rs/wgpu/issues for migration progress.

---

