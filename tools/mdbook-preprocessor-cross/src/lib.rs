//! Cross-book mdBook preprocessor (DOCS-00 / AUT-154).
//!
//! Expands two new tag types so the screen + wisp books can share
//! content and link to each other without drift:
//!
//! - `{{shared path/to/fragment.md}}` — inlines a markdown fragment
//!   from the shared root (default `_docs/shared/`). The same source
//!   file lands in both books.
//! - `{{wisp-link path/to/chunk}}` — emits a URL that adapts to which
//!   book is rendering. In the wisp book it becomes a relative
//!   `./path/to/chunk.html`; in the screen book it becomes an absolute
//!   path under the wisp-book base (`/screen/wisp/path/to/chunk.html`
//!   by default).
//!
//! Configure via `book.toml`:
//!
//! ```toml
//! [preprocessor.cross]
//! command = "mdbook-preprocessor-cross"
//! target = "wisp"            # or "screen"
//! shared-root = "../shared"  # relative to the book's root
//! wisp-base = "/screen/wisp" # absolute path of the wisp book
//! ```
//!
//! The binary (`src/main.rs`) is a thin shim around
//! [`transform`] + the mdBook preprocessor protocol; everything
//! testable lives here in the library.

use std::path::Path;
use std::sync::OnceLock;

use regex::{Captures, Regex};

/// Which book is being rendered. Drives [`transform`]'s URL emission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The screen / app book. `{{wisp-link …}}` emits an absolute URL.
    Screen,
    /// The wisp library book. `{{wisp-link …}}` emits a relative URL.
    Wisp,
}

impl Target {
    /// Parse from the string value in `book.toml`.
    #[must_use]
    pub fn from_config(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "wisp" => Target::Wisp,
            _ => Target::Screen,
        }
    }
}

/// Runtime configuration for a single book's preprocessor pass.
#[derive(Clone, Debug)]
pub struct Config {
    /// Which book is rendering.
    pub target: Target,
    /// Absolute path to the shared-fragments directory (default
    /// `../shared` resolved against the book root).
    pub shared_root: std::path::PathBuf,
    /// Absolute URL prefix to the wisp book on the deployed site
    /// (e.g. `/screen/wisp`). Only used when [`Target`] is
    /// [`Target::Screen`].
    pub wisp_base: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target: Target::Screen,
            shared_root: std::path::PathBuf::from("_docs/shared"),
            wisp_base: "/screen/wisp".to_owned(),
        }
    }
}

/// Run the preprocessor on a single chapter's markdown content.
///
/// Performs up to [`MAX_PASSES`] passes so that a shared fragment
/// containing a `{{wisp-link …}}` tag is itself expanded. Reports the
/// number of passes used (useful for tests).
#[must_use]
pub fn transform(content: &str, config: &Config) -> String {
    let mut current = content.to_owned();
    for _ in 0..MAX_PASSES {
        let next = expand_once(&current, config);
        if next == current {
            return current;
        }
        current = next;
    }
    current
}

/// Maximum number of expansion passes before we bail out. Two are
/// almost always enough (one for the `{{shared}}`, one for any
/// `{{wisp-link}}` inside the shared fragment).
pub const MAX_PASSES: usize = 4;

fn expand_once(content: &str, config: &Config) -> String {
    let after_shared = SHARED_RE
        .get_or_init(|| Regex::new(r"\{\{\s*shared\s+([^}]+?)\s*\}\}").unwrap())
        .replace_all(content, |caps: &Captures| inline_shared(&caps[1], config))
        .into_owned();
    LINK_RE
        .get_or_init(|| Regex::new(r"\{\{\s*wisp-link\s+([^}]+?)\s*\}\}").unwrap())
        .replace_all(&after_shared, |caps: &Captures| render_wisp_link(&caps[1], config))
        .into_owned()
}

static SHARED_RE: OnceLock<Regex> = OnceLock::new();
static LINK_RE: OnceLock<Regex> = OnceLock::new();

fn inline_shared(rel_path: &str, config: &Config) -> String {
    if !is_safe_relative(rel_path) {
        return format!(
            "<!-- mdbook-preprocessor-cross: refusing unsafe shared path {rel_path:?} -->"
        );
    }
    let full = config.shared_root.join(rel_path);
    match std::fs::read_to_string(&full) {
        Ok(s) => s,
        Err(e) => format!(
            "<!-- mdbook-preprocessor-cross: shared({rel_path}) error: {e} -->"
        ),
    }
}

fn render_wisp_link(target_path: &str, config: &Config) -> String {
    let clean = target_path
        .trim_end_matches(".html")
        .trim_end_matches(".md")
        .trim_start_matches('/');
    match config.target {
        Target::Wisp => format!("./{clean}.html"),
        Target::Screen => format!("{base}/{clean}.html", base = config.wisp_base),
    }
}

/// Refuse `..` and absolute paths to keep shared inclusions inside
/// the configured root.
fn is_safe_relative(p: &str) -> bool {
    let p_path = Path::new(p);
    if p_path.is_absolute() {
        return false;
    }
    !p_path.components().any(|c| {
        matches!(
            c,
            std::path::Component::ParentDir | std::path::Component::Prefix(_)
        )
    })
}

/// Walk the mdBook preprocessor `book` JSON value, calling `apply`
/// on every chapter's `content` field. The book value is mutated in
/// place.
pub fn walk_book(book: &mut serde_json::Value, mut apply: impl FnMut(&str) -> String) {
    fn walk_sections(
        sections: &mut serde_json::Value,
        apply: &mut dyn FnMut(&str) -> String,
    ) {
        let Some(arr) = sections.as_array_mut() else {
            return;
        };
        for item in arr {
            let Some(chapter) = item.get_mut("Chapter") else {
                continue;
            };
            if let Some(content) = chapter.get_mut("content") {
                if let Some(s) = content.as_str() {
                    let new = apply(s);
                    *content = serde_json::Value::String(new);
                }
            }
            if let Some(sub) = chapter.get_mut("sub_items") {
                walk_sections(sub, apply);
            }
        }
    }
    if let Some(sections) = book.get_mut("sections") {
        walk_sections(sections, &mut apply);
    }
}

/// Build a [`Config`] from the mdBook preprocessor `ctx` JSON value
/// (the first element of the `[ctx, book]` payload).
///
/// Reads:
/// - `ctx.root` for the book's root directory.
/// - `ctx.config.preprocessor.cross.target` ("wisp" | "screen").
/// - `ctx.config.preprocessor.cross.shared-root` (path, relative to root).
/// - `ctx.config.preprocessor.cross.wisp-base` (URL).
#[must_use]
pub fn config_from_ctx(ctx: &serde_json::Value) -> Config {
    let root = ctx
        .pointer("/root")
        .and_then(serde_json::Value::as_str)
        .map_or_else(|| std::path::PathBuf::from("."), std::path::PathBuf::from);
    let target = ctx
        .pointer("/config/preprocessor/cross/target")
        .and_then(serde_json::Value::as_str)
        .map_or(Target::Screen, Target::from_config);
    let shared_root_rel = ctx
        .pointer("/config/preprocessor/cross/shared-root")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("../shared");
    let shared_root = root.join(shared_root_rel);
    let wisp_base = ctx
        .pointer("/config/preprocessor/cross/wisp-base")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("/screen/wisp")
        .to_owned();
    Config {
        target,
        shared_root,
        wisp_base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn screen_cfg(shared_root: std::path::PathBuf) -> Config {
        Config {
            target: Target::Screen,
            shared_root,
            wisp_base: "/screen/wisp".into(),
        }
    }

    fn wisp_cfg(shared_root: std::path::PathBuf) -> Config {
        Config {
            target: Target::Wisp,
            shared_root,
            wisp_base: "/screen/wisp".into(),
        }
    }

    #[test]
    fn target_parses_case_insensitively() {
        assert_eq!(Target::from_config("wisp"), Target::Wisp);
        assert_eq!(Target::from_config("WISP"), Target::Wisp);
        assert_eq!(Target::from_config("screen"), Target::Screen);
        assert_eq!(Target::from_config("anything-else"), Target::Screen);
    }

    #[test]
    fn wisp_link_in_screen_book_emits_absolute_url() {
        let cfg = screen_cfg(std::path::PathBuf::from("/tmp"));
        let out = transform("see {{wisp-link chunks/blur}} for details", &cfg);
        assert_eq!(out, "see /screen/wisp/chunks/blur.html for details");
    }

    #[test]
    fn wisp_link_in_wisp_book_emits_relative_url() {
        let cfg = wisp_cfg(std::path::PathBuf::from("/tmp"));
        let out = transform("see {{wisp-link chunks/blur}} for details", &cfg);
        assert_eq!(out, "see ./chunks/blur.html for details");
    }

    #[test]
    fn wisp_link_strips_md_and_html_suffixes() {
        let cfg = wisp_cfg(std::path::PathBuf::from("/tmp"));
        assert!(transform("{{wisp-link foo.md}}", &cfg).contains("./foo.html"));
        assert!(transform("{{wisp-link foo.html}}", &cfg).contains("./foo.html"));
    }

    #[test]
    fn shared_inlines_fragment_contents() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("fragment.md"), "shared body").unwrap();
        let cfg = screen_cfg(tmp.path().to_path_buf());
        let out = transform("intro {{shared fragment.md}} outro", &cfg);
        assert_eq!(out, "intro shared body outro");
    }

    #[test]
    fn shared_inside_a_fragment_expands_too() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("inner.md"),
            "see {{wisp-link foo}}",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("outer.md"),
            "outer: {{shared inner.md}}",
        )
        .unwrap();
        let cfg = wisp_cfg(tmp.path().to_path_buf());
        let out = transform("{{shared outer.md}}", &cfg);
        assert!(out.contains("./foo.html"), "got: {out:?}");
    }

    #[test]
    fn shared_missing_file_emits_html_comment_not_panic() {
        let tmp = TempDir::new().unwrap();
        let cfg = screen_cfg(tmp.path().to_path_buf());
        let out = transform("{{shared no-such.md}}", &cfg);
        assert!(out.contains("error"), "got: {out:?}");
        assert!(out.contains("no-such.md"));
    }

    #[test]
    fn shared_with_parent_dir_is_refused() {
        let tmp = TempDir::new().unwrap();
        let cfg = screen_cfg(tmp.path().to_path_buf());
        let out = transform("{{shared ../escape.md}}", &cfg);
        assert!(out.contains("unsafe"));
    }

    #[test]
    fn content_without_tags_is_unchanged() {
        let cfg = screen_cfg(std::path::PathBuf::from("/tmp"));
        let original = "# heading\n\nplain markdown {with} non-tag braces.";
        assert_eq!(transform(original, &cfg), original);
    }

    #[test]
    fn whitespace_around_tag_args_is_tolerated() {
        let cfg = wisp_cfg(std::path::PathBuf::from("/tmp"));
        assert!(transform("{{wisp-link   chunks/blur }}", &cfg).contains("./chunks/blur.html"));
    }

    #[test]
    fn walk_book_rewrites_nested_chapters() {
        let cfg = wisp_cfg(std::path::PathBuf::from("/tmp"));
        let mut book = serde_json::json!({
            "sections": [
                {
                    "Chapter": {
                        "name": "Top",
                        "content": "top {{wisp-link a}}",
                        "sub_items": [
                            {
                                "Chapter": {
                                    "name": "Sub",
                                    "content": "sub {{wisp-link b}}",
                                    "sub_items": []
                                }
                            }
                        ]
                    }
                },
                { "Separator": null }
            ]
        });
        walk_book(&mut book, |s| transform(s, &cfg));
        let top = &book["sections"][0]["Chapter"]["content"];
        let sub = &book["sections"][0]["Chapter"]["sub_items"][0]["Chapter"]["content"];
        assert!(top.as_str().unwrap().contains("./a.html"));
        assert!(sub.as_str().unwrap().contains("./b.html"));
    }

    #[test]
    fn config_from_ctx_reads_all_fields() {
        let ctx = serde_json::json!({
            "root": "/path/to/book",
            "config": {
                "preprocessor": {
                    "cross": {
                        "target": "wisp",
                        "shared-root": "../shared",
                        "wisp-base": "/different/wisp"
                    }
                }
            }
        });
        let cfg = config_from_ctx(&ctx);
        assert_eq!(cfg.target, Target::Wisp);
        assert_eq!(cfg.shared_root, std::path::PathBuf::from("/path/to/book/../shared"));
        assert_eq!(cfg.wisp_base, "/different/wisp");
    }
}
