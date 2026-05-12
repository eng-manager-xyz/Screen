//! One-time icon regenerator.
//!
//! Tauri's Windows build pipeline (`tauri-winres` → `winres`) needs
//! a `.ico` file at `crates/app/icons/icon.ico` to embed Windows
//! resources. Tauri's macro `generate_context!()` separately needs
//! a `.png` at `crates/app/icons/icon.png` for the binary embed.
//!
//! Both files are committed to the repo so CI never regenerates
//! them. This binary exists so a future contributor can regenerate
//! the `.ico` from a new `icon.png` without installing
//! `ImageMagick` / Pillow / sips:
//!
//! ```sh
//! cargo run -p screen-app --example regen-icons
//! ```
//!
//! Pure std — no external crates. We wrap the existing PNG bytes in
//! a minimal valid ICO container (PNG-in-ICO format, supported by
//! Windows Vista+ and winres). No image resizing — the dimensions
//! are read from the PNG's IHDR chunk and recorded in the ICO
//! directory entry.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Resolve paths from the crate's manifest dir so the example
    // works regardless of where `cargo run` is invoked from.
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let png_path = crate_dir.join("icons/icon.png");
    let ico_path = crate_dir.join("icons/icon.ico");

    let png = std::fs::read(&png_path)?;
    let (width, height) = parse_png_dimensions(&png)?;
    // ICO width/height are 1 byte each; 0 means 256.
    let w_byte = if width == 256 {
        0
    } else {
        u8::try_from(width)?
    };
    let h_byte = if height == 256 {
        0
    } else {
        u8::try_from(height)?
    };

    let mut ico = Vec::with_capacity(22 + png.len());
    // ICONDIR (6 bytes): reserved=0, type=1 (icon), count=1.
    ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);
    // ICONDIRENTRY (16 bytes):
    ico.push(w_byte); // width
    ico.push(h_byte); // height
    ico.push(0); // palette colour count (0 = none)
    ico.push(0); // reserved
    ico.extend_from_slice(&1u16.to_le_bytes()); // colour planes
    ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    ico.extend_from_slice(&u32::try_from(png.len())?.to_le_bytes()); // image data size
    ico.extend_from_slice(&22u32.to_le_bytes()); // image data offset (after the 22-byte header)
    // Image data (the PNG as-is).
    ico.extend_from_slice(&png);

    std::fs::write(&ico_path, &ico)?;
    println!(
        "wrote {} ({}×{}, {} bytes)",
        ico_path.display(),
        width,
        height,
        ico.len()
    );
    Ok(())
}

/// Read the width/height fields from a PNG file's IHDR chunk. PNG
/// dimensions are stored big-endian in bytes 16..24 of any valid
/// PNG file (8-byte signature + 4-byte chunk length + 4-byte type
/// `"IHDR"` + 4-byte width + 4-byte height).
fn parse_png_dimensions(png: &[u8]) -> Result<(u32, u32), Box<dyn std::error::Error>> {
    if png.len() < 24 || &png[0..8] != b"\x89PNG\r\n\x1a\n" {
        return Err("not a PNG".into());
    }
    if &png[12..16] != b"IHDR" {
        return Err("PNG missing IHDR chunk".into());
    }
    let width = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let height = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    Ok((width, height))
}
