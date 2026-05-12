//! `dev-server` library exports — used by the integration tests in
//! `tests/` and by the `dev-server` binary in `src/main.rs`.
//!
//! See `src/main.rs` for the CLI entry point and the high-level
//! architecture overview. The library surface is split across three
//! modules:
//!
//! - [`live_reload`] — WebSocket fan-out + HTML injection middleware
//!   (M-DEV.1 / AUT-152).
//! - [`watcher`] — debounced file watcher + rebuild subprocess
//!   (M-DEV.2 / AUT-146).
//! - [`worker`] — JSON-IPC types for the optional persistent renderer
//!   worker (M-DEV.7 / AUT-150).

pub mod live_reload;
pub mod watcher;
pub mod worker;

use std::path::Path;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

use crate::live_reload::{
    ReloadBroadcaster, inject_into_html, reload_endpoint, response_is_html, ws_handler,
};

/// Maximum HTML body size we'll inject the live-reload client into.
/// Storybook stories are tiny (~15 lines without the shared CSS); a 4 MiB
/// ceiling is wildly generous and stops us from ballooning memory if
/// `ServeDir` ever streams a huge HTML asset.
pub const MAX_HTML_INJECT_BYTES: usize = 4 * 1024 * 1024;

/// Build the axum router that wires `ServeDir`, the live-reload
/// WebSocket endpoint, the manual `/__dev/reload` POST trigger, and the
/// HTML-injection middleware. Used by both `main.rs` and the integration
/// tests in `tests/smoke.rs`.
pub fn build_router(assets: &Path, broadcaster: ReloadBroadcaster) -> Router {
    let serve = ServeDir::new(assets).append_index_html_on_directories(true);
    Router::new()
        .route("/__dev/ws", get(ws_handler))
        .route("/__dev/reload", post(reload_endpoint))
        .fallback_service(serve)
        .with_state(broadcaster)
        .layer(middleware::from_fn(inject_live_reload))
        .layer(TraceLayer::new_for_http())
}

/// Middleware that re-reads the response body when the response is an
/// HTML document and splices in the live-reload client. Exposed at the
/// crate root so the integration tests can compose it directly if
/// needed.
pub async fn inject_live_reload(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_owned();
    let resp = next.run(req).await;
    let (mut parts, body) = resp.into_parts();
    if !response_is_html(&parts.headers, &path) {
        return Response::from_parts(parts, body);
    }
    let bytes = match to_bytes(body, MAX_HTML_INJECT_BYTES).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("inject: failed to buffer HTML body: {e}");
            return Response::from_parts(parts, Body::empty());
        }
    };
    let original = std::str::from_utf8(&bytes).unwrap_or("");
    let injected = inject_into_html(original);
    parts.headers.insert(
        axum::http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&injected.len().to_string()).unwrap(),
    );
    Response::from_parts(parts, Body::from(injected))
}
