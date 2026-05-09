# Code Conventions

Standards for the entire workspace. When unsure, prefer the convention used in adjacent recently-written code.

---

## Module organization

- **One primary type per file** when reasonable: `Sprite` in `sprite.rs`, `Container` in `container.rs`. Small helper types live alongside their primary.
- **Use the Rust 2018+ `parent.rs + parent/` pattern**, not `mod.rs`. A parent module like `texture` lives at `src/texture.rs` (which holds the module's primary type and declares submodules) plus `src/texture/video_texture.rs` etc. for variants. This avoids `clippy::module_inception` and keeps mixed type+declaration files idiomatic.
- The parent `.rs` file contains: module doc, submodule declarations (`pub mod x;`), and the module's primary type if there is one. No `mod.rs` files anywhere.
- Public API re-exported from crate root (`lib.rs`).
- Internal types stay `pub(crate)` until promoted.
- File length: aim for under 400 lines. Split when it grows.

## Naming

| Surface | Convention |
|---|---|
| Public types in `wisp` | Pixi-shaped where it maps: `Container`, `Sprite`, `Filter`, `Texture`, `RenderTexture`, `BlurFilter`, `MotionBlurFilter`, `DropShadowFilter` |
| Internals | Rust-idiomatic snake_case files, PascalCase types |
| WGSL files | `<purpose>.wgsl` matching the Rust struct: `filter_blur_h.wgsl` for `BlurFilter` horizontal pass |
| Methods returning `Self` | `with_*` for builder-style: `Sprite::new(...).with_anchor(...)` |
| Methods constructing | `from_*` for source-driven: `Texture::from_image(&app, &img)` |
| Methods modifying | `set_*` for setters: `sprite.set_tint(color)` |

## Error handling

- **Crate-level errors:** one `thiserror::Error` enum per crate, exported as `wisp::Error` / `app::Error`.
- **App boundary (`main.rs`, examples):** `anyhow::Result` for ergonomic propagation.
- **Library code:** never `.unwrap()`. `.expect("invariant: ...")` only when an invariant is provably upheld.
- **Tests/examples:** `.unwrap()` and `.expect(...)` are fine.
- Convert errors at crate boundaries with `From` impls. Don't pass opaque `dyn Error` through APIs.
- New error variants must have at least one caller. Don't pre-add variants.

## Testing strategy

| Code type | Test approach | Tooling |
|---|---|---|
| Pure logic (math, color, blend, transform composition) | Unit tests inline (`#[cfg(test)] mod tests`) | std `assert!` / `assert_eq!` |
| Scene graph behavior (children, traversal, dirty flags) | Integration tests in `crates/wisp/tests/` | std + `pretty_assertions` |
| Filter / shader output | Snapshot tests against reference PNG | `insta` + `image` for diff |
| Texture upload / video texture | Render-to-texture, read pixels, byte compare | std |
| Public API ergonomics | Doc tests on public items | rustdoc |
| Visual examples | Manual inspection only — no automated test |

**Snapshot test policy:** reference PNGs live in `crates/wisp/tests/snapshots/`. When a snapshot legitimately needs updating, regenerate with `INSTA_UPDATE=always cargo test`, then commit the new PNG with a justification in the commit message.

## Documentation

- **Every `pub` item** has a doc comment with at least a one-line summary.
- **Non-trivial public items** include a `# Examples` block.
- **Algorithms / non-obvious code** get a `// WHY:` comment, not `// WHAT:`.
- **WGSL files** have a header comment block:
  ```
  // <filter name>
  // <math reference, e.g., "Separable Gaussian, 9-tap, σ scaled by radius">
  // Bindings:
  //   group(0) binding(0): uniforms (BlurUniforms)
  //   group(1) binding(0): input texture
  //   group(1) binding(1): linear sampler
  ```
- Don't write multi-paragraph docstrings. One paragraph max for module/struct docs; one line for fields/methods.

## Clippy

- `clippy::pedantic` warnings on (already configured in workspace `Cargo.toml`).
- **Fix instead of `#[allow(...)]`** by default. If you must allow, add a `// reason: ...` comment.
- Workspace-wide allows live in `[workspace.lints.clippy]`. Per-item allows: `#[allow(clippy::lint_name, reason = "...")]`.

## QA toolchain

Every code drop runs `just gate` (fmt, check, lint, nextest, doctest). Higher tiers (`just pr`, `just release`, `just full`) add supply-chain auditing, coverage, semver, miri, mutation testing. See `_docs/QA.md` for the full breakdown.

Configuration files at the workspace root:
- `Justfile` — all tool recipes
- `rustfmt.toml` — `edition = "2024"`, `max_width = 100`, defaults otherwise
- `deny.toml` — license allowlist + advisory + ban + source policy
- `Cargo.toml` `[workspace.lints]` — clippy::pedantic + per-item allows

When `just gate` is red, fix it before doing anything else. Don't bypass with `#[allow]` or by skipping tests.

## Performance

- **Hot paths** (per-frame render, scene graph traversal): no allocations, no `Box<dyn Trait>`, minimize `Arc::clone`. Prefer `&` references and slices.
- **Cold paths** (init, load, config): readability > perf. `.collect()` to `Vec` is fine.
- **Hot path policy:** if a function is called per-frame, it's hot. Annotate with `#[inline]` if measured to help; don't sprinkle.
- Reach for `unsafe` only with: (a) measured perf justification, (b) safety comment block explaining invariants, (c) test coverage of edge cases.

## Dependencies

- New external dep: justify in the PROGRESS entry for that task. Favor:
  - Well-maintained (recent commits)
  - Widely depended-upon
  - Permissive license (MIT/Apache/BSD)
- **Don't add a dep for under ~50 lines of helper.** Inline it.
- **Pin major versions** in workspace `Cargo.toml` when adding crates with frequent breaking changes (`wgpu`, `winit`).
- Run `cargo deny check` before adding a dep at the workspace level.

## WGSL shader conventions

- **Group/binding layout** (consistent across all shaders):
  - `group(0)` — globals (viewport, time, resolution)
  - `group(1)` — per-draw uniforms (model matrix, color, filter params)
  - `group(2)` — textures (input texture(s) + sampler)
- **Entry points:** `main_vs` for vertex, `main_fs` for fragment.
- **Types:** prefer the short forms — `vec4f`, `vec3f`, `vec2f`, `f32`, `u32`, `i32`. Not `vec4<f32>`.
- **One file per pipeline.** Don't share entry points across files.
- Header comment block (see Documentation section above).

## Scene graph / API ergonomics

- **Public API stays stable across chunks within a milestone.** Breaking changes get their own task.
- **Composition over inheritance.** `Sprite { container: Container, ... }` not `Sprite extends Container`.
- **Builder methods return `Self`** for chaining: `Sprite::new(tex).with_anchor(...).with_tint(...)`.
- **Setters take `&mut self`** and return `&mut Self` for in-place mutation chains.
- **Avoid `Option<T>` parameters** in public API. Provide separate methods or sensible defaults.
- **Use `impl Into<T>`** for parameters where it reduces caller verbosity (e.g., `position: impl Into<Vec2>`).

## Backtracking avoidance

- **YAGNI.** Don't add features ahead of need. The chunk says what's needed.
- **Don't refactor unrelated code** in the same task as a feature add. If refactor is needed, stop and file an issue.
- **If a chunk's "Done when" feels wrong**: stop, file an `ISS-NN` with severity `question`, ask the user.
- **Public API changes** get their own task. Don't sneak them into other tasks.
- **One chunk = one commit** (default). Squashing only when chunks are genuinely inseparable.

## Logging & tracing

- Use `tracing` everywhere. Never `println!`/`eprintln!` in library code.
- Levels:
  - `error!` — unrecoverable, user-visible
  - `warn!` — recoverable, possibly bad input
  - `info!` — milestones (startup, shutdown, file loaded)
  - `debug!` — flow control, decisions
  - `trace!` — per-frame, per-pixel, very chatty
- **No `info!` in render hot paths.** Use `trace!`.
- Apps configure subscriber; libraries don't.

## Files & filesystem

- All paths use `std::path::Path` / `PathBuf`. **Never `String` for paths.**
- File I/O at boundaries only. Library functions take `&[u8]` / `&str` / readers, not paths.
- Examples can read paths directly.
