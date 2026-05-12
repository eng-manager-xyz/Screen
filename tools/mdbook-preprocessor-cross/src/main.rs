//! `mdbook-preprocessor-cross` binary entrypoint.
//!
//! mdBook invokes a preprocessor binary either with `supports <renderer>`
//! to ask whether the preprocessor supports that renderer (exit 0 = yes,
//! 1 = no), or with no args to run the actual transformation. When run,
//! the binary reads `[ctx, book]` JSON on stdin and writes the mutated
//! `book` JSON to stdout.
//!
//! All transformation logic lives in [`mdbook_preprocessor_cross::transform`]
//! so it can be unit-tested without spawning a process.

use std::io::{self, Read, Write};

use anyhow::{Context, Result};
use mdbook_preprocessor_cross::{config_from_ctx, transform, walk_book};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(first) = args.first()
        && first == "supports"
    {
        // mdBook 0.4 sends `supports <renderer>`. We support `html`.
        let renderer = args.get(1).map_or("", String::as_str);
        std::process::exit(i32::from(renderer != "html"));
    }

    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .context("read preprocessor payload from stdin")?;
    let payload: serde_json::Value = serde_json::from_str(&input).context("parse stdin json")?;
    let ctx = payload.get(0).context("payload missing [0] context")?;
    let mut book = payload
        .get(1)
        .cloned()
        .context("payload missing [1] book")?;
    let config = config_from_ctx(ctx);
    walk_book(&mut book, |s| transform(s, &config));
    let stdout = io::stdout();
    let mut out = stdout.lock();
    serde_json::to_writer(&mut out, &book).context("write book json to stdout")?;
    out.flush().context("flush stdout")?;
    Ok(())
}
