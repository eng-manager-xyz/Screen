//! Grep-based guardrail for the presentational contract (M-UI.23 /
//! AUT-143).
//!
//! Scans every file under `crates/ui-storybook/src/components/` and
//! rejects patterns that violate the "no state, no IPC, no timers"
//! rules documented in `_docs/book/src/ui/presentational-contract.md`.
//!
//! Story-only state lives in `stories/` (not scanned here). If a
//! component genuinely needs an exception (e.g. a future feature-gated
//! browser-side mount), add the file path to `ALLOWED_FILES`.

use std::fs;
use std::path::{Path, PathBuf};

/// Patterns that must not appear in any component file. The match is
/// a plain substring — wrap each in a way that catches common
/// idiomatic forms.
const FORBIDDEN: &[&str] = &[
    "RwSignal::new",
    "RwSignal<",
    "signal(",
    "create_signal",
    "Effect::new",
    "Effect::watch",
    "Action::new",
    "create_effect",
    "use_context::<",
    "provide_context",
    "tauri::",
    "wasm_bindgen::start",
    "set_interval",
    "set_timeout",
    "gloo_timers",
    "local_storage",
    "session_storage",
    "spawn_local",
    "std::fs::",
    "std::process::",
];

/// Files allowed to use otherwise-forbidden symbols. Empty today;
/// add a path here if and when a component genuinely needs an
/// exception (e.g. a future feature-gated browser-side mount).
const ALLOWED_FILES: &[&str] = &[];

#[test]
fn components_dir_has_no_app_state_or_ipc() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("components");
    assert!(root.exists(), "components dir missing: {}", root.display());
    let mut violations: Vec<String> = Vec::new();
    walk_rs(&root, &mut |path, contents| {
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        if ALLOWED_FILES.iter().any(|a| *a == rel) {
            return;
        }
        for needle in FORBIDDEN {
            if let Some(line_no) = find_line(contents, needle) {
                violations.push(format!(
                    "{rel}:{line_no} — forbidden pattern `{needle}` (see _docs/book/src/ui/presentational-contract.md)",
                ));
            }
        }
    });
    assert!(
        violations.is_empty(),
        "presentational-contract violations:\n  {}\n\nIf this is intentional, add the file path to ALLOWED_FILES and document why.",
        violations.join("\n  "),
    );
}

fn walk_rs<F: FnMut(&Path, &str)>(root: &Path, cb: &mut F) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs(&path, cb);
        } else if path.extension().is_some_and(|e| e == "rs")
            && let Ok(contents) = fs::read_to_string(&path)
        {
            cb(&path, &contents);
        }
    }
}

fn find_line(contents: &str, needle: &str) -> Option<usize> {
    for (i, line) in contents.lines().enumerate() {
        // Skip rustdoc + line comments; the contract permits naming
        // forbidden patterns in prose.
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }
        if line.contains(needle) {
            return Some(i + 1);
        }
    }
    None
}
