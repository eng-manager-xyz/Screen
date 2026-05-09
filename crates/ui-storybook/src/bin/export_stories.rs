//! `cargo run -p ui-storybook --bin export-stories`
//!
//! Headlessly renders every shipped UI story to a standalone HTML file under
//! `_docs/book/src/assets/ui/<id>.html`. The mdBook site embeds them either
//! inline (`<iframe src="...">`) or links to them as live demos.
//!
//! Each output is a complete `<html>` document with the storybook stylesheet
//! inlined, so it opens correctly in a browser tab without any external
//! resources. A future upgrade swaps these for PNGs via `headless_chrome`.

use std::path::Path;

use ui_storybook::stories::all_stories;

const STYLE: &str = include_str!("../../assets/style.css");

fn main() {
    let out_dir = Path::new("_docs/book/src/assets/ui");
    std::fs::create_dir_all(out_dir).expect("create assets dir");

    let stories = all_stories();
    println!(
        "exporting {} stories → {}",
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
    <style>{style}</style>
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
            style = STYLE,
            body = body,
        );

        let path = out_dir.join(format!("{}.html", story.id));
        std::fs::write(&path, html).expect("write html");
        println!("  ✓ {}", story.id);
    }

    println!("done.");
}
