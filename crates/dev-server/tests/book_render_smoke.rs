//! Book-render smoke test — spawns `mdbook serve` against the wisp
//! book and the screen book, asserts the rendered HTML for known
//! pages returns HTTP 200 with the expected content. This is the
//! integration test that confirms `just dev-book` / `just
//! dev-wisp-book` (and by extension `just dev-remote-book` over
//! Tailscale) keep working: if mdbook can serve the books locally,
//! the Tailscale Serve proxy will too.
//!
//! Skipped at runtime when `mdbook` isn't on PATH (Windows CI
//! doesn't install it). The skip uses an `eprintln!` so the reason
//! shows up in `cargo nextest run` output.
//!
//! This test pairs with `binary_smoke.rs`: the binary test catches
//! dev-server (storybook live-reload) regressions; this one catches
//! book-render regressions. Together they cover everything `just
//! dev` / `just dev-book` / Tailscale would expose.

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

fn free_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
    listener.local_addr().unwrap().port()
}

fn mdbook_on_path() -> bool {
    Command::new("mdbook")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn preprocessor_on_path() -> bool {
    Command::new("mdbook-preprocessor-cross")
        .arg("supports")
        .arg("html")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

struct ServerChild(Child);
impl Drop for ServerChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn smoke_test_book(book_dir: &Path, expect_paths: &[&str], expect_content: &[(&str, &str)]) {
    let port = free_port();
    let child = Command::new("mdbook")
        .arg("serve")
        .arg(book_dir)
        .arg("--hostname")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .current_dir(workspace_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn mdbook serve");
    let _guard = ServerChild(child);

    // mdbook serve does a full build before binding — generous budget.
    let deadline = Instant::now() + Duration::from_mins(1);
    let client = reqwest::Client::new();
    let root_url = format!("http://127.0.0.1:{port}/");
    let book_label = book_dir.display();
    loop {
        assert!(
            Instant::now() <= deadline,
            "mdbook serve for {book_label} never bound within 60 s"
        );
        if let Ok(resp) = client.get(&root_url).send().await
            && resp.status() == 200
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    for path in expect_paths {
        let url = format!("http://127.0.0.1:{port}{path}");
        let resp = client.get(&url).send().await.expect("GET");
        assert_eq!(
            resp.status(),
            200,
            "{book_label}: {url} returned {}",
            resp.status()
        );
    }

    for (path, needle) in expect_content {
        let url = format!("http://127.0.0.1:{port}{path}");
        let body = client
            .get(&url)
            .send()
            .await
            .expect("GET")
            .text()
            .await
            .expect("body");
        assert!(
            body.contains(needle),
            "{book_label}: {url} missing {needle:?} in body"
        );
    }
}

#[tokio::test]
async fn wisp_book_renders_known_pages() {
    if !mdbook_on_path() || !preprocessor_on_path() {
        eprintln!(
            "SKIP: mdbook or mdbook-preprocessor-cross not on PATH \
             (this is expected on Windows CI). Run `just preprocessor-build` \
             and `cargo install mdbook` locally to exercise this test."
        );
        return;
    }

    let book = workspace_root().join("_docs/wisp-book");
    smoke_test_book(
        &book,
        &[
            "/",
            "/intro.html",
            "/quickstart.html",
            "/wisp/overview.html",
            "/wisp/chunks/filter-blur.html",
        ],
        &[
            ("/intro.html", "Pixi-shaped"),
            ("/wisp/chunks/filter-blur.html", "Blur"),
        ],
    )
    .await;
}

#[tokio::test]
async fn screen_book_renders_known_pages() {
    if !mdbook_on_path() || !preprocessor_on_path() {
        eprintln!(
            "SKIP: mdbook or mdbook-preprocessor-cross not on PATH \
             (this is expected on Windows CI)."
        );
        return;
    }

    let book = workspace_root().join("_docs/book");
    smoke_test_book(
        &book,
        &[
            "/",
            "/intro.html",
            "/wisp-overview.html",
            "/playback/overview.html",
        ],
        &[("/wisp-overview.html", "Wisp at a glance")],
    )
    .await;
}
