# Issues, Deferrals, and Open Questions

Append entries during development for: bugs, deferred features, technical debt, decisions to revisit, open questions.

**Format:** newest at top. Issue IDs `ISS-NN` are sequential, never reused.

---

## Template

Copy and fill when filing a new issue.

```
## ISS-NN: <short title>
- **Filed:** YYYY-MM-DD
- **By:** <task ID like M0.4 — or "user">
- **Severity:** bug | deferral | question | tech-debt
- **Affects:** <crate / file / feature area>
- **Status:** open | resolved | closed-wontfix
- **Description:**
  <what's the issue, what was expected, what was observed, why it matters>
- **Resolution:** (fill in when closing)
  <how it was fixed, or why it was closed without fixing, with date>
```

---

## ISS-03: `app-ui` rustdoc has an unresolved intra-doc link to `playback::Player`
- **Filed:** 2026-05-09
- **By:** M-PREVIEW.1 (spotted during `just site`)
- **Severity:** tech-debt
- **Affects:** `app-ui` (`crates/app-ui/src/lib.rs:16` — `//! [`playback::Player`]`)
- **Status:** open
- **Description:**
  The crate-level docstring references `[`playback::Player`]`, but `app-ui`
  doesn't depend on the `playback` crate so rustdoc can't resolve the path.
  `just gate` is lenient (warn-level intra-doc links), so this isn't blocking,
  but `just docs-strict` (used at milestone close) will fail until either:
  (a) `playback` is added as a dep + the link is fully qualified, or
  (b) the link is downgraded to plain text (`playback::Player` without the
  `[..]` braces). Likely (b) is correct since `app-ui` doesn't actually
  call into `playback` yet — that wiring lands in M-PLAY.2.
- **Resolution:** (fill in when closed)

---

## ISS-02: Tauri 2 Linux backend pulls gtk-rs unmaintained crates
- **Filed:** 2026-05-09
- **By:** M1.1 (Tauri foundation setup)
- **Severity:** tech-debt
- **Affects:** `screen-app` (transitive — Linux-only)
- **Status:** open (16 advisories exempted in `deny.toml`)
- **Description:**
  Tauri 2's Linux WebView backend depends on `gtk-rs` GTK3 bindings (atk, gdk, gtk, gio, etc.) that have been archived upstream. RustSec emits ~16 unmaintained advisories. None are exploits — all are "no longer actively maintained." macOS/Windows backends don't pull these.
- **Resolution:**
  Exempted in `deny.toml` `[advisories].ignore` with reason. Re-evaluate when Tauri migrates to GTK4 (tracked in tauri-apps issues).

---

## ISS-01: `paste` crate unmaintained (transitive via wgpu)
- **Filed:** 2026-05-09
- **By:** side quest (QA toolchain setup)
- **Severity:** tech-debt
- **Affects:** `wisp` (transitive dep tree) — `paste 1.0.15` reaches us through `metal → wgpu-hal → wgpu`.
- **Status:** open (exempted in `deny.toml`)
- **Description:**
  RustSec advisory `RUSTSEC-2024-0436` flags `paste` as unmaintained (not vulnerable). Author archived the repo. Suggested alternatives: `pastey` (drop-in fork) or `with_builtin_macros`. We can't fix this directly — wgpu's `metal` backend depends on it.
- **Resolution:**
  Exempted in `deny.toml` `[advisories].ignore` with documented reason. Re-evaluate when wgpu releases a version that drops the dep, or when `metal` migrates to `pastey`. Track at https://github.com/gfx-rs/wgpu/issues for migration progress.

---

