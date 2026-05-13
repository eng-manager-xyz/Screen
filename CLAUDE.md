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

**`just gate` runs:** fmt → check → lint → nextest → doctest → docs → snapshots-check. All seven must pass. See `_docs/QA.md` for higher tiers and `_docs/TESTING.md` for the testing strategy.

### Asset choice for mdBook chapters

The chapter's hero asset is part of the delivery; pick the medium that *shows the feature*. The chunk isn't done if the asset doesn't communicate.

- **`<video controls autoplay muted loop playsinline>` (MP4) when motion is the feature.** Any story with `tick: Some(...)` — perspective rotation, motion blur, animated reveals, audio histograms scrolling, video playback — must ship an MP4 captured by `wisp-export-animated` (or the equivalent media exporter) and embed via the `<video>` tag. A static PNG of a frozen frame doesn't communicate motion.
- **PNG (`![](path)`) for static demos** — masks, filters applied to static content, layout, transforms with a clear single-frame story. Cheaper to generate, cheaper to review.
- **White / light backdrop** for *anything that uses alpha or subtle luminance differences* — drop shadows, glow, dim-outside, vignettes, motion-blur trails, near-transparent overlays. Black-on-black is the most-recurring "the feature is there, you just can't see it" mistake. The storybook exporter clears to BLACK by default; if your story needs a light backdrop, render a colored RT and attach it as a Sprite (Sprite-on-Sprite ordering follows scene-tree order; a Graphics backdrop paints AFTER Sprites and overlays them — that's the M-MASK / M-FILTER bug class).
- **Real content over synthetic gradients** for blend modes, color filters, and anything where structure matters. Bundle a license-clean image (e.g. the bundled Apollo 17 "Blue Marble" at `crates/wisp-storybook/assets/images/`) rather than two gradients overlapping. Real chroma + luminance + edges + dark regions makes per-mode behavior legible at a glance.
- **`just snapshots-wisp-animated`** runs the gstreamer-backed `wisp-export-animated` binary. It is intentionally *not* chained into `just snapshots-wisp` (it depends on the `gst-launch-1.0` CLI on PATH). Run it before commit when an animated story's `tick` changes. The committed `.mp4` is the source of truth for CI / mdBook — CI does not regenerate videos.

### Callout blocks via mdbook-admonish

mdBook chapters lift the **non-obvious, must-not-miss** facts into
[mdbook-admonish](https://github.com/tommilligan/mdbook-admonish)
callouts so they read at a glance instead of disappearing inside a
paragraph. Use sparingly — a chapter with five admonish blocks loses
the signal.

Pick the *meaning* first, then the type:

- ```admonish important``` — boundary rules / load-bearing decisions
  ("`wisp` must not depend on `media`", "screen space, not local
  space"). The reader breaking this rule costs the most.
- ```admonish warning``` — gotchas, lurking footguns ("Graphics
  paints after Sprites in `render_stage`", "lavapipe loses the
  device on multi-bind-group filter pipelines").
- ```admonish bug``` — known issues / conventions that exist because
  of a workaround ("`+y` flip — sprite samples up, glyphon writes
  down"). Pairs well with a link to the issue.
- ```admonish note``` — useful side info that isn't dangerous but is
  easy to miss ("`FlexibleTextRenderer` is opt-in").
- ```admonish tip``` — best-practice nudges ("prefer `Ellipse` or
  `RoundedRect` over `Circle`").
- ```admonish info``` — orientation / "reading the list" framing for
  a follow-on diagram or table.

Skip ```admonish example``` / ```admonish success``` — normal code
fences and chapter prose carry that load already.

### Live command output via mdbook-cmdrun

`<!-- cmdrun … -->` inlines a command's stdout into the rendered
page at build time via
[mdbook-cmdrun](https://github.com/FauconFan/mdbook-cmdrun). Use it
for content that would otherwise rot — directory listings, version
strings of vendored tools, count-of-tests for a crate. The command
runs from the chapter file's directory, so paths are usually
`../../../<thing>`.

```admonish warning title="cmdrun must be deterministic + CI-safe"
The command runs on the CI runner during `mdbook build`. No
network, no git state, no timestamps — those bake non-determinism
into the page and rot the moment the doc is rebuilt elsewhere.
Listing the workspace via `ls -1 ../../../crates` is fine; running
`git log` to embed a commit list is not.
```

### Chapter shape — what to cut

The book is for *readers*, not for tracking shipped work. Don't
include sections that duplicate Linear / PROGRESS.md / git history:

- **No `## Done when` checklists.** Acceptance criteria are tracked
  in Linear; once the chapter exists, every box would be checked.
  0 reader value, 5–10 lines of noise.
- **No `## What's next` / `## Up next` forward-references.** They go
  stale fast. The SUMMARY.md TOC is how readers navigate between
  chapters.
- **No `## Tests` enumeration tables.** "Tests cover X / Y / Z" is
  progress-tracking. If there's a non-obvious test invariant worth
  surfacing (a regression-guard intent), inline it in the prose for
  the feature it guards — not as a flat list.
- **Drop dev-jargon labels from titles** — "Tier-C", "M-BLEND.2",
  "dispatched nodes". Use feature names; if jargon is unavoidable,
  introduce it once at the top of the chapter.
- **Drop trailing `[api](...)` link soup.** One link near the top
  next to the title is enough; the rustdoc index lives at `/api/`.

When in doubt: would a reader who's never seen this codebase
benefit from this paragraph? If no, cut it.

### Diagrams in mdBook — mermaid only, no ASCII

**Every diagram in `_docs/book/src/**/*.md` is a `mermaid` code block.** ASCII / box-drawing / unicode-arrow diagrams are not accepted — the gate (`just gate` → `mermaid-check`) rejects new ones. Prefer types in this order:

1. **`sequenceDiagram`** when the diagram shows actors / processes / threads exchanging messages over time (lifecycle pumps, IPC flows, "shell calls X, X calls Y, Y returns Z"). **Default to sequenceDiagram for anything that has a time component or named participants.**
2. **`flowchart LR` / `flowchart TD`** for static pipelines / data flows / dispatch trees with no time component (filter chains, render-pass routing).
3. **`stateDiagram-v2`** for state machines (Playing / Paused / Buffering).
4. **`graph TD` / `classDiagram`** for hierarchies, crate layouts, or struct relationships.

Allowed exceptions (do NOT convert):
- **Math / formulas** — type-signature legends, conversion math, sample-rate arithmetic.
- **Shell pipelines** — actual `gst-launch-1.0 ! foo ! bar` syntax meant to be copy-pasted.
- **Directory trees** — `crates/wisp/src/...` listings (mermaid is poor at file trees; the indented text is more readable).

The `mermaid-check` gate fails on any non-allowlisted chapter containing box-drawing chars (`┌ │ └ ├ ═ ╔ ╗`) or the unicode arrow run `─►` / `──▶` / `◄──`. New violations show the offending file + line.

### README authoring — GitHub-readable, no preprocessing

Every crate has a `README.md` at the GitHub level — separate from
the mdBook chapters, separate from rustdoc. The README is what
shows up on `github.com/.../tree/main/crates/<name>/` and (for
publishable crates) on crates.io.

```admonish important title="READMEs use pure GitHub Flavored Markdown"
**No `mdbook-preprocessor-cross` tags in READMEs.** `{{shared}}` /
`{{wisp-link}}` are mdBook-only — GitHub doesn't run a
preprocessor. READMEs use:

- ` ```mermaid ` blocks (GitHub renders natively).
- GFM callouts: `> [!NOTE]`, `> [!TIP]`, `> [!IMPORTANT]`,
  `> [!WARNING]`, `> [!CAUTION]`.
- Relative image links to committed assets
  (`../../_docs/wisp-book/src/assets/wisp/foo.png` from a crate
  README).
- Absolute URLs into the published mdBook / rustdoc for deep-dive
  links (`https://eng-manager-xyz.github.io/Screen/...`).
```

**Canonical sections per README** (skip any that don't apply):

1. **Title + tagline** — `# `crate-name` — <one-line tagline>` then
   a 2-3 line blockquote summary.
2. **What it does** — paragraph. Frame the problem the crate
   solves, not the implementation.
3. **Where it fits** — a `mermaid flowchart LR` (architecture) or
   `sequenceDiagram` (lifecycle). One per README. Use the existing
   class palette (`fill:#7c2d12,stroke:#ea580c,color:#fed7aa` for
   wisp; `#14532d` for media; `#312e81` for UI; `#1e293b` for
   shell; `#374151` for other crates) so the crate's role is
   visually consistent across READMEs.
4. **Quickstart** — minimal rust / bash that gets a reader to
   "something visible" in <10 lines.
5. **Hero output** (where applicable) — `![alt](relative/path.png)`
   pointing at an existing committed asset. Don't add new images
   for the README; reuse the storybook / chapter PNGs.
6. **Public API at a glance** — markdown table of the top 5-10
   items. Link to the deployed rustdoc, not relative paths
   (rustdoc isn't checked into the repo).
7. **Runbook** — `Build + test` / `Run` / `Common tasks` /
   `Troubleshooting`. Operational, not aspirational.
8. **Deep dive** — links to the book chapter(s), examples dir,
   sibling-crate READMEs, CLAUDE.md sections.
9. **License** — one line. MIT.

**GFM callouts vs mdBook admonish.** Different platforms, different
syntax, same affordance:

- **`> [!NOTE]` / `[!TIP]` / `[!IMPORTANT]` / `[!WARNING]` /
  `[!CAUTION]`** — GitHub renders as styled callouts. Use in
  READMEs.
- **` ```admonish note ` etc.** — mdbook-admonish renders as styled
  callouts. Use in book chapters.
- If you inline a README into a book chapter via
  `mdbook-cmdrun`, the GFM blockquote renders as a plain
  blockquote in mdBook — acceptable fidelity loss for the DRY win.

**Mermaid in READMEs** — same `mermaid-check` rules apply
conceptually (no ASCII art) but no automated gate enforces it on
READMEs today. Hand-check.

**The five most-cited callouts in our READMEs:**

| Callout | When to use |
|---|---|
| `> [!IMPORTANT]` | Architectural boundary that, if violated, breaks publishing or correctness (wisp's no-upward-dep rule; Leptos's presentational contract). |
| `> [!WARNING]` | Operational footgun that costs a cycle (gtk-rs at compile time, `Icon?` gitignore, Tauri `beforeDevCommand` from parent). |
| `> [!CAUTION]` | Don't-do-this rules (don't add ffmpeg-next; both `icon.png` AND `icon.ico` must be tracked). |
| `> [!NOTE]` | Useful side info — env-var skips, OS-specific test skips, encoding gotchas. |
| `> [!TIP]` | Best-practice nudges — dev-loop choices, opt-in linker config. |

```admonish tip title="DRY README ↔ book content via cmdrun"
When a crate's README and its book chapter would share the bulk of
their content (overview + quickstart), use `mdbook-cmdrun` to
inline the README into the chapter:

\`\`\`markdown
<!-- cmdrun cat ../../../crates/<name>/README.md -->
\`\`\`

`cmdrun` runs at build time from the chapter's directory. The
chapter then wraps the README with book-specific deep-dive content.
This keeps the README authoritative (GitHub-level reading) and the
book chapter rich (preprocessed cross-links, deeper architecture).
```

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
- **`f32: From<i32>` does not exist.** Loop counters like `for i in -4..=4 { f32::from(i) }` won't compile. Either type-suffix the literal (`-4i16..=4`) so it resolves to `From<i16>`, or compute the loop value as `i16` from the start.
- **`f32` equality assertions need a tolerance.** clippy `float_cmp` rejects `assert_eq!(x, 64.0)`. Use `(a - b).abs() < 1e-6` for "exact" comparisons. Even the *clamp endpoints* of a `clamp(0.0, 64.0)` need tolerance because the lint doesn't know the value came from a clamp.
- **`(W as f32) * 0.35) as usize` is a triple-clippy fail** (`cast_precision_loss`, `cast_possible_truncation`, `cast_sign_loss`) for an integer-percent index calculation. Stay in integer arithmetic: `(W as usize * 35) / 100`. Same answer, no float involved.
- **Use `#[derive(Default)] + #[default]` on enums** instead of `impl Default for E { fn default() -> Self { Self::X } }` for unit-variant enums. clippy `derivable_impls` flags the manual impl.
- **`u32::try_from(x).expect(...)` for `usize` → `u32`**, never `x as u32` (clippy::cast_possible_truncation).
- **`f32 as u32` requires `#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "...")]`** even after `clamp` + `round` — the clamp bound isn't visible to clippy.
- **`iter.next_back()`**, never `iter.rev().next()` (clippy::manual_next_back).
- **Chained `if let Some(a) && let Some(b)`** (Rust 2024), never nested `if let` (clippy::collapsible_if).
- **No `let mut x` if `x` isn't mutated** (unused_mut).
- **No `1 * N`** (clippy::identity_op).
- **Use associated `Self::method` if `&self` isn't used** (clippy::unused_self).
- **`#[derive(Default)]` if the manual impl matches** (clippy::derivable_impls).
- **Iterator returns: declare `impl DoubleEndedIterator + ExactSizeIterator`** when callers need `.rev()` / `.len()` — bare `impl Iterator` drops those.

### Renderer batching / draw order

- **Pipelines batch by type, not by scene-graph insertion order.**
  `render_stage` collects all sprites, all graphics, all text, etc.,
  and submits them in pipeline-bucket order. Consequence: a
  full-canvas `Graphics` "backdrop" added BEFORE sprites in the scene
  tree is still drawn AFTER the sprites and will paint over them.
  **Pattern:** rely on the renderer's clear color for backdrops in
  stories and tests; reserve `Graphics` for foreground decoration. If
  you genuinely need a graphics-pipeline backdrop, it has to use a
  blend mode that doesn't replace the destination, or you have to
  accept the layering. This bit M-DYN.1's mask-texture story —
  storybook smoke passed because the backdrop alone exceeded the
  divergence threshold; the rendered PNG was clear-color + nothing.

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

### WGSL ↔ Rust uniform layout

- **`vec3<u32>` and `vec3<f32>` are 16-byte aligned in WGSL,** so a
  WGSL struct `{ x: u32, pad: vec3<u32> }` is 32 bytes, not 16. The
  matching `#[repr(C)]` Rust struct needs the equivalent trailing
  padding (`[u32; 7]` after the leading `u32`) or wgpu will reject
  the bind group with `Buffer is bound with size N where the shader
  expects M`. Caught M-VEC.11 / AUT-63 — the validation error is
  silent in `cargo check`, only appears when the pipeline runs. **Fix:**
  add `#[repr(C, align(16))]` and pad the Rust struct to match
  WGSL's alignment math, or use `vec4<u32>` / `vec2<u32>` if you
  don't actually need vec3 — the smaller alignment options
  avoid the surprise.

### wgpu API specifics

- **wgpu names shift between majors** — `ImageCopyTexture` → `TexelCopyTextureInfo`, `ImageDataLayout` → `TexelCopyBufferLayout` (renamed in 24). `request_adapter` returns `Option`, not `Result`. `request_device` takes `(descriptor, trace_path)`. Iterate via cargo errors when bumping.
- **Empty wgpu buffers panic when sliced.** `create_buffer_init` with `contents: &[]` produces a 0-byte buffer, then `buffer.slice(..)` aborts at `slice offset 0 is out of range for buffer of size 0`. Always `if batch.is_empty() { continue; }` before the buffer + draw path. (M0.15 caught this.)

### Tauri 2 specifics

- **`tauri::generate_context!()` requires `icons/icon.png` at compile time** even when `bundle.active = false`. The macro embeds the icon into the binary. Minimum: a real PNG file at `crates/app/icons/icon.png`. M1.1 caught this on macOS/Linux.
- **Windows builds *also* need `crates/app/icons/icon.ico`** for `tauri-winres` (the Windows resource compiler). Without it, the screen-app build script aborts with `package.metadata.tauri-winres does not exist; icons/icon.ico not found; required for generating a Windows Resource file during tauri-build`. **Regen via `cargo run -p screen-app --example regen-icons`** — pure-std Rust example that wraps the existing `icon.png` bytes in a minimal valid ICO container (PNG-in-ICO). Output is committed; CI never regenerates.
- **`tauri` feature `protocol-asset`** is required to use `convertFileSrc` in JS. Without it, build fails with "Tauri dependency features … does not match the allowlist."
- **`tauri::generate_context!` is a procedural macro** that depends on `tauri` at expansion time. `cargo machete` doesn't see this — add `[package.metadata.cargo-machete] ignored = ["tauri"]` to suppress the false positive.
- **Tauri 2's Linux backend pulls archived gtk-rs crates.** Expect ~16 RustSec advisories on Linux (RUSTSEC-2024-0411..0420 family + 2025-0075..0100). All unmaintained-only, none exploits. Add to `deny.toml` `[advisories].ignore` once. (M1.1, ISS-02.)

### Leptos discipline

```admonish important title="MANDATORY: invoke the `leptos-migration` skill BEFORE writing Leptos"
**Whenever you are about to write, edit, or review code that touches
`leptos::`, `#[component]`, `view!{ ... }`, signals, effects,
resources, actions, server fns, or anything Leptos — invoke the
`leptos-migration` skill first via the `Skill` tool.** It is the
durable source of truth for: the pinned version, the API name
changes from every prior major (0.1 → 0.7), the "strive to use"
0.8 idioms, and the project-specific landmines below. Doing this
before the first edit takes seconds and prevents the recurring class
of "this code looks correct for Leptos 0.6 but won't compile"
errors. Skill path: `.claude/skills/leptos-migration.md`.
```

- **Pinned version: `leptos = "0.8"` everywhere.** Both
  `crates/ui-storybook/Cargo.toml` and `crates/app-ui/Cargo.toml`
  pin to `"0.8"`. New crates that depend on Leptos must also pin to
  `"0.8"`. **Never** add `leptos = "0.7"` (or earlier) to a new
  Cargo.toml; never copy a `create_signal(cx, ...)` example from the
  internet without translating to `signal(...)` /
  `RwSignal::new(...)` first. The skill has the full name-changes
  table.
- **`#[component]` rewrites function shape.** It generates a
  builder-pattern struct + wrapper fn; clippy lints
  (`must_use_candidate`, `needless_pass_by_value`) fire on the
  *generated* code regardless of where you put `#[allow]` on the
  source fn. **Use module-level `#![allow(...)]`** in
  `components/mod.rs` rather than per-fn pragmas.
- **`leptos::prelude::*` re-exports `tachys::prelude::*`,** which
  brings `RenderHtml::to_html()` into scope. SSR test pattern:
  `view.into_view().to_html()` — synchronous, returns `String`,
  perfect for `insta`.
- **`<Show when=…>` requires the `when` closure to be `'static`.**
  If the `when` reads from a captured `String`, capture a `bool`
  instead and clone the `String` inside the body.
- **`Option<Children>` props take the bare value, NOT `Some(...)`.**
  The `#[prop(optional)]` macro wraps internally. Passing
  `Some(ToChildren::to_children(...))` produces `Option<Option<_>>`
  and you get "expected `Box<dyn FnOnce()…>`, found `Option<_>`".
  Pass `ToChildren::to_children(...)` directly or omit the prop.
- **Plain CSS over Tailwind in this workspace.** Keeps the toolchain
  Rust-only (no npm / standalone binary fetch). Class names mirror
  rust-ui's hooks so a future swap is search-and-replace, not a
  rewrite.

```admonish note title="When upgrading Leptos in the future"
1. Update the skill **first** — read the new version's release notes,
   add the new "strive to use" idioms + a quick-reference row to the
   name-changes table.
2. Bump the version in both `Cargo.toml`s.
3. Run `cargo update -p leptos@<old> --precise <new>` to push the
   lockfile.
4. `just gate`. The presentational contract usually catches
   breakages at the type-check step.
5. Update the "Pinned version" bullet above with the new number.
```

### Story testing pattern (insta + wgpu error scopes)

- **`insta` first-run UX:** initial run stores `*.snap.new` and FAILS the test (no baseline to compare). Accept by `mv *.snap.new *.snap` (or `cargo insta accept`). `INSTA_UPDATE=auto` does NOT auto-accept first-time snapshots — it only auto-accepts mismatches once a baseline exists.
- **wgpu validation as a "no console errors" gate:** `device.push_error_scope(ErrorFilter::Validation)` before story rendering, `pollster::block_on(device.pop_error_scope())` after — assert empty. Catches every wgpu validation issue silently and surfaces them as test failures rather than runtime console noise.
- **Quadrant fingerprint snapshot pattern:** for visual regression, render at small resolution (256×256), divide into a 4×4 quadrant grid, average each quadrant's RGBA, bucket to multiples of 8 (~3% tolerance), `insta::assert_yaml_snapshot!` the resulting `Vec<[u32; 4]>`. Robust to driver variation, fails on real visual changes, snapshot is human-readable in the diff.
- **Animated stories need `tick(stage, 0.0)` before rendering** so the test sees the deterministic initial frame, not the empty `build()`-only state. (Stories like `s_graphics_ellipse` populate the graphics inside `tick`, not `build`.)

### CI / GitHub Actions — universal rules

The rules in this subsection apply on **every** OS. Per-OS specifics
live in the three subsections below it.

- **`just fmt-fix` (or `cargo fmt --all`) before every commit, no
  exceptions.** CI's first step is `cargo fmt --all --check`. A
  stray multi-line array literal that rustfmt would collapse to
  one line burns 2-3 minutes of runner time just to fail on fmt
  before any real work runs. Local fmt costs <1s — no excuse.
- **`actionlint` is the local workflow validator.**
  `brew install actionlint`, then
  `actionlint .github/workflows/*.yml` before every CI edit.
  Catches YAML errors, shellcheck issues in `run:` blocks, and
  unset env refs. The gate doesn't run it remotely (yet) — it's a
  local-only smoke test, but it has caught every workflow mistake
  in this repo on first try.
- **Default every `run:` step to bash with `defaults.run.shell: bash`
  at the workflow level.** On `windows-latest`, GitHub Actions
  defaults to PowerShell, which doesn't understand bash-style `\`
  line continuations and crashes with
  `"Missing expression after unary operator '--'"` mid-`cargo
  clippy`. Setting `shell: bash` once at workflow level routes
  every step through Git Bash (preinstalled on windows-latest) and
  removes the entire class of continuation gotchas.
- **`just gate` must stay mdbook-free.** Site rendering belongs
  in `docs.yml` via `just site-check`, not in `just gate`. CI
  gate-screen doesn't install mdbook on any runner; if `gate`
  ever needs it, every matrix runner fails immediately. The
  `gate-screen` workflow also has an explicit `command -v mdbook`
  anti-regression step that fails fast with a clear `::error::`
  pointing at this section. (Burned cycle: DOCS-02 wired
  `shared-check` to `site` → mdbook; gate failed on every PR.)
- **`just gate` must stay python-free.** Use Rust binaries under
  `tools/` for any non-trivial text munging (see `tools/doc-gates`
  for the pattern). Windows ships `python` not `python3`, Git Bash
  PATH ordering varies, and adding an interpreter dep to every CI
  runner defeats the "one toolchain" story.
- **Never set `RUSTFLAGS: -D warnings` at the workflow `env:`
  level.** It promotes transitive-crate future-incompat warnings
  (`block v0.1.6`, `proc-macro-error2 v2.0.1`, …) into hard
  failures. We can't fix those upstream; they pour in any time
  `cargo doc --workspace` touches the dep tree. For docs-strict
  semantics, scope `RUSTDOCFLAGS` to a single command
  (`RUSTDOCFLAGS="-D warnings -D rustdoc::broken-intra-doc-links" cargo doc …`),
  not a workflow-wide env var.
- **Cache `target/` plus `~/.cargo/registry/{index,cache}` and
  `~/.cargo/git/db`**, keyed on `Cargo.lock`. Caching just
  `~/.cargo` and not `target/` halves the speedup; caching the
  workspace `target/` is the biggest single win. Use a per-OS
  cache key (`${{ runner.os }}-cargo-…`) so the three matrix
  runners don't fight over the same cache.
- **`dorny/paths-filter@v3` + synthetic aggregator** is the
  pattern for path-filtered jobs that must satisfy branch
  protection. Conditional jobs that skip return `skipped` —
  branch protection reads that as success, but if you mark
  `gate-wisp` as "required" directly, a screen-only PR is
  blocked because the required check never ran. Fix: add a
  `gate-all` job that `needs: [changes, gate-wisp, gate-screen]`
  + `if: always()`, inspects `needs.*.result`, and exits non-zero
  only if a triggered job actually failed. Make `gate-all` the
  required check.
- **Both gates trigger on shared workspace files** (`Cargo.lock`,
  `Cargo.toml`, `rust-toolchain.toml`, the workflow file itself).
  Workspace-wide changes affect everyone, so we run both gates.
  Per-crate paths split into `wisp` vs `screen` filters.
- **Three-OS matrix is the standard:**
  `[macos-latest, ubuntu-latest, windows-latest]` with
  `fail-fast: false`. See the per-OS subsections below for what
  to install and what env vars to set on each.
- **macOS is the truth runner for everything visual.** Real Apple
  Silicon Metal renders every wgpu test without skips, AND it's
  the OS where the canonical visual snapshots
  (`story_fingerprints_match_snapshot`) are captured. Ubuntu
  (lavapipe software Vulkan) and Windows (DX12) produce slightly
  different pixels — fine for build-path validation but they can't
  match macOS's bucketed-fingerprint snapshots, so visual-pixel
  tests skip on non-macOS. Per-OS snapshot files are explicitly
  rejected: they drift independently and double maintenance for
  zero new signal. **Cross-OS gates validate the build path +
  non-visual correctness; visual correctness is macOS's job.**
- **Lavapipe env vars (`WGPU_BACKEND`, `WGPU_POWER_PREF`,
  `WISP_SKIP_GPU_FILTER_TESTS`) are Ubuntu-only.** They exist to
  pin wgpu to lavapipe and to skip the 3 multi-bind-group filter
  tests that mesa's software Vulkan loses the device on. Don't
  set them on macOS or Windows — those have real GPUs and the
  skip env silently drops coverage of pipelines that run fine
  natively.
- **CI skip-pattern catalog.** Three flavours of skip, in
  increasing strength. Pick the weakest one that fits your
  failure mode.

  1. **Runtime probe** (`gstreamer_available()`,
     `skip_on_software_adapter()`). Test code probes the
     environment at runtime and early-returns with `eprintln!`
     if the prereq isn't there. **Use when** the binary loads
     fine but a runtime dependency might be absent (GStreamer
     CLI on Windows, lavapipe adapter on a non-Vulkan box).
     Cheapest to maintain — same test binary across OSes,
     skipping is data-driven.

  2. **Env-var gate**
     (`if std::env::var_os("WISP_SKIP_GPU_FILTER_TESTS").is_some()`).
     Test code checks an env var the CI workflow sets per-OS.
     **Use when** the prereq IS available but produces incorrect
     output on a specific runner — lavapipe builds the pipeline
     but corrupts the device, so we skip the test on Ubuntu CI
     while keeping it active on macOS / local-dev Linux with
     real GPU. The env var is the explicit opt-out signal.

  3. **Compile-time cfg** (`#![cfg(not(target_os = "windows"))]`
     at the top of a test file, or `cfg!(target_os = "windows")`
     inside a `#[test]` body). Test doesn't compile / doesn't
     run on the excluded OS. **Use when** the test binary can't
     even load — Tauri 2's `mock_builder` on Windows aborts
     with `STATUS_ENTRYPOINT_NOT_FOUND` because WebView2 SDK /
     loader DLL versions mismatch, so nextest can't even list
     tests. Cfg-skip is the only option when the binary itself
     won't initialise.

  **Don't mix flavours unnecessarily.** A test that runtime-probes
  AND env-var-gates AND cfg-skips for the same condition is
  unmaintainable; pick the one closest to the actual constraint.
- **Two-book Pages deploy composes into one artifact.**
  `actions/upload-pages-artifact@v3` accepts a single directory.
  Mount the wisp book at `target/book/wisp/` via
  `mdbook build _docs/wisp-book --dest-dir target/book/wisp` AFTER
  the screen book builds at `target/book/`. Result: one Pages
  site with path-based routing (`/Screen/`, `/Screen/wisp/`,
  `/Screen/api/`).
- **GitHub Pages URLs use the repo name's exact case.** This was
  the wisp-docs-404 bug: repo is named `Screen` (capital S), so
  Pages serves at `https://eng-manager-xyz.github.io/Screen/`.
  Every reference using lowercase `/screen/` 404s — there's no
  case-insensitive fallback on `*.github.io`. The trap:
  `book.toml`'s `site-url` field, `[preprocessor.cross]`'s
  `wisp-base`, README deep-dive links, and the cross-link-
  convention shared fragment all need the case to match exactly.
  **Source of truth**: the deploy workflow's
  `Evaluated environment url:` log line uses `github.repository`
  verbatim and is case-exact. Verify with
  `gh run view <deploy-job-id> --log | grep 'Evaluated environment url'`.
  **Anti-regression**: `doc-gates pages-url-check` (in `just gate`)
  scans every `.md` and `.toml` for the forbidden lowercase forms
  (see `FORBIDDEN_PAGES_URL_PREFIXES` in
  `tools/doc-gates/src/main.rs`) and fails fast with the exact
  line. If the repo is ever renamed, update both that list (add
  the old form to forbid future regression) and every committed
  reference to the published URL.
- **Post-build smoke test before upload.** docs.yml asserts
  well-known files (`wisp/overview.html`, `wisp/chunks/filter-blur.html`,
  `wisp-overview.html`) exist before `upload-pages-artifact`.
  Cheap insurance against "preprocessor silently dropped half
  the book" — catches the failure on PR rather than after deploy.
- **HTTPS push to a fresh GitHub repo can hit transient HTTP 400
  ("send-pack: unexpected disconnect").** Fix: `git config --local
  http.postBuffer 524288000` (500 MB). Default 1 MB buffer is
  enough for small commits but stalls on initial repo seeding
  with binary assets (PNGs, MP4 fixtures).
- **Keep rustdoc intra-doc links clean even though CI doesn't fail
  on them.** `cargo doc --workspace` emits warnings for broken
  intra-doc links, redundant explicit link targets, and public→
  private link leaks. We can't promote them to errors (`RUSTFLAGS:
  -D warnings` breaks on transitive future-incompat warnings — see
  separate rule), so the gate just lets them through. Fix them in
  the PR that introduces them — they accumulate fast (13 warnings
  built up before the DOCS-11 cleanup) and bury the rustdoc output
  in noise that masks real issues. Common forms:
  - **Type not in scope:** `[`Vector`]` in a sibling module →
    `[`Vector`](crate::scene::Vector)` with an absolute path.
  - **Stale reference** to a planned-but-unimplemented type
    (`manifest::RecordingManifest`) → drop the brackets, keep as
    inline `code`.
  - **GStreamer / system-API names** that look like Rust paths
    (`audiotestsrc`, `videotestsrc`) → inline code, never
    brackets.
  - **`some_fn`** referenced from a doc comment that's not in scope
    → spell out the path (`TypeName::method`) or drop the brackets.
  - **Private→public leak** (a public item documents a link to a
    private constant) → either make the constant `pub(crate)` and
    add `[gpmt]: crate::path::Type::method` reference-style links,
    or drop the brackets and keep as prose.
- **`.gitignore` globs can silently eat real directories.** The
  upstream macOS template's `Icon?` pattern (meant for Finder's
  `Icon\r` metadata file) matches our real `crates/app/icons/`
  directory on case-insensitive filesystems (macOS, Windows):
  glob `?` matches any single char, so `Icon?` matches `icons`
  (`Icon` + `s`). `git add` silently drops anything inside, and
  CI fails opaquely on the missing artefact. **Anti-regression:**
  `doc-gates required-files-check` (in `just gate`) runs
  `git ls-files --error-unmatch` over a hard-coded list of
  build-critical files (`crates/app/icons/icon.{png,ico}` today)
  and fails with a clear pointer at `git check-ignore -v <file>`.
  Add new entries to `REQUIRED_FILES` in
  `tools/doc-gates/src/main.rs` whenever a new build-critical
  asset gets committed.
- **Debug a "file exists but isn't tracked" mystery with
  `git check-ignore -v <path>`.** It prints the exact
  `.gitignore` line that matches. Files that survived an earlier
  commit before the bad pattern was added stay tracked
  (grandfathered) — that's why `icon.png` was fine and the new
  `icon.ico` wasn't, and why this class of failure looks like a
  random one-off rather than a pattern-overreach.

### CI — macOS (`macos-latest`)

The truth runner — see the universal rules above. This subsection
captures macOS-specific install + skip facts only.

- **Brew installs GStreamer for the screen gate:**
  `brew install gstreamer`. No further env vars (see universal
  rules — lavapipe env vars are Ubuntu-only).
- **e2e (Tier-2) tests are intentionally skipped.** `tauri-driver`
  + WKWebView support is incomplete upstream; the suite prints a
  clear skip message and exits 0 on macOS.

### CI — Ubuntu (`ubuntu-latest`)

Validates the Linux build path. This subsection captures the
Ubuntu-specific install + env + dep-pin facts — see the universal
rules above for the macOS-as-visual-truth and lavapipe-env-vars-
are-Ubuntu-only principles, and the "Lavapipe filter-test skip
pattern" section below for the guard discipline.

- **apt install — Tauri 2 toolchain:** `pkg-config libglib2.0-dev
  libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libssl-dev
  libayatana-appindicator3-dev librsvg2-dev build-essential`.
  `glib-sys`'s build script invokes `pkg-config --libs --cflags
  glib-2.0` and aborts if these dev headers aren't present — at
  `cargo doc` and `cargo check` time, not just at link time.
  Install before any cargo invocation. Affects *every* CI workflow
  that compiles the workspace, including docs.yml's
  `cargo doc --workspace`.
- **apt install — GStreamer:** `gstreamer1.0-tools
  gstreamer1.0-plugins-base gstreamer1.0-plugins-good
  gstreamer1.0-libav`. Without `gstreamer1.0-libav` the H.264
  fixture in `decode/tests/fixtures/sample.mp4` doesn't decode
  (libav carries the H.264 plugin on stock Ubuntu).
- **apt install — wgpu adapter + winit features:**
  `mesa-vulkan-drivers libvulkan1` (for lavapipe — without these
  the first wisp test that calls `Application::new` either hangs
  on adapter probe or aborts with "no adapters found") plus
  `libx11-dev libxkbcommon-dev libxkbcommon-x11-dev libxcb1-dev
  libxcursor-dev libxrandr-dev libxi-dev` (winit 0.30 x11/wayland
  backend headers).
- **Lavapipe env vars:** `WGPU_BACKEND=vulkan`,
  `WGPU_POWER_PREF=low`, and `WISP_SKIP_GPU_FILTER_TESTS=1`
  (skip-pattern catalog #2 — see universal rules).
- **winit 0.30 feature pin.** A transitive dep with
  `default-features = false` strips winit's `x11`/`wayland`
  features and cargo's feature unification can leave Linux
  without any backend (`compile_error!("The platform you're
  compiling for is not supported by winit")`). Pin
  `winit = { version = "0.30", features = ["x11", "wayland",
  "wayland-dlopen", "wayland-csd-adwaita"] }` explicitly in our
  Cargo.toml. **Also chase every edge that re-pulls winit:**
  eframe with `default-features = false` strips its proxy
  features (`x11` → `winit/x11`); add them back explicitly:
  `eframe = { default-features = false, features = ["wgpu",
  "default_fonts", "x11", "wayland"] }`. `cargo check
  --all-features` masks this via feature unification; `cargo doc`
  (no `--all-features`) is stricter and surfaces the gap.
- **e2e (Tier-2) tests are intentionally skipped in CI.**
  `tauri-driver` + WebKitGTK under xvfb proved flaky enough that
  the signal stopped being useful. Contributors run it locally
  before opening Tauri shell PRs (`_docs/book/src/app-ui/testing.md`).

### CI — Windows (`windows-latest`)

Validates the Windows build path (MSVC, WebView2, native DX12).
This subsection captures Windows-specific install + skip facts
only — see the universal rules above for the
`defaults.run.shell: bash` workflow setting and the
macOS-is-visual-truth-runner principle.

- **WebView2 + MSVC + Git Bash + Python are preinstalled.** No
  extra install steps for cargo to compile screen-app + run wisp
  tests. Bash recipes work via `#!/usr/bin/env bash` shebangs.
  Add a `bash --version` smoke step before `just gate` so a
  future runner image change that drops Git Bash fails with a
  clear error.
- **GStreamer intentionally NOT installed in CI.** Choco-install
  takes ~5 min per run for marginal coverage; the
  `gstreamer_available()` runtime guard (see "skip-pattern
  catalog" #1) skips the affected tests cleanly. Full Windows
  GStreamer coverage would use
  `choco install gstreamer gstreamer-devel --no-progress` +
  prepend `C:\Program Files\gstreamer\1.0\msvc_x86_64\bin` to
  `$GITHUB_PATH`.
- **Tauri 2's `mock_builder` aborts test discovery with
  `0xc0000139` (`STATUS_ENTRYPOINT_NOT_FOUND`).** The Tauri 2
  test path links transitively against `WebView2Loader.dll`; the
  preinstalled Edge WebView2 loader on `windows-latest` is
  missing an export that our pinned `tauri-runtime-wry` version
  needs. The binary fails to load AT LIST TIME — nextest can't
  even enumerate tests in `commands-*.exe`, much less run them.
  **Skip via cfg** (`#![cfg(not(target_os = "windows"))]` at the
  top of `crates/app/tests/commands.rs`) — runtime / env-var
  skips don't help when the binary won't load. Full Windows Tauri
  test coverage would require pinning a specific WebView2 SDK
  version; defer until we ship Windows binaries.
- **Tauri's `tauri-winres` requires `crates/app/icons/icon.ico`.**
  See "Tauri 2 specifics" — without it the screen-app build
  script aborts. Regen via
  `cargo run -p screen-app --example regen-icons`; commit the
  output. `doc-gates required-files-check` (in `just gate`)
  asserts both `icon.png` and `icon.ico` are tracked in git, so
  a future `.gitignore` overreach can't silently drop the file
  again.
- **Path separators:** Windows uses `\` natively but bash on
  Windows accepts `/` in cargo paths. Stick to `/` in Justfile
  recipes for cross-platform consistency.

### CI — Lavapipe filter-test skip pattern

Filter pipelines that work on Metal / hardware Vulkan / DX12 fail
on lavapipe with `Validation Error / Parent device is lost` or
`Buffer ... is invalid` at `get_mapped_range`. Refactoring the
pipelines to fit lavapipe compromises real-GPU design for a
software emulator's limits — wrong call. Skip on lavapipe
instead.

**Every filter that transitively runs
`crate::filter::blur::run_blur_pass` is in this class.** A new
story / test that touches *any* of these trips lavapipe unless
it's guarded:

| Filter / wrapper            | How it reaches `run_blur_pass`                |
| --------------------------- | --------------------------------------------- |
| `BlurFilter`                | direct                                        |
| `DropShadowFilter`          | alpha-extract → `run_blur_pass` → composite   |
| `MotionBlurFilter`          | directional `run_blur_pass`                   |
| `apply_privacy_blur`        | wraps `BlurFilter` via `apply_filter`         |
| `apply_privacy_blur_data`   | wraps `BlurFilter` via `apply_filter`         |

Anything new that calls `apply_filter(<one of the above>, …)`
falls into the same class. **Non-blur mask primitives**
(`apply_clip` / `apply_solid_redaction` / `apply_spotlight` /
`apply_dim_outside_data` / `apply_path_clip` /
`apply_mask_to_texture`) use single-bind-group pipelines and run
fine on lavapipe — **don't guard them**.

**Pre-PR checklist when adding a story or test:**

1. Grep your diff:
   `git diff main...HEAD -- crates/wisp-storybook/src/stories/ crates/wisp/tests/ | grep -E "BlurFilter|DropShadowFilter|MotionBlurFilter|apply_privacy_blur"`.
2. If anything matches AND the call site isn't behind a
   `WISP_SKIP_GPU_FILTER_TESTS` env guard, the story / test
   trips lavapipe.
3. **Storybook story** → add its id to `LAVAPIPE_INCOMPATIBLE` in
   `crates/wisp-storybook/tests/story_smoke.rs`. **`wisp` unit /
   integration test** → `if skip_on_software_adapter() { return; }`
   at the top, gated on `WISP_SKIP_GPU_FILTER_TESTS=1`.
4. Verify both paths locally:
   `WISP_SKIP_GPU_FILTER_TESTS=1 cargo nextest run -p wisp-storybook --test story_smoke`
   must pass (story is filtered or guard fires); without the env
   var, the same run must still exercise the pipeline (~1s+ for
   blur-touching paths). Both runs green = lavapipe-safe.

**Burned cycles so far:**

- M-MASK.2/.3/.4 — `apply_privacy_blur*` reached `BlurFilter`.
- M-TEXT.8 (`text-shadow-glow`) — `apply_filter(DropShadowFilter)`
  runs `run_blur_pass` internally; the story had no env-guard and
  only the *immediate* `BlurFilter` was in the explicit example
  list. Fix: add `text-shadow-glow` to `LAVAPIPE_INCOMPATIBLE` +
  expand the explicit-filter list (this table) to name
  `DropShadowFilter` and `MotionBlurFilter` so future authors
  cannot read "blur" too narrowly.

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
- **Tests that spawn external CLIs MUST have a runtime skip guard.**
  Pattern in `crates/decode/tests/gstreamer_integration.rs` —
  `gstreamer_available()` does a `--version` spawn check and returns
  `bool`; tests early-return with `eprintln!` when false. Apply to
  every integration test that calls `Command::new("...")` with a
  binary-name (vs absolute path), even when CI is supposed to apt-install
  it. Reason: on GitHub Actions Ubuntu runners we've observed the
  `gstreamer1.0-tools` package install successfully (`Setting up
  gstreamer1.0-tools` shows up in apt log) but later `cargo nextest`
  test processes still get `ENOENT` on `gst-discoverer-1.0` spawn — root
  cause unclear (possibly nextest process-isolation, possibly a CI
  runner image quirk), but the skip guard makes it a non-issue. The
  decode integration tests at positions 12-14 of the same nextest run
  successfully spawn gstreamer; the preview/screen-app tests at 23+
  fail with `ENOENT`. Same PATH inheritance, different outcome.
- **Production gstreamer errors should dump `PATH` for diagnosis.**
  `Error::Spawn` in `decode::gstreamer_pipe` includes the snapshot
  `PATH=...` so when this CI mystery recurs the log surfaces the exact
  lookup state. Modeled on the same approach the `Tauri 2 macOS dragdrop`
  diagnostics took.
- **Public `gstreamer_available()` helper** lives at
  `decode::gstreamer_pipe::gstreamer_available`. Reused by tests in
  `preview` and `screen-app` so the skip guard is a one-line check in
  every gstreamer-using integration test.

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
- **Two-book setup uses an in-repo preprocessor for cross-links and
  shared fragments** (`tools/mdbook-preprocessor-cross`). Tags:
  `\{\{shared rel/path.md\}\}` inlines from `_docs/shared/`;
  `\{\{wisp-link path\}\}` emits a per-book URL (relative inside
  wisp, absolute `/Screen/wisp/...` from screen). The preprocessor
  needs `target/debug` on PATH before `mdbook build`; recipes set
  `PATH="$(pwd)/target/debug:$PATH"`.
- **When documenting `\{\{shared X\}\}` syntax inside a shared
  fragment, escape the braces** (`\{\{` in source → renders as `{{`
  in the page) so the preprocessor doesn't recursively try to
  expand its own docs. Otherwise every page that inlines the
  fragment gets a runtime "no such file" error comment.
- **Rustdoc renders the preprocessor's own source as HTML under
  `target/book/api/`,** which contains the literal error-template
  string. Exclude `api/` from any "no preprocessor errors in
  rendered HTML" grep: `grep -rE 'mdbook-preprocessor-cross.*error' target/book --exclude-dir=api`.
- **mdbook static asset references in `book.toml` (mermaid.min.js,
  mdbook-admonish.css) must exist in the book's root, not just in
  the workspace's first book.** When extracting a second book,
  copy these alongside `book.toml` or `mdbook build` fails with
  "Unable to copy across static files" on the first build.
- **`just` reads `{{X}}` as variable interpolation in recipe
  bodies.** Strings like `{{shared X}}` or `{{wisp-link Y}}` in
  Justfile *comments* parse and fail with "Unknown start of token"
  / "Variable not defined". Use plain prose ("the shared X tag")
  or escape with backticks in comments.
- **Shell text-matching is a portability trap. Use a Rust binary.**
  Three different failure modes have hit us:
  1. **macOS `sed` lacks ERE `+` in BRE mode.** `sed 's/X+/Y/'`
     works on GNU sed (Linux) but fails on macOS without `-E`.
  2. **Python heredocs require `python3` on PATH.** Windows ships
     `python` not `python3`; Git Bash PATH ordering varies.
  3. **`grep -P` on Windows Git Bash falls back to byte-level
     matching for non-ASCII.** A character class
     `[┌│└├═╔╗]` becomes a byte-set including `\xE2`, which is
     the leading byte of every char in the U+2000–U+2FFF range
     — em dash (`—`), ellipsis (`…`), curly quotes, *all*
     box-drawing chars. The `mermaid-check` gate false-matched
     hundreds of lines on Windows for this reason. macOS / Linux
     glibc grep was fine.
  
  **The rule:** for any pattern matching beyond plain ASCII
  substrings, write a small Rust binary under `tools/`. Rust
  strings are UTF-8-by-construction; regex crate works at char
  level regardless of locale; `cargo build`-fast on warm cache.
  See `tools/doc-gates/` for the pattern — one lib + bin with
  subcommands (`shared-check`, `snapshots-check`, `mermaid-check`,
  `required-files-check`), ~300 LOC including 30 tests. Adding a
  new gate is one function + one match arm.

### mdBook live-reload (split-book serving)

- **`mdbook serve` has its own live-reload** — filesystem watch +
  websocket broadcast to a script injected into the rendered HTML.
  Different from the `dev-server` crate (which is for storybook
  asset reloads). For docs, prefer `mdbook serve` directly. The
  `dev-book` / `dev-wisp-book` recipes use it on ports 3001/3002.
- **mdbook's watch covers `src/` + `book.toml`, including
  `_docs/shared/`** (followed transitively through the
  preprocessor's `{{shared}}` inclusion). It does NOT cover the
  preprocessor's source — changes to
  `tools/mdbook-preprocessor-cross/src/lib.rs` need a Ctrl-C +
  re-run of `just dev-book` so `preprocessor-build` recompiles.
- **Cross-book absolute URLs (`/Screen/wisp/...`) don't resolve
  under `mdbook serve`.** Production deploys at that prefix; local
  serve runs at `/`. Use the in-book TOC for navigation; use `just
  site` + open `target/book/` for production-shape verification.

### Build hygiene

- **New error variants need a caller** (CONVENTIONS § Error handling). `cargo` warns; clippy errors at `-D warnings`.
- **`#[allow(clippy::*)]` requires `reason = "..."`** — no exceptions.
- **Cargo cache can lie when nextest + workspace-check race.** Symptom: `cargo check --workspace --all-targets --all-features` reports `E0599 no variant, associated function, or constant named X` for a method/variant that *is* in the source file (and a per-crate `cargo check -p X` succeeds on the same source). Cause: `cargo nextest run -p crate --test foo` builds the test crate against an older dep snapshot and leaves a stale dep hash; the next workspace check picks up that snapshot. **Fix:** `cargo clean -p <crate>` and rerun. Add a renderer / library new-API change in one stable order: edit source → `cargo check -p <crate>` → run tests → run gate. Don't interleave nextest of an in-flight test against the new API with workspace checks.
- **`target/` can balloon past 60 GB after pulling in axum/tokio/reqwest/tungstenite trees.** A full `just gate` on the dev-server crate set added ~50 GB on top of an already-warm cache and ran the laptop out of disk mid-link (`ENOSPC: clang -o ...rmeta`). Watch for `du -sh target` creeping past ~30 GB; `cargo clean -p <heaviest>` (`wisp-storybook` is one of the biggest) reclaims a couple of GB without nuking the workspace cache. If the harness itself starts erroring with `ENOSPC` on its task-output writes, only the user can recover — `cargo clean` from a real terminal.
- **Integration tests that spawn a sibling `[[bin]]` MUST locate it via `env!("CARGO_BIN_EXE_<name>")`** — never hand-roll `target/debug/<name>` and never call `cargo build` from inside a `#[test]`. Cargo guarantees `CARGO_BIN_EXE_*` is set at integration-test compile time AND that the bin is built as a dep of the test run; the hand-rolled path is wrong on Windows (missing `.exe`), wrong when `CARGO_TARGET_DIR` is set, and the in-test `cargo build` shim races under nextest's per-binary parallelism (multiple tests grab the same package-cache + build-dir locks; a spawn can win against the unfinished build with `Os { code: 2, NotFound }`). Burned this on `mdbook-preprocessor-cross::preprocessor_protocol::supports_html_renderer` in CI on the Gantt branch (May 2026). **Reference pattern:** `crates/dev-server/tests/binary_smoke.rs::binary_path` — one line: `PathBuf::from(env!("CARGO_BIN_EXE_<name>"))`. Both other integration tests in `tools/` (`doc-gates/tests/cli.rs`, `mdbook-preprocessor-cross/tests/preprocessor_protocol.rs`) have been migrated to this pattern. Apply prophylactically to every new binary-spawning integration test.

### Dev loop / dev-server

- **`format!` with the same named placeholder repeated inside one big format string can confuse the macro parser** with cryptic "expected `,`, found `{`" errors at high column numbers. Don't try to be clever with `"<a href=\"#{id}\" data-id=\"{id}\">{title}</a>"`. Either split into multiple `format!` calls, switch to `push_str` concatenation, or pre-format into intermediate variables. The exporter's `render_index` is the cautionary tale — see `crates/ui-storybook/src/exporter.rs`.
- **Axum 0.7 HTML-injection middleware lives at the crate root, not the route layer.** The pattern: `Router::new().route(...).fallback_service(ServeDir).with_state(...).layer(middleware::from_fn(inject_live_reload))`. `to_bytes(body, MAX)` collects the streaming `ServeDir` body so the middleware can splice in the live-reload script before `</body>`. Always update `Content-Length` after splicing.
- **`notify-debouncer-mini` calls your handler from its own thread, not the tokio runtime.** Forward batched events to a `tokio::sync::mpsc::unbounded_channel` and process them in a tokio task. Forgetting this gives a runtime panic on the first `tokio::spawn` from inside the handler.
- **`tailscale serve` (private) vs `tailscale funnel` (public) is the single decision** for remote dev. We use Serve. Don't flip to Funnel — it exposes the storybook to the open internet.
- **Tailscale Serve requires one-time enablement on the tailnet.** First `tailscale serve --bg ...` returns *"Serve is not enabled on your tailnet. To enable, visit: https://login.tailscale.com/f/serve?node=..."*. The URL approval is per-tailnet, not per-machine — once approved, every machine in the tailnet can register Serve routes. Document the URL clearly in the dev-remote runbook.
- **Binary-level integration tests catch what library tests can't.** The dev-server shutdown-signal bug (`.map(|s| async move { s.recv().await })` form dropped the inner Future and tripped self-shutdown at startup) was invisible to the lib `smoke.rs` tests because they bypassed `main.rs`. The fix: every binary should have a test that spawns the actual built binary (not the lib) and pokes it over its real I/O surface. `crates/dev-server/tests/binary_smoke.rs` is the reference pattern: `Command::new(binary_path())` + RAII `ServerChild` guard for kill-on-drop + `reqwest` over `127.0.0.1:<free_port()>`. Pair with `book_render_smoke.rs` which spawns `mdbook serve` and validates the books render — that's the "Tailscale doesn't regress" integration test: if mdbook serves locally, Tailscale Serve will tunnel it.
- **Linker config (`.cargo/config.toml`) belongs in `.gitignore`.** `-fuse-ld=lld` errors at link time if lld isn't installed; committing the config breaks fresh clones. Ship `.cargo/config.toml.example` as the template, document `brew install lld` in CLAUDE.md, let each dev opt in.

### Publishing crates to crates.io

> [!IMPORTANT]
> **Publishing is currently *staged but disabled*.** Every piece of
> the release-plz pipeline is wired (workflow file, Cargo.toml
> metadata, `release-plz.toml`, LICENSE + CHANGELOG inside the
> crate dir, `[lib].name = "wisp"` decoupling). The auto-trigger is
> commented out at the top of `.github/workflows/release-plz.yml`
> under the `ENABLE-PUBLISH-AUTO` marker. Re-enabling = uncommenting
> four lines + completing three setup steps (CARGO_REGISTRY_TOKEN
> secret, first manual publish, GHA PR-creation permission). The
> workflow file has the full enable-runbook in its header comment.
> Until then, no merge to main can publish anything — `just gate`
> stays green, infrastructure stays validated, but crates.io stays
> untouched.

- **`wisp` is the only crate that *will* be published.** Everything
  else has `publish = false` (workspace default). To prep a new
  crate for publishing: override `publish = true` in its
  Cargo.toml, add a `[[package]]` block to `release-plz.toml`,
  create `crates/<name>/CHANGELOG.md`, copy `LICENSE` into the
  crate dir.
- **Published name `screen-wisp` ≠ library name `wisp`.** crates.io's
  `wisp` is claimed by an unrelated tmux project; we publish as
  `screen-wisp` but keep `[lib].name = "wisp"` so internal +
  downstream code keeps `use wisp::...` working. Cargo handles the
  decoupling: `[package].name` is what `cargo -p <name>` and
  crates.io see; `[lib].name` is what Rust import statements see.
  Downstream consumers: `screen-wisp = "0.1"` in Cargo.toml, then
  `use wisp::...` in code.
- **`cargo -p <pkg>` takes the package name, not the workspace dep
  alias.** Once you rename to `screen-wisp`, every workflow + README
  + book chapter referencing `cargo run -p wisp` needs `-p
  screen-wisp`. Workspace-internal `wisp.workspace = true` deps
  still work via the alias `wisp = { package = "screen-wisp", ... }`
  in `[workspace.dependencies]`.
- **`release-plz` *will* drive the publishing flow once enabled.**
  Configured via `release-plz.toml` at repo root. Two GHA jobs in
  `.github/workflows/release-plz.yml`: `release-plz-pr` opens /
  updates a "Release PR" on every push to main; `release-plz-release`
  runs on the Release PR merge → tags + publishes. The Release PR
  is the CD opt-in moment — main is "ready to release, not yet
  released." **Today the auto-trigger is disabled** (see the
  warning callout above); enabling = four uncommented lines in the
  workflow file.
- **First publish is by hand.** release-plz can't claim an
  unreserved crate name; the initial `cargo publish -p screen-wisp`
  has to happen from a logged-in dev box with the
  `CARGO_REGISTRY_TOKEN` in `~/.cargo/credentials.toml`. After that,
  every release flows through release-plz.
- **Required secrets:** `CARGO_REGISTRY_TOKEN` (from
  https://crates.io/settings/tokens, publish-scope) under Settings →
  Secrets → Actions. `GITHUB_TOKEN` is provided by the runner; the
  workflow's `permissions:` block grants `contents: write` +
  `pull-requests: write`.
- **Conventional commits drive the semver bump.** `feat(wisp): …` →
  minor; `fix(wisp): …` → patch; `feat(wisp)!: …` or `BREAKING
  CHANGE:` footer → major. `chore:` / `docs:` / `ci:` / `refactor:`
  / `test:` don't trip a release. release-plz's `commit_parsers`
  list in `release-plz.toml` is the canonical mapping.
- **`include` field in Cargo.toml.** Defines exactly what lands in
  the .crate file. We list `src/`, `shaders/`, `examples/`,
  `tests/`, `Cargo.toml`, `README.md`, `CHANGELOG.md`, `LICENSE`.
  Things NOT in `include` (like local benchmark outputs, scratch
  files) are silently dropped — that's a feature. Verify with
  `just publish-wisp-files`.
- **Dry-run before opening a Release PR.** `just publish-wisp-dry`
  runs `cargo publish --dry-run` to catch metadata issues
  (missing fields, version collisions, dirty tree, unknown
  categories) before the workflow does.
- **`cargo semver-checks` runs as part of release-plz** (see
  `semver_check = true` in `release-plz.toml`). Catches API breaks
  that should have been `feat!:` but were committed as a minor
  `feat:`. Installed via taiki-e/install-action in the workflow.
- **Repository name case affects rustdoc deploy URL but not
  crates.io.** crates.io is its own DNS; the `repository = "..."`
  field is what crates.io shows. Use the case-exact GitHub URL
  (`https://github.com/eng-manager-xyz/Screen`); the GitHub Pages
  case mismatch (which 404s) is irrelevant to crates.io.

### Linear MCP / Cloudflare WAF

- **Linear's MCP edge is behind Cloudflare and rejects POST bodies containing literal `<script>` tags.** Saving an issue with HTML code snippets returns the Cloudflare "blocked" page. Symptom: `Streamable HTTP error: Error POSTing to endpoint: <!DOCTYPE html><html…>Sorry, you have been blocked</html>` in the MCP response. **Workaround:** describe the injection in prose ("inline JS that opens a WebSocket and calls `location.reload()`") or HTML-entity-encode the tags. The narrative content always lands; users can paste the literal code into the Linear UI later.

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
- **UI:** Leptos 0.8 (Rust → WASM) inside the Tauri webview. See `.claude/skills/leptos-migration.md` for version-by-version migration notes and "strive to use" 0.8 idioms.
- **Renderer:** `wisp` (in-repo, `crates/wisp`) — wgpu + WGSL
- **Editor preview:** native `winit` sibling window rendered by `wisp`
- **Capture:** `objc2`/ScreenCaptureKit (macOS), `windows-rs` (Windows), `pipewire-rs` (Linux)
- **Media (decode + playback + encode + mux):** GStreamer is the single media stack. Decode + playback ship today as a `gst-launch-1.0` CLI subprocess (see [GStreamer integration choice](#gstreamer-integration-choice) below); encode lands in M-EXPORT via `gstreamer-rs` Rust bindings + `appsrc` (so wisp's render-target frames push into the pipeline) with platform HW encoders (`vtenc_h264_hw` macOS, `mfh264enc` Windows, `vaapih264enc`/`nvh264enc` Linux).

```admonish important title="GStreamer is the only media library — do NOT add ffmpeg-next"
This project deliberately uses **only GStreamer** for decode, playback, encode, and mux. Earlier planning docs (now corrected) listed `ffmpeg-next` as a transitional MVP option; that path was dropped before any encode code shipped. Reasons captured in [AUT-144](https://linear.app/harwood/issue/AUT-144):

- One media stack instead of two — single build dependency, single license story (GStreamer LGPL vs. ffmpeg's GPL/LGPL split), one `deny.toml` entry, one mental model.
- GStreamer's element graph (`appsrc → encoder → mux → filesink`) is a strictly better fit for the "capture → wgpu compose → encode" live pipeline than ffmpeg's libavformat. The HW-encode coverage is equivalent across macOS/Windows/Linux.
- Decode + playback already use GStreamer (see `decode::gstreamer_pipe`, `media::gstreamer`). Adding ffmpeg would re-introduce a second toolchain for no product gain.

**Do not add `ffmpeg-next`, `ac-ffmpeg`, `ffmpeg-sys-next`, or any other ffmpeg binding crate to this workspace.** If you find yourself wanting one, the answer is a GStreamer element — open AUT-144 for the mapping table or extend it. Historical PROGRESS.md entries that mention ffmpeg are journal entries documenting the M0.21 pivot; they describe what happened and are not directives.
```

## Remote-first UI dev loop

`just dev` runs `dev-server` (axum + WebSocket live reload) against the storybook assets in `_docs/book/src/assets/ui/`, watches `crates/ui-storybook/src` + `assets/style.css`, and broadcasts a reload on every successful rebuild. `just dev-remote` adds `tailscale serve` for phone preview (see the [remote-dev runbook](_docs/book/src/conventions/remote-dev.md) — the ≤5-click setup).

```admonish important title="The dev-server crate is the home for dev-loop tooling"
Don't add cargo-watch / Trunk / browser-sync / Node tooling for the dev loop. The single Rust crate at `crates/dev-server/` owns: file watching, debouncing, subprocess rebuild, WebSocket live reload, HTML response injection. Each piece has tests in `crates/dev-server/tests/`. The presentational-contract grep (UI-23) already keeps `ui-storybook` honest; the new piece is **never bypass the watcher's coalescing** — rapid-fire saves should produce one rebuild, not N.
```

**The dev loop's invariants** (gates enforce):
- `dev_server::live_reload::INLINE_CLIENT` is injected ONLY into `text/html` responses ending in `.html` or `/`. CSS, JSON, PNG, etc. pass through byte-identical. (`tests/smoke.rs::css_response_is_byte_identical`.)
- `ui_storybook::exporter::export_all` produces `<id>.html` for every story + `index.html` + `style.css`. The cockpit `index.html` contains every story id from `all_stories()`. (`tests/index_html.rs`.)
- `render-worker` honours the JSON-IPC protocol in `dev_server::worker::{WorkerCommand, WorkerReply}`. The reply schema is part of the contract — don't rename fields. (`tests/render_worker.rs`.)

**Linker speedup is opt-in.** `.cargo/config.toml.example` ships the mold/lld config; users symlink or copy to `.cargo/config.toml` (gitignored) after `brew install lld` or `apt install mold`. Knocks ~30–50 % off warm incremental rebuilds. Don't commit `.cargo/config.toml` itself — would break fresh clones without the linker installed.

---

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
- **Leptos work invokes the `leptos-migration` skill first.** Any
  edit that touches `leptos::`, `#[component]`, `view!{}`, signals,
  effects, resources, actions, or server fns triggers a
  `Skill` call to `leptos-migration` (`.claude/skills/leptos-migration.md`)
  *before* the first edit. The skill has the pinned version
  (`"0.8"`), the version-by-version name-changes table, the
  "strive to use" 0.8 idioms (`signal()`, `Effect::watch`,
  `FromServerFnError`, `Websocket` server fns,
  `--cfg=erase_components`), and the project-specific landmines.
  See the **Leptos discipline** section above for full context.
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
