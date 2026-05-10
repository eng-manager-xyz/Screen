# `wisp-storybook` — interactive feature gallery for `wisp`

A native eframe (egui) window with one panel per shipped wisp feature
(triangles, sprites, graphics, text, mesh, every filter). Drives the
visual regression tests + the per-chunk PNGs embedded in the mdBook
prose site.

## Run locally

```bash
# from: repo root (just recipes are defined in the workspace Justfile)
just storybook

# Or directly — from: repo root (or anywhere inside the workspace).
cargo run -p wisp-storybook --release
```

The `--release` build is recommended; debug runs are noticeably slower
on the filter stories.

## Export story PNGs (headless)

Used by `just snapshots` to regenerate the assets embedded in the mdBook
chapters. Writes one PNG per story under
`_docs/book/src/assets/wisp/<id>.png` (path resolved relative to the
workspace root, so the cwd matters).

```bash
# from: repo root
cargo run -p wisp-storybook --bin wisp-export-stories
# or, for both wisp + ui storybooks:
just snapshots
```

## Test locally

```bash
# from: repo root (or anywhere inside the workspace)

# Story smoke + fingerprint snapshot tests.
cargo nextest run -p wisp-storybook

# Doctests.
cargo test -p wisp-storybook --doc
```

The fingerprint suite renders each story at 256×256, buckets a 4×4
quadrant grid to multiples of 8, and `insta`-snapshots the result.
Robust to driver variation; fails on real visual changes.

## Notes

- **Linux software-Vulkan (lavapipe):** see the same caveat as `wisp` —
  set `WISP_SKIP_GPU_FILTER_TESTS=1` if you're running on lavapipe and
  hitting "Parent device is lost" on filter stories.
- **First-run snapshots:** `insta` writes `*.snap.new` and fails the
  test on a fresh baseline. Accept with `cargo insta accept`.
