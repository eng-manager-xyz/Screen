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

