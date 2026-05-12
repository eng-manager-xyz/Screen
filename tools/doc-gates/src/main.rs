//! `doc-gates` CLI — exposes [`doc_gates::check_shared`],
//! [`doc_gates::check_snapshots`], and [`doc_gates::check_required_files`]
//! as subcommands so Justfile recipes and CI can call them without
//! spawning Python.
//!
//! ```sh
//! doc-gates shared-check          # walks both books + shared
//! doc-gates snapshots-check       # walks both books for `/assets/` refs
//! doc-gates required-files-check  # asserts must-have files are tracked in git
//! ```
//!
//! All subcommands hard-code the workspace's expected layout — this is
//! a project-internal tool, not a general-purpose CLI. The book roots,
//! shared root, and required-files list match what `just gate` /
//! `just site-check` expect.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str =
    "usage: doc-gates <shared-check|snapshots-check|mermaid-check|required-files-check>";

const BOOK_ROOTS: &[&str] = &["_docs/book/src", "_docs/wisp-book/src"];
const SHARED_ROOT: &str = "_docs/shared";

/// Chapters that are allowed to use ASCII art instead of mermaid.
/// Today: only directory-tree listings, where mermaid's flowchart
/// dialect produces worse output than indented text.
const MERMAID_ALLOWLIST: &[&str] = &["_docs/book/src/orientation/stack.md"];

/// Files that the CI build depends on but which are easy to lose
/// silently (e.g. a `.gitignore` overreach swallows them on `git
/// add`). Each entry causes a clear failure message that points
/// at `.gitignore` for debugging.
///
/// Add to this list when a new build-critical asset gets committed
/// — favour over-inclusion: the cost of a missed entry (opaque CI
/// failure in `tauri-build` / `mdbook` / whatever) is much higher
/// than the cost of one extra `git ls-files` call per gate.
const REQUIRED_FILES: &[&str] = &[
    // `tauri::generate_context!()` embeds this at macro expansion time.
    "crates/app/icons/icon.png",
    // `tauri-winres` requires this on Windows; without it the build
    // script aborts with `icons/icon.ico not found`. Regen via
    // `cargo run -p screen-app --example regen-icons`.
    "crates/app/icons/icon.ico",
];

fn main() -> ExitCode {
    let Some(cmd) = std::env::args().nth(1) else {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    };
    match cmd.as_str() {
        "shared-check" => run_shared(),
        "snapshots-check" => run_snapshots(),
        "mermaid-check" => run_mermaid(),
        "required-files-check" => run_required_files(),
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

fn run_mermaid() -> ExitCode {
    let roots: Vec<&Path> = BOOK_ROOTS.iter().map(Path::new).collect();
    let hits = doc_gates::check_mermaid(&roots, MERMAID_ALLOWLIST);
    if hits.is_empty() {
        println!("mermaid-check: no ASCII diagrams in mdBook chapters.");
        return ExitCode::SUCCESS;
    }
    // Group by file for readable output.
    let mut current: Option<&Path> = None;
    for hit in &hits {
        if current.map(Path::to_owned) != Some(hit.chapter.clone()) {
            eprintln!("ASCII DIAGRAM IN MDBOOK CHAPTER: {}", hit.chapter.display());
            current = Some(&hit.chapter);
        }
        eprintln!("  {}: {}", hit.line_number, hit.line);
    }
    let files = hits
        .iter()
        .map(|h| h.chapter.as_path())
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    eprintln!();
    eprintln!("Found {files} file(s) with ASCII diagrams.");
    eprintln!("Per CLAUDE.md 'Diagrams in mdBook — mermaid only, no ASCII',");
    eprintln!("convert to a ```mermaid block.");
    eprintln!("Prefer sequenceDiagram when participants exchange messages over time.");
    ExitCode::FAILURE
}

fn run_required_files() -> ExitCode {
    let missing = doc_gates::check_required_files(REQUIRED_FILES);
    for f in &missing {
        eprintln!("MISSING FROM GIT: {f}");
        eprintln!("  → file exists in the working tree but isn't tracked by git.");
        eprintln!("  → likely cause: a `.gitignore` pattern matches this path or its parent dir.");
        eprintln!("  → debug with: git check-ignore -v {f}");
        eprintln!("  → fix: tighten the pattern in .gitignore, then `git add {f}`.");
    }
    if missing.is_empty() {
        println!(
            "required-files-check: all {} required files tracked in git.",
            REQUIRED_FILES.len()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "required-files-check failed: {} file(s) untracked",
            missing.len()
        );
        ExitCode::FAILURE
    }
}
