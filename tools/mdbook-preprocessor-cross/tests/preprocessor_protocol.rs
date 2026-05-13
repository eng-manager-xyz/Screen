//! End-to-end test of the preprocessor binary against mdBook's
//! `[ctx, book]` JSON protocol.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Cargo guarantees integration tests in the same crate as a
/// `[[bin]]` see `CARGO_BIN_EXE_<name>` at compile time — the
/// absolute path to the freshly-built binary with the
/// platform-correct extension (`mdbook-preprocessor-cross` on Unix,
/// `mdbook-preprocessor-cross.exe` on Windows).
///
/// **Anti-regression:** earlier iterations called
/// `cargo build -p mdbook-preprocessor-cross` from inside each test
/// (`ensure_built()`) and hand-rolled `target/debug/<name>`. That
/// pattern raced under nextest's per-binary parallelism (multiple
/// tests blocked on the same cargo file lock), missed the `.exe`
/// suffix on Windows, and ignored any non-default `CARGO_TARGET_DIR`
/// nextest set. `CARGO_BIN_EXE_*` is platform-correct AND
/// guaranteed-built by cargo as a dep of integration-test execution.
/// See CLAUDE.md → "Build hygiene → integration tests that spawn a
/// sibling bin".
fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_mdbook-preprocessor-cross"))
}

#[test]
fn supports_html_renderer() {
    let status = Command::new(binary_path())
        .args(["supports", "html"])
        .status()
        .expect("spawn");
    assert!(status.success(), "supports html should exit 0");
}

#[test]
fn does_not_support_latex() {
    let status = Command::new(binary_path())
        .args(["supports", "latex"])
        .status()
        .expect("spawn");
    assert!(!status.success(), "supports latex should exit non-zero");
}

#[test]
fn rewrites_wisp_link_tags_in_book_payload() {
    let payload = serde_json::json!([
        {
            "root": "/tmp/fake-book-root",
            "config": {
                "preprocessor": {
                    "cross": {
                        "target": "wisp"
                    }
                }
            },
            "renderer": "html",
            "mdbook_version": "0.4.36"
        },
        {
            "sections": [
                {
                    "Chapter": {
                        "name": "Top",
                        "content": "see {{wisp-link chunks/blur}}",
                        "sub_items": []
                    }
                }
            ]
        }
    ]);
    let mut child = Command::new(binary_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("spawn");
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin
            .write_all(serde_json::to_string(&payload).unwrap().as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().expect("wait");
    assert!(output.status.success(), "preprocessor failed: {output:?}");
    let book: serde_json::Value = serde_json::from_slice(&output.stdout).expect("parse");
    let content = book["sections"][0]["Chapter"]["content"].as_str().unwrap();
    assert!(
        content.contains("./chunks/blur.html"),
        "wisp-link not rewritten: {content:?}"
    );
}
