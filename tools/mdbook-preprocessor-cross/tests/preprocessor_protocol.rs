//! End-to-end test of the preprocessor binary against mdBook's
//! `[ctx, book]` JSON protocol.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn binary_path() -> PathBuf {
    let root = workspace_root();
    let target = std::env::var("CARGO_TARGET_DIR")
        .map_or_else(|_| root.join("target"), PathBuf::from);
    target.join("debug").join("mdbook-preprocessor-cross")
}

fn ensure_built() {
    let status = Command::new(env!("CARGO"))
        .args(["build", "-p", "mdbook-preprocessor-cross"])
        .current_dir(workspace_root())
        .status()
        .expect("cargo build");
    assert!(status.success(), "pre-build");
}

#[test]
fn supports_html_renderer() {
    ensure_built();
    let status = Command::new(binary_path())
        .args(["supports", "html"])
        .status()
        .expect("spawn");
    assert!(status.success(), "supports html should exit 0");
}

#[test]
fn does_not_support_latex() {
    ensure_built();
    let status = Command::new(binary_path())
        .args(["supports", "latex"])
        .status()
        .expect("spawn");
    assert!(!status.success(), "supports latex should exit non-zero");
}

#[test]
fn rewrites_wisp_link_tags_in_book_payload() {
    ensure_built();
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
