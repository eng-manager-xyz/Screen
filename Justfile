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

# Tests via nextest. Excludes `app-e2e` because Tier-2 e2e tests require
# `tauri-driver` + `webkit2gtk-driver` that aren't on the default `gate`
# host. Run e2e separately via `just e2e`.
test:
    cargo nextest run --workspace --exclude app-e2e --all-features

# Doc tests (nextest doesn't run them).
doctest:
    cargo test --workspace --doc

# Generate docs (warn-level missing_docs allowed — used in `gate`).
docs:
    cargo doc --workspace --no-deps --document-private-items

# Strict docs — broken intra-doc links and rustdoc warnings become errors.
# Use this before milestone close; gate uses the lenient `docs` so backfill
# doesn't block unrelated chunks.
docs-strict:
    RUSTDOCFLAGS="-D warnings -D rustdoc::broken_intra_doc_links -D rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps --document-private-items

# Build the prose site (mdBook) and combine with rustdoc under target/book/.
# Output: target/book/index.html (prose) + target/book/api/<crate>/index.html (rustdoc).
#
# `--dest-dir` is resolved relative to the source dir, so we pass an absolute
# path rooted at the workspace.
# Build the in-repo mdBook preprocessor binary so `mdbook build` can find
# it on PATH via the recipes below. The first mdbook call after a clean
# would otherwise fail because the preprocessor isn't installed globally.
preprocessor-build:
    @cargo build -p mdbook-preprocessor-cross

# Build the `tools/doc-gates` binary used by `shared-check` /
# `snapshots-check`. Cheap incremental rebuild on warm cache; the
# cargo step short-circuits when nothing changed. Pure Rust + std +
# regex, no python / no external interpreter — same gate on macOS,
# Ubuntu, and Windows.
doc-gates-build:
    @cargo build -p doc-gates

site: docs preprocessor-build site-screen site-wisp site-wisp-chart site-wisp-animation
    @echo
    @echo "Open: file://$(pwd)/target/book/index.html  (screen project book)"
    @echo "      file://$(pwd)/target/book/wisp/index.html  (wisp library book)"
    @echo "      file://$(pwd)/target/book/wisp-chart/index.html  (wisp-chart book)"
    @echo "      file://$(pwd)/target/book/wisp-animation/index.html  (wisp-animation book)"

# Build the screen project book. Standalone recipe so CI can target a
# single book without rebuilding the other.
site-screen: preprocessor-build
    @mkdir -p target/book
    PATH="$(pwd)/target/debug:$PATH" mdbook build _docs/book --dest-dir "$(pwd)/target/book"
    @rm -rf target/book/api && cp -r target/doc target/book/api

# Build the wisp library book into `target/book/wisp/` so the two books
# compose into one site rooted at `target/book/`. Standalone target so
# the wisp-only CI gate can build it without touching the screen book.
site-wisp: preprocessor-build
    @mkdir -p target/book
    PATH="$(pwd)/target/debug:$PATH" mdbook build _docs/wisp-book --dest-dir "$(pwd)/target/book/wisp"

# Build the wisp-chart book into `target/book/wisp-chart/` (M-CHART.0 /
# AUT-180). Same preprocessor binary, third book mounted at
# `/Screen/wisp-chart/` on the deployed site.
site-wisp-chart: preprocessor-build
    @mkdir -p target/book
    PATH="$(pwd)/target/debug:$PATH" mdbook build _docs/wisp-chart-book --dest-dir "$(pwd)/target/book/wisp-chart"

# Build the wisp-animation book into `target/book/wisp-animation/`
# (M-ANIM.0 / AUT-227). Fourth book in the multi-book composition,
# mounted at `/Screen/wisp-animation/` on the deployed site. The
# preprocessor falls back to `target = "screen"` for now — the
# cross-link `Target` enum gains a dedicated `WispAnimation` variant
# only if/when this book starts inlining shared fragments.
site-wisp-animation: preprocessor-build
    @mkdir -p target/book
    PATH="$(pwd)/target/debug:$PATH" mdbook build _docs/wisp-animation-book --dest-dir "$(pwd)/target/book/wisp-animation"

# Regenerate per-feature screenshots / story HTML into _docs/book/src/assets/.
# Used by mdBook chapters; commit the output so docs build is reproducible.
snapshots: snapshots-wisp snapshots-ui

snapshots-wisp:
    cargo run -p wisp-storybook --bin wisp-export-stories
    cargo run -p wisp-storybook --bin wisp-export-text-screenshots

# Render animated stories to MP4 via gstreamer. Local-only —
# gstreamer must be installed (`brew install gstreamer`). Not chained
# into `just snapshots-wisp` because it depends on a non-Rust runtime
# tool; run explicitly when an animated story's tick changes.
snapshots-wisp-animated:
    cargo run -p wisp-storybook --bin wisp-export-animated

snapshots-ui:
    cargo run -p ui-storybook --bin ui-export-stories

# Render the media mdBook visual assets — waveform + histogram PNGs
# for the M-MEDIA.4/.5/.7/.8 chapters. Local-only (depends on
# gstreamer + the bundled MP3 fixture). CI consumes the committed
# PNGs. The companion video asset (media/video-capture.mp4) is
# regenerated by `just snapshots-media-video` since it spawns
# gst-launch directly.
snapshots-media:
    cargo run -p media --example render-mdbook-assets

snapshots-media-video:
    gst-launch-1.0 -q -e videotestsrc num-buffers=90 is-live=false ! \
      "video/x-raw,format=I420,width=320,height=240,framerate=30/1" ! \
      x264enc tune=zerolatency speed-preset=fast bitrate=1000 ! \
      mp4mux ! filesink location=_docs/book/src/assets/media/video-capture.mp4

# Snapshot completeness gate.
# Every mdBook chapter (in either book + the shared fragments)
# that references an asset under any `assets/` MUST have that asset
# committed. Re-running the storybook exporters across machines is
# non-deterministic (Metal vs lavapipe etc.), so we don't
# byte-compare. We DO verify that every referenced file exists —
# catches "added a chapter, forgot to commit the PNG." Run
# `just snapshots` locally before committing if you changed a
# story's rendered output.
#
# Implementation: `tools/doc-gates` Rust binary. Replaces the
# python heredoc that used to live here (DOCS-08) so the gate is
# pure Rust + just on macOS, Ubuntu, and Windows alike — no
# external interpreter required.
snapshots-check: doc-gates-build
    @target/debug/doc-gates snapshots-check

# Diagrams must be mermaid, not ASCII. Rejects any chapter under
# `_docs/book/src/` or `_docs/wisp-book/src/` containing box-drawing
# characters (┌ │ └ ├ ═ ╔ ╗) or the unicode arrow runs `─►` / `──▶`
# / `◄──`, outside of allowlisted files. The allowlist covers
# `orientation/stack.md` (directory-tree listing — mermaid is poor
# at file trees).
#
# Implementation: `tools/doc-gates` Rust binary. The previous
# `grep -P` implementation worked on macOS + Linux but false-
# matched em dashes / ellipses / curly quotes on Windows Git Bash
# because grep falls back to byte-level matching when the locale
# isn't UTF-8, and box-drawing chars share a leading UTF-8 byte
# with the entire `\xE2 \x__ \x__` range. Rust strings are
# char-level by construction — no locale dependency.
mermaid-check: doc-gates-build
    @target/debug/doc-gates mermaid-check

# Source-only drift gate for the two-book setup. Walks both books +
# shared for `\{\{shared X\}\}` tags and fails if `_docs/shared/X` is
# missing. Catches the common typo / missing-file case at source —
# no `mdbook` required, so `just gate` stays Rust-only and CI's
# gate-screen.yml doesn't need to install mdbook on every PR.
#
# The rendered-HTML belt-and-braces grep lives in `site-check`
# (depends on `site`, requires `mdbook` on PATH) and is invoked by
# `docs.yml` after both books are built. That keeps the
# concern-separation clean: `just gate` = Rust quality; `docs.yml`
# = site rendering + drift.
#
# Implementation: `tools/doc-gates` Rust binary. Replaces the
# python heredoc that used to live here (DOCS-08) so the gate is
# pure Rust + just on macOS, Ubuntu, and Windows alike — no
# external interpreter required.
shared-check: doc-gates-build
    @target/debug/doc-gates shared-check

# Anti-regression for the "silent .gitignore drop" failure mode.
# Some build-critical files (`crates/app/icons/icon.{png,ico}`) live
# in directories whose names can be matched by overly-broad
# .gitignore globs (the macOS template's `Icon?` pattern matched
# our real `icons/` dir on case-insensitive filesystems, eating
# `icon.ico` and making the Windows CI build fail opaquely deep
# inside `tauri-winres`). This step runs `git ls-files
# --error-unmatch` for each file in `REQUIRED_FILES` in
# `tools/doc-gates/src/main.rs` and fails fast with a clear message
# pointing at `.gitignore` + `git check-ignore -v <file>`.
#
# Add to the REQUIRED_FILES list (in `tools/doc-gates/src/main.rs`)
# whenever a new build-critical asset gets committed.
required-files-check: doc-gates-build
    @target/debug/doc-gates required-files-check

# Anti-regression for the wrong-case Pages URL trap.
# The repo is named `Screen` (capital S) so GitHub Pages serves at
# /Screen/. Every reference using lowercase `/screen/` would 404 in
# production. This step scans all .md + .toml files for the known
# wrong-case forms (see FORBIDDEN_PAGES_URL_PREFIXES in
# tools/doc-gates/src/main.rs) and fails fast with the exact line.
#
# Source of truth for the canonical URL: the docs.yml deploy job's
# "Evaluated environment url:" log line, which uses
# github.repository verbatim and is therefore case-exact.
pages-url-check: doc-gates-build
    @target/debug/doc-gates pages-url-check

# Full site-rendering drift gate. Builds both books, then greps
# rendered HTML for `mdbook-preprocessor-cross.*error` sentinels
# the source-level shared-check can't see (unreadable files,
# typo'd tag forms whose source variant matches but whose runtime
# variant doesn't). Requires `mdbook` + preprocessors on PATH.
# Invoked by `docs.yml` after both books are built; NOT invoked
# by `just gate` so the Rust gate stays mdbook-free.
#
# `target/book/api/` is excluded because rustdoc renders the
# preprocessor's own source (which contains the literal error
# template string) and the grep would self-match.
site-check: site shared-check
    #!/usr/bin/env bash
    set -euo pipefail
    if grep -rE 'mdbook-preprocessor-cross[^>]*error' target/book --exclude-dir=api 2>/dev/null; then
        echo "RUNTIME ERROR COMMENT IN RENDERED HTML — see above." >&2
        exit 1
    fi
    echo "site-check: both books rendered cleanly, no preprocessor error sentinels."

# Per-task gate. Run before marking any task done. Pure Rust —
# does NOT require mdbook (site rendering is gated by `just
# site-check` in docs.yml) and does NOT require python (text
# munging lives in `tools/doc-gates`). Runs identically on every
# supported CI runner (macOS, Ubuntu, Windows) and locally.
gate: fmt check lint test doctest docs snapshots-check mermaid-check shared-check required-files-check pages-url-check

# ─── Crate publishing (wisp → screen-wisp on crates.io) ───────────────────────

# Dry-run publish of the wisp crate (published as `screen-wisp` on
# crates.io because the bare name `wisp` is taken by an unrelated
# project). Verifies the crate is packageable without actually
# uploading. Run this locally before opening a release-plz Release
# PR if you want sanity ahead of the GHA run.
#
# `--allow-dirty` is OK for the dry-run since we're not actually
# uploading — real publishes happen via release-plz from clean git.
publish-wisp-dry:
    cargo publish -p screen-wisp --dry-run --allow-dirty

# List every file that would land in the .crate file uploaded to
# crates.io. Useful when reviewing the `include = [...]` list in
# `crates/wisp/Cargo.toml` — anything not in `include` is silently
# dropped, even if it's tracked in git.
publish-wisp-files:
    cargo package -p screen-wisp --list

# Build the wisp crate exactly the way crates.io will. Verifies the
# `[package].include` glob is right + the README + LICENSE land in
# the package + no unexpected files leak. Output:
# `target/package/screen-wisp-<version>.crate`.
publish-wisp-package:
    cargo package -p screen-wisp --allow-dirty

# ─── Remote-first UI dev loop (DEV-00..DEV-08 / AUT-145..AUT-153) ─────────────

# Local-only storybook dev loop. Watches ui-storybook src + assets,
# re-runs export-stories on change, browser auto-reloads via WebSocket.
# Visit http://127.0.0.1:3000/ to see the storybook index.
#
# For phone access over Tailscale, see `just dev-remote`.
dev:
    cargo run -p dev-server --release -- \
        --assets _docs/book/src/assets/ui \
        --watch crates/ui-storybook/src crates/ui-storybook/assets \
        --port 3000 \
        --host 127.0.0.1

# Remote-accessible storybook for phone preview. Starts `dev` in the
# background, exposes the local server via Tailscale Serve, prints the
# phone-reachable HTTPS URL. Requires Tailscale installed + signed in
# (see _docs/book/src/conventions/remote-dev.md for one-time setup).
#
# Stop with: `just dev-remote-stop`.
dev-remote:
    @echo "Booting dev-server in background (logs: /tmp/screen-dev-server.log)…"
    @nohup just dev > /tmp/screen-dev-server.log 2>&1 &
    @sleep 4
    @echo "Exposing via Tailscale Serve…"
    tailscale serve --bg http://127.0.0.1:3000
    @echo ""
    @echo "Phone URL:"
    @tailscale serve status | grep -Eo 'https://[^ ]+' | head -1 || echo "(check 'tailscale serve status' manually)"
    @echo ""
    @echo "Stop with:  just dev-remote-stop"

# Tear down the background dev-server + Tailscale Serve config opened by
# `just dev-remote`.
dev-remote-stop:
    @echo "Stopping Tailscale Serve…"
    -tailscale serve --https=443 off
    @echo "Killing background dev-server…"
    -pkill -f "target/release/dev-server" || true
    @echo "Done."

# ─── Local + remote book serving (DOCS-06 / AUT-160) ──────────────────────────
#
# `mdbook serve` has built-in live reload (filesystem watch + websocket
# auto-refresh in the browser), and runs the preprocessor on every
# rebuild, so changes to `{{shared}}` / `{{wisp-link}}` tags are picked
# up automatically. No dev-server crate involvement.
#
# Two books, two ports so you can run both at once. The `preprocessor-
# build` dependency rebuilds `mdbook-preprocessor-cross` first; mdbook's
# subsequent rebuilds skip it because `cargo build` short-circuits when
# nothing changed.

# Local screen project book. Visit http://127.0.0.1:3001/.
dev-book: preprocessor-build
    PATH="$(pwd)/target/debug:$PATH" mdbook serve _docs/book \
        --hostname 127.0.0.1 --port 3001 --open

# Local wisp library book. Visit http://127.0.0.1:3002/.
dev-wisp-book: preprocessor-build
    PATH="$(pwd)/target/debug:$PATH" mdbook serve _docs/wisp-book \
        --hostname 127.0.0.1 --port 3002 --open

# Local wisp-chart book. Visit http://127.0.0.1:3003/.
dev-wisp-chart-book: preprocessor-build
    PATH="$(pwd)/target/debug:$PATH" mdbook serve _docs/wisp-chart-book \
        --hostname 127.0.0.1 --port 3003 --open

# Local WebGPU demo for wisp-chart. Trunk serves the wasm bundle at
# http://127.0.0.1:8080 and rebuilds on file change. Requires
# `cargo install --locked trunk` once (Trunk is intentionally NOT a
# workspace dep — it's a build tool, not a runtime dep). Open in
# a WebGPU-capable browser (Chrome 113+ / Firefox 121+).
dev-wisp-chart-demo:
    @cd crates/wisp-chart-web && trunk serve

# Build the WebGPU demo for deployment under `/Screen/wisp-chart/demo/`.
# Output: target/wisp-chart-demo-dist/ (will be composed into the
# Pages artefact by docs.yml).
site-wisp-chart-demo:
    @cd crates/wisp-chart-web && trunk build --release --public-url /Screen/wisp-chart/demo/ --dist "$(pwd)/../../target/wisp-chart-demo-dist"

# Local full preview of the wisp-chart book including its
# `?chart=…&animate=…` iframe demos. Builds the book + the wasm
# demo, composes both under `target/book/wisp-chart/`, then serves
# the whole thing on http://127.0.0.1:3010/ so chapter iframes
# pointing at `../demo/?chart=…` resolve correctly (the same
# composition docs.yml does for the deployed site).
#
# No live-reload — re-run the recipe after edits. For live-reload
# during book authoring use `just dev-wisp-chart-book` (3003);
# during chart-fixture / animation work use `just dev-wisp-chart-demo`
# (8080); use `preview-wisp-chart` when you need the iframes to
# *also* work.
preview-wisp-chart: preprocessor-build
    @mkdir -p target/book
    PATH="$(pwd)/target/debug:$PATH" mdbook build _docs/wisp-chart-book --dest-dir "$(pwd)/target/book/wisp-chart"
    @cd crates/wisp-chart-web && trunk build --public-url /demo/ --dist "$(pwd)/../../target/book/wisp-chart/demo"
    @echo
    @echo "Book + demo composed at target/book/wisp-chart/."
    @echo "Open: http://127.0.0.1:3010/"
    @cd target/book/wisp-chart && python3 -m http.server 3010

# Publish all three books over Tailscale Serve (private to your tailnet).
# Run `just dev-book` and `just dev-wisp-book` in separate terminals
# first — this recipe only wires the Tailscale path proxies. After
# this, visit https://<MAC-NAME>.<TAILNET>.ts.net/ for the screen
# book and https://<MAC-NAME>.<TAILNET>.ts.net/wisp/ for the wisp
# book on any tailnet-enrolled device.
#
# Idempotent — re-running just refreshes the routes. Stop with
# `just dev-remote-book-stop`.
dev-remote-book:
    @echo "Registering Tailscale Serve proxies…"
    tailscale serve --bg --set-path / http://127.0.0.1:3001
    tailscale serve --bg --set-path /wisp http://127.0.0.1:3002
    tailscale serve --bg --set-path /wisp-chart http://127.0.0.1:3003
    @echo ""
    @echo "Routes:"
    @tailscale serve status || true
    @echo ""
    @echo "Stop with:  just dev-remote-book-stop"

# Tear down the book Tailscale Serve routes registered by
# `dev-remote-book`. Leaves the local `mdbook serve` processes alone
# (run them in foreground terminals you can Ctrl-C yourself).
dev-remote-book-stop:
    @echo "Removing Tailscale Serve routes…"
    -tailscale serve --https=443 off
    @echo "Done."

# Tier-2 e2e tests. Requires `tauri-driver` and (on Linux) `webkit2gtk-driver`
# + `xvfb`. Linux runs the suite under `xvfb-run` for headless display;
# macOS prints a clear skip message because `tauri-driver`'s WKWebView
# support is incomplete upstream (see _docs/book/src/app-ui/testing.md).
e2e:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
      Linux*)
        if ! command -v tauri-driver >/dev/null 2>&1; then
          echo "ERROR: tauri-driver not on PATH"
          echo "  install: cargo install --locked tauri-driver"
          exit 1
        fi
        if ! command -v xvfb-run >/dev/null 2>&1; then
          echo "ERROR: xvfb-run not on PATH"
          echo "  install: sudo apt-get install -y xvfb"
          exit 1
        fi
        echo "→ running Tier-2 e2e under xvfb-run …"
        xvfb-run --auto-servernum cargo nextest run -p app-e2e
        ;;
      Darwin*)
        echo "⚠ macOS Tier-2 e2e skipped — tauri-driver doesn't reliably"
        echo "  drive WKWebView. Use Linux CI for the gate; mac uses"
        echo "  manual smoke before tagging. Tier-1 (\`just gate\`) still"
        echo "  runs the IPC harness cross-platform."
        ;;
      *)
        echo "⚠ Tier-2 e2e: unrecognized platform $(uname -s); skipping."
        ;;
    esac

# Run the wisp-storybook GUI — one window with every shipped feature.
storybook:
    cargo run -p wisp-storybook --release

# Run the UI storybook (Leptos) in the browser via Trunk.
ui-storybook:
    cd crates/ui-storybook && trunk serve --no-default-features --features csr --open

# Build the recorder shell (Leptos CSR) — dev server with hot reload.
app-ui:
    cd crates/app-ui && trunk serve --open

# Production-build the recorder shell into `crates/app-ui/dist/`.
app-ui-build:
    cd crates/app-ui && trunk build --release

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
    cargo install --locked mdbook
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
