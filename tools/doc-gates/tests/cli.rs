//! Integration tests for the `doc-gates` binary.
//!
//! Builds the binary via `cargo build -p doc-gates` (handled by
//! cargo-nextest's binary-aware test harness) and runs it against
//! temp-dir fixtures. The CLI hard-codes the workspace's book
//! layout, so these tests cd into a fixture root and invoke
//! `doc-gates` with `cwd` set there.

use std::path::PathBuf;
use std::process::Command;

fn binary_path() -> PathBuf {
    let target = std::env::var("CARGO_TARGET_DIR")
        .map_or_else(|_| workspace_root().join("target"), PathBuf::from);
    target.join("debug").join("doc-gates")
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn write(p: impl AsRef<std::path::Path>, contents: &str) {
    let p = p.as_ref();
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, contents).unwrap();
}

#[test]
fn shared_check_succeeds_on_clean_fixture() {
    let tmp = tempfile::TempDir::new().unwrap();
    write(
        tmp.path().join("_docs/book/src/chap.md"),
        "before {{shared a.md}} after",
    );
    write(tmp.path().join("_docs/shared/a.md"), "x");
    std::fs::create_dir_all(tmp.path().join("_docs/wisp-book/src")).unwrap();

    let out = Command::new(binary_path())
        .arg("shared-check")
        .current_dir(tmp.path())
        .output()
        .expect("spawn doc-gates");
    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn shared_check_fails_on_missing_fragment() {
    let tmp = tempfile::TempDir::new().unwrap();
    write(
        tmp.path().join("_docs/book/src/chap.md"),
        "{{shared missing.md}}",
    );
    std::fs::create_dir_all(tmp.path().join("_docs/wisp-book/src")).unwrap();
    std::fs::create_dir_all(tmp.path().join("_docs/shared")).unwrap();

    let out = Command::new(binary_path())
        .arg("shared-check")
        .current_dir(tmp.path())
        .output()
        .expect("spawn doc-gates");
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("MISSING SHARED FRAGMENT"),
        "stderr={stderr}"
    );
    assert!(stderr.contains("missing.md"), "stderr={stderr}");
}

#[test]
fn snapshots_check_fails_on_missing_asset() {
    let tmp = tempfile::TempDir::new().unwrap();
    write(
        tmp.path().join("_docs/book/src/chap.md"),
        "![](assets/missing.png)",
    );
    std::fs::create_dir_all(tmp.path().join("_docs/wisp-book/src")).unwrap();
    std::fs::create_dir_all(tmp.path().join("_docs/shared")).unwrap();

    let out = Command::new(binary_path())
        .arg("snapshots-check")
        .current_dir(tmp.path())
        .output()
        .expect("spawn doc-gates");
    assert!(!out.status.success(), "expected failure");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("MISSING ASSET"), "stderr={stderr}");
}

#[test]
fn required_files_check_passes_for_tracked_files() {
    // Run from the workspace root so the binary's hard-coded
    // file list resolves against the real git repo.
    let out = Command::new(binary_path())
        .arg("required-files-check")
        .current_dir(workspace_root())
        .output()
        .expect("spawn doc-gates");
    assert!(
        out.status.success(),
        "exit={:?} stderr={}",
        out.status,
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("required-files-check: all"),
        "stdout={stdout}"
    );
}

#[test]
fn unknown_command_exits_with_usage() {
    let out = Command::new(binary_path())
        .arg("not-a-command")
        .output()
        .expect("spawn doc-gates");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown command"), "stderr={stderr}");
    assert!(stderr.contains("usage:"), "stderr={stderr}");
}
