# Justfile — software quality assurance for the screen workspace.
# Run `just` to list recipes. See `_docs/QA.md` for what each does and when to run it.
#
# Tiered targets:
#   gate    — every code drop. Fast (~30s). Runs locally before marking a task done.
#   pr      — before pushing a PR. gate + supply-chain audit + coverage report.
#   release — before publishing. pr + semver + msrv + perf + advanced safety.
#   full    — literally everything (slow).

set shell := ["bash", "-uc"]

# Default: list recipes
default:
    @just --list --unsorted

# ─── Per-task gate (fast, runs before every task closure) ─────────────────────

# Format check (no edits). Use `fmt-fix` to auto-format.
fmt:
    cargo fmt --all --check

# Apply formatting in place.
fmt-fix:
    cargo fmt --all

# Compiler correctness — type-check the whole workspace.
check:
    cargo check --workspace --all-targets --all-features

# Clippy — pedantic lints, treat warnings as errors.
lint:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Tests via nextest.
test:
    cargo nextest run --workspace --all-features

# Doc tests (nextest doesn't run them).
doctest:
    cargo test --workspace --doc

# Generate docs — catches broken doc links.
docs:
    cargo doc --workspace --no-deps --document-private-items

# Per-task gate. Run before marking any task done.
gate: fmt check lint test doctest

# Run the wisp-storybook GUI — one window with every shipped feature.
storybook:
    cargo run -p wisp-storybook --release

# ─── Supply chain & dependency hygiene ────────────────────────────────────────

# RustSec advisory check.
audit:
    cargo audit

# License + advisory + bans + sources policy.
deny:
    cargo deny check

# Find unused dependencies.
unused-deps:
    cargo machete

# Optional/nightly: stricter unused-deps detection.
unused-deps-strict:
    cargo +nightly udeps --workspace --all-targets

# All supply chain & dep checks.
# `cargo deny check` covers RustSec advisories; cargo-audit is available
# standalone via `just audit` but redundant in the chain.
security: deny unused-deps

# ─── Coverage ──────────────────────────────────────────────────────────────────

# LCOV report (for IDE integration).
coverage:
    cargo llvm-cov nextest --workspace --all-features --lcov --output-path lcov.info

# HTML report (for humans).
coverage-html:
    cargo llvm-cov nextest --workspace --all-features --html
    @echo "Open target/llvm-cov/html/index.html"

# ─── API stability ─────────────────────────────────────────────────────────────

# Detect breaking API changes (run before releasing).
semver:
    cargo semver-checks check-release

# Snapshot the public API surface.
public-api:
    cargo public-api --workspace

# Find minimum supported Rust version.
msrv:
    cargo msrv find

# ─── Performance ──────────────────────────────────────────────────────────────

# Run benchmarks.
bench:
    cargo bench --workspace

# Binary size analysis (largest crates).
bloat:
    cargo bloat --release --crates

# Generate a flamegraph for an example.
flamegraph EXAMPLE:
    cargo flamegraph --example {{EXAMPLE}}

# ─── Safety ───────────────────────────────────────────────────────────────────

# Count unsafe blocks across the dep tree.
geiger:
    cargo geiger

# Pure-Rust modules under miri (interpreter-based UB detection).
# wgpu/winit code can't run under miri — only modules that don't touch GPU/IO.
miri:
    cargo +nightly miri test -p wisp --lib -- color blend math

# Mutation testing — flips operators/conditions, asserts tests still fail.
mutants:
    cargo mutants --workspace

# ─── Tier aggregates ─────────────────────────────────────────────────────────

# Per-PR check — run before pushing.
pr: gate security coverage

# Per-release check — run before publishing.
release: pr semver msrv bench bloat geiger

# Everything (slow — includes mutants, miri).
full: release miri mutants

# ─── Bootstrap ────────────────────────────────────────────────────────────────

# Install QA tools needed for the per-task gate (run once per machine).
bootstrap:
    @echo "Installing per-task gate tools…"
    cargo install --locked cargo-nextest
    cargo install --locked cargo-deny
    cargo install --locked cargo-audit
    cargo install --locked cargo-machete
    @echo
    @echo "Optional tools (install on demand):"
    @echo "  cargo install --locked cargo-llvm-cov   # coverage"
    @echo "  cargo install --locked cargo-semver-checks"
    @echo "  cargo install --locked cargo-public-api"
    @echo "  cargo install --locked cargo-msrv"
    @echo "  cargo install --locked cargo-bloat"
    @echo "  cargo install --locked cargo-geiger"
    @echo "  cargo install --locked cargo-mutants"
    @echo "  cargo install --locked cargo-flamegraph # also needs perf/dtrace"
    @echo "  rustup component add miri --toolchain nightly"
    @echo "  rustup component add llvm-tools-preview"
