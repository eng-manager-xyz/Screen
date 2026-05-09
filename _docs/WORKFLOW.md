# Per-Task Workflow

The single loop you run for every task. No exceptions.

---

## 1. Pick the task

- Run `TaskList`
- Find the next `pending` task with no open `blockedBy` dependencies
- Tasks are M0.1 → M0.21 → M1.1 → ... → M4.3 in order; honor that
- Read the corresponding chunk in the milestone doc (`_docs/milestone-N-*.md`)
- **Note the "Done when:" line — that's your acceptance criteria. Do not exceed it.**

## 2. Mark in_progress

- `TaskUpdate` status to `in_progress`
- Only **one task** at `in_progress` at a time
- If something else is `in_progress`, finish or pause it first

## 3. Implement

Order:

1. **Plan the smallest unit** that satisfies "Done when". Often that's one new file plus minor edits.
2. **Write the test alongside (or first)** — see CONVENTIONS.md for which test type fits.
3. **Implement.**
4. **Stay inside the chunk.** If you spot adjacent work, file in `_docs/ISSUES.md` and continue. Do not expand scope.
5. **Don't refactor unrelated code.** If a refactor is genuinely needed, stop and ask.
6. **Add at least one test** matching the chunk's behavior. See `_docs/TESTING.md` "Per-chunk testing minimum" — unit / integration / snapshot / property / regression. Pure scaffolding chunks (module stubs, file moves) are exempt; everything else has a test.
7. **Add a storybook entry** if the chunk adds a renderable feature.
   - **wgpu side** (wisp): new file in `crates/wisp-storybook/src/stories/`,
     registered in `stories/mod.rs`, with a markdown write-up in
     `stories/writeups/`. Verify with `just storybook`.
   - **HTML side** (Leptos UI): new file in
     `crates/ui-storybook/src/components/`, registered in `components/mod.rs`
     with a `pub use`, and a story registered in `stories.rs`. Verify with
     `cargo test -p ui-storybook` (SSR snapshot regenerates; accept with
     `mv tests/snapshots/*.snap.new tests/snapshots/*.snap`).
   - Non-render features (math, capture, encode, file I/O) are exempt.

8. **Regenerate the asset.** `just snapshots` runs both headless exporters
   and writes the chunk's PNG/HTML to
   `_docs/book/src/assets/<crate>/<id>.{png,html}`. Commit the asset.

9. **Write the per-chunk mdBook chapter.** Path:
   `_docs/book/src/<crate>/chunks/<id>.md`. Template:
   - `# <Title> — M<n>.<m>`
   - One-paragraph what + why.
   - Embed the asset: `![](../../assets/<crate>/<id>.png)` for wisp, or
     `<iframe src="../../assets/ui/<id>.html" …>` for UI.
   - Recap the chunk's "Done when:" criteria.
   - Footer: `[<Type> API](../../api/<crate_name>/…)` link into rustdoc.

10. **Add the chapter to `SUMMARY.md`** under its milestone heading. mdBook
    will not see the file otherwise.

11. **Verify the chapter** with `just site` and visually open the resulting
    page at `target/book/<crate>/chunks/<id>.html`. The asset must render
    inline. An empty page = the gate failed.

## 4. Verify (`just gate` — recursive-fix loop)

**Hard rule:** before marking any task done, `just gate` must be green. If it's red, loop until it's green. There is no exit other than green.

```
loop:
    just gate
    if green: break
    diagnose, fix
    if approach fails: try a different approach
    if multiple approaches fail: file ISS-NN with everything tried, then try fresh
```

Never:
- Disable a failing test with `#[ignore]` (without ISS-NN + fix plan).
- `#[allow(clippy::*)]` without a documented `reason = "..."`.
- Comment out assertions.
- Bypass `cargo deny` / `cargo machete` findings (use exemptions in `deny.toml` / metadata, not silent skips).

```bash
just gate
```

`just gate` runs the full Tier 1 chain:
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo nextest run --workspace --all-features`
- `cargo test --workspace --doc`
- `cargo doc --workspace --no-deps --document-private-items` (catches missing-doc / broken-link warnings)

For milestone close, also run `just docs-strict` — it flips broken intra-doc
links and rustdoc warnings to errors via `RUSTDOCFLAGS="-D warnings"`.

If a chunk references a runnable example, also run it:

```bash
cargo run -p <crate> --example <name>
```

For visual examples: confirm by inspection. For headless examples: confirm output files exist and look right.

If any gate fails:
- Fix the underlying issue. Do not bypass.
- Don't `#[allow(...)]` clippy warnings without a documented reason.
- Don't disable tests to "fix later." Either pass them or file an ISS-NN and revert.
- See `_docs/QA.md` § "When the gate fails" for tool-specific guidance.

### Higher tiers (run periodically, not per-task)

- `just pr` — adds `cargo deny`, `cargo audit`, `cargo machete`, coverage. Run before pushing a PR (~3 min).
- `just release` — adds semver, msrv, bench, bloat, geiger. Run before tagging a release (~10+ min).
- `just full` — adds miri + mutants. Run when investigating UB or test quality (slow).

See `_docs/QA.md` for the full tier breakdown and tool-specific notes.

## 5. Document

- Append an entry to `_docs/PROGRESS.md` using the template at the bottom of that file. Newest entry at the top.
- If the chunk's "Done when" is fully satisfied, mark it ✅ in the milestone doc.
- If you partially completed the chunk and stopped: status `🚧 partial`, note what remains in the entry, do **not** mark the task done.

## 6. Mark done

- `TaskUpdate` status to `completed`
- Run `TaskList` to confirm the next task is unblocked

## 7. Commit (autonomous at natural boundaries)

- **Commit freely.** Local-only repo; commits are the time-machine. The user has delegated commit authority — don't ask.
- One chunk = one commit by default. Side quests are their own commits. Don't squash unless changes are genuinely inseparable.
- Conventional-commit format:
  - `feat(wisp): add Sprite API`
  - `fix(app): handle missing dropped path`
  - `test(wisp): snapshot blur filter`
  - `docs: update PROGRESS for M0.4`
  - `chore: bump wgpu to 24.x`
- Always include the Co-Authored-By trailer.
- Commit only after `just gate` is green for the chunk.

---

## Edge cases

### "I think the chunk is wrong"

Stop. File an issue in `_docs/ISSUES.md` with severity `question`. Tag the user. Do not silently reshape the chunk.

### "I need to add a dependency not listed in the milestone"

Check `_docs/CONVENTIONS.md` § Dependencies. If allowed, add it with a clear justification in the PROGRESS entry. If unsure: ask.

### "The test passes but the example looks broken"

The example is part of the gate. Don't mark done. File a bug if you can't immediately fix.

### "I want to skip a chunk"

Don't. The chunks are ordered for a reason — most have dependencies on prior work. If you genuinely need to reorder, ask the user.

### "I'm running out of context window mid-task"

1. `TaskUpdate` to keep status as `in_progress`
2. Append a `🚧 partial` entry to PROGRESS.md noting what's done and what remains
3. Note any half-edited files
4. Stop cleanly. The next session reads PROGRESS and resumes.

---

## Anti-patterns (do not do)

- Don't write a "fixme" comment and move on. Either fix it or file ISS-NN.
- Don't add a feature flag to keep buggy code "for later." Delete or fix.
- Don't mass-rename across files in the same task as adding a feature.
- Don't add error variants you don't handle. Each error variant has at least one caller.
- Don't expand a public API "while you're in there." Public API changes get their own task.
