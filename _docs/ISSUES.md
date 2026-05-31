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

