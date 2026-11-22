//! Writing pictures as PNG files (screens, captures for checks and research).
use gb_core::palette::{fb_to_rgba, Palette, PALETTES};
use gb_core::{ppu, Gb};
use std::path::Path;

/// Write top-down RGBA pixels as a PNG.
pub fn write_rgba(path: &Path, width: usize, height: usize, rgba: &[u8]) -> Result<(), String> {
    if let Some(d) = path.parent() {
        let _ = crate::storage::create_dir_all(d);
    }
    let mut bytes = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut bytes, width as u32, height as u32);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    crate::storage::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// Write a 160x144 screen of shades with a palette.
pub fn write_screen(path: &Path, fb: &[u8], pal: &Palette) -> Result<(), String> {
    let mut rgba = vec![0u8; ppu::W * ppu::H * 4];
    fb_to_rgba(fb, pal, &mut rgba);
    write_rgba(path, ppu::W, ppu::H, &rgba)
}

/// Save the Game Boy's current screen in the original green-grey colours.
pub fn save_png(gb: &Gb, path: &Path) {
    if let Err(e) = write_screen(path, gb.screen(), &PALETTES[0]) {
        eprintln!("could not write {}: {e}", path.display());
    }
}
