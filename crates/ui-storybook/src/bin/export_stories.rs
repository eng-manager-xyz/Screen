//! `cargo run -p ui-storybook --bin ui-export-stories`
//!
//! One-shot exporter: writes every story HTML + `style.css` + the
//! cockpit `index.html` into `_docs/book/src/assets/ui/`. Used by
//! `just snapshots-ui` and by CI to keep the committed asset set in
//! sync with the registry.
//!
//! The actual rendering lives in [`ui_storybook::exporter`] so the
//! long-lived `render-worker` binary (DEV-07 / AUT-150) can share
//! exactly the same code path.

use std::path::Path;

fn main() {
    let out_dir = Path::new("_docs/book/src/assets/ui");
    let count = ui_storybook::exporter::export_all(out_dir).expect("export stories");
    let dir = out_dir.display();
    println!("exporting {count} stories + shared style.css → {dir}");
    println!("  ✓ index.html ({count} stories)");
    println!("done.");
}
