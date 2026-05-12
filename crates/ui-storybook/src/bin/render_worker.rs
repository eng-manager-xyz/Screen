//! `cargo run -p ui-storybook --bin render-worker` — long-lived
//! renderer process for the dev loop (DEV-07 / AUT-150).
//!
//! Reads newline-delimited JSON commands on stdin, runs the requested
//! re-renders against the same code path as `ui-export-stories`, and
//! writes JSON replies on stdout. The protocol types live in
//! `dev_server::worker` and are mirrored here so both crates can build
//! independently — the worker re-derives the types from `serde_json`.
//!
//! Run alone:
//!
//! ```bash
//! cargo run -p ui-storybook --bin render-worker -- --assets _docs/book/src/assets/ui
//! ```
//!
//! Then on stdin:
//!
//! ```json
//! {"cmd":"hello"}
//! {"cmd":"rerender","ids":["button-variants"]}
//! {"cmd":"shutdown"}
//! ```
//!
//! Each command emits zero or more reply lines on stdout. The worker
//! exits 0 on `shutdown` or stdin EOF.

use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
enum WorkerCommand {
    Hello,
    Rerender { ids: Vec<String> },
    Shutdown,
}

#[derive(Debug, Serialize)]
#[serde(tag = "reply", rename_all = "snake_case")]
enum WorkerReply {
    Hello { story_count: usize },
    Done { id: String, elapsed_ms: u64 },
    BatchDone { elapsed_ms: u64 },
    Error { message: String },
}

#[derive(Debug)]
struct Args {
    assets: PathBuf,
}

fn parse_args() -> Args {
    // Minimal argv parser — keeps the worker binary tiny + dep-free.
    let mut args = std::env::args().skip(1);
    let mut assets: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--assets" => assets = args.next().map(PathBuf::from),
            other => {
                eprintln!("render-worker: unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }
    Args {
        assets: assets.unwrap_or_else(|| PathBuf::from("_docs/book/src/assets/ui")),
    }
}

fn main() {
    let args = parse_args();
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let cmd: WorkerCommand = match serde_json::from_str(&line) {
            Ok(c) => c,
            Err(e) => {
                write_reply(
                    &mut out,
                    &WorkerReply::Error {
                        message: format!("parse: {e}"),
                    },
                );
                continue;
            }
        };
        match cmd {
            WorkerCommand::Hello => {
                let reply = WorkerReply::Hello {
                    story_count: ui_storybook::exporter::story_count(),
                };
                write_reply(&mut out, &reply);
            }
            WorkerCommand::Rerender { ids } => {
                let start = Instant::now();
                match ui_storybook::exporter::export_subset(&args.assets, &ids) {
                    Ok(report) => {
                        for (id, ms) in report {
                            write_reply(&mut out, &WorkerReply::Done { id, elapsed_ms: ms });
                        }
                        let total = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
                        write_reply(&mut out, &WorkerReply::BatchDone { elapsed_ms: total });
                    }
                    Err(e) => write_reply(
                        &mut out,
                        &WorkerReply::Error {
                            message: format!("export: {e}"),
                        },
                    ),
                }
            }
            WorkerCommand::Shutdown => break,
        }
    }
}

fn write_reply<W: Write>(out: &mut W, reply: &WorkerReply) {
    let json = serde_json::to_string(reply).unwrap_or_else(|_| "{\"reply\":\"error\"}".into());
    if writeln!(out, "{json}").is_err() {
        std::process::exit(0);
    }
    let _ = out.flush();
}
