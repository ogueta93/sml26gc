//! Shared look and small pieces of the interface: colours, layout sizes, buttons, word
//! wrapping, and textures made from the game's captured graphics.
use cart_model::art::{LevelArt, Look, WorldArt};
use cart_model::levels::{COLS, ROWS};
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_pressed};

pub const PAL: [u32; 4] = [0xC4CFA1, 0x8B956D, 0x4D533C, 0x1F1F1F];
/// Explanations, notes and hints: a blue that reads well on the panels' light shade.
pub const INFO: Color = Color::new(0.10, 0.30, 0.61, 1.0);
/// The side panel's narrowest width; it can be dragged wider.
pub const PANEL_MIN: f32 = 300.0;
pub const STATUS_H: f32 = 26.0;
pub const BLOCK: f32 = 16.0;
pub const ZOOMS: [f32; 7] = [0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0];

pub fn col(i: usize) -> Color {
    let c = PAL[i];
    Color::from_rgba((c >> 16) as u8, (c >> 8) as u8, c as u8, 255)
}

/// Marks drawn over the game's pictures (objects, map points): colours none of the game's
/// four shades is near, each over a dark border.
pub const MARK: Color = Color::new(1.0, 0.25, 0.85, 1.0);
pub const MARK_SELECTED: Color = Color::new(1.0, 0.9, 0.1, 1.0);

/// A rectangle outline that shows on any picture: a dark border under a bright line.
pub fn marker(r: Rect, thick: f32, c: Color) {
    draw_rectangle_lines(r.x - 1.0, r.y - 1.0, r.w + 2.0, r.h + 2.0, thick + 2.0, Color::new(0.0, 0.0, 0.0, c.a));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, thick, c);
}

/// What kind of message the status says.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Info,
    /// Something was done that changed what is edited.
    Done,
    /// Something was not done, or needs a look, without harm.
    Warning,
    /// Something failed.
    Error,
}

impl Severity {
    pub fn colour(self) -> Color {
        match self {
            Severity::Info => col(3),
            Severity::Done => Color::new(0.08, 0.40, 0.14, 1.0),
            Severity::Warning => Color::new(0.80, 0.42, 0.0, 1.0),
            Severity::Error => Color::new(0.67, 0.16, 0.12, 1.0),
        }
    }

    /// A sign before the text, so the kind shows without the colour too.
    fn sign(self) -> &'static str {
        match self {
            Severity::Info => "",
            Severity::Done => "+ ",
            Severity::Warning => "! ",
            Severity::Error => "x ",
        }
    }
}

/// Text in a kind's colour and weight, without its sign (a message's further lines).
fn kind_text(text: &str, x: f32, y: f32, size: f32, kind: Severity) {
    let c = kind.colour();
    put_text(&text, x, y, size, c);
    if kind != Severity::Info {
        put_text(&text, x + 1.0, y, size, c);
    }
}

/// `t` cut into lines no wider than `width` pixels at `size` (a word wider than that has a
/// line of its own).
pub fn wrap_px(t: &str, width: f32, size: f32) -> Vec<String> {
    lines_of(t, |s| text_size(s, size as u16).width <= width)
}

/// Is `c` written without spaces between words (Japanese), so a line may break before it?
/// A character as wide as two Latin ones: Japanese, Chinese and Korean.
fn wide(c: char) -> bool {
    ('\u{3000}'..='\u{9FFF}').contains(&c) || ('\u{FF00}'..='\u{FFEF}').contains(&c) || ('\u{AC00}'..='\u{D7AF}').contains(&c) || ('\u{1100}'..='\u{11FF}').contains(&c)
}

/// Japanese punctuation that never begins a line: it stays with what comes before it.
fn closes(c: char) -> bool {
    "、。，．）」』】ー！？：".contains(c)
}

/// `t` cut into lines that `fits` accepts, at spaces and between Japanese characters (a piece
/// that never fits has a line of its own).
fn lines_of(t: &str, fits: impl Fn(&str) -> bool) -> Vec<String> {
    // the pieces a line may break between, each with whether a space goes before it
    let mut pieces: Vec<(String, bool)> = Vec::new();
    for (i, word) in t.split(' ').enumerate() {
        let mut space = i > 0;
        let mut cur = String::new();
        for c in word.chars() {
            if wide(c) && !closes(c) && !cur.is_empty() {
                pieces.push((std::mem::take(&mut cur), space));
                space = false;
            }
            cur.push(c);
        }
        pieces.push((cur, space));
    }
    let mut lines: Vec<String> = vec![String::new()];
    for (piece, space) in pieces {
        let last = lines.last_mut().expect("never empty");
        let tried = if last.is_empty() { piece.clone() } else if space { format!("{last} {piece}") } else { format!("{last}{piece}") };
        if !last.is_empty() && !fits(&tried) {
            lines.push(piece);
        } else {
            *last = tried;
        }
    }
    lines
}

/// A message on as many lines as it needs, from `y` (the first line's base) down; returns the
/// height it took.
pub fn message_lines(t: &str, x: f32, y: f32, width: f32, size: f32, kind: Severity) -> f32 {
    let step = size + 2.0;
    let lines = wrap_px(&format!("{}{t}", kind.sign()), width - 12.0, size);
    for (i, line) in lines.iter().enumerate() {
        kind_text(line, x, y + i as f32 * step, size, kind);
    }
    lines.len() as f32 * step
}

/// A status bar's message: one line in the bar from `left` to `right`, the lines it does not
/// fit growing above the bar (over the view) on the same background.
pub fn status_message(t: &str, left: f32, right: f32, bar_top: f32, bar_h: f32, size: f32, kind: Severity) {
    let width = right - left - 4.0;
    let lines = wrap_px(&format!("{}{t}", kind.sign()), width - 12.0, size);
    let extra = (lines.len().max(1) - 1) as f32 * (size + 2.0);
    if extra > 0.0 {
        draw_rectangle(left, bar_top - extra, right - left, extra, col(1));
        draw_rectangle(left, bar_top - extra, right - left, 1.0, col(2));
    }
    let base = bar_top + bar_h / 2.0 + size / 2.0 - 4.0 - extra;
    message_lines(t, left + 8.0, base, width, size, kind);
}

/// Text with a dark shadow, readable on any picture.
pub fn marked_text(t: &str, x: f32, y: f32, size: f32, c: Color) {
    put_text(t, x + 1.0, y + 1.0, size, Color::new(0.0, 0.0, 0.0, c.a));
    put_text(t, x, y, size, c);
}

pub use crate::texts::{tr, zone_name, zone_short};
pub use i18n::Lang;
use fonts::Face;

/// Split text into lines of at most `n` characters at spaces.
pub fn wrap(t: &str, n: usize) -> Vec<String> {
    // a Japanese character takes the room of two
    lines_of(t, |s| s.chars().map(|c| if wide(c) { 2 } else { 1 }).sum::<usize>() <= n)
}

thread_local! {
    /// The font of the language in use (`fonts::Face`), and the fonts loaded so far: each the
    /// first time a language needs it.
    static FACE: std::cell::Cell<Face> = const { std::cell::Cell::new(Face::BuiltIn) };
    static FONTS: std::cell::RefCell<Vec<(Face, Option<Font>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The texts from now on are in `lang`, drawn with its font (Japanese, Chinese, Korean and
/// Russian have their own).
pub fn set_text_language(lang: Lang) {
    FACE.with(|f| f.set(Face::of(lang)));
}

/// `f` with the texts in `lang`'s font, then back to the font in use.
pub fn with_text_language<R>(lang: Lang, f: impl FnOnce() -> R) -> R {
    let current = FACE.with(|f| f.replace(Face::of(lang)));
    let r = f();
    FACE.with(|f| f.set(current));
    r
}

/// The font texts are drawn with: None, the built-in one.
fn with_font<R>(f: impl FnOnce(Option<&Font>) -> R) -> R {
    let face = FACE.with(|f| f.get());
    FONTS.with(|fonts| {
        let mut fonts = fonts.borrow_mut();
        if !fonts.iter().any(|e| e.0 == face) {
            // in the browser, until the font has come: the built-in one
            let Some(bytes) = host::font_files::bytes(face) else { return f(None) };
            let font = load_ttf_font_from_bytes(&bytes).ok();
            warm(font.as_ref());
            fonts.push((face, font));
        }
        f(fonts.iter().find(|e| e.0 == face).and_then(|e| e.1.as_ref()))
    })
}

/// The sizes the editor writes in, those labels shrink to fit included.
const TEXT_SIZES: std::ops::RangeInclusive<u16> = 8..=26;

/// `font`'s letters measured in every size, before anything is drawn with it: macroquad makes
/// a font's glyph texture anew when it fills up, and text already drawn in that frame would
/// point at the old one (the browser logs "glBindTexture called with an already deleted
/// texture"; on macOS the texture could even stay blank, `inset::seal_drawing`).
fn warm(font: Option<&Font>) {
    let ascii: String = (' '..='~').collect();
    for size in TEXT_SIZES {
        measure_text(&ascii, font, size, 1.0);
    }
}

/// Draw text at `(x, y)` (its baseline) in the language's font.
pub fn put_text(t: &str, x: f32, y: f32, size: f32, c: Color) -> TextDimensions {
    let d = with_font(|font| draw_text_ex(t, x, y, TextParams { font, font_size: size as u16, color: c, ..Default::default() }));
    offer_cut_tip(Rect::new(x, y - d.offset_y, d.width, d.height.max(size)), t);
    d
}

/// How big text is at `size` in the language's font.
pub fn text_size(t: &str, size: u16) -> TextDimensions {
    with_font(|font| measure_text(t, font, size, 1.0))
}

thread_local! {
    /// The texts `fit` cut this frame (as shown, whole), and the whole one under the mouse.
    static CUT: std::cell::RefCell<Vec<(String, String)>> = const { std::cell::RefCell::new(Vec::new()) };
    static CUT_TIP: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// `t` cut to fit `w` pixels at `size` (ending in ".."). A cut text shows whole in a tooltip
/// while the mouse is on it, wherever it is drawn (`put_text`, `button`).
pub fn fit(t: &str, w: f32, size: f32) -> String {
    let width = |s: &str| text_size(s, size as u16).width;
    if width(t) <= w {
        return t.to_string();
    }
    let mut s = t.to_string();
    while !s.is_empty() && width(&format!("{s}..")) > w {
        s.pop();
    }
    let shown = format!("{s}..");
    CUT.with(|c| c.borrow_mut().push((shown.clone(), t.to_string())));
    shown
}

/// With the mouse on `r`, which shows `text`: the whole of a cut text in it becomes the
/// frame's tooltip.
fn offer_cut_tip(r: Rect, text: &str) {
    if !r.contains(mouse_position().into()) {
        return;
    }
    let whole = CUT.with(|c| c.borrow().iter().rev().find(|(shown, _)| text.contains(shown.as_str())).map(|(_, w)| w.clone()));
    if whole.is_some() {
        CUT_TIP.with(|t| *t.borrow_mut() = whole);
    }
}

/// The whole text cut under the mouse this frame, if any.
pub fn take_cut_tip() -> Option<String> {
    CUT_TIP.with(|t| t.borrow_mut().take())
}

/// A new frame: no texts cut yet.
pub fn new_frame_texts() {
    CUT.with(|c| c.borrow_mut().clear());
    CUT_TIP.with(|t| *t.borrow_mut() = None);
}

/// The tag of a level or zone made in the editor: a stripe along its button's foot.
pub fn user_tag(r: Rect, active: bool) {
    draw_rectangle(r.x + 3.0, r.y + r.h - 5.0, r.w - 6.0, 3.0, if active { col(1) } else { col(2) });
}

/// The line under a picker that says what the tag means; returns its height.
pub fn user_tag_legend(x: f32, y: f32, text: &str) -> f32 {
    user_tag(Rect::new(x, y, 22.0, 12.0), false);
    put_text(text, x + 28.0, y + 11.0, 13.0, INFO);
    16.0
}

/// A tooltip with `t` by the mouse, over everything.
pub fn draw_tooltip(t: &str) {
    let m: Vec2 = mouse_position().into();
    let d = text_size(t, 15);
    let (w, h) = (d.width + 12.0, 22.0);
    let x = (m.x + 14.0).min(screen_width() - w - 4.0);
    let y = (m.y + 18.0).min(screen_height() - h - 4.0);
    draw_rectangle(x, y, w, h, col(3));
    draw_rectangle_lines(x, y, w, h, 1.0, col(1));
    put_text(t, x + 6.0, y + 15.0, 15.0, col(0));
}

/// Draw a button, its label made smaller if it doesn't fit (and cut, whole in a tooltip, if it
/// still doesn't); true when it was clicked this frame.
pub fn button(r: Rect, label: &str, active: bool, size: f32) -> bool {
    let hover = r.contains(mouse_position().into());
    let (bg, fg) = if active { (col(3), col(0)) } else if hover { (col(1), col(3)) } else { (col(0), col(3)) };
    framed_label(r, label, bg, fg, col(3), 2.0, size);
    offer_cut_tip(r, label);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// A button set apart from the choices beside it (dark, with a thin frame): for the actions
/// under a list of things to choose. Unavailable: faded, and a click does nothing.
pub fn action_button(r: Rect, label: &str, size: f32, available: bool) -> bool {
    let hover = r.contains(mouse_position().into());
    if !available {
        framed_label(r, label, col(1), col(2), col(2), 1.0, size);
        offer_cut_tip(r, label);
        return false;
    }
    let bg = if hover { col(3) } else { col(2) };
    framed_label(r, label, bg, col(0), col(3), 1.0, size);
    offer_cut_tip(r, label);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// A button that does nothing now: faded, its label still readable.
pub fn unavailable_button(r: Rect, label: &str, size: f32) {
    framed_label(r, label, col(0), col(1), col(1), 2.0, size);
    offer_cut_tip(r, label);
}

/// The text lands on whole pixels: the font blurs between them.
fn framed_label(r: Rect, label: &str, bg: Color, fg: Color, frame: Color, thick: f32, size: f32) {
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, thick, frame);
    let mut size = size;
    let mut dim = text_size(label, size as u16);
    while dim.width > r.w - 6.0 && size > 8.0 {
        size -= 1.0;
        dim = text_size(label, size as u16);
    }
    let cut;
    let label = if dim.width > r.w - 6.0 {
        cut = fit(label, r.w - 6.0, size);
        dim = text_size(&cut, size as u16);
        cut.as_str()
    } else {
        label
    };
    let x = (r.x + (r.w - dim.width) / 2.0).round();
    let y = (r.y + (r.h + dim.offset_y) / 2.0 - 1.0).round();
    put_text(label, x, y, size, fg);
}

fn texture(img: &Image) -> Texture2D {
    let t = Texture2D::from_image(img);
    t.set_filter(FilterMode::Nearest);
    t
}

/// An object's picture (transparent where it draws nothing).
pub fn look_texture(l: &Look) -> Texture2D {
    let mut img = Image::gen_image_color(l.w as u16, l.h as u16, Color::new(0.0, 0.0, 0.0, 0.0));
    for y in 0..l.h {
        for x in 0..l.w {
            if let Some(s) = l.px[y * l.w + x] {
                img.set_pixel(x as u32, y as u32, col(s as usize));
            }
        }
    }
    texture(&img)
}

/// One pixel per block: the block's average shade, for the whole-level overview.
pub fn make_overview(art: &LevelArt, map: &[u8]) -> Texture2D {
    let mut mean = [0usize; 128];
    for id in 0..128u8 {
        let mut s = 0;
        for y in 0..16 {
            for x in 0..16 {
                s += art.block_pixel(id, x, y) as usize;
            }
        }
        mean[id as usize] = (s + 128) / 256;
    }
    let mut img = Image::gen_image_color(COLS as u16, ROWS as u16, BLACK);
    for (i, &b) in map.iter().enumerate() {
        img.set_pixel((i % COLS) as u32, (i / COLS) as u32, col(mean[b as usize & 0x7F].min(3)));
    }
    texture(&img)
}

/// The level's 128 blocks, 16 per row.
pub fn make_atlas(art: &LevelArt) -> Texture2D {
    let mut img = Image::gen_image_color(256, 128, BLACK);
    for id in 0..128u8 {
        for y in 0..16 {
            for x in 0..16 {
                let s = art.block_pixel(id, x, y) as usize;
                img.set_pixel((id as u32 % 16) * 16 + x as u32, (id as u32 / 16) * 16 + y as u32, col(s));
            }
        }
    }
    texture(&img)
}

/// The world map's 256 tiles, 16 per row.
pub fn make_world_atlas(art: &WorldArt) -> Texture2D {
    let mut img = Image::gen_image_color(128, 128, BLACK);
    for t in 0..=255u8 {
        for y in 0..8 {
            for x in 0..8 {
                img.set_pixel((t as u32 % 16) * 8 + x as u32, (t as u32 / 16) * 8 + y as u32, col(art.pixel(t, x, y) as usize));
            }
        }
    }
    texture(&img)
}
