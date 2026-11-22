//! The cartridge's own title screen for the start screen: the cartridge runs from power-on
//! with no buttons pressed (its sound thrown away), one Game Boy frame per window frame, so
//! the title moves as it does in the game.
use gb_core::palette::{fb_to_rgba, Palette};
use gb_core::ppu::{H, W};
use gb_core::{Buttons, Gb};
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

/// Frames from power-on until the title screen is up (research `titleframes`: from about 100).
const TO_TITLE: u32 = 110;

pub struct TitlePreview {
    gb: Gb,
    tex: Texture2D,
    rgba: Vec<u8>,
    palette: Palette,
    /// The cartridge file it runs.
    pub rom: PathBuf,
}

impl TitlePreview {
    /// The title of cartridge `rom`, None when it cannot be read or run.
    pub fn new(rom: &Path, palette: Palette) -> Option<TitlePreview> {
        let mut gb = Gb::new(host::storage::read(rom).ok()?, 44100).ok()?;
        for _ in 0..TO_TITLE {
            gb.run_frame_with(Buttons::NONE);
        }
        let rgba = vec![0u8; W * H * 4];
        let tex = Texture2D::from_rgba8(W as u16, H as u16, &rgba);
        tex.set_filter(FilterMode::Nearest);
        let mut p = TitlePreview { gb, tex, rgba, palette, rom: rom.to_path_buf() };
        p.show();
        Some(p)
    }

    /// One more frame of the title.
    pub fn step(&mut self) {
        self.gb.run_frame_with(Buttons::NONE);
        self.show();
    }

    fn show(&mut self) {
        fb_to_rgba(self.gb.screen(), &self.palette, &mut self.rgba);
        self.tex.update_from_bytes(W as u32, H as u32, &self.rgba);
    }

    pub fn texture(&self) -> &Texture2D {
        &self.tex
    }
}
