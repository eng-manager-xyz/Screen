//! CI + local gates for the two-book mdBook setup.
//!
//! Two checks, both pure Rust + regex + stdlib (no Python, no
//! external CLIs). Replaces the python heredocs that used to live
//! in the Justfile (DOCS-08 / AUT-162):
//!
//! - [`check_shared`] — walks the book trees for `{{shared X}}`
//!   tags and reports any where `_docs/shared/X` is missing.
//! - [`check_snapshots`] — walks the same trees for `![](path)`
//!   and `src="path"` references whose URL contains `/assets/`,
//!   resolves each path relative to the chapter's directory, and
//!   reports any missing.
//!
//! The binary (`src/main.rs`) is a thin shim around these two
//! functions; everything testable lives here in the library.

use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// One missing / broken reference found by [`check_shared`] or
/// [`check_snapshots`]. The fields are pre-rendered for printing.
#[derive(Debug, PartialEq, Eq)]
pub struct Issue {
    /// Chapter (`.md` file) that contained the bad reference.
    pub chapter: PathBuf,
    /// The raw reference text (e.g. `note.md` or `../assets/foo.png`).
    pub reference: String,
    /// Why it failed (e.g. "expected at /abs/path/note.md").
    pub note: String,
}

impl Issue {
    /// One-line report with the given prefix, e.g. `MISSING ASSET:
    /// chap.md → ../foo.png (resolved /abs/foo.png)`.
    #[must_use]
    pub fn report(&self, prefix: &str) -> String {
        format!(
            "{prefix}: {chapter} → {reference} ({note})",
            chapter = self.chapter.display(),
            reference = self.reference,
            note = self.note,
        )
    }
}

fn shared_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\{\{\s*shared\s+([^}]+?)\s*\}\}").unwrap())
}

fn asset_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    // Three capture groups: `![alt](path)` markdown, `src="path"`
    // HTML attribute, `url('path')` / `url("path")` CSS value
    // (typically inline `style="background: url(...)"`).
    R.get_or_init(|| {
        Regex::new(r#"!\[[^\]]*\]\(([^)]+)\)|src="([^"]+)"|url\(['"]?([^'")]+)['"]?\)"#).unwrap()
    })
}

/// Walk `book_roots` and assert every `{{shared X}}` reference
/// has a matching file under `shared_root`. Returns an empty
/// vector when all references resolve. References inside fenced
/// code blocks (```) are skipped — they're documentation showing
/// the tag syntax, not real cross-references.
#[must_use]
pub fn check_shared(book_roots: &[&Path], shared_root: &Path) -> Vec<Issue> {
    let mut issues = Vec::new();
    for root in book_roots {
        for md in walk_md(root) {
            let Ok(text) = std::fs::read_to_string(&md) else {
                continue;
            };
            for_each_prose_line(&text, |line| {
                for cap in shared_re().captures_iter(line) {
                    let raw = cap.get(1).unwrap().as_str().trim();
                    let path = shared_root.join(raw);
                    if !path.is_file() {
                        issues.push(Issue {
                            chapter: md.clone(),
                            reference: raw.to_owned(),
                            note: format!("expected at {}", path.display()),
                        });
                    }
                }
            });
        }
    }
    issues
}

/// Walk `book_roots` and assert every `![](path)` / `src="path"`
/// reference whose URL contains an `assets/` path segment resolves
/// to an existing file (paths are taken relative to the chapter's
/// directory). `http://` and `https://` references are skipped, as
/// are matches inside fenced code blocks (```) since those are
/// usually mdBook syntax examples rather than real embeds.
#[must_use]
pub fn check_snapshots(book_roots: &[&Path]) -> Vec<Issue> {
    let mut issues = Vec::new();
    for root in book_roots {
        for md in walk_md(root) {
            let Ok(text) = std::fs::read_to_string(&md) else {
                continue;
            };
            let chapter_dir = md.parent().unwrap_or(Path::new("."));
            for_each_prose_line(&text, |line| {
                for cap in asset_re().captures_iter(line) {
                    let r = cap
                        .get(1)
                        .or_else(|| cap.get(2))
                        .or_else(|| cap.get(3))
                        .unwrap()
                        .as_str();
                    if r.starts_with("http://") || r.starts_with("https://") {
                        continue;
                    }
                    // Match `assets/` as a path segment — either at
                    // the start of the URL or following a `/`. The
                    // legacy python check used `'/assets/' in ref`
                    // which missed bare `assets/foo.png` references.
                    if !(r.starts_with("assets/") || r.contains("/assets/")) {
                        continue;
                    }
                    let resolved = normalize(&chapter_dir.join(r));
                    if !resolved.exists() {
                        issues.push(Issue {
                            chapter: md.clone(),
                            reference: r.to_owned(),
                            note: format!("resolved {}", resolved.display()),
                        });
                    }
                }
            });
        }
    }
    issues
}

/// Iterate over each line of `text` that is **prose** — i.e. not
/// inside a fenced code block (a line that starts with `\`\`\``
/// toggles the fence state). Matches inside fences are
/// documentation, not real references, and should not be
/// validated.
fn for_each_prose_line(text: &str, mut f: impl FnMut(&str)) {
    let mut in_fence = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        f(line);
    }
}

/// Recursively collect every `.md` file under `root`. Missing
/// directories yield an empty iterator (no panic) so callers can
/// pass several book roots and have the absent ones silently
/// contribute nothing.
#[must_use]
pub fn walk_md(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|e| e == "md") {
                out.push(p);
            }
        }
    }
    out
}

/// Run `git ls-files --error-unmatch -- <path>` and return whether
/// the file is tracked. A non-success exit means git considers the
/// path either ignored, untracked, or non-existent. Used by
/// [`check_required_files`] to surface the silent failure mode where
/// a real source file is in the working tree but not in the git
/// index — usually because a `.gitignore` pattern overreaches and
/// `git add` silently dropped it.
fn is_tracked_by_git(path: &str) -> bool {
    let Ok(output) = Command::new("git")
        .args(["ls-files", "--error-unmatch", "--"])
        .arg(path)
        .output()
    else {
        return false;
    };
    output.status.success()
}

/// One wrong-case Pages URL found by [`check_pages_urls`]. The
/// path + line number locate the violation; the matched substring
/// is the exact wrong-case prefix.
#[derive(Debug, PartialEq, Eq)]
pub struct PagesUrlViolation {
    /// File (`.md` or `.toml`) that contained the wrong-case URL.
    pub file: PathBuf,
    /// 1-based line number.
    pub line_number: usize,
    /// The offending line text (trimmed).
    pub line: String,
    /// The wrong-case prefix that triggered the match.
    pub matched: String,
}

/// Walk `roots` recursively (any depth, any file type matching
/// `extensions`) and report every line that contains one of the
/// `forbidden` substrings. Used to catch wrong-case GitHub Pages
/// URLs before they ship as 404s.
///
/// The root cause: GitHub Pages serves at `<user>.github.io/<repo>/`
/// using the repo name's *exact* case. A repo named `Screen` is
/// served at `/Screen/`; references to `/screen/` 404. There's no
/// case-insensitive fallback.
///
/// Source of truth for the canonical URL: the deploy workflow's
/// `Evaluated environment url:` log line, which uses
/// `github.repository` verbatim. Keep `forbidden` in sync with what
/// has appeared in past drift; never add the *correct* form to
/// `forbidden` by mistake.
#[must_use]
pub fn check_pages_urls(
    roots: &[&Path],
    extensions: &[&str],
    forbidden: &[&str],
) -> Vec<PagesUrlViolation> {
    let mut hits = Vec::new();
    for root in roots {
        for path in walk_files_with_extensions(root, extensions) {
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            for (i, line) in text.lines().enumerate() {
                for f in forbidden {
                    if line.contains(f) {
                        hits.push(PagesUrlViolation {
                            file: path.clone(),
                            line_number: i + 1,
                            line: line.trim().to_owned(),
                            matched: (*f).to_owned(),
                        });
                        break;
                    }
                }
            }
        }
    }
    hits
}

/// Recursively collect every file under `root` whose extension is
/// in `extensions`. Internal helper for [`check_pages_urls`] —
/// generalises [`walk_md`] to multiple extension filters.
fn walk_files_with_extensions(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                // Skip `target/` and `.git/` to avoid scanning build
                // output / repo metadata.
                if let Some(name) = p.file_name().and_then(|n| n.to_str())
                    && (name == "target" || name == ".git" || name == "node_modules")
                {
                    continue;
                }
                stack.push(p);
            } else if let Some(ext) = p.extension().and_then(|e| e.to_str())
                && extensions.contains(&ext)
            {
                out.push(p);
            }
        }
    }
    out
}

/// One ASCII-diagram violation found by [`check_mermaid`]:
/// a chapter whose prose contains box-drawing characters or
/// the arrow runs reserved for mermaid diagrams.
#[derive(Debug, PartialEq, Eq)]
pub struct AsciiDiagramMatch {
    /// Chapter (`.md` file) that contained the violation.
    pub chapter: PathBuf,
    /// 1-based line number of the first offending line.
    pub line_number: usize,
    /// The offending line text (trimmed).
    pub line: String,
}

/// Walk `book_roots` for chapters that use ASCII art for diagrams
/// where mermaid is required. Reports every line with a
/// box-drawing character (`┌ │ └ ├ ═ ╔ ╗`) or one of the arrow
/// sequences (`─►`, `──▶`, `◄──`). Files whose paths match any
/// entry in `allowlist` (interpreted as suffix match relative to
/// the workspace root) are skipped — used for directory-tree
/// listings where mermaid is the wrong shape.
///
/// Pure Rust because `grep -P` on Windows Git Bash misbehaves with
/// multibyte UTF-8 character classes: it falls back to byte-level
/// matching and a class like `[┌│└├═╔╗]` collides with any
/// character whose UTF-8 starts with `\xE2` — em-dash (`—`),
/// ellipsis (`…`), curly quotes — producing thousands of false
/// positives. Rust strings are UTF-8 natively, no locale games.
#[must_use]
pub fn check_mermaid(book_roots: &[&Path], allowlist: &[&str]) -> Vec<AsciiDiagramMatch> {
    const BOX_CHARS: &[char] = &['┌', '│', '└', '├', '═', '╔', '╗'];
    const ARROWS: &[&str] = &["─►", "──▶", "◄──"];
    let mut hits = Vec::new();
    for root in book_roots {
        for md in walk_md(root) {
            if allowlist.iter().any(|a| md.ends_with(a)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&md) else {
                continue;
            };
            for (i, line) in text.lines().enumerate() {
                let has_box = line.chars().any(|c| BOX_CHARS.contains(&c));
                let has_arrow = ARROWS.iter().any(|a| line.contains(a));
                if has_box || has_arrow {
                    hits.push(AsciiDiagramMatch {
                        chapter: md.clone(),
                        line_number: i + 1,
                        line: line.trim().to_owned(),
                    });
                }
            }
        }
    }
    hits
}

/// Returns the subset of `paths` that exist as files in the working
/// tree but are NOT tracked in the git index — typically because of
/// a `.gitignore` overreach. Empty vector means every required file
/// is committed.
///
/// `cwd` should be the workspace root so the relative paths resolve
/// against the right git repo.
#[must_use]
pub fn check_required_files(paths: &[&str]) -> Vec<String> {
    let mut missing = Vec::new();
    for p in paths {
        if !is_tracked_by_git(p) {
            missing.push((*p).to_owned());
        }
    }
    missing
}

/// Normalise `..` components in a path without touching the
/// filesystem. `Path::canonicalize` requires the file to exist,
/// which defeats our "report missing files" purpose.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write(p: impl AsRef<Path>, contents: &str) {
        let p = p.as_ref();
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, contents).unwrap();
    }

    #[test]
    fn check_shared_passes_when_all_refs_resolve() {
        let tmp = TempDir::new().unwrap();
        let book = tmp.path().join("book/src");
        let shared = tmp.path().join("shared");
        write(shared.join("note.md"), "hello");
        write(book.join("chap.md"), "before {{shared note.md}} after");
        let issues = check_shared(&[book.as_path()], shared.as_path());
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_shared_reports_missing_ref_with_chapter_and_resolution() {
        let tmp = TempDir::new().unwrap();
        let book = tmp.path().join("book/src");
        let shared = tmp.path().join("shared");
        std::fs::create_dir_all(&shared).unwrap();
        write(book.join("chap.md"), "{{shared missing.md}}");
        let issues = check_shared(&[book.as_path()], shared.as_path());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].reference, "missing.md");
        assert!(issues[0].note.contains("expected at"));
        assert_eq!(issues[0].chapter, book.join("chap.md"));
    }

    #[test]
    fn check_shared_walks_nested_directories() {
        let tmp = TempDir::new().unwrap();
        let book = tmp.path().join("book/src");
        let shared = tmp.path().join("shared");
        write(shared.join("yes.md"), "y");
        write(book.join("a/b/c.md"), "{{shared yes.md}} {{shared no.md}}");
        let issues = check_shared(&[book.as_path()], shared.as_path());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].reference, "no.md");
    }

    #[test]
    fn check_shared_tolerates_whitespace_inside_tag() {
        let tmp = TempDir::new().unwrap();
        let book = tmp.path().join("book/src");
        let shared = tmp.path().join("shared");
        write(shared.join("note.md"), "y");
        write(book.join("c.md"), "{{ shared   note.md   }}");
        let issues = check_shared(&[book.as_path()], shared.as_path());
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_shared_handles_missing_book_root_silently() {
        let tmp = TempDir::new().unwrap();
        let issues = check_shared(
            &[tmp.path().join("does-not-exist").as_path()],
            tmp.path().join("shared").as_path(),
        );
        assert!(issues.is_empty());
    }

    #[test]
    fn check_snapshots_passes_when_all_assets_present() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(src.join("assets/foo.png"), "<binary>");
        write(src.join("chap.md"), "![alt](assets/foo.png)");
        let issues = check_snapshots(&[src.as_path()]);
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_snapshots_reports_missing_with_resolved_path() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(src.join("chap.md"), "![](assets/missing.png)");
        let issues = check_snapshots(&[src.as_path()]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].reference, "assets/missing.png");
        assert!(issues[0].note.contains("resolved"));
    }

    #[test]
    fn check_snapshots_resolves_parent_dir_in_relative_path() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(src.join("assets/foo.png"), "<binary>");
        // Chapter at `sub/chap.md`, asset at `../assets/foo.png`
        // must resolve to `book/src/assets/foo.png` and succeed.
        write(src.join("sub/chap.md"), "![](../assets/foo.png)");
        let issues = check_snapshots(&[src.as_path()]);
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_snapshots_handles_iframe_src() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(src.join("assets/foo.html"), "<html/>");
        write(
            src.join("chap.md"),
            r#"<iframe src="assets/foo.html"></iframe>"#,
        );
        let issues = check_snapshots(&[src.as_path()]);
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_snapshots_skips_external_urls() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(
            src.join("chap.md"),
            "![](https://example.com/assets/foo.png)",
        );
        let issues = check_snapshots(&[src.as_path()]);
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_snapshots_only_checks_assets_paths() {
        // Refs without `/assets/` in the URL (e.g. API links,
        // intra-doc links) are intentionally ignored — they're
        // not in the snapshot pipeline.
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(src.join("chap.md"), "![](../api/wisp/index.html)");
        let issues = check_snapshots(&[src.as_path()]);
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn issue_report_formats_with_prefix() {
        let issue = Issue {
            chapter: PathBuf::from("c.md"),
            reference: "x.md".into(),
            note: "missing".into(),
        };
        assert_eq!(
            issue.report("MISSING SHARED FRAGMENT"),
            "MISSING SHARED FRAGMENT: c.md → x.md (missing)"
        );
    }

    #[test]
    fn walk_md_excludes_non_markdown_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(root.join("a.md"), "");
        write(root.join("b.png"), "");
        write(root.join("sub/c.md"), "");
        write(root.join("sub/d.txt"), "");
        let mut files: Vec<_> = walk_md(root)
            .into_iter()
            .map(|p| p.strip_prefix(root).unwrap().to_owned())
            .collect();
        files.sort();
        assert_eq!(
            files,
            vec![PathBuf::from("a.md"), PathBuf::from("sub/c.md")]
        );
    }

    #[test]
    fn check_snapshots_skips_references_inside_fenced_code_blocks() {
        // The classic case: a chapter shows mdBook image syntax as
        // a documentation example. The reference inside the fence
        // doesn't have to resolve to a real file.
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(
            src.join("chap.md"),
            "Use this syntax:\n\n```markdown\n![](assets/example.png)\n```\n",
        );
        let issues = check_snapshots(&[src.as_path()]);
        assert!(
            issues.is_empty(),
            "should skip refs inside code fences; got {issues:?}"
        );
    }

    #[test]
    fn check_snapshots_still_catches_refs_outside_code_blocks() {
        // Sanity: fenced regions don't accidentally swallow the
        // entire file. A ref in plain prose after a fence still
        // gets validated.
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(
            src.join("chap.md"),
            "```\nfenced\n```\n\n![](assets/real.png)\n",
        );
        let issues = check_snapshots(&[src.as_path()]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].reference, "assets/real.png");
    }

    #[test]
    fn check_snapshots_catches_css_url_refs() {
        // CSS `url('path')` inside an inline `style="background:
        // url(...)"` is the pattern wisp-chart's chapters use to
        // place a PNG behind the live-WebGPU <iframe>. The gate
        // must fail the build when that PNG is missing — same
        // contract as `![](path)`.
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(
            src.join("chap.md"),
            "<div style=\"background: url('assets/missing.png');\"></div>\n",
        );
        let issues = check_snapshots(&[src.as_path()]);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].reference, "assets/missing.png");
    }

    #[test]
    fn check_snapshots_passes_css_url_when_asset_exists() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("book/src");
        write(
            src.join("chap.md"),
            "<div style=\"background: url('assets/exists.png');\"></div>\n",
        );
        write(src.join("assets/exists.png"), "fake-png-bytes");
        let issues = check_snapshots(&[src.as_path()]);
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_shared_skips_references_inside_fenced_code_blocks() {
        // Documenting the `{{shared X}}` tag syntax inside a code
        // fence shouldn't require `_docs/shared/X` to exist.
        let tmp = TempDir::new().unwrap();
        let book = tmp.path().join("book/src");
        let shared = tmp.path().join("shared");
        std::fs::create_dir_all(&shared).unwrap();
        write(
            book.join("chap.md"),
            "Example:\n\n```markdown\n{{shared example.md}}\n```\n",
        );
        let issues = check_shared(&[book.as_path()], shared.as_path());
        assert!(issues.is_empty(), "got {issues:?}");
    }

    #[test]
    fn check_pages_urls_clean_when_no_forbidden() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root.join("a.md"),
            "Visit https://example.github.io/Screen/ — all good.\n",
        );
        let hits = check_pages_urls(&[root], &["md"], &["https://example.github.io/screen/"]);
        assert!(hits.is_empty(), "got {hits:?}");
    }

    #[test]
    fn check_pages_urls_finds_wrong_case_form() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root.join("a.md"),
            "Visit https://example.github.io/screen/wisp/ for docs.\n",
        );
        let hits = check_pages_urls(&[root], &["md"], &["https://example.github.io/screen/"]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].matched, "https://example.github.io/screen/");
        assert_eq!(hits[0].line_number, 1);
    }

    #[test]
    fn check_pages_urls_scans_toml_when_extension_listed() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(root.join("book.toml"), "site-url = \"/screen/\"\n");
        let hits = check_pages_urls(&[root], &["toml"], &["/screen/"]);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].file, root.join("book.toml"));
    }

    #[test]
    fn check_pages_urls_skips_target_and_git_dirs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root.join("target/book/x.md"),
            "should-not-match /screen/ ever\n",
        );
        write(root.join("src/a.md"), "should-match /screen/ here\n");
        let hits = check_pages_urls(&[root], &["md"], &["/screen/"]);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].file.ends_with("src/a.md"));
    }

    #[test]
    fn check_mermaid_passes_clean_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root.join("a.md"),
            "# Title\n\nNormal prose with em-dash — and ellipsis…\n",
        );
        let hits = check_mermaid(&[root], &[]);
        assert!(
            hits.is_empty(),
            "em-dash + ellipsis must not trigger; got {hits:?}"
        );
    }

    #[test]
    fn check_mermaid_catches_box_drawing_chars() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(root.join("a.md"), "┌──────┐\n│ box  │\n└──────┘\n");
        let hits = check_mermaid(&[root], &[]);
        assert!(!hits.is_empty());
        assert!(hits.iter().any(|h| h.line.contains('┌')));
    }

    #[test]
    fn check_mermaid_catches_arrow_runs() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(root.join("a.md"), "Producer ──▶ Consumer\n");
        let hits = check_mermaid(&[root], &[]);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn check_mermaid_does_not_match_emdash_or_curly_quotes() {
        // Em dash (U+2014, UTF-8 `\xE2\x80\x94`) shares its leading
        // byte with box-drawings (`┌` = U+250C = `\xE2\x94\x8C`).
        // grep -P on Windows Git Bash false-matches them in byte
        // mode; this test pins the Rust path to char-level matching.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(
            root.join("a.md"),
            "Tauri ↔ Leptos integration — see below.\n\
             Em-dash everywhere — and curly “quotes” too.\n\
             Ellipsis … and arrow run that's NOT box-drawing: → ← ↑ ↓\n",
        );
        let hits = check_mermaid(&[root], &[]);
        assert!(hits.is_empty(), "got false positives: {hits:?}");
    }

    #[test]
    fn check_mermaid_respects_allowlist_by_path_suffix() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        write(root.join("sub/stack.md"), "├── crates\n├── _docs\n");
        // Without allowlist: matches.
        assert!(!check_mermaid(&[root], &[]).is_empty());
        // With allowlist: skipped.
        let cleared = check_mermaid(&[root], &["sub/stack.md"]);
        assert!(cleared.is_empty(), "got {cleared:?}");
    }

    #[test]
    fn check_required_files_returns_empty_when_all_tracked() {
        // The workspace's `Cargo.toml` is always tracked — use it
        // as a known-good fixture without setting up a separate
        // git repo.
        let missing = check_required_files(&["Cargo.toml"]);
        assert!(missing.is_empty(), "Cargo.toml should be tracked");
    }

    #[test]
    fn check_required_files_reports_untracked_file() {
        // A file we definitely never commit — `target/` is in
        // .gitignore. The string is enough; we don't even need it
        // to exist for `git ls-files --error-unmatch` to fail.
        let missing = check_required_files(&["target/this-is-not-tracked.txt"]);
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0], "target/this-is-not-tracked.txt");
    }

    #[test]
    fn check_required_files_mixes_tracked_and_untracked() {
        let missing =
            check_required_files(&["Cargo.toml", "target/missing-1.txt", "target/missing-2.txt"]);
        assert_eq!(missing.len(), 2);
        assert!(missing.iter().any(|f| f.ends_with("missing-1.txt")));
        assert!(missing.iter().any(|f| f.ends_with("missing-2.txt")));
    }

    #[test]
    fn normalize_resolves_parent_dirs() {
        assert_eq!(normalize(&PathBuf::from("a/b/../c")), PathBuf::from("a/c"));
        assert_eq!(normalize(&PathBuf::from("./a/./b")), PathBuf::from("a/b"));
    }
}
