//! The start screen when the game is opened without a cartridge on the command line. First the
//! base cartridge (`host::base_cartridge`): while there is no valid one, the screen only asks
//! for it. Then what to play, one at a time in a selector: VANILLA (the base cartridge) or a
//! project of the editor (`editor/projects/<name>/`, any that has its project file; its
//! cartridge is made from it over the base cartridge when missing or older,
//! `host::projects::ensure_rom`), each with its own saves, shown as a cartridge whose label
//! holds the cartridge's own title screen, running (`title_preview`). Choosing only selects:
//! Play starts the game, Edit opens the project in the editor.
use super::platform;
use super::title_preview::TitlePreview;
use crate::domain::settings::Settings;
use gb_core::palette::PALETTES;
use host::BaseProblem;
use macroquad::prelude::*;
use std::path::PathBuf;

/// Something to play: a cartridge and the folder its saves go to.
#[derive(Clone, Debug, PartialEq)]
pub struct Play {
    /// None: the original cartridge (VANILLA).
    pub project: Option<String>,
    pub rom: PathBuf,
}

impl Play {
    pub fn saves(&self) -> PathBuf {
        host::saves_dir(self.project.as_deref())
    }
}

pub enum Choice {
    /// Nothing yet.
    Wait,
    Play(Play),
    /// Open the system's dialog to choose the base cartridge.
    Browse,
    /// Turn the debug / cheats box on or off.
    Cheats,
    /// The language chosen in the list (already shown; to keep in the settings).
    Language(i18n::Lang),
    /// Open the editor on this project, or (None) on a new one.
    Edit(Option<String>),
    Quit,
}

/// What the selector says about a choice, read when the list is.
struct Card {
    kind: &'static str,
    name: String,
    about: String,
}

pub struct Picker {
    base: Result<PathBuf, BaseProblem>,
    plays: Vec<Play>,
    cards: Vec<Card>,
    pub selected: usize,
    /// Why the last choice didn't work.
    pub error: Option<String>,
    /// The "Debug / cheats" box.
    pub cheats: bool,
    lang: i18n::Lang,
    /// The language list, when open: the row the keys are on.
    languages: Option<usize>,
    /// Where the language button was last drawn (the list opens from it).
    language_at: Rect,
    /// When the list was last read (projects made in the editor meanwhile show up).
    listed: f64,
    /// The base cartridge's title screen, and the palette it is shown with.
    preview: Option<TitlePreview>,
    palette: usize,
    /// Where the top row's left part ends (the fullscreen button after it), this frame.
    title_end: f32,
}

const BG: Color = Color::new(0.06, 0.08, 0.1, 1.0);
const PANEL: Color = Color::new(0.11, 0.14, 0.15, 1.0);
const RAISED: Color = Color::new(0.16, 0.2, 0.19, 1.0);
const HOVER: Color = Color::new(0.21, 0.27, 0.23, 1.0);
const LINE: Color = Color::new(0.25, 0.3, 0.28, 1.0);
const TEXT: Color = Color::new(0.93, 0.95, 0.9, 1.0);
const DIM: Color = Color::new(0.58, 0.64, 0.58, 1.0);
const ACCENT: Color = Color::new(0.55, 0.78, 0.36, 1.0);
const ACCENT_DARK: Color = Color::new(0.08, 0.12, 0.06, 1.0);
const RED: Color = Color::new(0.95, 0.45, 0.4, 1.0);
/// The cartridge: its grey plastic, the label paper and its ink.
const CART: Color = Color::new(0.62, 0.63, 0.61, 1.0);
const CART_LIGHT: Color = Color::new(0.75, 0.76, 0.74, 1.0);
const CART_DARK: Color = Color::new(0.49, 0.5, 0.49, 1.0);
const CART_EDGE: Color = Color::new(0.33, 0.34, 0.33, 1.0);
const PAPER: Color = Color::new(0.93, 0.92, 0.87, 1.0);
const INK: Color = Color::new(0.12, 0.12, 0.12, 1.0);
/// The label's stripe for the original game (a project's is the accent).
const STRIPE_ORIGINAL: Color = Color::new(0.82, 0.29, 0.27, 1.0);
/// The cartridge's parts, top to bottom: the grip, the label's margin round the title, the
/// name under it, the foot; its sides; the cut top right corner.
const GRIP: f32 = 30.0;
const FRAME: f32 = 8.0;
const NAME: f32 = 38.0;
const FOOT: f32 = 18.0;
const SIDE: f32 = 22.0;
const CUT: f32 = 16.0;
const CART_TOP: f32 = 54.0;
/// Under the cartridge: its kind, its about and the dots.
const SELECTOR_TEXT: f32 = 74.0;
/// The top row's height (the language and fullscreen buttons), the side margins, a foot note's line.
const TOP_ROW: f32 = 30.0;
const MARGIN: f32 = 24.0;
const NOTE_LINE: f32 = 17.0;
/// The widest the content gets (centred in a larger window).
const COLUMN: f32 = 560.0;

/// A text's width at `size`.
fn width(t: &str, size: f32) -> f32 {
    with_font(|font| measure_text(t, font, size as u16, 1.0).width)
}

/// Each language's name written in itself, as the start screen's font for it draws it (the Latin
/// ones without accents, as the built-in font has none).
const LANGUAGE_NAMES: [&str; i18n::LANGS] = ["English", "Espanol", "日本語", "Portugues", "Francais", "Deutsch", "Italiano", "Bahasa Indonesia", "Русский", "简体中文", "한국어"];

/// The same names in Latin letters, while a language's font is not loaded (in the browser it is
/// downloaded when the language is chosen).
const LATIN_NAMES: [&str; i18n::LANGS] = ["English", "Espanol", "Nihongo", "Portugues", "Francais", "Deutsch", "Italiano", "Bahasa Indonesia", "Russkiy", "Zhongwen", "Hangugeo"];

/// The start screen's texts in every language.
const START_TEXTS: &str = include_str!("../../i18n/start.txt");

thread_local! {
    /// The fonts (`fonts::Face`), each loaded the first time it is drawn, and the one in use.
    static FONTS: std::cell::RefCell<Vec<(fonts::Face, Option<Font>)>> = const { std::cell::RefCell::new(Vec::new()) };
    static FACE: std::cell::Cell<fonts::Face> = const { std::cell::Cell::new(fonts::Face::BuiltIn) };
    /// The language list is open: the screen behind it takes no keys or clicks.
    static BLOCKED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Text from now on is drawn with `lang`'s font (the built-in one for the Latin languages).
/// A font still being downloaded draws with the built-in one until it arrives.
fn set_font(lang: i18n::Lang) {
    let face = fonts::Face::of(lang);
    FACE.set(face);
    FONTS.with(|f| {
        let mut f = f.borrow_mut();
        if !f.iter().any(|(had, _)| *had == face) {
            let font = match platform::font_bytes(face) {
                Some(bytes) => load_ttf_font_from_bytes(&bytes).ok(),
                None if face == fonts::Face::BuiltIn => None,
                None => return,
            };
            warm(font.as_ref());
            f.push((face, font));
        }
    });
}

/// The sizes the start screen writes in.
const TEXT_SIZES: [u16; 6] = [12, 14, 15, 16, 18, 20];

/// `font`'s letters measured in every size, before anything is drawn: macroquad makes a font's
/// glyph texture anew when it fills up, and text already drawn in that frame would point at
/// the old one (the browser logs "glBindTexture called with an already deleted texture").
fn warm(font: Option<&Font>) {
    let ascii: String = (' '..='~').collect();
    for size in TEXT_SIZES {
        measure_text(&ascii, font, size, 1.0);
    }
}

/// `lang`'s name, written in itself when its font is loaded (and drawn with it from now on).
fn language_name(lang: i18n::Lang) -> &'static str {
    let face = fonts::Face::of(lang);
    let loaded = FONTS.with(|f| f.borrow().iter().any(|(had, _)| *had == face)) || platform::font_ready(face);
    if loaded {
        set_font(lang);
        LANGUAGE_NAMES[lang as usize]
    } else {
        FACE.set(fonts::Face::BuiltIn);
        LATIN_NAMES[lang as usize]
    }
}

fn with_font<R>(f: impl FnOnce(Option<&Font>) -> R) -> R {
    let face = FACE.get();
    FONTS.with(|cell| f(cell.borrow().iter().find(|(had, _)| *had == face).and_then(|(_, font)| font.as_ref())))
}

fn blocked() -> bool {
    BLOCKED.get()
}

/// Key `k` was pressed, and the screen takes keys (no list open over it).
fn pressed(k: KeyCode) -> bool {
    !blocked() && is_key_pressed(k)
}

/// Text at `(x, y)` (its baseline) in the language's font.
fn put(t: &str, x: f32, y: f32, size: f32, c: Color) {
    with_font(|font| draw_text_ex(t, x, y, TextParams { font, font_size: size as u16, color: c, ..Default::default() }));
}

fn centred(t: &str, cx: f32, y: f32, size: f32, c: Color) {
    put(t, cx - width(t, size) / 2.0, y, size, c);
}

/// A button: its label left, its key at the right in a small box; true when clicked.
fn button(r: Rect, label: &str, key: &str, primary: bool, on: bool) -> bool {
    let m: Vec2 = mouse_position().into();
    let hover = on && !blocked() && r.contains(m);
    let (fill, fg) = match (primary && on, hover) {
        (true, false) => (ACCENT, ACCENT_DARK),
        (true, true) => (Color::new(0.64, 0.86, 0.44, 1.0), ACCENT_DARK),
        (false, true) => (HOVER, TEXT),
        (false, false) => (RAISED, if on { TEXT } else { DIM }),
    };
    draw_rectangle(r.x, r.y, r.w, r.h, fill);
    if !primary {
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, if on { LINE } else { PANEL });
    }
    let size = if r.h >= 44.0 { 22.0 } else { 18.0 };
    let keys = !key.is_empty() && platform::has_keyboard();
    let kw = if keys { width(key, 14.0) + 12.0 } else { 0.0 };
    let room = r.w - kw - 28.0;
    let mut label = label.to_string();
    while width(&label, size) > room && label.pop().is_some() {}
    put(&label, r.x + 16.0, r.y + r.h / 2.0 + size * 0.35, size, fg);
    if keys {
        let kx = r.x + r.w - kw - 10.0;
        let ky = r.y + (r.h - 20.0) / 2.0;
        draw_rectangle_lines(kx, ky, kw, 20.0, 1.0, if primary && on { ACCENT_DARK } else { DIM });
        put(key, kx + 6.0, ky + 14.0, 14.0, if primary && on { ACCENT_DARK } else { DIM });
    }
    hover && is_mouse_button_pressed(MouseButton::Left)
}

/// The selector's round arrow buttons, and the room between them and the card.
const ARROW_R: f32 = 22.0;
const ARROW_GAP: f32 = 16.0;
/// The least room between an arrow and the cartridge, on a narrow screen.
const MIN_ARROW_GAP: f32 = 12.0;

/// A round arrow button pointing left (`-1`) or right (`1`); true when clicked.
fn arrow(cx: f32, cy: f32, dir: f32, on: bool) -> bool {
    let m: Vec2 = mouse_position().into();
    let hover = !blocked() && on && (m - vec2(cx, cy)).length() <= ARROW_R;
    draw_circle(cx, cy, ARROW_R, if hover { HOVER } else { RAISED });
    draw_circle_lines(cx, cy, ARROW_R, 1.0, LINE);
    let c = if on { TEXT } else { LINE };
    draw_triangle(vec2(cx + 7.0 * dir, cy), vec2(cx - 5.0 * dir, cy - 9.0), vec2(cx - 5.0 * dir, cy + 9.0), c);
    hover && is_mouse_button_pressed(MouseButton::Left)
}

impl Picker {
    pub fn new(settings: &Settings) -> Picker {
        let mut p = Picker { base: Err(BaseProblem::NotChosen), plays: Vec::new(), cards: Vec::new(), selected: 0, error: None, cheats: settings.cheats, lang: i18n::Lang::ALL.get(settings.lang).copied().unwrap_or_default(), languages: None, language_at: Rect::new(0.0, 0.0, 0.0, 0.0), listed: 0.0, preview: None, palette: settings.palette, title_end: 0.0 };
        p.reload(&settings.last_play);
        p
    }

    /// Check the base cartridge again and list what can be played, `last` chosen.
    pub fn reload(&mut self, last: &str) {
        self.listed = get_time();
        self.base = host::base_cartridge();
        self.plays.clear();
        if let Ok(base) = &self.base {
            self.plays.push(Play { project: None, rom: base.clone() });
            for name in host::projects().into_iter().filter(|n| host::projects::playable(n)) {
                let rom = host::project_rom_in(&name).unwrap_or_else(|| host::project_rom(&name, base));
                self.plays.push(Play { project: Some(name), rom });
            }
        }
        self.cards = self.plays.iter().map(|p| self.card(p)).collect();
        match &self.base {
            Ok(base) if self.preview.as_ref().is_none_or(|p| p.rom != *base) => {
                self.preview = TitlePreview::new(base, PALETTES[self.palette % PALETTES.len()]);
            }
            Ok(_) => {}
            Err(_) => self.preview = None,
        }
        self.selected = self.plays.iter().position(|p| p.project.as_deref().unwrap_or("") == last).unwrap_or(0);
    }

    fn card(&self, p: &Play) -> Card {
        let Some(name) = &p.project else {
            return Card { kind: self.tr("THE ORIGINAL GAME"), name: host::VANILLA.into(), about: self.tr("Super Mario Land 2 as it came, with its own saves").into() };
        };
        let file = host::project_file(name);
        let text = host::storage::read_to_string(&file).unwrap_or_default();
        // the project file's third line counts its levels (the game's 32 and the new ones)
        let new_levels = text.lines().nth(2).and_then(|l| l.strip_prefix("levels ")).and_then(|n| n.trim().parse::<usize>().ok()).map(|n| n.saturating_sub(32));
        let saved = host::storage::modified(&file).map(|t| self.ago(t));
        let mut about = Vec::new();
        if let Some(n) = new_levels.filter(|&n| n > 0) {
            about.push(format!("{n} {}", if n == 1 { self.tr("new level") } else { self.tr("new levels") }));
        }
        if let Some(s) = saved {
            about.push(s);
        }
        Card { kind: self.tr("EDITOR PROJECT"), name: name.clone(), about: about.join("  -  ") }
    }

    fn ago(&self, t: std::time::SystemTime) -> String {
        let s = host::clock::now().duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
        let (n, text) = match s {
            0..=59 => return self.tr("saved just now").into(),
            60..=3599 => (s / 60, self.tr("saved %n min ago")),
            3600..=86399 => (s / 3600, self.tr("saved %n h ago")),
            _ => (s / 86400, self.tr("saved %n days ago")),
        };
        text.replace("%n", &n.to_string())
    }

    /// The language button: its name written in itself and a small arrow; a click (or L) opens
    /// the list of the languages.
    fn language_button(&mut self, r: Rect) {
        self.language_at = r;
        let name = language_name(self.lang);
        set_font(self.lang);
        let clicked = button(r, name, "L", false, true);
        let ax = r.x + r.w - width("L", 14.0) - 40.0;
        let ay = r.y + r.h / 2.0;
        draw_triangle(vec2(ax - 5.0, ay - 3.0), vec2(ax + 5.0, ay - 3.0), vec2(ax, ay + 3.0), DIM);
        if clicked || pressed(KeyCode::L) {
            self.languages = Some(self.lang as usize);
        }
    }

    /// The open language list, over the screen, under its button (or over it when there is no
    /// room below): each language written in itself; a click or Enter chooses, Esc, L or a click
    /// outside closes it.
    fn language_list(&mut self, screen_h: f32) -> Choice {
        let Some(mut at) = self.languages else { return Choice::Wait };
        const ROW: f32 = 30.0;
        let b = self.language_at;
        let h = ROW * i18n::LANGS as f32 + 8.0;
        let y = if b.y + b.h + 4.0 + h <= screen_h { b.y + b.h + 4.0 } else { b.y - 4.0 - h };
        let list = Rect::new(b.x, y, b.w, h);
        let m: Vec2 = mouse_position().into();
        if is_key_pressed(KeyCode::Down) {
            at = (at + 1) % i18n::LANGS;
        }
        if is_key_pressed(KeyCode::Up) {
            at = (at + i18n::LANGS - 1) % i18n::LANGS;
        }
        let row_at = |i: usize| Rect::new(list.x + 4.0, list.y + 4.0 + i as f32 * ROW, list.w - 8.0, ROW);
        if mouse_delta_position() != Vec2::ZERO {
            if let Some(i) = (0..i18n::LANGS).find(|&i| row_at(i).contains(m)) {
                at = i;
            }
        }
        draw_rectangle(list.x + 4.0, list.y + 6.0, list.w, list.h, Color::new(0.0, 0.0, 0.0, 0.4));
        draw_rectangle(list.x, list.y, list.w, list.h, PANEL);
        draw_rectangle_lines(list.x, list.y, list.w, list.h, 1.0, LINE);
        for (i, lang) in i18n::Lang::ALL.into_iter().enumerate() {
            let r = row_at(i);
            if i == at {
                draw_rectangle(r.x, r.y, r.w, r.h, HOVER);
            }
            let name = language_name(lang);
            put(name, r.x + 12.0, r.y + r.h / 2.0 + 6.0, 18.0, if lang == self.lang { ACCENT } else { TEXT });
        }
        set_font(self.lang);
        self.languages = Some(at);
        let click = is_mouse_button_pressed(MouseButton::Left);
        if (click && row_at(at).contains(m)) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            self.languages = None;
            self.lang = i18n::Lang::ALL[at];
            return Choice::Language(self.lang);
        }
        if (click && !list.contains(m)) || is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::L) {
            self.languages = None;
        }
        Choice::Wait
    }

    /// The language of the texts from `host`: the game's.
    pub fn lang(&self) -> i18n::Lang {
        self.lang
    }

    /// The cartridge to play: the base one, or the project's (made when needed).
    pub fn ready(&self, play: &Play) -> Result<Play, String> {
        let (Some(name), Ok(base)) = (&play.project, &self.base) else { return Ok(play.clone()) };
        host::projects::ensure_rom(base, name).map(|rom| Play { project: play.project.clone(), rom }).map_err(|e| e.text(self.lang()))
    }

    /// `en` in the game's language (`game/i18n/start.txt`).
    fn tr(&self, en: &'static str) -> &'static str {
        static CATALOGUE: std::sync::OnceLock<i18n::Dictionary> = std::sync::OnceLock::new();
        CATALOGUE.get_or_init(|| i18n::Dictionary::parse(START_TEXTS).expect("the start screen's catalogue")).get(self.lang, en)
    }

    /// Where the "Open" dialog starts: the folder of the cartridge chosen before.
    pub fn start_dir(&self) -> Option<PathBuf> {
        host::base_cartridge::chosen_before().and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    pub fn dialog_texts(&self) -> (&'static str, &'static str) {
        (self.tr("Choose the Super Mario Land 2 cartridge"), self.tr("Game Boy cartridges"))
    }

    /// Draw the screen and read the keys and mouse.
    pub fn frame(&mut self) -> Choice {
        set_font(self.lang);
        clear_background(BG);
        let (w, h) = (screen_width(), screen_height());
        let col_w = (w - 2.0 * MARGIN).min(COLUMN);
        let x = (w - col_w) / 2.0;
        // the top row: the title, the fullscreen button at the right
        let fullscreen = platform::can_fullscreen().then(|| Rect::new(x + col_w - TOP_ROW, 10.0, TOP_ROW, TOP_ROW));
        self.title_end = x + col_w - fullscreen.map_or(0.0, |r| r.w + 10.0);
        if let Some(r) = fullscreen {
            fullscreen_button(r);
        }
        platform::place_page_button(platform::PageButton::Fullscreen, fullscreen.map(|r| crate::domain::touch::Rect::new(r.x, r.y, r.w, r.h)));
        let open = self.languages.is_some();
        let coming = platform::cartridge_coming();
        BLOCKED.set(open || coming);
        let mut choice = match self.base.clone() {
            Err(problem) => self.cartridge_screen(&problem, x, col_w, h),
            Ok(base) => self.play_screen(&base, w, h),
        };
        BLOCKED.set(false);
        if coming {
            platform::place_page_button(platform::PageButton::Browse, None);
            self.loading(w, h);
            return Choice::Wait;
        }
        if let Some(e) = &self.error {
            let lines: Vec<String> = e.lines().flat_map(|l| wrap(l, col_w, 16.0)).collect();
            let top = h - 44.0 - lines.len() as f32 * 19.0;
            for (i, line) in lines.iter().enumerate() {
                put(line, x, top + i as f32 * 19.0, 16.0, RED);
            }
        }
        if open {
            choice = self.language_list(h);
        }
        choice
    }

    /// Over the screen while a cartridge file is on its way (chosen in the browser, being read):
    /// a turning wheel and what it waits for.
    fn loading(&self, w: f32, h: f32) {
        draw_rectangle(0.0, 0.0, w, h, Color::new(BG.r, BG.g, BG.b, 0.85));
        let text = self.tr("Loading the cartridge...");
        let panel_w = (width(text, 20.0) + 64.0).min(w - 2.0 * MARGIN);
        let panel = Rect::new((w - panel_w) / 2.0, h / 2.0 - 70.0, panel_w, 140.0);
        draw_rectangle(panel.x, panel.y, panel.w, panel.h, PANEL);
        draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 1.0, LINE);
        let (cx, cy) = (w / 2.0, panel.y + 50.0);
        let turn = (get_time() * 10.0) as usize;
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::FRAC_PI_4;
            let lit = (turn + 8 - i) % 8;
            let c = Color::new(ACCENT.r, ACCENT.g, ACCENT.b, 1.0 - lit as f32 * 0.11);
            draw_circle(cx + 18.0 * a.cos(), cy + 18.0 * a.sin(), 4.0, c);
        }
        centred(text, cx, panel.y + 112.0, 20.0, TEXT);
    }

    /// The title at the top left, where there is room for it before `end`.
    fn title(&self, x: f32, end: f32) {
        const TITLE: &str = "SUPER MARIO LAND 2";
        if x + width(TITLE, 16.0) + 10.0 <= end {
            put(TITLE, x, 31.0, 16.0, ACCENT);
        }
    }

    /// The lines at the screen's foot, `col_w` wide: in the browser, where the files are kept
    /// and, on an iPhone, how to play without the browser's bars; then `last`.
    fn foot_notes(&self, last: Option<String>, col_w: f32) -> Vec<(String, Color)> {
        let mut notes = Vec::new();
        if platform::keeps_files_in_browser() {
            notes.push(self.tr("Kept in this browser only: clearing its data for this site deletes them."));
        }
        if platform::home_screen_hint() {
            notes.push(self.tr("To play without the browser's bars: Share, then Add to Home Screen."));
        }
        let mut lines: Vec<(String, Color)> = notes.into_iter().flat_map(|n| wrap(n, col_w, 14.0)).map(|l| (l, DIM)).collect();
        lines.extend(last.into_iter().flat_map(|n| wrap(&n, col_w, 14.0)).map(|l| (l, DIM)));
        lines
    }

    fn draw_foot_notes(lines: &[(String, Color)], x: f32, h: f32) {
        for (i, (line, c)) in lines.iter().enumerate() {
            put(line, x, h - 16.0 - (lines.len() - 1 - i) as f32 * NOTE_LINE, 14.0, *c);
        }
    }

    /// No usable base cartridge: why, and choosing one. Shorter on a low screen (a phone held
    /// across).
    fn cartridge_screen(&mut self, problem: &BaseProblem, x: f32, col_w: f32, h: f32) -> Choice {
        self.title(x, self.title_end);
        let notes = self.foot_notes(None, col_w);
        let low = h < 480.0 + notes.len() as f32 * NOTE_LINE;
        let (title_y, title_size) = if low { (66.0, 24.0) } else { (82.0, 34.0) };
        put(self.tr("Base cartridge"), x, title_y, title_size, TEXT);
        let lines: Vec<String> = problem.text(self.lang()).lines().flat_map(|l| wrap(l, col_w - 40.0, 18.0)).collect();
        let note = wrap(self.tr("The game and the editor share it; the file may have any name."), col_w - 40.0, 15.0);
        let card_h = if low { 30.0 + 23.0 * lines.len() as f32 + 18.0 * note.len() as f32 } else { 150.0_f32.max(40.0 + 23.0 * lines.len() as f32 + 18.0 * note.len() as f32) };
        let card = Rect::new(x, title_y + 26.0, col_w, card_h);
        draw_rectangle(card.x, card.y, card.w, card.h, PANEL);
        draw_rectangle_lines(card.x, card.y, card.w, card.h, 1.0, LINE);
        let mut y = card.y + if low { 26.0 } else { 34.0 };
        let c = if *problem == BaseProblem::NotChosen { TEXT } else { RED };
        for line in &lines {
            put(line, x + 20.0, y, 18.0, c);
            y += 23.0;
        }
        // the note at the card's foot, its lines stacked up from there
        for (i, line) in note.iter().enumerate() {
            put(line, x + 20.0, card.y + card.h - if low { 14.0 } else { 18.0 } - (note.len() - 1 - i) as f32 * 18.0, 15.0, DIM);
        }
        let mut choice = Choice::Wait;
        let gap = if low { 12.0 } else { 24.0 };
        let browse = Rect::new(x, card.y + card.h + gap, col_w, 48.0);
        platform::place_page_button(platform::PageButton::Browse, Some(crate::domain::touch::Rect::new(browse.x, browse.y, browse.w, browse.h)));
        if button(browse, self.tr("Choose the cartridge file..."), "O", true, true) || pressed(KeyCode::O) || pressed(KeyCode::Enter) {
            choice = Choice::Browse;
        }
        let row = Rect::new(x, browse.y + browse.h + 10.0, col_w, if platform::has_keyboard() { 38.0 } else { 44.0 });
        let half = (col_w - 10.0) / 2.0;
        if platform::CAN_QUIT {
            self.language_button(Rect::new(row.x, row.y, half, row.h));
            if button(Rect::new(x + half + 10.0, row.y, half, row.h), self.tr("Quit"), "Esc", false, true) || pressed(KeyCode::Escape) {
                choice = Choice::Quit;
            }
        } else {
            self.language_button(row);
        }
        Self::draw_foot_notes(&notes, x, h);
        choice
    }

    /// What to play: the selector, then its actions and the settings row. On a low screen held
    /// across, the selector at the left and the rest at the right.
    fn play_screen(&mut self, base: &std::path::Path, w: f32, h: f32) -> Choice {
        // every two seconds, to show projects made or renamed in the editor
        if get_time() - self.listed > 2.0 {
            let keep = self.plays.get(self.selected).and_then(|p| p.project.clone()).unwrap_or_default();
            self.reload(&keep);
        }
        let col_w = (w - 2.0 * MARGIN).min(COLUMN);
        let x = (w - col_w) / 2.0;
        let actions_h = 60.0 + if platform::can_edit() { 60.0 } else { 0.0 } + 34.0;
        let name = format!("{} {}", self.tr("Base cartridge:"), base.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
        let notes = self.foot_notes(Some(name.clone()), col_w);
        let notes_h = notes.len() as f32 * NOTE_LINE + 27.0;
        let stacked_room = h - CART_TOP - SELECTOR_TEXT - actions_h - notes_h;
        let across = w > h && cartridge_size(1.0).1 > stacked_room;
        // the language, at the top right (left of the fullscreen button)
        let lw = 220.0_f32.min(col_w / 2.0);
        self.language_button(Rect::new(self.title_end - lw, 10.0, lw, TOP_ROW));
        self.title(x, self.title_end - lw);
        if across {
            let full = (w - 2.0 * MARGIN).min(2.0 * COLUMN);
            let fx = (w - full) / 2.0;
            let left = (full - 20.0) / 2.0;
            let scale = cartridge_scale(h - CART_TOP - SELECTOR_TEXT, left);
            self.selector(fx, left, scale);
            let right = full - left - 20.0;
            let notes = self.foot_notes(Some(name), right);
            let choice = self.actions(fx + left + 20.0, CART_TOP + 10.0, right);
            Self::draw_foot_notes(&notes, fx + left + 20.0, h);
            choice
        } else {
            let scale = cartridge_scale(stacked_room, col_w);
            let below = self.selector(x, col_w, scale);
            let choice = self.actions(x, below + SELECTOR_TEXT, col_w);
            Self::draw_foot_notes(&notes, x, h);
            choice
        }
    }

    /// The cartridge chosen, `col_w` wide from `x`, its title `scale` times its size: arrows
    /// either side, its kind and about, a dot per choice. Gives the cartridge's foot.
    fn selector(&mut self, x: f32, col_w: f32, scale: f32) -> f32 {
        let n = self.plays.len();
        let cx = x + col_w / 2.0;
        let (cart_w, cart_h) = cartridge_size(scale);
        let cart = Rect::new((cx - cart_w / 2.0).round(), CART_TOP, cart_w, cart_h);
        let wheel = if blocked() { 0.0 } else { mouse_wheel().1 };
        let step = |d: i32, s: &mut usize| *s = (*s as i32 + d).rem_euclid(n as i32) as usize;
        let mid = cart.y + cart.h / 2.0;
        // beside the cartridge, closer to it where the column is narrow, never over it
        let left = (cart.x - ARROW_GAP - ARROW_R).max(x + ARROW_R).min(cart.x - MIN_ARROW_GAP - ARROW_R);
        let right = (cart.x + cart.w + ARROW_GAP + ARROW_R).min(x + col_w - ARROW_R).max(cart.x + cart.w + MIN_ARROW_GAP + ARROW_R);
        if arrow(left, mid, -1.0, n > 1) || pressed(KeyCode::Left) || pressed(KeyCode::Up) || (wheel > 0.0) {
            step(-1, &mut self.selected);
        }
        if arrow(right, mid, 1.0, n > 1) || pressed(KeyCode::Right) || pressed(KeyCode::Down) || (wheel < 0.0) {
            step(1, &mut self.selected);
        }
        if let Some(p) = &mut self.preview {
            p.step();
        }
        let original = self.plays[self.selected].project.is_none();
        let c = &self.cards[self.selected];
        draw_cartridge(cart, scale, self.preview.as_ref().map(|p| p.texture()), &c.name, original);
        let below = cart.y + cart.h;
        centred(c.kind, cx, below + 24.0, 14.0, if original { STRIPE_ORIGINAL } else { ACCENT });
        if let Some(line) = wrap(&c.about, col_w, 15.0).first() {
            centred(line, cx, below + 44.0, 15.0, DIM);
        }
        if n == 1 {
            if let Some(line) = wrap(self.tr("Projects made in the editor show here too."), col_w, 14.0).first() {
                centred(line, cx, below + 64.0, 14.0, DIM);
            }
        } else {
            let dots = 10.0 * n as f32;
            for i in 0..n {
                let dx = cx - dots / 2.0 + 5.0 + i as f32 * 10.0;
                if i == self.selected {
                    draw_circle(dx, below + 59.0, 3.5, ACCENT);
                } else {
                    draw_circle_lines(dx, below + 59.0, 3.0, 1.0, DIM);
                }
            }
            let place = format!("{} / {n}", self.selected + 1);
            put(&place, x + col_w - width(&place, 14.0), below + 64.0, 14.0, DIM);
        }
        below
    }

    /// Play, editing, and the settings row, `col_w` wide from `(x, y)`.
    fn actions(&mut self, x: f32, y: f32, col_w: f32) -> Choice {
        let mut choice = Choice::Wait;
        let project = self.plays[self.selected].project.clone();
        let mut y = y;
        if button(Rect::new(x, y, col_w, 50.0), self.tr("Play"), "Enter", true, true) || pressed(KeyCode::Enter) || pressed(KeyCode::KpEnter) {
            choice = Choice::Play(self.plays[self.selected].clone());
        }
        y += 60.0;
        if platform::can_edit() {
            let half = (col_w - 10.0) / 2.0;
            let edit = if project.is_some() { self.tr("Edit in the editor") } else { self.tr("The original is not edited") };
            if (button(Rect::new(x, y, half, 40.0), edit, "E", false, project.is_some()) || pressed(KeyCode::E)) && project.is_some() {
                choice = Choice::Edit(project.clone());
            }
            if button(Rect::new(x + half + 10.0, y, half, 40.0), self.tr("New project"), "N", false, true) || pressed(KeyCode::N) {
                choice = Choice::Edit(None);
            }
            y += 60.0;
        }
        // the settings row, quieter
        draw_line(x, y - 12.0, x + col_w, y - 12.0, 1.0, LINE);
        let columns = if platform::CAN_QUIT { 3.0 } else { 2.0 };
        let cell = (col_w - 10.0 * (columns - 1.0)) / columns;
        let row_h = if platform::has_keyboard() { 34.0 } else { 40.0 };
        let cheats = format!("{} {}", if self.cheats { "[x]" } else { "[ ]" }, self.tr("Cheats"));
        if button(Rect::new(x, y, cell, row_h), &cheats, "C", false, true) || pressed(KeyCode::C) {
            choice = Choice::Cheats;
        }
        let browse = Rect::new(x + cell + 10.0, y, cell, row_h);
        platform::place_page_button(platform::PageButton::Browse, Some(crate::domain::touch::Rect::new(browse.x, browse.y, browse.w, browse.h)));
        if button(browse, self.tr("Cartridge..."), "O", false, true) || pressed(KeyCode::O) {
            choice = Choice::Browse;
        }
        if platform::CAN_QUIT && (button(Rect::new(x + 2.0 * (cell + 10.0), y, cell, row_h), self.tr("Quit"), "Esc", false, true) || pressed(KeyCode::Escape)) {
            choice = Choice::Quit;
        }
        choice
    }
}

/// The largest of the cartridge's sizes that fits `room_h` high and `room_w` wide with its arrows
/// beside it.
fn cartridge_scale(room_h: f32, room_w: f32) -> f32 {
    [2.0, 1.5, 1.0, 0.75, 0.6].into_iter().find(|&s| cartridge_size(s).1 <= room_h && cartridge_size(s).0 + 2.0 * (2.0 * ARROW_R + MIN_ARROW_GAP) <= room_w).unwrap_or(0.6)
}

/// The fullscreen button in `r`: the page enters or leaves fullscreen on a tap there.
fn fullscreen_button(r: Rect) {
    let m: Vec2 = mouse_position().into();
    let hover = !blocked() && r.contains(m);
    draw_rectangle(r.x, r.y, r.w, r.h, if hover { HOVER } else { RAISED });
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, LINE);
    super::macroquad_touch::fullscreen_icon(crate::domain::touch::Rect::new(r.x, r.y, r.w, r.h), platform::is_fullscreen(), TEXT);
}

/// The cartridge's width and height with its title screen `scale` times its size.
fn cartridge_size(scale: f32) -> (f32, f32) {
    (160.0 * scale + 2.0 * SIDE, GRIP + 144.0 * scale + 2.0 * FRAME + NAME + FOOT)
}

/// The cartridge's outline filled with `c`: a rectangle with its top right corner cut.
fn cartridge_shape(r: Rect, c: Color) {
    draw_rectangle(r.x, r.y, r.w - CUT, r.h, c);
    draw_rectangle(r.x + r.w - CUT, r.y + CUT, CUT, r.h - CUT, c);
    draw_triangle(vec2(r.x + r.w - CUT, r.y), vec2(r.x + r.w, r.y + CUT), vec2(r.x + r.w - CUT, r.y + CUT), c);
}

/// A cartridge in `r`: grip ridges, a recessed label with `title` (the title screen, `scale`
/// times its size) and `name` under it, its stripe red for the original game, and the foot.
fn draw_cartridge(r: Rect, scale: f32, title: Option<&Texture2D>, name: &str, original: bool) {
    cartridge_shape(Rect::new(r.x + 6.0, r.y + 8.0, r.w, r.h), Color::new(0.0, 0.0, 0.0, 0.35));
    cartridge_shape(r, CART);
    let corners = [vec2(r.x, r.y), vec2(r.x + r.w - CUT, r.y), vec2(r.x + r.w, r.y + CUT), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
    for (i, a) in corners.iter().enumerate() {
        let b = corners[(i + 1) % corners.len()];
        draw_line(a.x, a.y, b.x, b.y, 2.0, CART_EDGE);
    }
    draw_line(r.x + 2.0, r.y + 2.0, r.x + r.w - CUT - 1.0, r.y + 2.0, 1.0, CART_LIGHT);
    // the grip
    for i in 0..4 {
        let y = r.y + 8.0 + i as f32 * 5.0;
        draw_line(r.x + 18.0, y, r.x + r.w - CUT - 12.0, y, 1.0, CART_DARK);
        draw_line(r.x + 18.0, y + 1.0, r.x + r.w - CUT - 12.0, y + 1.0, 1.0, CART_LIGHT);
    }
    // the label in its recess
    let (tw, th) = (160.0 * scale, 144.0 * scale);
    let recess = Rect::new(r.x + 12.0, r.y + GRIP, r.w - 24.0, th + 2.0 * FRAME + NAME);
    draw_rectangle(recess.x, recess.y, recess.w, recess.h, CART_DARK);
    draw_line(recess.x, recess.y + recess.h, recess.x + recess.w, recess.y + recess.h, 1.0, CART_LIGHT);
    let paper = Rect::new(recess.x + 3.0, recess.y + 3.0, recess.w - 6.0, recess.h - 6.0);
    draw_rectangle(paper.x, paper.y, paper.w, paper.h, PAPER);
    draw_rectangle(paper.x, paper.y, paper.w, 3.0, if original { STRIPE_ORIGINAL } else { ACCENT });
    let (ix, iy) = ((r.x + (r.w - tw) / 2.0).round(), (recess.y + FRAME).round());
    draw_rectangle(ix - 2.0, iy - 2.0, tw + 4.0, th + 4.0, INK);
    match title {
        Some(t) => draw_texture_ex(t, ix, iy, WHITE, DrawTextureParams { dest_size: Some(vec2(tw, th)), ..Default::default() }),
        None => draw_rectangle(ix, iy, tw, th, Color::new(0.2, 0.22, 0.2, 1.0)),
    }
    let mut size = 22.0;
    while width(name, size) > paper.w - 16.0 && size > 12.0 {
        size -= 1.0;
    }
    let mut shown = name.to_string();
    while width(&shown, size) > paper.w - 16.0 && shown.pop().is_some() {}
    centred(&shown, r.x + r.w / 2.0, (iy + th + 2.0 + NAME / 2.0 + size * 0.35).round(), size, INK);
    // the foot, an arrow the way it goes in
    let (fx, fy) = (r.x + r.w / 2.0, r.y + r.h - FOOT / 2.0);
    draw_triangle(vec2(fx - 6.0, fy - 3.0), vec2(fx + 6.0, fy - 3.0), vec2(fx, fy + 4.0), CART_DARK);
}

/// Split text into lines that fit `width` pixels at font size `size` (breaking long words
/// such as paths anywhere).
fn wrap(text: &str, width: f32, size: f32) -> Vec<String> {
    let fits = |s: &str| with_font(|font| measure_text(s, font, size as u16, 1.0).width) <= width;
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in text.split(' ') {
        let cand = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if fits(&cand) {
            cur = cand;
            continue;
        }
        if !cur.is_empty() {
            lines.push(std::mem::take(&mut cur));
        }
        // a word longer than the line: cut it
        for ch in word.chars() {
            cur.push(ch);
            if !fits(&cur) {
                let last = cur.pop().unwrap();
                lines.push(std::mem::take(&mut cur));
                cur.push(last);
            }
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

#[cfg(test)]
mod start_texts {
    #[test]
    fn every_start_screen_text_has_every_language() {
        let d = i18n::Dictionary::parse(super::START_TEXTS).unwrap();
        let missing: Vec<&str> = d.entries(i18n::Lang::En).map(|e| e.0).filter(|en| !d.missing(en).is_empty()).collect();
        assert!(missing.is_empty(), "{missing:?}");
    }
}
