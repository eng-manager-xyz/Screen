//! List every display + window SCK can see (M-SCK.1 / AUT-268).
//!
//! Triggers the macOS Screen Recording permission prompt on first
//! run. Granting it once covers every SCK path.
//!
//! ```bash
//! cargo run -p media --example list_screen_sources
//! ```
//!
//! Acceptance criterion for AUT-268: prints every attached display
//! with `display-<id>` + dims, and every visible window with
//! `window-<id>` + title + owning app.

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!(
        "list_screen_sources is macOS-only — ScreenCaptureKit is not \
         available on this platform. Skipping."
    );
}

#[cfg(target_os = "macos")]
fn main() {
    println!("== Displays ==");
    match media::screen::list_displays() {
        Ok(displays) if displays.is_empty() => {
            eprintln!("no displays surfaced by SCK — unusual; check Screen Recording permission");
        }
        Ok(displays) => {
            for d in &displays {
                let tag = if d.is_primary { " (primary)" } else { "" };
                println!(
                    "  {tag:>10}  {label:<32}  id={id}",
                    tag = tag,
                    label = d.label,
                    id = d.id
                );
            }
        }
        Err(err) => {
            eprintln!("displays error: {err}");
            std::process::exit(1);
        }
    }

    println!();
    println!("== Visible windows ==");
    match media::screen::list_windows() {
        Ok(windows) if windows.is_empty() => {
            eprintln!("no visible windows surfaced by SCK");
        }
        Ok(windows) => {
            println!("(showing {} visible normal-layer windows)", windows.len());
            for w in &windows {
                let title = if w.label.is_empty() {
                    "(untitled)"
                } else {
                    &w.label
                };
                println!(
                    "  {dims:<12}  {app:<32}  {title}",
                    dims = format!("{}×{}", w.width, w.height),
                    app = w.display_name,
                    title = title,
                );
            }
        }
        Err(err) => {
            eprintln!("windows error: {err}");
            std::process::exit(1);
        }
    }
}
