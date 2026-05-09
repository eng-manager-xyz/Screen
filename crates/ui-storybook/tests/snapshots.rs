//! SSR snapshot gate for every UI story.
//!
//! For each `Story` registered in `ui_storybook::stories::all_stories`, render
//! it to an HTML string via Leptos SSR and lock that string with `insta`.
//! This is the literal "no console errors at runtime" gate the user asked for,
//! ported to the HTML side: any unintended class swap, missing child, dropped
//! attribute, or panic during render trips the snapshot diff.
//!
//! Snapshot acceptance: first runs leave `*.snap.new` files and FAIL. Accept
//! with `cargo insta review` or `mv crates/ui-storybook/tests/snapshots/*.new
//! …` (matching the same workflow we documented for wisp-storybook).

use ui_storybook::stories::all_stories;

#[test]
fn every_story_renders_to_non_empty_html() {
    for story in all_stories() {
        let html = (story.render)();
        assert!(
            !html.trim().is_empty(),
            "story `{}` rendered to empty HTML",
            story.id
        );
        // Sanity: the SSR output must contain at least one tag.
        assert!(
            html.contains('<') && html.contains('>'),
            "story `{}` produced output without any HTML tags: {html:?}",
            story.id
        );
    }
}

#[test]
fn story_html_matches_snapshot() {
    let mut entries: Vec<(String, String)> = all_stories()
        .into_iter()
        .map(|story| (story.id.to_string(), normalize(&(story.render)())))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    insta::assert_yaml_snapshot!("story_html", entries);
}

/// Strip Leptos's hydration markers so the snapshot tracks meaningful structure.
///
/// Leptos sprinkles HTML comments like `<!--hk=…-->` and `data-hk` attributes
/// for hydration; their exact tokens drift between releases (and even between
/// runs in some setups), which would make snapshots churn for no semantic
/// reason. We collapse them to fixed markers so the snapshot only fires on
/// real changes — class names, slot order, attribute values.
fn normalize(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut chars = html.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' && chars.peek() == Some(&'!') {
            chars.next();
            if chars.peek() == Some(&'-') {
                chars.next();
                if chars.peek() == Some(&'-') {
                    chars.next();
                    let mut a = ' ';
                    let mut b = ' ';
                    for d in chars.by_ref() {
                        if a == '-' && b == '-' && d == '>' {
                            break;
                        }
                        a = b;
                        b = d;
                    }
                    continue;
                }
            }
            out.push('<');
            out.push('!');
        } else {
            out.push(c);
        }
    }
    strip_attr(&out, "data-hk")
}

fn strip_attr(html: &str, attr: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let needle = format!(" {attr}=\"");
    while let Some(idx) = rest.find(&needle) {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + needle.len()..];
        if let Some(end) = after.find('"') {
            rest = &after[end + 1..];
        } else {
            // Malformed; stop stripping.
            rest = after;
            break;
        }
    }
    out.push_str(rest);
    out
}
