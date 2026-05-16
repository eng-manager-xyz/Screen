//! Tray-icon raster regenerator (M-TRAY.0 / AUT-249).
//!
//! Renders the SVG source at `crates/app/icons/tray.svg` (a 22×22 black
//! filled circle) to three `HiDPI` PNG outputs:
//!
//! - `crates/app/icons/tray.png`    — 22×22 (1×)
//! - `crates/app/icons/tray@2x.png` — 44×44 (2×)
//! - `crates/app/icons/tray@3x.png` — 66×66 (3×)
//!
//! Pure std — no external crates. The output is a minimal 8-bit
//! greyscale + alpha PNG with the alpha channel carrying a
//! supersampled filled-circle SDF. Grey channel is zero everywhere so
//! macOS treats the image as a **template icon** and tints it to the
//! current menubar foreground colour (light/dark/active states).
//!
//! Why pure std: the regen example is a project-internal contributor
//! affordance; pulling `image` / `tiny-skia` / `png` into dev-deps
//! for one example burns workspace dep budget. The encoder uses
//! uncompressed DEFLATE blocks wrapped in a zlib stream — bigger
//! file size than zlib compression would give, but the inputs are
//! tiny (< 5 KB even at 3×) so the cost is irrelevant.
//!
//! Run:
//!
//! ```sh
//! cargo run -p screen-app --example regen-tray-icons
//! ```
//!
//! Idempotent — produces identical bytes for identical inputs.

#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    reason = "PNG dimensions + alpha quantization are bounded enough that the casts are demonstrably safe; example binary, not hot-path code"
)]

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let icons_dir = crate_dir.join("icons");

    // (file_name, output dimension in pixels)
    //
    // The SVG viewBox is 22×22 with a circle at (11, 11) radius 7.
    // The PNG sizes are the viewBox scaled by 1×/2×/3× for HiDPI.
    let outputs = [
        ("tray.png", 22u32),
        ("tray@2x.png", 44u32),
        ("tray@3x.png", 66u32),
    ];

    for (name, dim) in outputs {
        let pixels = render_filled_circle(dim);
        let png = encode_png_gray_alpha(dim, dim, &pixels);
        let path = icons_dir.join(name);
        std::fs::write(&path, &png)?;
        println!(
            "wrote {} ({}×{}, {} bytes)",
            path.display(),
            dim,
            dim,
            png.len()
        );
    }

    Ok(())
}

/// Render a centred filled black circle to a greyscale+alpha pixel
/// buffer, anti-aliased via 4×4 supersampling.
///
/// Output layout matches PNG's raw pre-filter stream: each row is
/// prefixed with a filter byte (always 0 = None filter), followed by
/// `width` pixels of (grey, alpha) pairs. Grey is always 0 so macOS
/// treats the image as a template icon.
fn render_filled_circle(dim: u32) -> Vec<u8> {
    // Mirror the SVG: viewBox 22×22, circle at (11, 11), radius 7.
    // Scale factor = dim / 22.
    let cx = 11.0_f64;
    let cy = 11.0_f64;
    let r = 7.0_f64;
    let scale = f64::from(dim) / 22.0_f64;
    let radius_px = r * scale;
    let radius_sq = radius_px * radius_px;
    let center_px_x = cx * scale;
    let center_px_y = cy * scale;

    let supersample: u32 = 4;
    let ss_f = f64::from(supersample);
    let samples_per_pixel = supersample * supersample;

    // 1 filter byte per row + 2 bytes per pixel (grey, alpha).
    let row_bytes = 1 + (dim as usize) * 2;
    let mut buf = Vec::with_capacity(row_bytes * dim as usize);

    for y in 0..dim {
        buf.push(0u8); // filter byte = None
        for x in 0..dim {
            let mut inside: u32 = 0;
            for sy in 0..supersample {
                for sx in 0..supersample {
                    let px = f64::from(x) + (f64::from(sx) + 0.5) / ss_f;
                    let py = f64::from(y) + (f64::from(sy) + 0.5) / ss_f;
                    let dx = px - center_px_x;
                    let dy = py - center_px_y;
                    if dx * dx + dy * dy <= radius_sq {
                        inside += 1;
                    }
                }
            }
            let alpha = (inside * 255) / samples_per_pixel;
            buf.push(0); // grey (always zero — template icon)
            buf.push(alpha as u8);
        }
    }

    buf
}

/// Encode an 8-bit grey+alpha pixel buffer as a PNG.
///
/// `pixels` is the raw filter-byte-prefixed stream produced by
/// [`render_filled_circle`] — i.e. PNG IDAT contents before zlib
/// wrapping.
fn encode_png_gray_alpha(width: u32, height: u32, pixels: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() + 128);

    // PNG signature.
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);

    // IHDR chunk: 13 bytes.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(4); // colour type — greyscale + alpha
    ihdr.push(0); // compression method (zlib/deflate)
    ihdr.push(0); // filter method (adaptive)
    ihdr.push(0); // interlace (none)
    write_chunk(&mut out, *b"IHDR", &ihdr);

    // IDAT chunk: zlib-wrapped uncompressed DEFLATE stream over `pixels`.
    let idat = zlib_uncompressed(pixels);
    write_chunk(&mut out, *b"IDAT", &idat);

    // IEND chunk: empty.
    write_chunk(&mut out, *b"IEND", &[]);

    out
}

/// Append one PNG chunk (length, type, data, CRC32) to `out`.
fn write_chunk(out: &mut Vec<u8>, chunk_type: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&chunk_type);
    out.extend_from_slice(data);

    // CRC covers the chunk type + data (not the length).
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&chunk_type);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// Wrap `data` in a minimum-viable zlib stream using uncompressed
/// DEFLATE blocks (BTYPE = 00). One zlib stream may contain multiple
/// stored blocks if the input exceeds 65535 bytes; for tray icons the
/// inputs are well below this so a single stored block suffices.
fn zlib_uncompressed(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 16);

    // zlib header: CMF = 0x78 (deflate, 32 KiB window), FLG = 0x01
    // (lowest compression level, no preset dict). 0x7801 mod 31 == 0,
    // which is the zlib-spec parity check.
    out.push(0x78);
    out.push(0x01);

    // Stored blocks, ≤ 65 535 bytes each.
    let mut offset = 0;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let block_len = remaining.min(0xFFFF);
        let is_last = (offset + block_len) == data.len();

        // Block header: BFINAL (bit 0) + BTYPE (bits 1-2) = 00 (stored).
        out.push(u8::from(is_last));

        // LEN + NLEN (one's-complement), both little-endian u16.
        let len = block_len as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());

        out.extend_from_slice(&data[offset..offset + block_len]);
        offset += block_len;
    }

    // Adler-32 of the original `data`, big-endian.
    out.extend_from_slice(&adler32(data).to_be_bytes());

    out
}

/// PNG CRC-32 (poly 0xEDB88320, init 0xFFFFFFFF, xorout 0xFFFFFFFF).
fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Adler-32 (zlib's stream checksum).
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_known_vector() {
        // IEND chunk's CRC is famously 0xAE426082 over the type bytes.
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
    }

    #[test]
    fn adler32_matches_known_vector() {
        // Adler-32 of "Wikipedia" == 0x11E60398 per the RFC's example.
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn encoded_png_starts_with_signature() {
        let pixels = render_filled_circle(22);
        let png = encode_png_gray_alpha(22, 22, &pixels);
        assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn encoded_png_ends_with_iend() {
        let pixels = render_filled_circle(22);
        let png = encode_png_gray_alpha(22, 22, &pixels);
        // Last 8 bytes are IEND's type + CRC (length 0 immediately before).
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn center_pixel_is_fully_opaque() {
        let pixels = render_filled_circle(22);
        // Row 11, column 11; each row is 1 filter byte + 22*2 pixel bytes.
        let row_bytes = 1 + 22 * 2;
        let row_start = 11 * row_bytes;
        let pixel_offset = row_start + 1 + 11 * 2;
        // grey, then alpha.
        assert_eq!(pixels[pixel_offset], 0);
        assert_eq!(pixels[pixel_offset + 1], 255);
    }

    #[test]
    fn corner_pixel_is_fully_transparent() {
        let pixels = render_filled_circle(22);
        let row_bytes = 1 + 22 * 2;
        // Pixel (0, 0): row 0, col 0 → after filter byte.
        let pixel_offset = 1;
        assert_eq!(pixels[pixel_offset], 0);
        assert_eq!(pixels[pixel_offset + 1], 0);

        // Pixel (21, 21): last row, last col.
        let last_row = 21 * row_bytes;
        let last_pixel = last_row + 1 + 21 * 2;
        assert_eq!(pixels[last_pixel], 0);
        assert_eq!(pixels[last_pixel + 1], 0);
    }
}
