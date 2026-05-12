//! Persistent renderer worker IPC (M-DEV.7 / AUT-150).
//!
//! `dev-server` can be launched with `--worker` to spawn the
//! `render-worker` binary as a long-lived child process. The watcher
//! then sends [`WorkerCommand::Rerender`] messages over the worker's
//! stdin (newline-delimited JSON), and the worker rebuilds the
//! requested story set without re-linking. Replies come back over
//! stdout as [`WorkerReply`] lines.
//!
//! When the worker isn't requested the watcher falls back to the
//! subprocess-per-change path defined in [`crate::watcher`].

use serde::{Deserialize, Serialize};

/// One command line sent to a `render-worker` over stdin.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum WorkerCommand {
    /// Liveness probe. Worker replies with [`WorkerReply::Hello`].
    Hello,
    /// Re-render the named story ids. Empty list means "rerender all".
    Rerender {
        /// Story ids to rebuild. Empty = all stories.
        ids: Vec<String>,
    },
    /// Graceful shutdown — worker drains and exits.
    Shutdown,
}

/// One reply line emitted by a `render-worker` on stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "reply", rename_all = "snake_case")]
pub enum WorkerReply {
    /// Response to [`WorkerCommand::Hello`].
    Hello {
        /// Number of stories the worker is currently serving.
        story_count: usize,
    },
    /// One story finished rendering.
    Done {
        /// The story id that was rebuilt.
        id: String,
        /// Wall-clock time the render took, in milliseconds.
        elapsed_ms: u64,
    },
    /// All stories in a batch finished.
    BatchDone {
        /// Total wall-clock time for the batch, in milliseconds.
        elapsed_ms: u64,
    },
    /// Non-fatal error rendering one story. Worker keeps serving.
    Error {
        /// Human-readable error.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_round_trips_json() {
        let cmd = WorkerCommand::Rerender {
            ids: vec!["button-variants".into()],
        };
        let s = serde_json::to_string(&cmd).unwrap();
        assert!(s.contains("\"cmd\":\"rerender\""));
        let back: WorkerCommand = serde_json::from_str(&s).unwrap();
        assert_eq!(cmd, back);
    }

    #[test]
    fn reply_round_trips_json() {
        let r = WorkerReply::Done {
            id: "tray-record-popover-default".into(),
            elapsed_ms: 42,
        };
        let s = serde_json::to_string(&r).unwrap();
        assert!(s.contains("\"reply\":\"done\""));
        let back: WorkerReply = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }

    #[test]
    fn hello_command_has_no_payload() {
        let s = serde_json::to_string(&WorkerCommand::Hello).unwrap();
        assert_eq!(s, r#"{"cmd":"hello"}"#);
    }

    #[test]
    fn shutdown_command_has_no_payload() {
        let s = serde_json::to_string(&WorkerCommand::Shutdown).unwrap();
        assert_eq!(s, r#"{"cmd":"shutdown"}"#);
    }
}
