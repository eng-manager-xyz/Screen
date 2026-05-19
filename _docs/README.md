# `_docs/` index

All project docs live here. Categories:

## Operational (read often)

| File | Purpose |
|---|---|
| `PROGRESS.md` | Append-only log of completed tasks. Newest at top. **Read first when resuming cold.** |
| `WORKFLOW.md` | The per-task workflow. Test, clippy, fmt, document, mark done. |
| `CONVENTIONS.md` | Code standards: naming, modules, error handling, testing strategy, WGSL conventions. |
| `ISSUES.md` | Bugs, deferrals, technical debt, open questions. Append-only. |

## Milestone plans (read when starting a milestone)

| File | Purpose |
|---|---|
| `milestone-0-renderer.md` | M0: build the `wisp` crate. 21 chunks. |
| `milestone-1-drop-zone-player.md` | M1: Tauri+Leptos drop-zone + HTML5 video player. 11 chunks. |
| `milestone-2-record-and-export.md` | M2 (M-RECORD-EXPORT): coordinated capture + multi-format encode + save to disk. 14 chunks. **Current milestone.** |

## Architecture & design (read when designing or onboarding)

| File | Purpose |
|---|---|
| `synthesis-and-stack.md` | Stack decision rationale, locked architecture, crate inventory, validation spike plan. |
| `recorder-features-and-render-api.md` | Recorder feature inventory + Pixi-shaped Rust API design for `wisp` + WGSL shader inventory. |

## Research (read when validating assumptions about competitors)

| File | Purpose |
|---|---|
| `screen-studio-research.md` | Comprehensive feature inventory of Screen Studio (the commercial reference). |
| `openscreen-research.md` | Deep-dive on siddharthvaddem/openscreen (the open-source reference). |

---

## When in doubt: which doc do I read?

- **"What's the next task?"** → `PROGRESS.md` (last entry) + TaskList
- **"How do I work on a task?"** → `WORKFLOW.md`
- **"What's the code standard for X?"** → `CONVENTIONS.md`
- **"Is this a known issue?"** → `ISSUES.md`
- **"What's the API for `wisp`?"** → `recorder-features-and-render-api.md` §3
- **"Why did we pick this stack?"** → `synthesis-and-stack.md` §4
- **"What's the recorder MVP?"** → `recorder-features-and-render-api.md` §7

## Doc growth policy

- Operational docs (`PROGRESS.md`, `ISSUES.md`) are append-only — never edit history except to mark resolution.
- Milestone docs are stable once published; updates go in PROGRESS / ISSUES.
- Architecture docs are amended (with date markers) when decisions change.
- Research docs are frozen — they reflect a point in time. Re-do research, don't edit.
