//! Smoke + live-reload integration tests for `dev-server`
//! (AUT-145 / AUT-152). Boots the server on an OS-picked port against a
//! tempdir, hits it via reqwest + tokio-tungstenite, asserts shape.

use std::path::PathBuf;
use std::time::Duration;

use dev_server::build_router;
use dev_server::live_reload::ReloadBroadcaster;
use futures::{SinkExt, StreamExt};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Spawn the router on a free port, returning `(socket, broadcaster)`.
async fn spawn_server(assets: &std::path::Path) -> (std::net::SocketAddr, ReloadBroadcaster) {
    let broadcaster = ReloadBroadcaster::default();
    let app = build_router(assets, broadcaster.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    // Give the listener a moment to bind.
    tokio::time::sleep(Duration::from_millis(20)).await;
    (addr, broadcaster)
}

fn write_asset(tmp: &TempDir, name: &str, contents: &str) -> PathBuf {
    let p = tmp.path().join(name);
    std::fs::write(&p, contents).unwrap();
    p
}

#[tokio::test]
async fn html_response_has_inline_client_injected() {
    let tmp = TempDir::new().unwrap();
    write_asset(
        &tmp,
        "hello.html",
        "<!doctype html><html><body><h1>hi</h1></body></html>",
    );
    let (addr, _bc) = spawn_server(tmp.path()).await;
    let url = format!("http://{addr}/hello.html");
    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());
    let body = resp.text().await.unwrap();
    assert!(
        body.contains("dev-server live reload"),
        "injection missing from html body: {body:?}"
    );
    assert!(body.contains("<h1>hi</h1>"));
}

#[tokio::test]
async fn css_response_is_byte_identical() {
    let tmp = TempDir::new().unwrap();
    let css = ":root { --bg: #000; }";
    write_asset(&tmp, "style.css", css);
    let (addr, _bc) = spawn_server(tmp.path()).await;
    let url = format!("http://{addr}/style.css");
    let resp = reqwest::get(&url).await.unwrap();
    assert!(resp.status().is_success());
    let body = resp.text().await.unwrap();
    assert_eq!(body, css, "CSS body must not be modified by middleware");
    assert!(!body.contains("dev-server live reload"));
}

#[tokio::test]
async fn reload_endpoint_broadcasts_to_ws_clients() {
    let tmp = TempDir::new().unwrap();
    write_asset(&tmp, "x.html", "<!doctype html><html><body></body></html>");
    let (addr, _bc) = spawn_server(tmp.path()).await;

    let ws_url = format!("ws://{addr}/__dev/ws");
    let (mut ws, _resp) = tokio_tungstenite::connect_async(&ws_url).await.unwrap();

    // Trigger reload via the manual POST endpoint.
    let reload_url = format!("http://{addr}/__dev/reload");
    let resp = reqwest::Client::new()
        .post(&reload_url)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);

    // Expect a reload message on the socket within 1 second.
    let msg = tokio::time::timeout(Duration::from_secs(1), ws.next())
        .await
        .expect("timed out waiting for reload")
        .expect("ws stream ended")
        .expect("ws message");
    assert_eq!(msg, Message::Text("reload".into()));

    let _ = ws.send(Message::Close(None)).await;
}

#[tokio::test]
async fn missing_asset_returns_404() {
    let tmp = TempDir::new().unwrap();
    let (addr, _bc) = spawn_server(tmp.path()).await;
    let url = format!("http://{addr}/nope.html");
    let resp = reqwest::get(&url).await.unwrap();
    assert_eq!(resp.status(), 404);
}
