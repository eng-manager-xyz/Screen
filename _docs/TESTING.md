# Testing Strategy

> **The premise:** AI writes the next layer; the test suite owns the memory of what the system is supposed to be. Every chunk we ship leaves behind anti-regression gravity — at least one test (unit / integration / snapshot / property / regression) per meaningful behavior.

This doc adapts a comprehensive Rust testing recommendation to our specific project shape. We are **not** an AI-runtime product. We are a 2D GPU renderer library (`wisp`) and a Tauri desktop app (`screen-app`). Several pieces from the original recommendation don't fit and have been dropped. See `_docs/PROGRESS.md` for the side-quest entry that established this strategy.

---

## The recursive-fix loop (NON-NEGOTIABLE)

When `just gate` fails, you must loop until it's green. There is no exit other than green.

```
loop:
    run `just gate`
    if green: break
    diagnose
    fix
    if you can't fix this iteration:
        try a different approach
    if multiple approaches fail:
        file ISS-NN documenting what you tried
        try another approach
    NEVER:
        - bypass with #[allow] without documented reason
        - disable a test (no #[ignore] without an ISS-NN reference)
        - comment out an assert
        - skip a chunk because tests are inconvenient
```

**Failure is a loop iteration, not a stop condition.** This rule is enforced in `CLAUDE.md` and `_docs/WORKFLOW.md` § 4.

---

## Testing pyramid (our 5 layers)

| Layer | Tool | What it covers |
|---|---|---|
| 1. Unit | std `#[test]`, `rstest` | Pure Rust logic (color, blend, math, transform composition) — fast, deterministic, lives in-file |
| 2. Integration | std `#[test]` in `tests/` | Renderer behavior via `RenderTexture` + pixel readback — full wgpu device |
| 3. Snapshot | `insta` | Filter outputs (PNG diffs vs reference), project-file JSON, scene serialization — the anti-regression spine |
| 4. Property | `proptest` | Invariants across many inputs: Transform compose-decompose, Color round-trips, Rect operations |
| 5. Compile-fail | `trybuild` | Typestate / macros — when we have them (none yet) |

Layers we **don't** use:
- **CLI tests (`assert_cmd`)** — no CLI yet. Add when `screen render` headless mode lands.
- **HTTP mocks (`wiremock`)** — no network calls.
- **DB integration (`testcontainers`)** — no DB.
- **Fuzz (`cargo-fuzz`)** — no parsers handling untrusted input. Add when project files become user-shareable.
- **AI evals (Promptfoo / OpenAI Evals)** — AI is our writer, not our runtime.

---

## Per-tool brutally honest assessment

### MUST — locked in now

- **`cargo-nextest`** — installed. Default runner. Faster isolation, JUnit output, retries for flaky-test detection. `just test` uses it; `just doctest` is the doc-test exception nextest doesn't run.
- **`insta`** — added to `wisp` dev-deps. Used heavily from M0.16 (filter snapshots) onwards. Reference PNGs live in `crates/wisp/tests/snapshots/`. Update with `INSTA_UPDATE=always cargo nextest run` and commit the diff.
- **`rstest`** — added to `wisp` dev-deps. Use for table-driven tests where multiple input/output rows share the same assertion shape. Already useful for `Color::rgba_u8`, `Rect::contains`.
- **`proptest`** — added to `wisp` dev-deps. Use for: `Transform` composition associativity, `Color` round-trip equivalence, `Rect::contains` boundary properties.
- **`cargo-llvm-cov`** — install on demand (`cargo install --locked cargo-llvm-cov` + `rustup component add llvm-tools-preview`). Recipe: `just coverage` (LCOV) and `just coverage-html` (browse).
- **The recursive-fix loop** — locked in CLAUDE.md hard rules.

### SHOULD — install/use when relevant code lands

- **`assert_fs`** — when project file I/O lands (M0.11 RenderTexture readback PNGs; M0.21 headless export PNG dumps). Provides isolated tempdirs.
- **`trybuild`** — when we ship a typestate API. Filter chain composition might warrant it; revisit at M0.16.
- **`cargo-mutants`** — recipe exists (`just mutants`). Run nightly once we have ≥50 tests to mutate against.
- **`miri`** — recipe exists (`just miri`). Pure-Rust modules only.
- **`cargo-fuzz`** — when project file deserializer accepts files from other users. Not on the immediate horizon.

### DOESN'T FIT (skip entirely or until product shape changes)

- **`assert_cmd`** — no CLI. Possibly fits when `screen render` headless mode lands.
- **`wiremock`** — no HTTP services in the runtime.
- **`testcontainers`** — no databases.
- **AI eval frameworks** — wrong product domain.
- **`predicates`** — bundled with `assert_cmd`; not standalone-useful.
- **80% coverage threshold (now)** — premature. We have 15 tests over 3 modules. Use **ratcheting** later: never decrease coverage. The hard threshold goes in once we cross ~70% naturally.

---

## Per-chunk testing minimum (anti-regression gravity)

For every chunk in the milestone docs, at least one of these must land alongside the implementation:

| If the chunk adds … | Minimum test contribution |
|---|---|
| Pure-Rust logic (math, color, blend, transform) | Unit tests in same file. Use `rstest` if there are ≥3 cases sharing a shape. |
| A scene-graph node (Sprite, Container, Graphics, Text) | Integration test in `tests/` rendering the node to a `RenderTexture` + pixel assertion (1+ pixel sample sufficient at M0; insta snapshot of the full RT in M0.16+). |
| A filter (Blur, DropShadow, MotionBlur, ColorMatrix) | `insta` snapshot test against a reference PNG; a synthetic input scene; tolerance defined per filter. |
| A serializer / file format | `insta` JSON/YAML snapshot + `proptest` round-trip test (parse → serialize → parse). |
| A texture path (image, video, render) | Integration test uploading known bytes, reading them back, comparing. |
| A bug fix | A regression test named `regression_iss_NN_<short_desc>`. The test should fail before the fix and pass after. |
| Pure scaffolding (M0.2 module stubs) | No test required. Examples and downstream chunks provide the coverage. |

If a chunk doesn't fit any of the above, it's probably too small (merge into the next chunk) or it's pure refactor (don't refactor in the same task as a feature — see CONVENTIONS).

---

## Examples-as-tests

For chunks that ship runnable examples (M0.5 `hello_triangle`, M0.6 `hello_quad`, etc.), the example is part of the test contract:

- The example **must** build clean as part of `just gate` (it does — `clippy --all-targets` includes examples).
- The example **must** run without panicking on a working host. If the chunk references a visible example and a snapshot or pixel-readback test isn't feasible, the example serves as the integration test, with the user's manual run logged in PROGRESS as "visual confirmation pending user run."
- For headless examples (M0.21 `headless_export`), the example **is** a runnable test — `cargo run -p screen-wisp --example headless_export` should produce expected output files. Add an `assert_fs` test that runs it programmatically once the pattern is set.

---

## Folder layout (target)

```
crates/wisp/
├─ src/
│  └─ <code with #[cfg(test)] mod tests>      # Layer 1 (unit)
├─ tests/                                       # Layer 2 (integration)
│  ├─ render_quad.rs
│  ├─ render_filter_chain.rs
│  └─ snapshots/                                # Layer 3 (insta references)
│     ├─ blur_radius_8.png
│     ├─ drop_shadow_offset_8_8.png
│     └─ project_format_v1.snap.json
├─ examples/
│  ├─ hello_triangle.rs
│  ├─ hello_quad.rs
│  └─ recorder_mock.rs
└─ benches/                                     # criterion (M0.20+)
   └─ render_throughput.rs
```

Property tests (Layer 4) live alongside the code they test, in `#[cfg(test)] mod tests` blocks. `proptest!` macro doesn't need a separate folder.

Compile-fail tests (Layer 5) live in `crates/wisp/tests/compile_fail/` when we have typestate APIs. Not yet.

---

## Coverage policy

- **Now:** measure with `just coverage`, but no threshold gate. Most coverage will come from M0.16+ (filters with snapshot tests).
- **Mid-M0 (~chunk 12):** introduce a soft threshold around the actual achieved coverage (probably 60-70%). Document in PROGRESS when it's set.
- **End of M0:** consider hard `--fail-under-lines` threshold for `just pr`. Target 80% for pure-Rust modules; renderer code may sit lower because GPU paths are integration-tested.
- **Examples and `main.rs` are excluded from coverage** — they're integration drivers, not testable units.

Configure exclusions in `Cargo.toml` `[package.metadata.llvm-cov]` or via `.config/llvm-cov.toml` when ratcheting starts.

---

## When tests fail

This is the recursive-fix loop in operational form:

1. **Read the failure carefully.** What was expected, what was observed, where in the code.
2. **Reproduce minimally.** Can you isolate the failing case? `cargo nextest run -p screen-wisp --filter <test_name>`.
3. **Diagnose.** Is the test wrong, the code wrong, or the spec wrong?
4. **Fix.**
   - Code wrong → fix the code.
   - Test wrong → fix the test (and explain why in commit message / PROGRESS).
   - Spec wrong → file ISS-NN, propose the spec change, get user agreement, then fix.
5. **Re-run `just gate`.**
6. **If still red:** go to step 2. Try a different approach. Read more carefully.
7. **If you've genuinely exhausted approaches:** file ISS-NN with everything tried. Continue trying with a fresh angle.

What you must **never** do:
- `#[allow(clippy::*)]` to bypass a clippy failure (without `reason = "..."` and a documented justification).
- `#[ignore]` a failing test (without an ISS-NN reference and a fix plan).
- Comment out an assertion to make it pass.
- `cargo machete --skip` an unused dep that should be removed.
- Skip a snapshot review by blindly accepting `INSTA_UPDATE=always` output without checking the diff.

The contract is simple: **green gate → can mark task done; red gate → not done**. There is no third state.

---

## Tooling install summary

Already installed on this machine:
- `cargo-nextest`, `cargo-deny`, `cargo-machete` (Tier 1 in `_docs/QA.md`)

To install for testing strategy:
```bash
cargo install --locked cargo-llvm-cov
rustup component add llvm-tools-preview
```

Dev-dependencies on `wisp`:
- `rstest` — table-driven tests
- `insta` — snapshot tests (with `json`, `yaml`, `redactions` features)
- `proptest` — property tests

Dev-dependencies on `wisp` later:
- `assert_fs` — when project file I/O lands
- `trybuild` — when we have typestate APIs

`cargo-mutants` (`just mutants`), `miri` (`just miri`), `cargo-fuzz`, etc. are post-MVP / nightly tools — install on demand from `just bootstrap` output.
