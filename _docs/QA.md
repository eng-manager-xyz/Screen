# Quality Assurance

The full toolchain we run against this workspace. Every code drop runs the per-task gate; pull requests add the security tier; releases add the slow tier.

The canonical entry point is `just`. Run `just` (no args) to list all recipes.

---

## Convention: every code drop runs the gate

> **Hard rule:** before marking any task done, `just gate` must be green.
> If it's red, keep iterating until it's green. Don't ship red gates.

This is enforced by `_docs/WORKFLOW.md` § 4 ("Verify").

---

## Tiers

### Tier 1 — `just gate` (per-task, ~30s)

Runs before every task closure. The fast loop.

| Tool | Recipe | Purpose |
|---|---|---|
| `rustfmt` | `just fmt` | Format check (no edits) |
| `cargo check` | `just check` | Compiler correctness across workspace |
| `clippy` | `just lint` | Pedantic lints, treat warnings as errors |
| `cargo nextest` | `just test` | Run unit + integration tests (faster than `cargo test`) |
| `cargo test --doc` | `just doctest` | Doc tests (nextest skips these) |

`just gate` chains all five.

### Tier 2 — `just pr` (pre-push, ~3 min)

Adds supply-chain auditing and coverage. Run before pushing a PR.

| Tool | Recipe | Purpose |
|---|---|---|
| `cargo deny` | `just deny` | License, advisory, ban, source policy (covers RustSec) |
| `cargo machete` | `just unused-deps` | Find unused dependencies |

> `cargo audit` is available standalone (`just audit`) but is **not** in the `security` chain because `cargo deny check` already runs the same RustSec advisory check, and the two tools collide on `~/.cargo/advisory-db` (each wants to manage that dir). Use `just audit` only if you specifically want cargo-audit's report format.
| `cargo llvm-cov` | `just coverage` | LCOV coverage report |

`just pr` chains gate + security + coverage.

### Tier 3 — `just release` (pre-publish, ~10+ min)

Adds API stability + performance + safety audits. Run before tagging a release.

| Tool | Recipe | Purpose |
|---|---|---|
| `cargo semver-checks` | `just semver` | Detect breaking API changes |
| `cargo msrv` | `just msrv` | Find minimum supported Rust version |
| `cargo public-api` | `just public-api` | Snapshot public API surface |
| `cargo bench` | `just bench` | Run criterion benchmarks |
| `cargo bloat` | `just bloat` | Binary size analysis |
| `cargo geiger` | `just geiger` | Count `unsafe` across dep tree |

`just release` chains pr + semver + msrv + bench + bloat + geiger.

### Tier 4 — `just full` (slow, on demand)

Adds mutation + miri. Use when investigating test quality or hunting UB.

| Tool | Recipe | Purpose |
|---|---|---|
| `cargo +nightly miri` | `just miri` | Interpreter-based UB detection (pure-Rust modules only — wgpu/winit can't run under miri) |
| `cargo mutants` | `just mutants` | Mutation testing — flips operators, asserts tests fail |

`just full` chains release + miri + mutants.

### Optional / not in any tier

| Tool | Use when |
|---|---|
| `cargo flamegraph` | Profiling a hot path — `just flamegraph <example>` |
| `cargo +nightly udeps` | Stricter unused-deps than `machete` (nightly only) |
| `cargo vet` | Supply-chain audits when distributing the binary |
| `cargo supply-chain` | Visualize the dep tree's maintainer surface |
| `twiggy` | WASM binary size — N/A until we target WASM |

---

## Bootstrap on a fresh machine

```bash
just bootstrap
```

That installs the Tier 1 tools (`nextest`, `deny`, `audit`, `machete`) and prints install commands for the rest. Run them as needed; don't pre-install everything.

To install everything via cargo:

```bash
cargo install --locked \
  cargo-nextest cargo-deny cargo-audit cargo-machete \
  cargo-llvm-cov cargo-semver-checks cargo-public-api \
  cargo-msrv cargo-bloat cargo-geiger cargo-mutants \
  cargo-flamegraph
rustup component add miri --toolchain nightly
rustup component add llvm-tools-preview
```

---

## Configuration files

| File | Owns |
|---|---|
| `Justfile` (project root) | All tool recipes |
| `rustfmt.toml` | `cargo fmt` style — edition + `max_width` only, defaults otherwise |
| `deny.toml` | License allowlist, advisory policy, ban list, source policy |
| `Cargo.toml` workspace lints | `clippy::pedantic` + per-crate allows |

---

## When the gate fails

1. **`fmt`:** run `just fmt-fix` and re-run gate.
2. **`check`:** the code doesn't compile. Fix the build first; nothing else matters.
3. **`lint`:** clippy found something. Fix it (don't `#[allow]` without a documented reason — see `CONVENTIONS.md` § Clippy).
4. **`test`:** a test failed. Read the output, fix the test or the code (whichever is wrong).
5. **`doctest`:** a doc example doesn't compile or doesn't produce the asserted output. Fix the doc.
6. **`audit`:** a dep has a known advisory. Update the dep, or document the exemption in `deny.toml` with a reason.
7. **`deny`:** a license, source, or ban rule fired. Update `deny.toml` (with care — this is a security gate).
8. **`unused-deps`:** remove the unused dep from the offending `Cargo.toml`.

When in doubt, file an issue in `_docs/ISSUES.md` and ask before reshaping the gate.

---

## Tool-specific notes

### nextest

- Faster than `cargo test`, runs each test in a separate process (better isolation).
- Doc tests are NOT run by nextest — `just doctest` covers them.
- Configuration via `.config/nextest.toml` (not added yet; defaults are fine).

### miri

- Pure-Rust modules only. wgpu/winit code aborts under miri (no GPU access).
- The `just miri` recipe filters to `color`, `blend`, `math` test names. Adjust as new pure-Rust modules land.
- First run is slow (compiles miri-instrumented stdlib); subsequent runs are faster.

### cargo-deny

- License allowlist in `deny.toml` may need expansion as deps are added. The error message tells you which crate has which license; add it to `[licenses].allow` if it's compatible (MIT/Apache/BSD families typically are).
- Multi-version policy is `warn` (not `deny`) — wgpu drags in older versions of common crates and we tolerate that for now.

### cargo-audit

- Runs against the RustSec advisory database. False positives are rare.
- If a transitive dep has an unfixable advisory, document the exemption in `deny.toml` `[advisories].ignore` with a reason and a date to revisit.

### cargo-mutants

- Slow. Run when you want to know if your tests actually exercise the code paths you think they do.
- Don't include in PR gate — too slow.

### cargo-llvm-cov

- Requires `llvm-tools-preview` rustup component.
- LCOV output works with most IDE coverage gutters.
- HTML report is at `target/llvm-cov/html/index.html`.
