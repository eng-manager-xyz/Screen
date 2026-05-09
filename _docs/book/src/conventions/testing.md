# Testing

Three layers, all running under `just gate`:

## 1. Unit / property / snapshot

`cargo nextest run --workspace --all-features`. Lives next to the code it
exercises. Snapshots via `insta`.

## 2. Integration tests

In each crate's `tests/` directory. For storybooks specifically:

- `wisp-storybook/tests/story_smoke.rs` — every story renders without a
  wgpu validation error scope warning ("no console errors at runtime").
- `wisp-storybook/tests/story_fingerprints.rs` — quadrant-bucketed RGBA
  averages, locked to insta YAML. Regression gate for visual changes.
- `ui-storybook/tests/snapshots.rs` — SSR HTML for every story, locked to
  insta. Regression gate for class swaps, missing children, attribute drift.

## 3. Doctests

Every `# Examples` block in a `///` doc runs via `cargo test --doc`. Doctests
are the anti-rot mechanism for documentation — if the example stops compiling,
the gate fails.

## Recursive-fix loop

If `just gate` is red, loop until green. Never disable tests, never `#[allow]`
clippy without a `reason = "…"`, never bypass `cargo deny` / `cargo audit` /
`cargo machete`.
