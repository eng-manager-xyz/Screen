//! Stable-API contracts on the story registry (M-UI.0 / AUT-120):
//!
//! - Every shipped story id is unique.
//! - Every shipped story id is kebab-case (matches a strict regex).
//! - Every shipped story carries a non-empty category + title.
//!
//! These tests are deliberately separate from the SSR snapshot gate —
//! the snapshot tracks visual structure, this file tracks the registry
//! contract that downstream tooling (`ui-export-stories` filenames,
//! mdBook `<iframe src="../assets/ui/<id>.html">`, `just snapshots-check`)
//! all depend on.

use ui_storybook::stories::all_stories;

#[test]
fn every_story_id_is_unique() {
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
    for story in all_stories() {
        assert!(
            seen.insert(story.id),
            "duplicate story id `{}` — story ids must be unique because they're \
             also the on-disk asset filenames under `_docs/book/src/assets/ui/<id>.html`",
            story.id
        );
    }
}

#[test]
fn every_story_id_is_kebab_case() {
    // kebab-case: lowercase ASCII letters / digits / single hyphens. Must
    // start with a letter and not end with a hyphen. Empty disallowed.
    for story in all_stories() {
        let id = story.id;
        assert!(!id.is_empty(), "empty story id");
        assert!(
            id.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
            "story id `{id}` must start with a lowercase ASCII letter",
        );
        assert!(
            !id.ends_with('-'),
            "story id `{id}` must not end with a hyphen",
        );
        for c in id.chars() {
            assert!(
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-',
                "story id `{id}` contains invalid char `{c}` \
                 (only lowercase ASCII, digits, and `-` allowed)",
            );
        }
        assert!(
            !id.contains("--"),
            "story id `{id}` contains double hyphens",
        );
    }
}

#[test]
fn every_story_has_non_empty_metadata() {
    for story in all_stories() {
        assert!(
            !story.category.trim().is_empty(),
            "story `{}` has empty category — categories drive sidebar grouping",
            story.id
        );
        assert!(
            !story.title.trim().is_empty(),
            "story `{}` has empty title",
            story.id
        );
    }
}

#[test]
fn category_set_is_within_known_buckets() {
    // Soft contract — when a new category appears, it should also appear
    // here AND in the mdBook `components.md` index. Tightens the
    // discipline so we don't end up with a category typo splintering the
    // sidebar.
    let known: &[&str] = &[
        "Primitives",
        "Editor",
        "Player",
        "Recorder",
        "Shell",
        "App surfaces",
        "Compositions",
        "Menus",
        "Library",
        "Inspector",
        "Cursor",
    ];
    for story in all_stories() {
        assert!(
            known.contains(&story.category),
            "story `{}` uses category `{}` not in the known list — \
             add it to `category_set_is_within_known_buckets` AND to \
             `_docs/book/src/ui/components.md` when introducing it",
            story.id,
            story.category,
        );
    }
}

#[test]
fn fixture_smoke_every_builder_returns_non_empty() {
    use ui_storybook::fixtures::{cursor, devices, editor, library, recorder, workspaces};

    assert!(
        !devices::sample_microphones().is_empty(),
        "devices::microphones"
    );
    assert!(!devices::sample_cameras().is_empty(), "devices::cameras");
    assert!(
        !devices::sample_system_audio().is_empty(),
        "devices::system_audio"
    );
    assert!(!workspaces::sample_workspaces().is_empty(), "workspaces");
    assert!(
        !recorder::sample_overlay_options().is_empty(),
        "recorder::overlay_options"
    );
    assert!(
        !library::sample_recordings().is_empty(),
        "library::recordings"
    );
    assert!(
        !editor::sample_dope_sheet_tracks().is_empty(),
        "editor::dope_sheet_tracks"
    );
    assert!(
        !editor::sample_dope_sheet_dense().is_empty(),
        "editor::dope_sheet_dense"
    );
    assert!(
        !cursor::sample_cursor_styles().is_empty(),
        "cursor::cursor_styles"
    );
}
