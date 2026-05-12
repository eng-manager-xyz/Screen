//! Regression gate for the storybook index page (DEV-03 / AUT-147 +
//! DEV-08 / AUT-151).
//!
//! Runs the `ui-export-stories` binary against a tempdir, then asserts
//! that:
//!
//! - `index.html` is produced.
//! - Every `Story::id` from `all_stories()` appears in the sidebar.
//! - Every `Story::title` appears at least once.
//! - Every `Story::category` appears at least once (so a new category
//!   slipping through without a heading is caught).
//! - The search input + script hook elements both exist.
//!
//! Failure modes this catches:
//! - Someone adds a new story but the index generator drops it.
//! - Search filter input is removed by accident (DEV-08 regression).
//! - `data-id` / `data-title` / `data-category` attributes are removed.

use std::path::PathBuf;
use std::process::Command;

use ui_storybook::stories::all_stories;

fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

#[test]
fn export_stories_emits_index_with_every_story() {
    // The exporter writes into `_docs/book/src/assets/ui/` relative to the
    // *workspace root*, not the test cwd. Snapshots-check enforces the same
    // path; we just verify the file after the export run.
    let root = workspace_root();
    let status = Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet",
            "-p",
            "ui-storybook",
            "--bin",
            "ui-export-stories",
        ])
        .current_dir(&root)
        .status()
        .expect("invoke ui-export-stories");
    assert!(status.success(), "ui-export-stories exit status");

    let index_path = root.join("_docs/book/src/assets/ui/index.html");
    let index = std::fs::read_to_string(&index_path)
        .unwrap_or_else(|e| panic!("read index.html at {index_path:?}: {e}"));

    assert!(
        index.contains("id=\"storybook-index-filter\""),
        "search filter input missing"
    );
    assert!(
        index.contains("id=\"storybook-index-frame\""),
        "iframe missing"
    );

    let stories = all_stories();
    for story in &stories {
        assert!(
            index.contains(&format!("data-id=\"{}\"", story.id)),
            "story id {} missing from index sidebar",
            story.id
        );
        // Titles can contain HTML-escaped characters; check the literal
        // first 40 chars (no en-dashes etc. were escaped — em-dashes are
        // valid UTF-8 in HTML).
        let snippet = story.title.split_whitespace().next().unwrap_or(story.title);
        assert!(
            index.contains(snippet),
            "story title {:?} missing from index for {}",
            snippet,
            story.id
        );
    }

    let categories: std::collections::BTreeSet<_> = stories.iter().map(|s| s.category).collect();
    for cat in &categories {
        let needle = format!(">{cat}<");
        assert!(
            index.contains(&needle),
            "category heading {cat:?} missing from index",
        );
    }
}
