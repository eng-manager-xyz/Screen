//! Headless story renderer (DEV-07 / AUT-150).
//!
//! Hosts the actual HTML emission for stories + the index cockpit.
//! Both the one-shot `ui-export-stories` binary and the long-lived
//! `render-worker` binary call into this module — the former so a
//! single `cargo run` produces the full asset set, the latter so a
//! warm process can re-render targeted story ids without re-linking
//! between iterations of the dev loop.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

use crate::stories::{Story, all_stories};

const STYLE: &str = include_str!("../assets/style.css");
const INDEX_SCRIPT: &str = include_str!("bin/index_script.js");

/// Export every shipped story plus `style.css` plus `index.html` into
/// `out_dir`. Creates the directory if missing. Returns the number of
/// stories written.
///
/// # Errors
/// Returns an `io::Error` if any file write fails.
pub fn export_all(out_dir: &Path) -> std::io::Result<usize> {
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("style.css"), STYLE)?;
    let stories = all_stories();
    for s in &stories {
        write_story_html(s, out_dir)?;
    }
    std::fs::write(out_dir.join("index.html"), render_index(&stories))?;
    Ok(stories.len())
}

/// Re-export the subset of stories identified by `ids`. Empty `ids`
/// means "re-export everything plus the index". The shared
/// `style.css` is rewritten on every call so a CSS edit picked up by
/// the worker still propagates.
///
/// Returns a vector of `(id, elapsed_ms)` tuples for everything
/// rendered, in the order rendered.
///
/// # Errors
/// Returns an `io::Error` if any file write fails or a story id is
/// unknown to the registry.
pub fn export_subset(out_dir: &Path, ids: &[String]) -> std::io::Result<Vec<(String, u64)>> {
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("style.css"), STYLE)?;
    let stories = all_stories();
    if ids.is_empty() {
        let mut out = Vec::with_capacity(stories.len());
        for s in &stories {
            let start = Instant::now();
            write_story_html(s, out_dir)?;
            let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
            out.push((s.id.to_owned(), elapsed_ms));
        }
        std::fs::write(out_dir.join("index.html"), render_index(&stories))?;
        return Ok(out);
    }
    let by_id: BTreeMap<&str, &Story> = stories.iter().map(|s| (s.id, s)).collect();
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let s = by_id.get(id.as_str()).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("unknown story id: {id}"),
            )
        })?;
        let start = Instant::now();
        write_story_html(s, out_dir)?;
        let elapsed_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        out.push((s.id.to_owned(), elapsed_ms));
    }
    Ok(out)
}

/// How many stories are currently registered (for `WorkerReply::Hello`).
#[must_use]
pub fn story_count() -> usize {
    all_stories().len()
}

fn write_story_html(story: &Story, out_dir: &Path) -> std::io::Result<()> {
    let body = (story.render)();
    let html = format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>{title}</title>
    <link rel="stylesheet" href="./style.css" />
    <style>
      body {{ padding: 24px; }}
    </style>
  </head>
  <body>
    {body}
  </body>
</html>
"#,
        title = escape_html(story.title),
        body = body,
    );
    let path = out_dir.join(format!("{}.html", story.id));
    std::fs::write(path, html)
}

fn render_index(stories: &[Story]) -> String {
    let mut grouped: BTreeMap<&'static str, Vec<&Story>> = BTreeMap::new();
    for s in stories {
        grouped.entry(s.category).or_default().push(s);
    }
    for v in grouped.values_mut() {
        v.sort_by_key(|s| s.id);
    }

    let mut sidebar = String::new();
    for (category, items) in &grouped {
        let cat = escape_html(category);
        let cat_lc = escape_html(&category.to_ascii_lowercase());
        sidebar.push_str("          <li class=\"storybook-index-category\" data-category=\"");
        sidebar.push_str(&cat_lc);
        sidebar.push_str("\">\n            <span class=\"storybook-index-heading\">");
        sidebar.push_str(&cat);
        sidebar.push_str("</span>\n            <ul>\n");
        for s in items {
            let id = escape_html(s.id);
            let title = escape_html(s.title);
            let title_lc = escape_html(&s.title.to_ascii_lowercase());
            sidebar.push_str("              <li class=\"storybook-index-row\" data-id=\"");
            sidebar.push_str(&id);
            sidebar.push_str("\" data-title=\"");
            sidebar.push_str(&title_lc);
            sidebar.push_str("\" data-category=\"");
            sidebar.push_str(&cat_lc);
            sidebar.push_str("\"><a href=\"#");
            sidebar.push_str(&id);
            sidebar.push_str("\" data-id=\"");
            sidebar.push_str(&id);
            sidebar.push_str("\">");
            sidebar.push_str(&title);
            sidebar.push_str("</a></li>\n");
        }
        sidebar.push_str("            </ul>\n          </li>\n");
    }

    let first_id = stories.first().map_or("", |s| s.id);
    let count = stories.len();

    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover" />
    <title>ui-storybook · {count} stories</title>
    <link rel="stylesheet" href="./style.css" />
    <style>
      body {{ margin: 0; height: 100vh; overflow: hidden; }}
    </style>
  </head>
  <body data-first-story="{first}">
    <div class="storybook-index-root">
      <aside class="storybook-index-sidebar">
        <div class="storybook-index-topbar">
          <span class="storybook-index-title">ui-storybook</span>
          <span class="storybook-index-count">{count}</span>
        </div>
        <div class="storybook-index-search">
          <input
            id="storybook-index-filter"
            type="search"
            placeholder="Filter (press /)"
            autocomplete="off"
            spellcheck="false"
          />
        </div>
        <nav>
          <ul class="storybook-index-list">
{sidebar}          </ul>
        </nav>
      </aside>
      <main class="storybook-index-main">
        <header class="storybook-index-header">
          <span class="storybook-index-current" id="storybook-index-current">{first}</span>
          <a class="storybook-index-open" id="storybook-index-open" href="./{first}.html" target="_blank" rel="noopener">Open in new tab ↗</a>
        </header>
        <iframe
          class="storybook-index-frame"
          id="storybook-index-frame"
          title="story preview"
          src="./{first}.html"
        ></iframe>
      </main>
    </div>
    <script>
{script}
    </script>
  </body>
</html>
"#,
        count = count,
        first = escape_html(first_id),
        sidebar = sidebar,
        script = INDEX_SCRIPT,
    )
}

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn escape_html_handles_special_chars() {
        assert_eq!(escape_html("a&b"), "a&amp;b");
        assert_eq!(escape_html("<x>"), "&lt;x&gt;");
        assert_eq!(escape_html("a\"b"), "a&quot;b");
    }

    #[test]
    fn export_all_writes_every_story_plus_index_plus_css() {
        let tmp = TempDir::new().unwrap();
        let count = export_all(tmp.path()).unwrap();
        assert!(count > 0);
        assert!(tmp.path().join("style.css").exists());
        assert!(tmp.path().join("index.html").exists());
        // At least one story file should exist (use the first registered).
        let stories = all_stories();
        let first = stories.first().expect("at least one story");
        assert!(tmp.path().join(format!("{}.html", first.id)).exists());
    }

    #[test]
    fn export_subset_writes_only_named_stories() {
        let tmp = TempDir::new().unwrap();
        // Seed CSS so the dir is in a known state.
        export_all(tmp.path()).unwrap();
        // Remove a story file, ensure export_subset rewrites it.
        let stories = all_stories();
        let first = stories.first().expect("at least one story");
        let p = tmp.path().join(format!("{}.html", first.id));
        std::fs::remove_file(&p).unwrap();
        let written = export_subset(tmp.path(), &[first.id.to_owned()]).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].0, first.id);
        assert!(p.exists());
    }

    #[test]
    fn export_subset_empty_renders_all() {
        let tmp = TempDir::new().unwrap();
        let written = export_subset(tmp.path(), &[]).unwrap();
        assert_eq!(written.len(), all_stories().len());
        assert!(tmp.path().join("index.html").exists());
    }

    #[test]
    fn export_subset_unknown_id_is_an_error() {
        let tmp = TempDir::new().unwrap();
        let err = export_subset(tmp.path(), &["nope-not-a-real-story".into()]).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn story_count_matches_all_stories() {
        assert_eq!(story_count(), all_stories().len());
    }
}
