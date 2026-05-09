# screen

> A cinematic screen recorder in the Screen Studio / OpenScreen lineage, built
> as an all-Rust stack. Tauri 2 shell, Leptos UI, custom wgpu renderer, GStreamer
> decode, native preview window. Library-first: the renderer (`wisp`) and the
> player (`playback`) are reusable in their own right.

```text
              ┌──────────────────────────┐
              │      screen-app          │   Tauri 2 shell
              │   (native binary)        │   ◀── M-INT.2 will land here
              └──────────┬───────────────┘
                         │   serves frontendDist
                         ▼
              ┌──────────────────────────┐
              │      app-ui              │   Leptos CSR app
              │   (WASM via Trunk)       │
              └──┬─────────────────┬─────┘
                 │ uses            │ uses
                 ▼                 ▼
        ┌────────────────┐  ┌────────────────┐
        │  ui-storybook  │  │   playback     │ ◀── orchestrator
        │ (component     │  │   (Player)     │
        │  library +     │  └──┬──────────┬──┘
        │  gallery)      │     │          │
        └────────────────┘     ▼          ▼
                       ┌────────────┐ ┌────────────┐
                       │   decode   │ │    wisp    │
                       │  (Video    │ │  (wgpu     │
                       │   Stream + │ │   renderer)│
                       │   GStreamer│ └────────────┘
                       │   pipe)    │
                       └────────────┘
```

The codebase also carries a parallel **theatre metaphor** that you'll see
referenced in commits and docs (`Stage`, `Cast`, `Wings`, `Acts`, `Scenes`,
`Rehearsal`). It's the navigation language for the project — see
[`_docs/book/src/orientation/metaphor.md`](./_docs/book/src/orientation/metaphor.md).

---

## Status

| Track | Where |
|---|---|
| **`wisp` renderer** | ✅ M0 complete — 21 chunks: stage, transforms, sprites, graphics (SDF rect/ellipse/gradient), bitmap text, blur / drop-shadow / motion-blur / color-matrix filters, mesh perspective. |
| **Tauri shell + drop-zone player** | ✅ M1 complete — 11 chunks consolidated, vanilla HTML/JS frontend with `convertFileSrc` + HTML5 video. |
| **Component library** | ✅ ui-storybook with 18 stories: Button, Card, DopeSheet, DropZone, PlayerControls, RecordingToolbar, StatusBar. SSR snapshot regression gate. |
| **Decode → wisp pipeline** | ✅ M-DEC.1 + M-PLAY.1 + M-DEC.2 — `VideoStream` trait, `MockVideoStream`, `Player` state machine, `GstreamerPipeStream` reading real MP4s end-to-end. |
| **Recorder shell (Leptos CSR)** | ✅ M-INT.1 — `app-ui` builds via Trunk; mounts `<App>` composing existing components. |
| **Tauri ↔ Leptos integration** | ⏳ M-INT.2 — `tauri.conf.json` frontendDist swap + OS file-drop wiring. |
| **Native winit preview window** | ⏳ M-PREVIEW.1 — wisp surface as a sibling window to the Tauri webview. |
| **Tauri ↔ player IPC** | ⏳ M-PLAY.2 — load / play / pause / seek dispatched from Leptos to the native player loop. |

3 chunks separate us from the first end-to-end "drag MP4 → see it play through wisp" demo. Full chunk-by-chunk log in [`_docs/PROGRESS.md`](./_docs/PROGRESS.md).

---

## Quick start

### Prerequisites

The project pins to **Rust nightly** (see `rust-toolchain.toml`); rustup will
auto-install it on first build.

#### Required

```bash
# Rust toolchain (nightly + wasm32 target)
rustup target add wasm32-unknown-unknown

# Task runner — one place for every QA recipe.
brew install just                # macOS
# or: cargo install --locked just

# QA tools used by `just gate`
cargo install --locked cargo-nextest cargo-deny cargo-audit cargo-machete

# Documentation site builder
cargo install --locked mdbook

# WASM bundler for the Leptos shell
cargo install --locked trunk

# Video decode backend
brew install gstreamer           # macOS — the cask name is just `gstreamer`
# or: apt install gstreamer1.0-tools gstreamer1.0-plugins-good gstreamer1.0-libav
```

`just bootstrap` automates the Rust-side tools once Homebrew + Rust are
present.

#### Optional / on demand

```bash
cargo install --locked cargo-llvm-cov   # coverage
cargo install --locked cargo-semver-checks
cargo install --locked cargo-public-api
cargo install --locked cargo-msrv
cargo install --locked cargo-bloat
cargo install --locked cargo-geiger
cargo install --locked cargo-mutants
rustup component add miri --toolchain nightly
rustup component add llvm-tools-preview
```

### Build & run

```bash
git clone https://github.com/eng-manager-xyz/Screen.git
cd Screen

# Verify the gate is green from a fresh checkout (~1–2 min on first build).
just gate

# Build everything once.
cargo build --workspace
```

### Try it now (deepest demo currently shippable)

```bash
# End-to-end: decode an MP4 → upload to GPU → render through wisp.
# Output: 7 PNGs at _docs/book/src/assets/playback/playfile_NN.png
cargo run -p playback --example play_file
```

The committed test fixture (`crates/decode/tests/fixtures/sample.mp4`,
11 KB) is opened via GStreamer, streamed frame-by-frame as BGRA bytes,
uploaded to a `wisp::VideoTexture`, rendered through the same `Sprite`
pipeline the live recorder will use, and written to disk.

```bash
# Browse the recorder shell (Leptos CSR app).
just app-ui          # localhost:8080 — the Leptos CSR shell

# Browse the wgpu story gallery (native eframe window).
just storybook
```

### View the offline engineering site

```bash
just site
# Opens target/book/index.html — prose chapters + per-feature
# screenshots + full rustdoc API reference.
```

---

## Repository layout

```
screen/
├─ CLAUDE.md                # Auto-loaded into every Claude Code session.
│                            Architecture, conventions, anti-patterns,
│                            the 11-step per-task workflow.
├─ Justfile                 # Every QA recipe — `just` to list.
├─ rust-toolchain.toml      # Pins nightly.
├─ deny.toml                # cargo-deny: license / advisory / source policy.
├─ rustfmt.toml             # Formatter config.
├─ Cargo.toml               # Workspace + shared lints.
│
├─ crates/
│  ├─ wisp/                 # The wgpu renderer. Pixi-shaped public API:
│  │                          Stage, Container, Sprite, Graphics, Text,
│  │                          Mesh, Transform, RenderTexture, VideoTexture,
│  │                          BlurFilter, DropShadowFilter, MotionBlurFilter,
│  │                          ColorMatrixFilter.
│  │
│  ├─ decode/               # VideoStream trait + MockVideoStream +
│  │                          GstreamerPipeStream. The codec-agnostic seam
│  │                          between video files and wisp.
│  │
│  ├─ playback/             # Player state machine — owns the boxed
│  │                          VideoStream + a wisp VideoTexture; pumps
│  │                          frames at the source frame rate.
│  │
│  ├─ ui-storybook/         # Leptos component library + isolated visual
│  │                          gallery + SSR snapshot tests. Feature-tested
│  │                          building blocks for the recorder.
│  │
│  ├─ app-ui/               # The actual Leptos CSR shell. Trunk-built.
│  │                          Composes ui-storybook components into the
│  │                          recorder surface.
│  │
│  ├─ wisp-storybook/       # Interactive wgpu gallery (eframe), one window
│  │                          showing every renderable wisp feature.
│  │
│  └─ app/                  # Tauri 2 shell — native binary.
│
└─ _docs/
   ├─ PROGRESS.md           # Append-only log, newest at top. Every chunk
   │                          gets an entry here.
   ├─ WORKFLOW.md           # Canonical 11-step per-task workflow.
   ├─ TESTING.md            # Anti-regression gravity, per-chunk minimums.
   ├─ QA.md                 # `just gate` definition + tier system.
   ├─ CONVENTIONS.md        # Code conventions.
   ├─ ISSUES.md             # Known bugs / deferrals / open questions.
   ├─ milestone-0-renderer.md, milestone-1-drop-zone-player.md
   │
   └─ book/                 # mdBook prose site (rendered to target/book/).
      └─ src/
         ├─ orientation/    # What this is, stack, theatre metaphor.
         ├─ conventions/    # Workflow, testing, docs gate, screenshots.
         ├─ wisp/           # Renderer overview + per-chunk chapters.
         ├─ ui/             # UI components + per-chunk chapters.
         ├─ decode/         # Decoder overview.
         ├─ playback/       # Player overview + real-MP4 chapter.
         ├─ app-ui/         # Recorder shell overview.
         ├─ milestones/     # Per-milestone summaries.
         └─ assets/         # Per-feature screenshots — committed.
```

---

## Engineering workflow (the short version)

> Full canonical version in [`_docs/WORKFLOW.md`](./_docs/WORKFLOW.md). What follows
> is a TL;DR of the 11-step per-task contract that every chunk goes through.

For every chunk:

1. **Pick** the next unblocked task.
2. **Mark in_progress** (one task at a time).
3. **Implement** the smallest unit that satisfies the chunk's "Done when:".
4. **Test** — unit / snapshot / integration / property / regression.
5. **Story** — every renderable feature ships with a story in
   `crates/wisp-storybook/` (wgpu) or `crates/ui-storybook/` (HTML/Leptos).
6. **Asset** — `just snapshots` regenerates the chunk's PNG/HTML under
   `_docs/book/src/assets/<crate>/<id>.{png,html}`.
7. **Chapter** — write `_docs/book/src/<crate>/chunks/<id>.md`, embed the
   asset, add to `SUMMARY.md`.
8. **Check** — `just gate` must be green. Loop until it is.
9. **Site** — `just site` rebuilds the engineering site, verify the chapter
   renders.
10. **Update** — append entry to `_docs/PROGRESS.md`, file new issues in
    `_docs/ISSUES.md`.
11. **Mark completed** + **commit**. Conventional-commit format with the
    workflow checklist in the body.

### `just gate`

```bash
just gate    # fmt + check + lint + nextest + doctest + cargo doc
```

All six must pass. Failures loop until green — never disable tests, never
`#[allow]` clippy without `reason = "..."`, never bypass `cargo deny` /
`cargo audit` / `cargo machete`.

### Higher tiers

```bash
just pr           # gate + cargo deny + cargo audit + cargo machete + coverage
just docs-strict  # rustdoc with broken-link enforcement (milestone close)
just release      # pr + semver + msrv + bench + bloat + geiger
just full         # everything (slow — adds miri + mutants)
```

---

## Documentation

- **`CLAUDE.md`** — auto-loaded into every Claude Code session. Architecture,
  conventions, the per-task workflow, plus an
  ever-growing list of *anti-patterns we've earned* (each one cost a
  recursive-fix iteration somewhere; capturing them prophylactically).
- **`_docs/PROGRESS.md`** — append-only log, newest at top. The only
  durable cross-session record of what's been done.
- **`_docs/WORKFLOW.md`** — canonical 11-step per-task workflow.
- **`_docs/book/`** — the mdBook prose site (`just site` to build/open).
  Includes per-feature chapters with embedded screenshots.
- **rustdoc** — every public item has a `///` doc. `missing_docs` is a
  workspace-level lint; `just docs-strict` flips broken intra-doc links
  to errors.

---

## Stories — every renderable feature is captured

Every chunk that renders something gets a story. Stories are deterministic
constructions of a feature in isolation; they drive:

1. The interactive gallery (`just storybook` for wgpu, `just app-ui` for HTML).
2. Integration tests (`tests/story_smoke.rs`, `tests/story_fingerprints.rs`,
   `tests/snapshots.rs`).
3. Per-chunk mdBook chapters with embedded PNG / iframe HTML.

This is the project's **anti-regression gravity**: a renderable feature without
a story isn't shippable.

| Crate | Stories | Asset format |
|---|---|---|
| `wisp` | 12 (M0.6 → M0.19) | 256×256 PNG |
| `ui-storybook` | 18 (Button, Card, DopeSheet, DropZone, PlayerControls, RecordingToolbar, StatusBar, compositions) | Standalone HTML with stylesheet inlined |
| `playback` | 2 example outputs (`timed_playback`, `play_file`) | PNG sequences |

Regenerate any time:

```bash
just snapshots          # both storybooks
just snapshots-wisp     # wgpu stories only
just snapshots-ui       # Leptos UI stories only
```

---

## Stack

| Layer | Choice |
|---|---|
| Shell | Tauri 2 (multi-window) |
| UI | Leptos 0.7 (Rust → WASM) inside the Tauri webview |
| Renderer | `wisp` (this repo) — wgpu + WGSL, Pixi-shaped public API |
| Editor preview | native `winit` sibling window rendered by `wisp` |
| Decode | GStreamer 1.x (CLI-pipe today; `gstreamer-rs` Rust bindings later) |
| Capture | `objc2`/ScreenCaptureKit (macOS), `windows-rs` (Windows), `pipewire-rs` (Linux) |
| Encode | `ffmpeg-next` for MVP; `VideoToolbox` / `MediaFoundation` HW paths in v2 |

Locked 2026-05-09. Stack changes require an entry in `_docs/ISSUES.md`.

---

## Contributing

This codebase grew under one set of conventions; new contributors should read
in this order:

1. [`CLAUDE.md`](./CLAUDE.md) — top-level conventions + the anti-patterns list.
2. [`_docs/WORKFLOW.md`](./_docs/WORKFLOW.md) — the 11-step per-task contract.
3. [`_docs/TESTING.md`](./_docs/TESTING.md) — testing strategy + per-chunk
   minimums.
4. [`_docs/CONVENTIONS.md`](./_docs/CONVENTIONS.md) — code standards.
5. The current milestone doc (e.g. `_docs/milestone-0-renderer.md`).
6. [`_docs/ISSUES.md`](./_docs/ISSUES.md) — known bugs / deferrals.

> Anything that costs a recursive-fix iteration *and isn't already in
> CLAUDE.md* is a missing rehearsal note. Add it the same commit you fix
> the bug.

---

## License

MIT. Workspace `Cargo.toml` declares `license = "MIT"` for every crate.

---

## Acknowledgements

The renderer's public API shape is informed by [PixiJS](https://pixijs.com)
(decade-refined 2D scene-graph design). The recorder UX takes cues from
[Screen Studio](https://screen.studio) and the open-source
[OpenScreen](https://github.com/siddharthvaddem/openscreen) project.
