//! The picture through macroquad: shades → palette colours → texture, scaled to the window
//! (whole-number scaling when "sharp" is chosen), and the window's full-screen switch.
use crate::domain::settings::Scale;
use crate::domain::touch::Rect;
use crate::ports::Window;
use gb_core::palette::{fb_to_rgba, Palette};
use gb_core::ppu::{H, W};
use macroquad::prelude::*;

pub struct MacroquadDisplay {
    tex: Texture2D,
    rgba: Vec<u8>,
}

impl MacroquadDisplay {
    pub fn new() -> MacroquadDisplay {
        let rgba = vec![0u8; W * H * 4];
        let tex = Texture2D::from_rgba8(W as u16, H as u16, &rgba);
        tex.set_filter(FilterMode::Nearest);
        MacroquadDisplay { tex, rgba }
    }

    /// Draw a Game Boy picture (shades 0-3) centred in `area` (None: the whole window).
    pub fn present(&mut self, screen: &[u8], palette: &Palette, scale: Scale, area: Option<Rect>) {
        fb_to_rgba(screen, palette, &mut self.rgba);
        self.tex.update_from_bytes(W as u32, H as u32, &self.rgba);
        clear_background(Color::from_hex(0x101010));
        let area = area.unwrap_or(Rect::new(0.0, 0.0, screen_width(), screen_height()));
        let (sw, sh) = (area.w, area.h);
        let fit = (sw / W as f32).min(sh / H as f32);
        // whole screen pixels: on a phone a CSS pixel is several of them
        let dpi = screen_dpi_scale();
        let (w, h) = match scale {
            Scale::Stretched => (sw, sh),
            Scale::Fitted => (W as f32 * fit, H as f32 * fit),
            Scale::Sharp => {
                let whole = (fit * dpi).floor().max(1.0) / dpi;
                let whole = if whole * W as f32 > sw || whole * H as f32 > sh { fit } else { whole };
                (W as f32 * whole, H as f32 * whole)
            }
        };
        draw_texture_ex(
            &self.tex,
            (area.x + (sw - w) / 2.0).floor(),
            (area.y + (sh - h) / 2.0).floor(),
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() },
        );
    }

    /// A message instead of the game (no cartridge, unreadable file...).
    pub fn show_error(&self, text: &str) {
        clear_background(Color::from_hex(0x101010));
        for (i, line) in text.lines().enumerate() {
            draw_text(line, 20.0, 60.0 + i as f32 * 30.0, 26.0, WHITE);
        }
    }
}

/// Save what the window shows as a PNG (upright: export_png writes macroquad's bottom-up
/// screen data the right way round).
pub fn save_window_png(out: &std::path::Path) {
    let img = get_screen_data();
    if let Some(d) = out.parent() {
        let _ = host::storage::create_dir_all(d);
    }
    img.export_png(&out.to_string_lossy());
}

pub struct MacroquadWindow;

impl Window for MacroquadWindow {
    /// In the browser, the page's own fullscreen (it is allowed shortly after a key or a tap).
    fn set_fullscreen(&mut self, on: bool) {
        #[cfg(target_arch = "wasm32")]
        host::web::call(host::web::Call::SetFullscreen, if on { 1.0 } else { 0.0 });
        #[cfg(not(target_arch = "wasm32"))]
        set_fullscreen(on);
    }
}
