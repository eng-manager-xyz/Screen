//! Live-reload plumbing (M-DEV.1 / AUT-152).
//!
//! Three pieces:
//!
//! 1. [`INLINE_CLIENT`] — the vanilla-JS snippet injected into every HTML
//!    response. It opens a WebSocket back to the server, reloads the page
//!    on receiving the message `reload`, and auto-reconnects when the
//!    socket closes (which doubles as a "wait for server restart and
//!    pick up the latest" handler).
//! 2. [`ReloadBroadcaster`] — a thin wrapper around
//!    [`tokio::sync::broadcast`] that fans `"reload"` strings out to every
//!    connected WebSocket. The file watcher (`watcher.rs`) calls
//!    [`ReloadBroadcaster::send_reload`] after a successful rebuild.
//! 3. [`inject_into_html`] — middleware-friendly helper that splices
//!    [`INLINE_CLIENT`] into an HTML body before the closing `</body>` tag
//!    (or at the end if no closing tag exists).

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use tokio::sync::broadcast;
use tracing::{debug, warn};

/// Inline live-reload client. Injected at the end of every HTML body.
///
/// Kept under 20 lines of vanilla JS so it doesn't dominate the snapshot
/// diff for the asset files. The path constants here MUST match the routes
/// wired in `main.rs` (`/__dev/ws`, `/__dev/reload`).
pub const INLINE_CLIENT: &str = r"
<!-- dev-server live reload (injected by crates/dev-server) -->
<script>
(function () {
  function connect() {
    var proto = location.protocol === 'https:' ? 'wss://' : 'ws://';
    var ws = new WebSocket(proto + location.host + '/__dev/ws');
    ws.onmessage = function (e) { if (e.data === 'reload') { location.reload(); } };
    ws.onclose = function () { setTimeout(function () { location.reload(); }, 1500); };
    ws.onerror = function () { try { ws.close(); } catch (_) {} };
  }
  connect();
})();
</script>
";

/// Broadcast handle shared across the router state, the watcher, and the
/// optional manual `/__dev/reload` POST endpoint.
#[derive(Clone)]
pub struct ReloadBroadcaster {
    tx: broadcast::Sender<String>,
}

impl ReloadBroadcaster {
    /// Create a broadcaster with the given channel capacity. 16 is plenty
    /// for the "phone + laptop browser" use case.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Subscribe a new WebSocket client to the broadcast.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<String> {
        self.tx.subscribe()
    }

    /// Broadcast `"reload"` to every connected client. Never errors —
    /// `send` only fails when there are zero receivers, which is the
    /// expected idle state.
    pub fn send_reload(&self) {
        let _ = self.tx.send("reload".to_owned());
    }

    /// How many WebSocket clients are currently subscribed.
    #[must_use]
    pub fn receiver_count(&self) -> usize {
        self.tx.receiver_count()
    }
}

impl Default for ReloadBroadcaster {
    fn default() -> Self {
        Self::new(16)
    }
}

/// axum WebSocket upgrade handler. Bound to `GET /__dev/ws`.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(broadcaster): State<ReloadBroadcaster>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, broadcaster))
}

async fn handle_socket(mut socket: WebSocket, broadcaster: ReloadBroadcaster) {
    let mut rx = broadcaster.subscribe();
    debug!(
        "live-reload: client connected ({} total)",
        broadcaster.receiver_count()
    );
    loop {
        tokio::select! {
            biased;
            msg = rx.recv() => {
                match msg {
                    Ok(text) => {
                        if socket.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("live-reload: dropped {} reload events (slow client)", skipped);
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            client_msg = socket.recv() => {
                // None, close frame, and errors all terminate the loop.
                let still_alive = matches!(client_msg, Some(Ok(msg)) if !matches!(msg, Message::Close(_)));
                if !still_alive {
                    break;
                }
            }
        }
    }
    debug!("live-reload: client disconnected");
}

/// axum handler for `POST /__dev/reload`. Triggers a broadcast.
/// Used both by `curl` for manual testing and by the file watcher
/// for programmatic rebuild signalling.
pub async fn reload_endpoint(State(broadcaster): State<ReloadBroadcaster>) -> StatusCode {
    broadcaster.send_reload();
    StatusCode::NO_CONTENT
}

/// Splice [`INLINE_CLIENT`] into `body` immediately before `</body>` (case
/// insensitive). If no closing body tag exists, append at the end.
#[must_use]
pub fn inject_into_html(body: &str) -> String {
    let close_tag = "</body>";
    if let Some(idx) = find_case_insensitive(body, close_tag) {
        let mut out = String::with_capacity(body.len() + INLINE_CLIENT.len());
        out.push_str(&body[..idx]);
        out.push_str(INLINE_CLIENT);
        out.push_str(&body[idx..]);
        out
    } else {
        let mut out = String::with_capacity(body.len() + INLINE_CLIENT.len());
        out.push_str(body);
        out.push_str(INLINE_CLIENT);
        out
    }
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    let h = haystack.to_ascii_lowercase();
    let n = needle.to_ascii_lowercase();
    h.find(&n)
}

/// Decide whether a response should have the inline client injected.
/// Currently: only `text/html` responses with the request path ending in
/// `.html` (or `/`, since that resolves to `index.html` via `ServeDir`).
#[must_use]
pub fn response_is_html(headers: &HeaderMap, request_path: &str) -> bool {
    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let ct_is_html = ct.starts_with("text/html");
    let lower = request_path.to_ascii_lowercase();
    // Case-insensitive ".html" suffix check — already lowercased above.
    let path_is_html = lower == "/" || lower.ends_with('/') || has_html_suffix(&lower);
    ct_is_html && path_is_html
}

#[inline]
fn has_html_suffix(lower: &str) -> bool {
    lower.len() >= 5 && &lower[lower.len() - 5..] == ".html"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inject_before_close_body() {
        let html = "<html><body><h1>hi</h1></body></html>";
        let out = inject_into_html(html);
        assert!(out.contains("<h1>hi</h1>"));
        assert!(out.contains("dev-server live reload"));
        let close_idx = out.find("</body>").expect("close tag preserved");
        let injected_idx = out.find("dev-server live reload").expect("injected");
        assert!(injected_idx < close_idx, "injection must precede </body>");
    }

    #[test]
    fn inject_appends_when_no_close_body() {
        let html = "<html><body>fragment";
        let out = inject_into_html(html);
        assert!(out.ends_with("</script>\n"), "got: {out:?}");
        assert!(out.starts_with(html));
    }

    #[test]
    fn inject_is_case_insensitive() {
        let html = "<HTML><BODY>x</BODY></HTML>";
        let out = inject_into_html(html);
        assert!(out.contains("dev-server live reload"));
        let injected_idx = out.find("dev-server live reload").expect("injected");
        let close_idx = out.to_ascii_lowercase().find("</body>").expect("close");
        assert!(injected_idx < close_idx);
    }

    #[test]
    fn response_is_html_filters_css() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "text/css".parse().unwrap());
        assert!(!response_is_html(&headers, "/style.css"));
    }

    #[test]
    fn response_is_html_accepts_html() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            "text/html; charset=utf-8".parse().unwrap(),
        );
        assert!(response_is_html(&headers, "/button-variants.html"));
        assert!(response_is_html(&headers, "/"));
    }

    #[test]
    fn broadcaster_send_with_no_subscribers_does_not_panic() {
        let b = ReloadBroadcaster::new(4);
        b.send_reload(); // should be a no-op when no subscribers
        assert_eq!(b.receiver_count(), 0);
    }

    #[tokio::test]
    async fn broadcaster_fans_out_to_subscribers() {
        let b = ReloadBroadcaster::new(4);
        let mut rx1 = b.subscribe();
        let mut rx2 = b.subscribe();
        b.send_reload();
        assert_eq!(rx1.recv().await.unwrap(), "reload");
        assert_eq!(rx2.recv().await.unwrap(), "reload");
    }
}
