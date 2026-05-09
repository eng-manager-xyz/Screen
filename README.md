# `screen` — the production

A cinematic screen recorder built as an all-Rust stack. The codebase is laid out like a theatre production: a **Stage** (the renderer), a **Cast** (scene-graph nodes), **Acts and Scenes** (milestones and chunks), **Rehearsals** (tests), and a **Playbill** (interactive feature gallery) you can flip through to see every scene we've blocked so far.

This README is the map. Use it to find your way around the source tree and the docs in `_docs/`.

---

## The cast of metaphors

Every concrete artifact in this project has a theatre name. Use these terms to navigate the source code, conversations, and commits:

| Theatre term | What it actually is | Where it lives |
|---|---|---|
| **The Production** | The whole project | the workspace root |
| **The Stage** | `wisp` — the 2D scene graph + filter chain on `wgpu` | `crates/wisp/` (literal `Stage` type at `src/scene.rs`) |
| **The Wings** | `screen-app` — the Tauri+Leptos shell that brings the Stage to the audience | `crates/app/` (M1 fills this in) |
| **The Playbill** | `wisp-storybook` — interactive gallery with every shipped scene | `crates/wisp-storybook/` |
| **Acts** | Milestones (M0, M1, …) — major phases | `_docs/milestone-N-*.md` |
| **Scenes** | Chunks within an act (M0.5, M0.6, …) — single units of work | listed inside each milestone doc |
| **The Cast** | Scene-graph node types — the players on the Stage | `crates/wisp/src/scene/` |
| **Choreography** | `Transform` — how cast members move (position / scale / rotation / pivot) | `crates/wisp/src/scene/transform.rs` |
| **Backdrops** | Textures — image, video, render | `crates/wisp/src/texture/` |
| **Sets** | Vector primitives drawn by `Graphics` (rect, ellipse, line, gradient) | `crates/wisp/src/scene/graphics.rs` |
| **Props** | Sprites — small textured quads carried by the cast | `crates/wisp/src/scene/sprite.rs` |
| **Lighting** | Filters (blur, drop shadow, motion blur, color matrix) — the atmosphere | `crates/wisp/src/filter/` (lands M0.16+) |
| **Rehearsals** | Tests — unit, integration, snapshot, property | `crates/*/tests/` and `#[cfg(test)]` blocks |
| **Dress Rehearsal** | `just gate` — must pass green before any scene is "blocked" (marked done) | `Justfile` |
| **Tech Week** | Side quests — the off-stage scaffolding that makes the show possible | QA toolchain, testing spine, storybook |
| **The Promptbook** | `_docs/PROGRESS.md` — append-only log of every scene that's been performed | newest entries at top |
| **Stage Directions** | `_docs/WORKFLOW.md` — what to do for every scene | step-by-step |
| **House Rules** | `_docs/CONVENTIONS.md` — code standards and naming | naming, errors, tests, modules |
| **Tech Notes** | `_docs/TESTING.md` and `_docs/QA.md` — the testing pyramid + tooling tiers | what gates exist and why |
| **Call Sheet** | `_docs/ISSUES.md` — bugs / deferrals / open questions to revisit | append-only |
| **The Director's Bible** | `CLAUDE.md` — auto-loaded into every Claude Code session | the non-negotiable loop lives here |

> **The members of the Cast are concrete:** `Container` (ensemble member), `Sprite` (featured player carrying a Backdrop), `Graphics` (scenic painter), `Text` (the captioner), `Mesh` (the special-effects performer — M0.19). They all share a `Container` aspect that gives them a place on the Stage and a Choreography of their own.

---

## Quick start

```bash
# Bootstrap the QA toolchain (one-time per machine).
just bootstrap

# The dress rehearsal — must pass before marking any chunk done.
just gate

# Open the Playbill — interactive feature gallery.
just storybook

# Supply-chain gate (license + advisories + unused deps).
just security

# See all recipes.
just
```

---

## Acts in progress

### Act 0 — `wisp` Foundations *(in progress)*

Building the Stage from the ground up.

- **Scenes M0.1 – M0.14: completed.** Workspace, wgpu device, scene graph, transforms, sprites, textures, render targets, graphics primitives, gradients.
- **Scenes M0.15 – M0.21: pending.** Bitmap text, the Lighting rig (filters), Mesh + custom WGSL, the proof-point examples (recorder_mock, headless_export).
- See `_docs/milestone-0-renderer.md` for the full scene breakdown.

### Act 1 — Tauri + Leptos drop-zone player *(pending)*

The Wings — drop in an MP4, see it play. Validates the Tauri+Leptos toolchain before wiring anything renderer-heavy.

- 11 scenes, see `_docs/milestone-1-drop-zone-player.md`.

---

## The non-negotiable loop

For every scene we block, this is the discipline (defined fully in `CLAUDE.md`):

```
1. TEST    → at least one rehearsal (unit / integration / snapshot / property)
2. STORY   → if it's a renderable feature, add a Playbill entry with a write-up
3. CHECK   → `just gate` — loop recursively until green; never bypass
4. UPDATE  → PROGRESS, ISSUES, milestone tick
5. STATUS  → mark the scene "blocked" (done) in the task list
```

If `just gate` is red, **keep iterating until it's green.** Never `#[allow]` past clippy without `reason = "..."`. Never `#[ignore]` a failing rehearsal without an `ISS-NN`. Never bypass `cargo deny` or `cargo machete` without a documented exemption.

---

## Layout

```
screen/                                # The Production
├─ README.md                           # this file — map to the production
├─ CLAUDE.md                           # The Director's Bible (auto-loaded)
├─ Justfile                            # `just <recipe>` — every gate and tool
├─ rustfmt.toml                        # House Rules: formatting
├─ deny.toml                           # House Rules: licenses + advisories + bans
├─ Cargo.toml                          # workspace root + shared lints
├─ rust-toolchain.toml                 # nightly
├─ crates/
│  ├─ wisp/                            # The Stage — 2D renderer
│  │  ├─ src/scene/                    # The Cast (Container, Sprite, Graphics, Text, Mesh)
│  │  ├─ src/scene/transform.rs        # Choreography
│  │  ├─ src/texture/                  # Backdrops
│  │  ├─ src/filter/                   # Lighting (M0.16+)
│  │  ├─ src/render/                   # internal — pipelines and pass orchestration
│  │  ├─ shaders/                      # WGSL — what the Stage's lighting board executes
│  │  ├─ examples/                     # standalone scene rehearsals (winit windows)
│  │  └─ tests/                        # integration rehearsals (RenderTexture pixel-readback)
│  ├─ wisp-storybook/                  # The Playbill — interactive gallery
│  │  ├─ src/main.rs                   # eframe entry (the curtain-up)
│  │  ├─ src/app.rs                    # 4/5 canvas + 1/5 write-up sidebar
│  │  └─ src/stories/                  # one file per scene (s_*.rs + writeups/*.md)
│  └─ app/                             # The Wings — Tauri+Leptos shell (M1)
└─ _docs/
   ├─ README.md                        # index of all docs
   ├─ PROGRESS.md                      # The Promptbook (newest at top)
   ├─ WORKFLOW.md                      # Stage Directions
   ├─ CONVENTIONS.md                   # House Rules (code)
   ├─ TESTING.md                       # Tech Notes — testing strategy
   ├─ QA.md                            # Tech Notes — gate tiers + tools
   ├─ ISSUES.md                        # Call Sheet
   ├─ milestone-0-renderer.md          # Act 0 scenes
   ├─ milestone-1-drop-zone-player.md  # Act 1 scenes
   ├─ synthesis-and-stack.md           # production-design rationale
   ├─ recorder-features-and-render-api.md  # full feature inventory
   ├─ openscreen-research.md           # research — open-source reference
   └─ screen-studio-research.md        # research — commercial reference
```

---

## How to read the source

### "Show me what `wisp` can render right now"

```bash
just storybook
```

The Playbill opens with every shipped scene grouped by category. Top-bar menu navigates between them; the right sidebar (1/5 of the window) explains what each scene demonstrates and why it matters for the recorder.

### "Where does Sprite live?"

```
crates/wisp/src/scene/sprite.rs            # the Cast member
crates/wisp/src/render/sprite_pipeline.rs  # how the Stage renders it
crates/wisp/shaders/sprite.wgsl            # the lighting cue (shader)
crates/wisp/tests/render_sprite.rs         # the rehearsals
crates/wisp-storybook/src/stories/         # the Playbill entries
   s_sprite_batcher.rs                     # 100-sprite batching demo
```

### "What's been done? What's next?"

```
_docs/PROGRESS.md                          # newest entry first — every blocked scene
TaskList                                   # via Claude Code's task tool — what's pending
```

### "How do I add a new scene?"

Open `_docs/WORKFLOW.md` § 3 ("Implement"). The short version: start with the rehearsals (tests), add the implementation, add a Playbill entry if the feature is renderable, run `just gate` recursively until green, then mark the scene done.

### "What rules govern the code?"

`_docs/CONVENTIONS.md`. The high points: parent-module file pattern (no `mod.rs`), `clippy::pedantic` on, `#[allow]` only with `reason = "..."`, error types per crate via `thiserror`, no panics in library code outside tests, snapshot tests for filter outputs, `proptest` for invariants, no features added ahead of need (`cargo machete` enforces it).

---

## What's in the Tech Week toolchain

The infrastructure that makes the show possible — installed once via `just bootstrap`:

| Tool | Tier | Purpose |
|---|---|---|
| `just` | gate | recipe runner |
| `cargo-nextest` | gate | faster, isolated test runner |
| `cargo-deny` | pr | license + advisory + ban + source policy |
| `cargo-machete` | pr | unused-dep detection (caught real YAGNI violations already) |
| `cargo-llvm-cov` | pr | coverage measurement |
| `cargo-mutants` | full | mutation testing |
| `miri` | full | UB detection on pure-Rust modules |
| `cargo-flamegraph` | optional | profiling |

See `_docs/QA.md` for the full tier breakdown — `gate` (~30s) → `pr` (~3 min) → `release` (~10 min) → `full` (slow, on demand).

---

## Why this metaphor

The wisp library has a literal `Stage` type. It's the root of the scene graph — every renderable lives under it. Building from there: `Container` is the ensemble player, `Sprite` carries a Backdrop (a Texture), `Graphics` paints scenery (the Sets), and Filters provide the Lighting that gives each scene its mood.

When you say "block scene M0.14" or "the Promptbook says we ran the dress rehearsal yesterday", the meaning is unambiguous. The metaphor names map one-to-one to source files and tools, so navigating the codebase becomes navigating a production you already understand.

When in doubt, the Director's Bible (`CLAUDE.md`) is the entry point. It loads automatically on every Claude Code session and lays out the non-negotiable loop in two screens.
