//! macOS: renders the app icon (original artwork, `src/icon.rs`) as an `.iconset` folder, which `iconutil` turns into
//! the `.icns` that Praetorium.app shows in the Finder and the Dock. `tools/macos/build-app.sh` runs it:
//!
//! `cargo run --release -p nexus-app --example macos_iconset -- <folder>.iconset`
//!
//! The design sits the way macOS lays out app icons: on a rounded square with a margin around it (824 of 1024
//! points), so it lines up with the Dock's other icons instead of filling the whole tile.

use std::path::PathBuf;

#[allow(dead_code)]
#[path = "../src/icon.rs"]
mod icon;

/// The share of the tile the rounded square fills (Apple's app icon grid: 824 of 1024 points).
const FILL: f32 = 824.0 / 1024.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = std::env::args_os().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("Praetorium.iconset"));
    std::fs::create_dir_all(&dir)?;
    for points in [16u32, 32, 128, 256, 512] {
        for scale in [1u32, 2] {
            let n = points * scale;
            let name = if scale == 1 { format!("icon_{points}x{points}.png") } else { format!("icon_{points}x{points}@2x.png") };
            image::save_buffer(dir.join(name), &macos_rgba(n), n, n, image::ColorType::Rgba8)?;
        }
    }
    println!("{}", dir.display());
    Ok(())
}

/// Place the original command mark on the macOS icon grid.
fn macos_rgba(n: u32) -> Vec<u8> {
    let inner = ((n as f32 * FILL).round() as u32).max(1);
    let off = (n - inner) / 2;
    let raw = icon::rgba(inner);
    let mut out = vec![0u8; (n * n * 4) as usize];
    for y in 0..inner {
        let start = (y * inner * 4) as usize;
        let dest = (((y + off) * n + off) * 4) as usize;
        out[dest..dest + inner as usize * 4].copy_from_slice(&raw[start..start + inner as usize * 4]);
    }
    out
}
