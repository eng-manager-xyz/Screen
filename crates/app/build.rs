//! Build script — invokes `tauri_build` to bake the app context (icons,
//! `tauri.conf.json`) into the binary at compile time.

fn main() {
    tauri_build::build();
}
