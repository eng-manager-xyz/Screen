//! `cargo run -p ui-storybook --bin export-stories`
//!
//! Headlessly renders every shipped UI story to an HTML file under
//! `_docs/book/src/assets/ui/<id>.html`. The mdBook site embeds them either
//! inline (`<iframe src="...">`) or links to them as live demos.
//!
//! Stories share a single sibling stylesheet (`./style.css` in the same
//! directory) rather than inlining the full CSS into every HTML file —
//! that change cut the committed asset payload from ~117k lines of
//! duplicated CSS to ~1.7k. The exporter writes `style.css` once at the
//! start of the run, then every story HTML `<link>`s to it.

use std::path::Path;

use ui_storybook::stories::all_stories;

const STYLE: &str = include_str!("../../assets/style.css");

fn main() {
    let out_dir = Path::new("_docs/book/src/assets/ui");
    std::fs::create_dir_all(out_dir).expect("create assets dir");

    // Write the shared stylesheet once. Every story HTML links to this
    // file via a relative `./style.css` href.
    let style_path = out_dir.join("style.css");
    std::fs::write(&style_path, STYLE).expect("write shared style.css");

    let stories = all_stories();
    println!(
        "exporting {} stories + shared style.css → {}",
        stories.len(),
        out_dir.display()
    );

    for story in stories {
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
            title = story.title,
            body = body,
        );

        let path = out_dir.join(format!("{}.html", story.id));
        std::fs::write(&path, html).expect("write html");
        println!("  ✓ {}", story.id);
    }

    println!("done.");
}
