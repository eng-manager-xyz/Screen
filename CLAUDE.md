# Project: `screen` — cinematic screen recorder

This file is auto-loaded into every Claude Code session. **Read it first** when picking up cold.

---

## ⚠️ NON-NEGOTIABLE: test → check → update → loop

**Every code drop ships with at least one test, runs the full QA suite, updates the durable docs, and recursively retries until green. No exceptions.**

After any non-trivial change — adding code, editing config, removing a dep, fixing a bug:

```
1. TEST:    add at least one test (see _docs/TESTING.md "anti-regression gravity")
            - unit / integration / snapshot / property / regression
            - chunks that don't fit any layer are scaffolding-only
2. STORY:   for any chunk that adds a *renderable* feature, add a story to
            `crates/wisp-storybook/src/stories/` (wgpu) or
            `crates/ui-storybook/src/stories.rs` (Leptos) with a write-up.
            Non-render chunks (math, capture, encode, file I/O) are exempt.
3. ASSET:   `just snapshots` → regenerates the chunk's PNG/HTML under
            `_docs/book/src/assets/<crate>/<id>.{png,html}`. Commit it.
4. CHAPTER: write a per-chunk mdBook chapter at
            `_docs/book/src/<crate>/chunks/<id>.md`:
              - `# <Title> — M<n>.<m>`
              - one-paragraph what + why
              - `![](../../assets/<crate>/<id>.png)` (or `<iframe>` for UI)
              - "Done when" recap from the milestone doc
              - link into the rustdoc: `[api](../../api/<crate_name>/…)`
            Add the chapter to `_docs/book/src/SUMMARY.md` under its milestone.
5. CHECK:   `just gate`        →  loop recursively until green
            `just site`         →  visually verify the chapter renders
6. UPDATE:  PROGRESS.md         →  what changed, what was verified
            ISSUES.md           →  if you found a bug or deferral
            milestone doc       →  ✅ a chunk's "Done when:" if satisfied
7. STATUS:  TaskUpdate          →  mark task completed only when gate is green
```

**`just gate` runs:** fmt → check → lint → nextest → doctest. All five must pass. See `_docs/QA.md` for higher tiers and `_docs/TESTING.md` for the testing strategy.

### The recursive-fix loop

When `just gate` fails, you **must** loop until it's green. There is no exit other than green:

```
loop:
    run `just gate`
    if green: break
    diagnose, fix
    if approach fails: try a different approach
    if multiple approaches fail: file ISS-NN with everything tried, then try a fresh angle
```

What you must **never** do:
- `#[allow(clippy::*)]` to bypass clippy without a documented `reason = "..."`.
- `#[ignore]` a failing test without an ISS-NN reference + fix plan.
- Comment out an assertion to make it pass.
- Bypass `cargo deny` / `cargo machete` findings without an exemption + filed issue.
- Mark a task done with a red gate.

If you skip the test, you've broken the AI-codebase contract (the test suite is the executable memory).
If you skip the check, you've broken the gate contract.
If you skip the update, you've broken the cross-session-memory contract.
All three are equally fatal.

This convention is enforced by `_docs/WORKFLOW.md` § 4 and `_docs/TESTING.md`. Any time the loop is violated — including by me in a previous turn — call it out and fix it before proceeding.

---

## ⚠️ Notes from rehearsal — anti-patterns we've earned

Every rule below cost a recursive-fix iteration somewhere in the source. **Apply them prophylactically.** Repeating a documented mistake is a worse miss than a fresh one — the lesson is here to be applied.

**The discipline:** every time the loop closes on a NEW mistake — diagnose, fix, then add a one-line lesson here. The cost is one line in this file; the cost of recreation is a recursive-fix iteration.

### Cast hygiene (Rust idioms / clippy)

- **`f32::from(x)` for `u8`/`u16` → `f32`**, never `x as f32` (clippy::cast_precision_loss).
- **`u32::try_from(x).expect(...)` for `usize` → `u32`**, never `x as u32` (clippy::cast_possible_truncation).
- **`f32 as u32` requires `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "...")]`** even after `clamp` + `round` — the clamp bound isn't visible to clippy.
- **`iter.next_back()`**, never `iter.rev().next()` (clippy::manual_next_back).
- **Chained `if let Some(a) && let Some(b)`** (Rust 2024), never nested `if let` (clippy::collapsible_if).
- **No `let mut x` if `x` isn't mutated** (unused_mut).
- **No `1 * N`** (clippy::identity_op).
- **Use associated `Self::method` if `&self` isn't used** (clippy::unused_self).
- **`#[derive(Default)]` if the manual impl matches** (clippy::derivable_impls).
- **Iterator returns: declare `impl DoubleEndedIterator + ExactSizeIterator`** when callers need `.rev()` / `.len()` — bare `impl Iterator` drops those.

### Foreign-type wrappers

- **Public structs containing wgpu / winit types need manual `impl Debug`** — those crates don't all derive Debug.

### Search/replace discipline

- **NEVER `replace_all` on a substring** that appears in unrelated identifiers. M0.11 disaster: `_texture` → `texture` clobbered `render_texture`, `video_texture`, `create_texture`, `write_texture`, `wgpu_texture`. **Audit matches first** or use `Edit` with surrounding context.

### Coordinate / pixel-readback

- **sRGB byte-exact tests use `Rgba8Unorm`** (linear). `Rgba8UnormSrgb` is for display — clear color `0.251` becomes `137` on read-back, not `64`.
- **Interior pixel sampling:** for a primitive in NDC `[-0.5, +0.5]` rendered to an N-row image, valid interior rows are `~N/4..3N/4`. Boundary rows hit the SDF anti-alias band; rows just outside the rect read as the clear color (M0.14 bug — picked row 24 in a 32-row image, which is at NDC y=-0.53, outside).

### Slotmap & ownership

- **`NodeId`s aren't unique across distinct `SlotMap`s** — both start at slot 0/gen 1 and collide. For staleness tests: `destroy(id)` then re-use the same map.

### Dependencies

- **YAGNI for deps** — `cargo machete` is the gate. M0.2 pre-added `slotmap` / `image` / `fontdue` ahead of need; machete caught it. Don't add until the first `use` site.
- **Embedding-host wgpu version match first.** When adding an embedding host (`eframe`, `iced`, etc.), bump it to whichever major aligns with our wgpu *before* any integration. egui 0.29 = wgpu 22; egui 0.31 = wgpu 24 — we burned a build cycle on this.
- **GUI deps bring font licenses.** Expect new entries in `deny.toml` (`OFL-1.1`, `Ubuntu-font-1.0`) when adding eframe / similar. These are routine, not red flags.

### wgpu API specifics

- **wgpu names shift between majors** — `ImageCopyTexture` → `TexelCopyTextureInfo`, `ImageDataLayout` → `TexelCopyBufferLayout` (renamed in 24). `request_adapter` returns `Option`, not `Result`. `request_device` takes `(descriptor, trace_path)`. Iterate via cargo errors when bumping.
- **Empty wgpu buffers panic when sliced.** `create_buffer_init` with `contents: &[]` produces a 0-byte buffer, then `buffer.slice(..)` aborts at `slice offset 0 is out of range for buffer of size 0`. Always `if batch.is_empty() { continue; }` before the buffer + draw path. (M0.15 caught this.)

### Tauri 2 specifics

- **`tauri::generate_context!()` requires `icons/icon.png` at compile time** even when `bundle.active = false`. The macro embeds the icon into the binary. Minimum: a real PNG file at `crates/app/icons/icon.png`. (M1.1 caught this — a one-line Python script generates a 32×32 transparent PNG when needed.)
- **`tauri` feature `protocol-asset`** is required to use `convertFileSrc` in JS. Without it, build fails with "Tauri dependency features … does not match the allowlist."
- **`tauri::generate_context!` is a procedural macro** that depends on `tauri` at expansion time. `cargo machete` doesn't see this — add `[package.metadata.cargo-machete] ignored = ["tauri"]` to suppress the false positive.
- **Tauri 2's Linux backend pulls archived gtk-rs crates.** Expect ~16 RustSec advisories on Linux (RUSTSEC-2024-0411..0420 family + 2025-0075..0100). All unmaintained-only, none exploits. Add to `deny.toml` `[advisories].ignore` once. (M1.1, ISS-02.)

### Leptos `#[component]` specifics

- **`#[component]` rewrites function shape.** It generates a builder-pattern struct + wrapper fn; clippy lints (`must_use_candidate`, `needless_pass_by_value`) fire on the *generated* code regardless of where you put `#[allow]` on the source fn. **Use module-level `#![allow(...)]`** in `components/mod.rs` rather than per-fn pragmas.
- **`leptos::prelude::*` re-exports `tachys::prelude::*`,** which brings `RenderHtml::to_html()` into scope. SSR test pattern: `view.into_view().to_html()` — synchronous, returns `String`, perfect for `insta`.
- **`<Show when=…>` requires the `when` closure to be `'static`.** If the `when` reads from a captured `String`, capture a `bool` instead and clone the `String` inside the body.
- **Plain CSS over Tailwind in this workspace.** Keeps the toolchain Rust-only (no npm / standalone binary fetch). Class names mirror rust-ui's hooks so a future swap is search-and-replace, not a rewrite.

### Story testing pattern (insta + wgpu error scopes)

- **`insta` first-run UX:** initial run stores `*.snap.new` and FAILS the test (no baseline to compare). Accept by `mv *.snap.new *.snap` (or `cargo insta accept`). `INSTA_UPDATE=auto` does NOT auto-accept first-time snapshots — it only auto-accepts mismatches once a baseline exists.
- **wgpu validation as a "no console errors" gate:** `device.push_error_scope(ErrorFilter::Validation)` before story rendering, `pollster::block_on(device.pop_error_scope())` after — assert empty. Catches every wgpu validation issue silently and surfaces them as test failures rather than runtime console noise.
- **Quadrant fingerprint snapshot pattern:** for visual regression, render at small resolution (256×256), divide into a 4×4 quadrant grid, average each quadrant's RGBA, bucket to multiples of 8 (~3% tolerance), `insta::assert_yaml_snapshot!` the resulting `Vec<[u32; 4]>`. Robust to driver variation, fails on real visual changes, snapshot is human-readable in the diff.
- **Animated stories need `tick(stage, 0.0)` before rendering** so the test sees the deterministic initial frame, not the empty `build()`-only state. (Stories like `s_graphics_ellipse` populate the graphics inside `tick`, not `build`.)

### CI / GitHub Actions / Linux runner

- **`just fmt-fix` (or `cargo fmt --all`) before every commit, no exceptions.**
  CI's first step is `cargo fmt --all --check`. A stray multi-line array
  literal that rustfmt would collapse to one line burns 2-3 minutes of
  runner time just to fail on fmt before any real work runs. Local fmt
  costs <1s — no excuse.

- **`macos-latest` is the truth runner for wgpu tests.** GitHub-hosted
  Linux runners only have lavapipe (mesa's software Vulkan), which loses
  the device on multi-bind-group filter pipelines. macOS runners have
  real Apple Silicon Metal — same backend as the dev box — and run all
  117 tests without skips. macos-latest minutes are free on public
  repos; on private repos they're 10× the multiplier so use a matrix
  judiciously. **Default the gate to a matrix `[macos-latest,
  ubuntu-latest]` with `fail-fast: false`**: macOS validates "real
  hardware passes everything"; Linux validates the build path
  (gtk-rs/winit/apt deps) with the lavapipe-affected tests skipped via
  `WISP_SKIP_GPU_FILTER_TESTS=1`.
- **Don't rely on Linux GPU tests in CI without real hardware.**
  Filter pipelines that work on Metal/hardware-Vulkan/D3D will fail on
  lavapipe with `Validation Error / Parent device is lost`. Refactoring
  the pipelines to fit lavapipe is the wrong call — it compromises
  real-GPU design for a software emulator's limits. Either run on
  macos-latest, gate the test on an env-var skip, or bring real
  hardware via a self-hosted runner.

- **winit 0.30 fails to compile on Linux with
  `compile_error!("The platform you're compiling for is not supported by
  winit")` if `x11` and `wayland` features aren't active.** Both are
  defaults, so this normally Just Works — but a transitive dep somewhere
  in our tree pulls winit with `default-features = false`, and cargo's
  feature unification then leaves Linux without any backend. **Fix:**
  pin `winit = { version = "0.30", features = ["x11", "wayland",
  "wayland-dlopen", "wayland-csd-adwaita"] }` explicitly in our
  Cargo.toml so the unified feature set always carries a Linux backend.
  Also apt-install the matching headers for CI (`libx11-dev`,
  `libxkbcommon-dev`, `libxkbcommon-x11-dev`, `libxcb1-dev`,
  `libxcursor-dev`, `libxrandr-dev`, `libxi-dev`).
  **AND** chase down every dep edge that re-pulls winit. eframe with
  `default-features = false` strips its own `x11`/`wayland` features
  (which proxy to `winit/x11`/`winit/wayland`); add them back
  explicitly: `eframe = { default-features = false, features = ["wgpu",
  "default_fonts", "x11", "wayland"] }`. The `cargo check --all-features`
  workspace gate masks this because feature unification activates them
  via SOME other edge; `cargo doc` (no `--all-features`) is stricter
  and surfaces the gap.
- **Never set `RUSTFLAGS: -D warnings` at the workflow `env:` level.**
  It promotes transitive-crate future-incompat warnings (`block v0.1.6`,
  `proc-macro-error2 v2.0.1`, …) into hard failures. We can't fix those
  upstream warnings; they pour in any time `cargo doc --workspace`
  touches the dep tree. For docs-strict semantics, scope `RUSTDOCFLAGS`
  to a single command (`RUSTDOCFLAGS="-D warnings -D rustdoc::broken-intra-doc-links" cargo doc …`),
  not a workflow-wide env var.
- **wgpu on Linux CI needs `mesa-vulkan-drivers` + `libvulkan1`** so
  lavapipe (software Vulkan) is available as the wgpu adapter. Without
  this the first wisp test that calls `Application::new` either hangs on
  adapter probe or aborts with "no adapters found". Pair with
  `WGPU_BACKEND=vulkan` + `WGPU_POWER_PREF=low` in the workflow env so
  wgpu doesn't spend cycles probing every backend.
- **Lavapipe loses the device on multi-bind-group filter pipelines.**
  Symptom: `wgpu error: Validation Error / In Device::create_render_pipeline,
  label = 'wisp::blur pipeline' / Parent device is lost`. Real adapters
  (Metal, hardware Vulkan) build the same pipelines fine. Pattern: add
  a `skip_on_software_adapter()` helper guard at the top of affected
  tests, gated on `WISP_SKIP_GPU_FILTER_TESTS=1` set only in the CI
  workflow's `$GITHUB_ENV`. Local dev and real-hardware CI leave the
  env var unset and run the tests normally — gate stays green without
  losing the assertion on real GPUs.
- **Tauri 2 on Ubuntu requires the gtk-rs build toolchain at `cargo doc` /
  `cargo check` time, not just at link time.** `glib-sys`'s build script
  invokes `pkg-config --libs --cflags glib-2.0` and aborts if the dev
  headers aren't present. **Install before any cargo invocation in CI:**
  `pkg-config libglib2.0-dev libgtk-3-dev libwebkit2gtk-4.1-dev
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  build-essential` (the official Tauri 2 prerequisite list). Affects
  *every* CI workflow that compiles the workspace, not just the gate —
  the docs workflow's `cargo doc --workspace` hits the same wall.
- **GStreamer in CI: install `gstreamer1.0-tools gstreamer1.0-plugins-base
  gstreamer1.0-plugins-good gstreamer1.0-libav`.** Without
  `gstreamer1.0-libav` the H.264 fixture in `decode/tests/fixtures/sample.mp4`
  doesn't decode (libav is what carries the H.264 plugin on stock Ubuntu).
- **Cache `target/` plus `~/.cargo/registry/{index,cache}` and
  `~/.cargo/git/db`** keyed on `Cargo.lock`. Caching just `~/.cargo` and
  not `target/` halves the speedup; caching the workspace `target/`
  yields the biggest win.
- **HTTPS push to a fresh GitHub repo can hit transient HTTP 400
  ("send-pack: unexpected disconnect").** Fix: `git config --local
  http.postBuffer 524288000` (500 MB). The default 1 MB buffer is
  enough for small commits but stalls on initial repo seeding with
  binary assets (PNGs, MP4 fixtures).

### Trunk + Leptos CSR

- **`data-cargo-features="…"` only if the feature actually exists.** Trunk
  forwards `--features X` to `cargo build`; if the crate doesn't declare
  feature `X`, cargo fails with `does not contain this feature`. For an
  app-ui crate that just depends on leptos's csr feature, drop the
  attribute entirely. (M-INT.1 burnt one cycle on this.)
- **`crate-type = ["cdylib", "rlib"]`** — `cdylib` for Trunk's wasm-bindgen
  step, `rlib` so `cargo check --workspace` (native) still type-checks the
  crate. Drop `rlib` and the workspace gate goes red on native targets.
- **`<link data-trunk rel="copy-dir">` is the way to pull peer-crate
  assets** (e.g. `../ui-storybook/assets`) into the Trunk dist. Don't
  symlink and don't `<link rel="stylesheet" href="../...">` — neither
  survives the dev server.
- **`#[wasm_bindgen(start)]` on a top-level fn is the Trunk entry point**.
  No need for an explicit `<script>main()</script>` in `index.html`;
  wasm-bindgen invokes it automatically.

### GStreamer / external CLI integration

- **`brew install gstreamer` is the prerequisite, not `gstreamer-tools` or
  `gst-launch`.** The cask name is just `gstreamer` and pulls in the CLI
  binaries (`gst-launch-1.0`, `gst-discoverer-1.0`) plus enough plugins to
  decode H.264/AAC out of the box. Don't waste a cycle searching for the
  right cask name — it's `gstreamer`.
- **CLI-pipe over `gstreamer-rs` for first integration.** Spawning the
  GStreamer CLI as a subprocess (`gst-launch-1.0 -q filesrc ! decodebin
  ! videoconvert ! video/x-raw,format=BGRA ! fdsink fd=1`) avoids any
  compile-time integration with libgstreamer. Works on any machine with
  `brew install gstreamer`. Upgrading to `gstreamer-rs` Rust bindings is a
  later chunk; the `VideoStream` trait makes it a one-line swap at the
  call site.
- **`gst-discoverer-1.0` for metadata, `gst-launch-1.0` for the stream.**
  Discover before launch; the launch pipeline can't carry caps in a way
  the consumer can read out, so `width × height × 4` for `read_exact`
  must come from a separate probe.
- **`fdsink fd=1` is the stdout fdsink.** Don't try `filesink location=-`
  or shell redirection inside `gst-launch-1.0`'s arg parser — they don't
  work. `fdsink fd=1` is the canonical way to emit the raw stream on
  stdout.
- **Drop-kill the child.** Implementing `Drop` for the stream struct with
  `child.kill()` + `child.wait()` matters: `gst-launch-1.0` will keep
  decoding into a dropped pipe and burn CPU otherwise.

### mdBook / engineering site

- **`mdbook build --dest-dir` is resolved relative to the source dir, not the
  cwd.** `mdbook build _docs/book --dest-dir ../../target/book` looks correct
  but lands the output at `<one-up-from-project>/target/book`. **Pass an
  absolute path:** `--dest-dir "$(pwd)/target/book"`. (Found this turn.)
- **mdBook 0.5 dropped `multilingual` and `copy-fonts` from `book.toml`.**
  Older book.toml files crash with a deserialization error rather than a
  helpful warning. Strip those keys; mdBook surfaces the full set of valid
  keys in the error message. (Found this turn.)
- **Two binaries with the same name across crates collide in `cargo doc`.**
  `target/doc/<bin-name>/` is one namespace, so duplicate names abort with
  "document output filename collision". Prefix per-crate (e.g.
  `wisp-export-stories`, `ui-export-stories`).
- **mdBook chapters cannot reference the rustdoc directly via intra-doc
  links.** Link to `api/<crate_name>/index.html` (note: underscores, not
  hyphens, in the crate path — `wisp_storybook`, not `wisp-storybook`).
- **`additional-css` paths in book.toml resolve relative to the source dir.**
  If you reference a stylesheet that doesn't exist mdBook silently emits a
  broken `<link>` rather than failing the build.

### Build hygiene

- **New error variants need a caller** (CONVENTIONS § Error handling). `cargo` warns; clippy errors at `-D warnings`.
- **`#[allow(clippy::*)]` requires `reason = "..."`** — no exceptions.

### When you hit a NEW mistake

1. Fix the issue (recursive-fix loop).
2. **Add a one-line lesson here** under the right category.
3. Commit the lesson alongside the fix (or as `docs:` if separated).
4. Future runs apply the lesson prophylactically — that's the whole point.

---

## What this project is

A native screen recorder in the Screen Studio / OpenScreen lineage, built as an all-Rust stack. Two parallel deliverables:

- **`wisp`** — a Pixi-equivalent 2D scene graph + filter chain library on `wgpu`. Pixi-shaped public API, scoped to power the recorder.
- **`screen-app`** — the Tauri 2 + Leptos recorder application that consumes `wisp`.

Library is means; the app is the goal.

## Stack (locked 2026-05-09)

- **Shell:** Tauri 2 (multi-window)
- **UI:** Leptos 0.7 (Rust → WASM) inside the Tauri webview
- **Renderer:** `wisp` (in-repo, `crates/wisp`) — wgpu + WGSL
- **Editor preview:** native `winit` sibling window rendered by `wisp`
- **Capture:** `objc2`/ScreenCaptureKit (macOS), `windows-rs` (Windows), `pipewire-rs` (Linux)
- **Encode:** `ffmpeg-next` for MVP; VideoToolbox / Media Foundation HW paths in v2

## Current milestone

**M0** — building `wisp` (Pixi-equivalent on wgpu). 21 chunks, see `_docs/milestone-0-renderer.md`.

After M0: **M1** — Tauri+Leptos drop-zone + video player, see `_docs/milestone-1-drop-zone-player.md`.

---

## Per-task workflow (full version in `_docs/WORKFLOW.md`)

For every task in the task list:

1. **Pick.** `TaskList` → find next unblocked task.
2. **Read.** Open the corresponding chunk in the milestone doc. Note the "Done when:" criteria — that's the contract.
3. **Mark in_progress.** `TaskUpdate` status. Only one task in_progress at a time.
4. **Implement.** Smallest unit that satisfies "Done when". Stay inside the chunk's scope; file adjacent work in `ISSUES.md`.
5. **Test.** Unit / snapshot / integration as appropriate (see `CONVENTIONS.md` § Testing).
6. **CHECK.** `just gate` — fix until green. If a chunk references an example, also `cargo run -p <crate> --example <name>` and verify by output.
7. **UPDATE.** Append to `PROGRESS.md` (template at bottom). File any new issues in `ISSUES.md`. Tick "Done when:" in the milestone doc if satisfied.
8. **Mark completed.** `TaskUpdate` → completed. Confirm next task is unblocked.
9. **Commit.** Autonomous at natural boundaries — typically one chunk = one commit. Conventional-commit format (`feat(wisp): …`, `fix(app): …`, `test(wisp): …`, `docs: …`, `chore: …`). Local repo is the time-machine; commit freely so individual chunks can be rolled back without losing the rest.

---

## Hard rules (the full list)

- **Every meaningful chunk ships with at least one test** (unit / integration / snapshot / property / regression). See `_docs/TESTING.md` "anti-regression gravity".
- **Every renderable feature ships with a storybook story.** New visible behavior must show up in `just storybook` (wgpu side) or `just ui-storybook` (HTML/Leptos side) with a write-up in the right sidebar. Non-render features (math, capture, encode, file I/O) are exempt.
- **Every UI component (Leptos / HTML) ships with a story.** Both isolated views and compositions must appear in `crates/ui-storybook/src/stories.rs` and pass the SSR snapshot gate (`tests/snapshots.rs`). The convention parallels wisp's "every renderable chunk ships with a story" — same shape, different rendering layer.
- **Every public item has a `///` doc + every crate has a `//!` header.** `missing_docs` is a workspace `warn` lint; `just docs-strict` (in `just gate`) treats broken intra-doc links and rustdoc warnings as errors. New public items without docs trip the gate.
- **Every visible chunk regenerates its asset under `_docs/book/src/assets/<crate>/<id>.{png,html}`.** `just snapshots` runs the headless exporters for both storybooks; the resulting files are committed and embedded into the mdBook chapters via standard markdown. Without an asset, the chunk's docs page renders empty — that's the gate.
- **Every chunk gets its own mdBook chapter at `_docs/book/src/<crate>/chunks/<id>.md`** and is linked into `SUMMARY.md` under its milestone heading. The chapter MUST embed the chunk's screenshot/HTML (no asset = empty page = gate fails). Milestone close requires `just site` to render every chapter green and `just docs-strict` to pass without broken intra-doc links.
- **`just gate` must be green before any task is marked done.** No exceptions.
- **Recursive-fix loop:** if `just gate` is red, loop until green. Never disable tests, never `#[allow]` clippy without reason, never bypass deny/machete findings.
- **Append to `PROGRESS.md` for every completed task.** It's the only durable record across context windows.
- **File issues in `ISSUES.md`** for anything you can't fix inside the current chunk.
- Don't expand task scope. Adjacent work → `ISSUES.md`.
- Don't refactor unrelated code in the same task.
- Don't add features ahead of need (YAGNI). `cargo machete` enforces this for deps.
- Don't `#[allow(clippy::*)]` without a `reason = "..."`.
- Don't `#[ignore]` a test without an ISS-NN reference + fix plan.
- Don't bypass `cargo deny` / `cargo audit` / `cargo machete` findings without a documented exemption + an issue filed.
- **Commit autonomously at natural boundaries** (per chunk, per side quest, per logical milestone). Local-only repo; commits are the time-machine. Use conventional-commit format. The "wait for explicit user request" rule is repealed for this project.
- One task `in_progress` at a time.

---

## Where docs live

All planning docs are in `_docs/`. Read `_docs/README.md` for the full index.

Critical docs to load when starting cold:
1. `_docs/PROGRESS.md` — what's been completed (newest at top)
2. `_docs/WORKFLOW.md` — the per-task workflow you must follow
3. `_docs/TESTING.md` — testing strategy + per-chunk test minimums + recursive-fix loop
4. `_docs/QA.md` — tooling tiers and `just gate` definition
5. `_docs/CONVENTIONS.md` — code standards
6. The current milestone doc (e.g., `_docs/milestone-0-renderer.md`)
7. `_docs/ISSUES.md` — known bugs / deferrals / open questions

## Resume protocol (cold session)

User says "continue" or "where are we":

1. Read `_docs/PROGRESS.md` (top 3-5 entries) — what was last done.
2. `TaskList` — find any `in_progress` task; otherwise the next available `pending`.
3. Read the corresponding milestone doc chunk.
4. Read `_docs/ISSUES.md` for any open issues affecting the area.
5. **Confirm tooling:** `just --version` to confirm `just` is on PATH. If not, the user needs `brew install just` (and possibly `cargo install --locked cargo-nextest cargo-deny cargo-machete`).
6. State a one-sentence summary: e.g., *"Last done: M0.5 hello triangle. Next: M0.6 textured quad pipeline (#25). One open issue: ISS-01 (paste unmaintained, exempted)."*

## Project root structure

```
screen/
├─ CLAUDE.md                # this file (auto-loaded)
├─ Justfile                 # all QA recipes — run `just` to list
├─ rustfmt.toml             # formatter config
├─ deny.toml                # cargo-deny: license/advisory/ban/source policy
├─ Cargo.toml               # [workspace], shared lints
├─ rust-toolchain.toml      # nightly
├─ crates/
│  ├─ app/                  # screen-app — Tauri+Leptos shell (M1+)
│  ├─ wisp/                 # the renderer library (M0+)
│  ├─ wisp-storybook/       # wgpu story gallery (eframe)
│  └─ ui-storybook/         # Leptos UI gallery (SSR snapshots; Trunk for browser)
└─ _docs/                   # all planning, research, ops docs
   └─ book/                 # mdBook prose site (rendered to target/book/)
      └─ src/assets/        # per-crate generated screenshots / story HTML
```
