//! `doc-gates` CLI — exposes [`doc_gates::check_shared`] and
//! [`doc_gates::check_snapshots`] as subcommands so Justfile recipes
//! and CI can call them without spawning Python.
//!
//! ```sh
//! doc-gates shared-check     # walks both books + shared
//! doc-gates snapshots-check  # walks both books for `/assets/` refs
//! ```
//!
//! Both subcommands hard-code the workspace's book layout — this is
//! a project-internal tool, not a general-purpose CLI. The book roots
//! and shared root match what `just gate` / `just site-check` expect.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "usage: doc-gates <shared-check|snapshots-check>";

const BOOK_ROOTS: &[&str] = &["_docs/book/src", "_docs/wisp-book/src"];
const SHARED_ROOT: &str = "_docs/shared";

fn main() -> ExitCode {
    let Some(cmd) = std::env::args().nth(1) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match cmd.as_str() {
        "shared-check" => run_shared(),
        "snapshots-check" => run_snapshots(),
        other => {
            eprintln!("unknown command: {other}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run_shared() -> ExitCode {
    let roots: Vec<&Path> = BOOK_ROOTS.iter().map(Path::new).collect();
    let issues = doc_gates::check_shared(&roots, Path::new(SHARED_ROOT));
    for i in &issues {
        eprintln!("{}", i.report("MISSING SHARED FRAGMENT"));
    }
    if issues.is_empty() {
        println!("shared-check: source-level shared() references resolve.");
        ExitCode::SUCCESS
    } else {
        eprintln!("shared-check failed: {} broken reference(s)", issues.len());
        ExitCode::FAILURE
    }
}

fn run_snapshots() -> ExitCode {
    let roots: Vec<&Path> = BOOK_ROOTS.iter().map(Path::new).collect();
    let issues = doc_gates::check_snapshots(&roots);
    for i in &issues {
        eprintln!("{}", i.report("MISSING ASSET"));
    }
    if issues.is_empty() {
        println!("snapshots-check: all referenced assets present.");
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "snapshots-check failed: {} broken reference(s)",
            issues.len()
        );
        ExitCode::FAILURE
    }
}
