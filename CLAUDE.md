# Project: `screen` — cinematic screen recorder

This file is auto-loaded into every Claude Code session. **Read it first** when picking up cold.

---

## ⚠️ NON-NEGOTIABLE: test → check → update → loop

**Every code drop ships with at least one test, runs the full QA suite, updates the durable docs, and recursively retries until green. No exceptions.**

After any non-trivial change — adding code, editing config, removing a dep, fixing a bug:

```
1. TEST:   add at least one test (see _docs/TESTING.md "anti-regression gravity")
           - unit / integration / snapshot / property / regression
           - chunks that don't fit any layer are scaffolding-only
2. CHECK:  `just gate`        →  loop recursively until green
3. UPDATE: PROGRESS.md         →  what changed, what was verified
           ISSUES.md           →  if you found a bug or deferral
           milestone doc       →  ✅ a chunk's "Done when:" if satisfied
4. STATUS: TaskUpdate          →  mark task completed only when gate is green
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
│  └─ wisp/                 # the renderer library (M0+)
└─ _docs/                   # all planning, research, ops docs
```
