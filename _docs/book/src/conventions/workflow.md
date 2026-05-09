# Per-task workflow

Full canonical version lives in `_docs/WORKFLOW.md`; the shape is:

1. **Pick** the next unblocked task.
2. **Read** the chunk's "Done when:" criteria in the milestone doc.
3. **Mark in_progress** (one task at a time).
4. **Implement** the smallest change that satisfies the contract.
5. **Test** — unit / snapshot / integration / property as appropriate.
6. **CHECK** — `just gate` must be green before close.
7. **UPDATE** — append to `PROGRESS.md`; file new issues in `ISSUES.md`.
8. **Mark completed** and confirm next task is unblocked.
9. **Commit** at natural boundaries (typically one chunk = one commit).

If the chunk is renderable, also:

- Add a story to the appropriate storybook (`wisp-storybook` or
  `ui-storybook`).
- Run `just snapshots` to regenerate the asset under
  `_docs/book/src/assets/<crate>/<id>.{png,html}`.
- Reference the asset from the chunk's mdBook page.

If the chunk introduces public API:

- Add `///` doc to every new public item (`missing_docs` enforces it).
- Update the crate's `//!` header if the architecture changed.
- At least one `# Examples` doctest on each new public function.
