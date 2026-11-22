//! Pause menu drawn at Game Boy resolution in the screen's four shades: key bindings,
//! display, sound, language, the game's own pause, back to the map, file select, quit, and
//! with the cheats mode on, the cheats list and the item picker.
//! It changes `Settings` in place; saving them, switching full screen and carrying out the
//! cheats is the caller's job.
use super::cheats::{draw_icon, Cheat, Cheats, Icon};
use super::devices::{Devices, Prompts};
use super::i18n::{texts, Texts, LANGS, LANG_NAMES};
use super::keys::{self, Key, DEFAULT_KEYS};
use super::overlay::{text_width, Canvas};
use super::pad::{self, Input, PadButton, DEFAULT_PAD};
use super::settings::{Scale, Settings};
use super::touch::{TouchShow, TouchSize};
use crate::ports::{Gamepad, Keyboard};
use cart_model::cheats::Item;
use cart_model::GameMode;
use gb_core::ppu::H;

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    Controls,
    Display,
    Sound,
    Language,
    Waiting(usize),
    /// Asking before leaving the level or the game: holds the main item.
    Confirm(usize),
    Cheats,
    Summon,
    /// The level list for "Go to level".
    Levels,
}

pub enum Action {
    None,
    Close,
    PressStart,
    Quit,
    BackToMap,
    /// Start the level being played again (no life lost).
    RestartLevel,
    FileSelect,
    /// Turn a lasting cheat on or off, or do a one-time one.
    Cheat(Cheat),
    Summon(Item),
    GoTo(u8),
}

// Main menu items (indexes into Texts::main).
const IT_CONTINUE: usize = 0;
const IT_CONTROLS: usize = 1;
const IT_DISPLAY: usize = 2;
const IT_SOUND: usize = 3;
const IT_LANGUAGE: usize = 4;
const IT_PAUSE: usize = 5;
const IT_QUIT: usize = 6;
const IT_MAP: usize = 7;
const IT_FILES: usize = 8;
const IT_CHEATS: usize = 9;
const IT_RESTART: usize = 10;
/// Rows of the level list shown at once (Left and Right move by a page).
const LEVEL_ROWS: usize = 10;
/// Controls rows on the screen at once (the rest scroll into view).
const CONTROLS_SHOWN: usize = 15;

/// A row of the Controls screen; which there are depends on the device's inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ControlsRow {
    /// The keyboard's or the controller's bindings, with both there.
    Input,
    Binding(usize),
    Restore,
    /// The on-screen controls: when they show, their size, vibration.
    Touch,
    TouchSize,
    Vibration,
}
/// Language rows on the screen at once (the rest scroll into view).
const LANGUAGES_SHOWN: usize = 8;

/// What the menu reads in one frame: the keyboard, and the controller when it plays.
#[derive(Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub ok: bool,
    pub back: bool,
    /// Esc: leaves the waiting for a key or button.
    pub escape: bool,
    /// The key and controller button that went down last (to bind them).
    pub key: Option<Key>,
    pub button: Option<PadButton>,
}

impl MenuInput {
    /// The keyboard's arrows, Enter and Space and the bound keys, and with `pad` (a connected
    /// controller) its D-pad, left stick and bound buttons.
    pub fn read(keys: &dyn Keyboard, pad: Option<&dyn Gamepad>, st: &Settings) -> MenuInput {
        let any = |ks: &[Key]| ks.iter().any(|&k| keys.pressed(k));
        let mut m = MenuInput {
            up: any(&[Key::Up, st.keys[keys::UP]]),
            down: any(&[Key::Down, st.keys[keys::DOWN]]),
            left: any(&[Key::Left, st.keys[keys::LEFT]]),
            right: any(&[Key::Right, st.keys[keys::RIGHT]]),
            ok: any(&[Key::Enter, Key::KpEnter, Key::Space, st.keys[keys::A]]),
            back: any(&[st.keys[keys::MENU], st.keys[keys::B]]),
            escape: keys.pressed(Key::Escape),
            key: keys.last_pressed(),
            button: None,
        };
        if let Some(p) = pad {
            let dir = |i: usize| p.pressed(st.pad[i]) || p.pressed(pad::stick(i));
            m.up |= dir(keys::UP);
            m.down |= dir(keys::DOWN);
            m.left |= dir(keys::LEFT);
            m.right |= dir(keys::RIGHT);
            m.ok |= p.pressed(st.pad[keys::A]);
            m.back |= p.pressed(st.pad[keys::B]) || p.pressed(st.pad[keys::MENU]);
            m.button = p.last_pressed();
        }
        m
    }
}

/// The cheats as the menu shows them.
pub struct CheatMenu {
    pub state: Cheats,
    /// The items' pictures (only known in a level).
    pub icons: Option<[Icon; 5]>,
    /// The level being played has a secret exit.
    pub secret_exit: bool,
    /// The levels to go to: number and name (the editor's new levels have one).
    pub levels: Vec<(u8, String)>,
}

pub struct Menu {
    screen: Screen,
    sel: usize,
    blink: u32,
    items: Vec<usize>,
    in_level: bool,
    cheats: Option<CheatMenu>,
    /// One-time cheats already done while the menu is open.
    done: Vec<Cheat>,
    devices: Devices,
}

impl Menu {
    fn label<'a>(&self, t: &'a Texts, item: usize) -> &'a str {
        t.main[item]
    }

    /// `mode` is the screen the game is on (a level, the map...).
    pub fn new(mode: GameMode) -> Menu {
        let in_level = mode == GameMode::Playing;
        let on_map = mode == GameMode::Map;
        // No "continue" item: Esc (or B) closes the menu. In a level the game's own pause comes first.
        let mut items = Vec::new();
        if in_level {
            items.push(IT_PAUSE);
        }
        items.extend([IT_CONTROLS, IT_DISPLAY, IT_SOUND, IT_LANGUAGE]);
        if in_level {
            items.push(IT_MAP);
        }
        if in_level || on_map {
            items.push(IT_FILES);
        }
        items.push(IT_QUIT);
        Menu { screen: Screen::Main, sel: 0, blink: 0, items, in_level, cheats: None, done: Vec::new(), devices: Devices::default() }
    }

    /// With the cheats mode on: the "Debug / cheats" entry, after the game's own pause.
    pub fn with_cheats(mut self, cheats: CheatMenu) -> Menu {
        let at = if self.in_level { 1 } else { 0 };
        self.items.insert(at, IT_CHEATS);
        self.cheats = Some(cheats);
        self
    }

    /// In a level of a test game or with the cheats mode on: "Restart level", just above
    /// "Back to map".
    pub fn with_restart(mut self) -> Menu {
        if self.in_level {
            let at = self.pos(IT_MAP);
            self.items.insert(at, IT_RESTART);
        }
        self
    }

    /// A test game of a level from the editor: no "Back to map" (the level is what is tried;
    /// Restart level starts it again).
    pub fn without_back_to_map(mut self) -> Menu {
        self.items.retain(|&i| i != IT_MAP);
        self
    }

    /// Where the program cannot be quit from its menu (the browser): no "Quit".
    pub fn without_quit(mut self) -> Menu {
        self.items.retain(|&i| i != IT_QUIT);
        self
    }

    /// A test game from the editor: no "File select" (it keeps no saves to choose from).
    pub fn without_file_select(mut self) -> Menu {
        self.items.retain(|&i| i != IT_FILES);
        self
    }

    /// The inputs there are now (Controls shows only theirs).
    pub fn set_devices(&mut self, devices: Devices) {
        self.devices = devices;
    }

    fn controls_rows(&self) -> Vec<ControlsRow> {
        let d = self.devices;
        let mut rows = Vec::new();
        if d.keyboard && d.controller {
            rows.push(ControlsRow::Input);
        }
        if d.keyboard || d.controller {
            rows.extend((0..keys::KEYS).map(ControlsRow::Binding));
            rows.push(ControlsRow::Restore);
        }
        if d.touch {
            rows.extend([ControlsRow::Touch, ControlsRow::TouchSize, ControlsRow::Vibration]);
        }
        rows
    }

    fn controls_row(&self, row: ControlsRow) -> usize {
        self.controls_rows().iter().position(|&r| r == row).unwrap_or(0)
    }

    fn usable(&self, c: Cheat) -> bool {
        let secret = c != Cheat::SecretExit || self.cheats.as_ref().is_some_and(|m| m.secret_exit);
        (self.in_level || !c.needs_level()) && secret
    }

    fn row_of(c: Cheat) -> usize {
        Cheat::ALL.iter().position(|&x| x == c).unwrap_or(0)
    }

    fn level_count(&self) -> usize {
        self.cheats.as_ref().map_or(0, |m| m.levels.len())
    }

    fn pos(&self, item: usize) -> usize {
        self.items.iter().position(|&i| i == item).unwrap_or(0)
    }

    /// For previews: show one screen ("main", "controls", "wait", "display", "sound", "language",
    /// "confirm", "cheats", "summon").
    pub fn preview(name: &str, icons: Option<[Icon; 5]>) -> Menu {
        let screen = match name {
            "controls" => Screen::Controls,
            "wait" => Screen::Waiting(4),
            "display" => Screen::Display,
            "sound" => Screen::Sound,
            "language" => Screen::Language,
            "confirm" => Screen::Confirm(IT_MAP),
            "cheats" => Screen::Cheats,
            "summon" => Screen::Summon,
            "levels" => Screen::Levels,
            _ => Screen::Main,
        };
        let mut m = Menu::new(GameMode::Playing).with_restart().with_cheats(CheatMenu { state: Cheats { invincible: true, ..Cheats::default() }, icons, secret_exit: true, levels: (0..40).map(|l| (l, if l >= 32 { "MY LEVEL".to_string() } else { String::new() })).collect() });
        m.screen = screen;
        m.devices = Devices { keyboard: true, controller: true, touch: true, touch_shown: true };
        m
    }

    fn count(&self) -> usize {
        match self.screen {
            Screen::Main => self.items.len(),
            Screen::Confirm(_) => 2,
            Screen::Controls => self.controls_rows().len().max(1),
            Screen::Display => 3,
            Screen::Sound => 1,
            Screen::Language => LANGS,
            Screen::Waiting(_) => 0,
            Screen::Cheats => Cheat::ALL.len(),
            Screen::Summon => Item::ALL.len(),
            Screen::Levels => self.level_count().max(1),
        }
    }

    fn go(&mut self, s: Screen, sel: usize) {
        self.screen = s;
        self.sel = sel;
    }

    pub fn update(&mut self, input: &MenuInput, st: &mut Settings) -> Action {
        self.blink = self.blink.wrapping_add(1);
        if let Some(shown) = self.devices.input(st.input) {
            st.input = shown;
        }
        if let Screen::Waiting(b) = self.screen {
            let row = self.controls_row(ControlsRow::Binding(b));
            if input.escape || self.devices.input(st.input).is_none() {
                self.go(Screen::Controls, row);
                return Action::None;
            }
            // A key or button already used elsewhere swaps places, so no binding is lost.
            match st.input {
                Input::Keyboard => {
                    if let Some(k) = input.key.filter(|k| k.bindable()) {
                        if let Some(other) = st.keys.iter().position(|&x| x == k) {
                            st.keys[other] = st.keys[b];
                        }
                        st.keys[b] = k;
                        self.go(Screen::Controls, row);
                    }
                }
                Input::Controller => {
                    if let Some(p) = input.button.filter(|p| p.bindable()) {
                        if let Some(other) = st.pad.iter().position(|&x| x == p) {
                            st.pad[other] = st.pad[b];
                        }
                        st.pad[b] = p;
                        self.go(Screen::Controls, row);
                    }
                }
            }
            return Action::None;
        }
        let (up, down, left, right, ok, back) = (input.up, input.down, input.left, input.right, input.ok, input.back);
        let n = self.count();
        self.sel = self.sel.min(n - 1);
        // Yes/No sit side by side, so they are chosen with Left/Right; lists use Up/Down.
        let (prev, next) = if matches!(self.screen, Screen::Confirm(_) | Screen::Summon) { (left, right) } else { (up, down) };
        if prev {
            self.sel = (self.sel + n - 1) % n;
        }
        if next {
            self.sel = (self.sel + 1) % n;
        }
        if back {
            let back_to = match self.screen {
                Screen::Main => return Action::Close,
                Screen::Controls => self.pos(IT_CONTROLS),
                Screen::Display => self.pos(IT_DISPLAY),
                Screen::Sound => self.pos(IT_SOUND),
                Screen::Confirm(item) => self.pos(item),
                Screen::Cheats => self.pos(IT_CHEATS),
                Screen::Summon => {
                    self.go(Screen::Cheats, Self::row_of(Cheat::Summon));
                    return Action::None;
                }
                Screen::Levels => {
                    self.go(Screen::Cheats, Self::row_of(Cheat::GoToLevel));
                    return Action::None;
                }
                _ => self.pos(IT_LANGUAGE),
            };
            self.go(Screen::Main, back_to);
            return Action::None;
        }
        let step: i32 = if right { 1 } else if left { -1 } else { 0 };
        match self.screen {
            Screen::Main => {
                if ok {
                    match self.items[self.sel] {
                        IT_CONTINUE => return Action::Close,
                        IT_CONTROLS => {
                            // a connected controller comes first: its bindings are shown
                            if self.devices.controller {
                                st.input = Input::Controller;
                            }
                            self.go(Screen::Controls, 0)
                        }
                        IT_DISPLAY => self.go(Screen::Display, 0),
                        IT_SOUND => self.go(Screen::Sound, 0),
                        IT_LANGUAGE => self.go(Screen::Language, st.lang),
                        IT_PAUSE => return Action::PressStart,
                        IT_CHEATS => self.go(Screen::Cheats, 0),
                        // "No" is selected first, so a stray key press never leaves the level.
                        item => self.go(Screen::Confirm(item), 1),
                    }
                }
            }
            Screen::Confirm(item) => {
                if ok {
                    if self.sel == 0 {
                        return match item {
                            IT_MAP => Action::BackToMap,
                            IT_RESTART => Action::RestartLevel,
                            IT_FILES => Action::FileSelect,
                            _ => Action::Quit,
                        };
                    }
                    self.go(Screen::Main, self.pos(item));
                }
            }
            Screen::Controls => {
                let turn = |i: usize| (i as i32 + if step == 0 { 1 } else { step }).rem_euclid(3) as usize;
                let change = ok || step != 0;
                match self.controls_rows().get(self.sel).copied() {
                    Some(ControlsRow::Input) if change => {
                        st.input = if st.input == Input::Keyboard { Input::Controller } else { Input::Keyboard };
                    }
                    Some(ControlsRow::Binding(b)) if ok => self.screen = Screen::Waiting(b),
                    Some(ControlsRow::Restore) if ok => match st.input {
                        Input::Keyboard => st.keys = DEFAULT_KEYS,
                        Input::Controller => st.pad = DEFAULT_PAD,
                    },
                    Some(ControlsRow::Touch) if change => st.touch = TouchShow::ALL[turn(TouchShow::ALL.iter().position(|&t| t == st.touch).unwrap_or(0))],
                    Some(ControlsRow::TouchSize) if change => st.touch_size = TouchSize::ALL[turn(TouchSize::ALL.iter().position(|&t| t == st.touch_size).unwrap_or(1))],
                    Some(ControlsRow::Vibration) if change => st.vibrate = !st.vibrate,
                    _ => {}
                }
            }
            Screen::Display => {
                let change = step != 0 || ok;
                match self.sel {
                    0 if change => {
                        st.palette = ((st.palette as i32 + if step == 0 { 1 } else { step } + 3) % 3) as usize;
                    }
                    1 if change => {
                        st.fullscreen = !st.fullscreen;
                    }
                    2 if change => {
                        let i = Scale::ALL.iter().position(|&s| s == st.scale).unwrap_or(0) as i32;
                        st.scale = Scale::ALL[(i + if step == 0 { 1 } else { step }).rem_euclid(Scale::ALL.len() as i32) as usize];
                    }
                    _ => {}
                }
            }
            Screen::Sound => {
                if step != 0 {
                    st.volume = (st.volume as i32 + step).clamp(0, 10) as u8;
                }
            }
            Screen::Language => {
                if ok {
                    st.lang = self.sel;
                    st.lang_auto = false;
                }
            }
            Screen::Cheats => {
                if ok {
                    let Some(&c) = Cheat::ALL.get(self.sel) else {
                        self.go(Screen::Main, self.pos(IT_CHEATS));
                        return Action::None;
                    };
                    if !self.usable(c) {
                        return Action::None;
                    }
                    if c == Cheat::Summon {
                        self.go(Screen::Summon, 0);
                        return Action::None;
                    }
                    if c == Cheat::GoToLevel {
                        self.go(Screen::Levels, 0);
                        return Action::None;
                    }
                    if let Some(m) = self.cheats.as_mut() {
                        m.state.toggle(c);
                    }
                    if !c.toggles() && !self.done.contains(&c) {
                        self.done.push(c);
                    }
                    return Action::Cheat(c);
                }
            }
            Screen::Summon => {
                if ok {
                    return Action::Summon(Item::ALL[self.sel]);
                }
            }
            Screen::Levels => {
                let n = self.level_count();
                if n > 0 && step != 0 {
                    self.sel = (self.sel as i32 + step * LEVEL_ROWS as i32).clamp(0, n as i32 - 1) as usize;
                }
                if ok {
                    if let Some(&(l, _)) = self.cheats.as_ref().and_then(|m| m.levels.get(self.sel)) {
                        return Action::GoTo(l);
                    }
                }
            }
            Screen::Waiting(_) => {}
        }
        Action::None
    }

    pub fn draw(&self, px: &mut [u8], st: &Settings) {
        let t = texts(st.lang);
        let mut c = Canvas { px };
        c.veil();
        let blink_on = (self.blink / 16) % 2 == 0;
        let shade = |i: usize| if i == self.sel { 3 } else { 2 };
        match self.screen {
            Screen::Main => {
                let n = self.items.len() as i32;
                let gap = if n > 8 { 11 } else { 12 };
                let h = 28 + n * gap;
                let (x, w) = (14, 132);
                let y = ((H as i32 - h) / 2).max(6);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.pause);
                for (i, &item) in self.items.iter().enumerate() {
                    let ty = y + 12 + i as i32 * gap;
                    c.text(x + 18, ty, self.label(t, item), shade(i));
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_main, st));
            }
            Screen::Confirm(item) => {
                let (x, y, w, h) = (14, 40, 132, 64);
                c.frame(x, y, w, h);
                c.title(x, y, w, self.label(t, item));
                c.text(x + (w - text_width(t.sure)) / 2, y + 14, t.sure, 3);
                for (i, label) in [t.yes, t.no].iter().enumerate() {
                    let tx = x + 30 + i as i32 * 50;
                    c.text(tx, y + 32, label, shade(i));
                    if i == self.sel {
                        c.cursor(tx - 9, y + 32, 3);
                    }
                }
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_language, st));
            }
            Screen::Controls => {
                // the input, every binding of it, Restore, then the on-screen controls (only the
                // inputs there are), between the title and the footer (the footer says how to go
                // back); the rows scroll with the cursor, a small arrow at the side showing there
                // are more above or below
                let rows = self.controls_rows();
                let shown = rows.len().min(CONTROLS_SHOWN);
                let h = 16 + shown as i32 * 8;
                let (x, y, w) = (2, if shown == CONTROLS_SHOWN { 5 } else { (H as i32 - h) / 2 }, 156);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.controls);
                let top = (self.sel + 1).saturating_sub(shown).min(rows.len() - shown);
                for (i, row) in rows.iter().enumerate().skip(top).take(shown) {
                    let ty = y + 9 + (i - top) as i32 * 8;
                    let (name, value) = match *row {
                        ControlsRow::Input => (t.input, if st.input == Input::Controller { t.controller } else { t.keyboard }),
                        ControlsRow::Restore => (t.restore, ""),
                        ControlsRow::Touch => (t.touch, t.touch_modes[TouchShow::ALL.iter().position(|&m| m == st.touch).unwrap_or(0)]),
                        ControlsRow::TouchSize => (t.touch_size, t.touch_sizes[TouchSize::ALL.iter().position(|&m| m == st.touch_size).unwrap_or(1)]),
                        ControlsRow::Vibration => (t.vibration, if st.vibrate { t.on } else { t.off }),
                        ControlsRow::Binding(b) => (t.buttons[b], binding_label(st, b)),
                    };
                    c.text(x + 16, ty, name, shade(i));
                    c.text(x + w - 8 - text_width(value), ty, value, 3);
                    if i == self.sel {
                        c.cursor(x + 8, ty, 3);
                    }
                }
                if top > 0 {
                    c.more(x + 5, y + 10, true);
                }
                if top + shown < rows.len() {
                    c.more(x + 5, y + 9 + (shown as i32 - 1) * 8 + 4, false);
                }
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_controls, st));
            }
            Screen::Waiting(b) => {
                let (x, y, w, h) = (14, 44, 132, 56);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.buttons[b]);
                let press = if st.input == Input::Controller { t.press_button } else { t.press_key };
                if blink_on {
                    c.text(x + (w - text_width(press)) / 2, y + 14, press, 3);
                }
                c.text(x + 18, y + 28, t.now, 2);
                c.text(x + 22 + text_width(t.now), y + 28, binding_label(st, b), 3);
                c.text(x + (w - text_width(t.cancel)) / 2, y + 40, t.cancel, 2);
            }
            Screen::Display => {
                let (x, y, w, h) = (6, 36, 148, 71);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.display);
                let rows = [
                    (t.colours, t.palettes[st.palette % 3]),
                    (t.fullscreen, if st.fullscreen { t.yes } else { t.no }),
                    (t.scale, match st.scale {
                        Scale::Sharp => t.sharp,
                        Scale::Fitted => t.fitted,
                        Scale::Stretched => t.stretched,
                    }),
                ];
                for (i, (l, v)) in rows.iter().enumerate() {
                    let ty = y + 14 + i as i32 * 13;
                    c.text(x + 18, ty, l, shade(i));
                    if !v.is_empty() {
                        let vx = x + w - 14 - text_width(v);
                        c.text(vx, ty, v, 3);
                        if i == self.sel {
                            c.text(vx - 8, ty, "<", 2);
                            c.text(x + w - 12, ty, ">", 2);
                        }
                    }
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                    if i == 0 {
                        // palette swatch: the four shades of the chosen colours
                        let sx = x + 18 + text_width(l) + 4;
                        c.rect(sx, ty - 1, 18, 9, 3);
                        for k in 0..4u8 {
                            c.rect(sx + 1 + k as i32 * 4, ty, 4, 7, k);
                        }
                    }
                }
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_change, st));
            }
            Screen::Sound => {
                let (x, y, w, h) = (14, 48, 132, 46);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.sound);
                c.text(x + 18, y + 13, t.volume, shade(0));
                let bx = x + 18;
                let by = y + 24;
                c.rect(bx - 1, by - 1, 10 * 9 + 1, 9, 3);
                c.rect(bx, by, 10 * 9 - 1, 7, 0);
                for i in 0..10 {
                    if i < st.volume as i32 {
                        c.rect(bx + 1 + i * 9, by + 1, 7, 5, 3);
                    } else {
                        c.rect(bx + 1 + i * 9, by + 3, 7, 1, 1);
                    }
                }
                c.text(bx + 94, by, &format!("{}", st.volume), 3);
                c.cursor(x + 9, y + 13, 3);
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_change, st));
            }
            Screen::Language => {
                // the languages, each written in itself: the rows scroll with the cursor, a
                // small arrow at the side showing there are more above or below
                let rows = LANGS;
                let h = 13 + LANGUAGES_SHOWN as i32 * 11 + 9;
                let (x, y, w) = (22, (gb_core::ppu::H as i32 - h) / 2, 116);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.language);
                let top = (self.sel + 1).saturating_sub(LANGUAGES_SHOWN).min(rows - LANGUAGES_SHOWN);
                for i in top..top + LANGUAGES_SHOWN {
                    let ty = y + 13 + (i - top) as i32 * 11;
                    c.text(x + 18, ty, LANG_NAMES[i], shade(i));
                    // the language in use is marked with a filled box, like a checked option
                    c.rect(x + w - 16, ty, 7, 7, 3);
                    c.rect(x + w - 15, ty + 1, 5, 5, 0);
                    if i == st.lang {
                        c.rect(x + w - 14, ty + 2, 3, 3, 3);
                    }
                    if i == self.sel {
                        c.cursor(x + 9, ty, 3);
                    }
                }
                if top > 0 {
                    c.more(x + 5, y + 14, true);
                }
                if top + LANGUAGES_SHOWN < rows {
                    c.more(x + 5, y + 13 + (LANGUAGES_SHOWN as i32 - 1) * 11 + 4, false);
                }
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_language, st));
            }
            Screen::Cheats => {
                let (x, y, w, h) = (2, 5, 156, 136);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.main[IT_CHEATS]);
                let state = self.cheats.as_ref().map(|m| m.state).unwrap_or_default();
                for (i, &cheat) in Cheat::ALL.iter().enumerate() {
                    let ty = y + 9 + i as i32 * 9;
                    let usable = self.usable(cheat);
                    c.text(x + 16, ty, t.cheats[i], if usable { shade(i) } else { 1 });
                    let value = if cheat.toggles() {
                        if state.is_on(cheat) { t.on } else { t.off }
                    } else if self.done.contains(&cheat) {
                        t.done
                    } else {
                        ""
                    };
                    c.text(x + w - 8 - text_width(value), ty, value, 3);
                    if i == self.sel {
                        c.cursor(x + 8, ty, 3);
                    }
                }
                let footer = match Cheat::ALL.get(self.sel) {
                    Some(&ch) if !self.usable(ch) => if self.in_level { t.no_secret } else { t.only_level },
                    _ => t.footer_language,
                };
                c.footer(x, y + h - 2, w, &self.prompt(footer, st));
            }
            Screen::Summon => {
                let (x, y, w, h) = (14, 36, 132, 68);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.cheats[Self::row_of(Cheat::Summon)]);
                let icons = self.cheats.as_ref().and_then(|m| m.icons.as_ref());
                for i in 0..Item::ALL.len() {
                    let ix = x + 10 + i as i32 * 23;
                    if i == self.sel {
                        c.rect(ix - 2, y + 10, 20, 20, 3);
                        c.rect(ix - 1, y + 11, 18, 18, 0);
                    }
                    if let Some(icons) = icons {
                        draw_icon(&mut c, ix, y + 12, &icons[i]);
                    }
                }
                let name = t.items[self.sel.min(Item::ALL.len() - 1)];
                c.text(x + (w - text_width(name)) / 2, y + 38, name, 3);
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_summon, st));
            }
            Screen::Levels => {
                let (x, y, w, h) = (6, 8, 148, 130);
                c.frame(x, y, w, h);
                c.title(x, y, w, t.cheats[Self::row_of(Cheat::GoToLevel)]);
                let levels = self.cheats.as_ref().map(|m| m.levels.as_slice()).unwrap_or(&[]);
                let first = self.sel / LEVEL_ROWS * LEVEL_ROWS;
                for (i, (l, name)) in levels.iter().enumerate().skip(first).take(LEVEL_ROWS) {
                    let ty = y + 12 + (i - first) as i32 * 10;
                    let label = if name.is_empty() { format!("{} {l:02}", t.level) } else { format!("{l:02} {}", name.to_uppercase()) };
                    c.text(x + 16, ty, &label, shade(i));
                    if i == self.sel {
                        c.cursor(x + 8, ty, 3);
                    }
                }
                c.footer(x, y + h - 2, w, &self.prompt(t.footer_summon, st));
            }
        }
    }
}

impl Menu {
    /// `footer` naming the buttons of the input that plays (`Prompts`): the keyboard's Enter and
    /// Esc as written, or the controller's or the on-screen A and B in their place.
    fn prompt(&self, footer: &str, st: &Settings) -> String {
        let (ok, back) = match self.devices.prompts() {
            Prompts::Keyboard => return footer.to_string(),
            Prompts::Controller => (st.pad[keys::A].label(), st.pad[keys::B].label()),
            Prompts::Touch => ("A", "B"),
        };
        let (enter, esc) = footer_keys(texts(st.lang));
        let named = |part: &str| match part.split_once(':') {
            Some((k, word)) if k == enter => format!("{ok}:{word}"),
            Some((k, word)) if k == esc => format!("{back}:{word}"),
            _ => part.to_string(),
        };
        footer.split(' ').map(named).collect::<Vec<_>>().join(" ")
    }
}

/// How a language's footers name Enter and Esc: the keys of its main footer ("ENTER:OK ESC:...").
fn footer_keys(t: &Texts) -> (&'static str, &'static str) {
    let mut keys = t.footer_main.split_whitespace().filter_map(|p| p.split_once(':').map(|(k, _)| k));
    (keys.next().unwrap_or(""), keys.next().unwrap_or(""))
}

/// Binding `i`'s key or button for the input chosen.
fn binding_label(st: &Settings, i: usize) -> &'static str {
    match st.input {
        Input::Keyboard => st.keys[i].label(st.lang),
        Input::Controller => st.pad[i].label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One key going down per frame.
    struct Press(Option<Key>);
    impl Keyboard for Press {
        fn pressed(&self, k: Key) -> bool {
            self.0 == Some(k)
        }
        fn held(&self, k: Key) -> bool {
            self.0 == Some(k)
        }
        fn last_pressed(&self) -> Option<Key> {
            self.0
        }
    }

    fn step(m: &mut Menu, st: &mut Settings, k: Key) -> Action {
        let input = MenuInput::read(&Press(Some(k)), None, st);
        m.update(&input, st)
    }

    #[test]
    fn the_footers_name_the_buttons_of_the_input_that_plays() {
        for lang in 0..LANGS {
            let t = texts(lang);
            let (enter, esc) = footer_keys(t);
            for footer in [t.footer_main, t.footer_controls, t.footer_change, t.footer_language, t.footer_summon] {
                for part in footer.split_whitespace() {
                    let key = part.split_once(':').map(|(k, _)| k);
                    assert!(matches!(key, Some(k) if k == enter || k == esc || k == "</>"), "{lang}: {footer}");
                }
            }
        }
        let st = Settings::default();
        let mut m = Menu::new(GameMode::Playing);
        assert_eq!(m.prompt(texts(0).footer_main, &st), "ENTER:OK  ESC:RESUME");
        m.set_devices(Devices { controller: true, ..Devices::default() });
        assert_eq!(m.prompt(texts(0).footer_main, &st), format!("{}:OK  {}:RESUME", DEFAULT_PAD[keys::A].label(), DEFAULT_PAD[keys::B].label()));
        m.set_devices(Devices { keyboard: false, controller: false, touch: true, touch_shown: true });
        assert_eq!(m.prompt(texts(0).footer_summon, &st), "</>:CHOOSE A:OK");
    }

    #[test]
    fn controls_opens_on_a_connected_controller() {
        let mut st = Settings::default();
        let mut m = Menu::new(GameMode::Playing);
        m.set_devices(Devices { controller: true, ..Devices::default() });
        // Original pause, Controls
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Enter);
        assert!(m.screen == Screen::Controls);
        assert_eq!(st.input, Input::Controller);
    }

    #[test]
    fn change_palette_rebind_and_leave() {
        let mut st = Settings::default();
        let mut m = Menu::new(GameMode::Playing);
        m.set_devices(Devices { controller: true, ..Devices::default() });
        // Original pause, Controls, Display...
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Right);
        assert_eq!(st.palette, 1);
        step(&mut m, &mut st, Key::Escape);
        // Controls (on the controller's bindings): the keyboard's, then bind A to Space.
        step(&mut m, &mut st, Key::Up);
        step(&mut m, &mut st, Key::Enter);
        assert_eq!(st.input, Input::Controller);
        step(&mut m, &mut st, Key::Right);
        assert_eq!(st.input, Input::Keyboard);
        for _ in 0..5 {
            step(&mut m, &mut st, Key::Down);
        }
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Space);
        assert_eq!(st.keys[keys::A], Key::Space);
        step(&mut m, &mut st, Key::Escape);
        assert!(matches!(step(&mut m, &mut st, Key::Escape), Action::Close));
    }

    #[test]
    fn controls_shows_only_the_inputs_there_are() {
        let mut m = Menu::new(GameMode::Map);
        m.set_devices(Devices { keyboard: false, controller: false, touch: true, touch_shown: true });
        assert_eq!(m.controls_rows(), [ControlsRow::Touch, ControlsRow::TouchSize, ControlsRow::Vibration]);
        m.set_devices(Devices { keyboard: false, controller: true, touch: true, touch_shown: true });
        let rows = m.controls_rows();
        assert_eq!((rows[0], rows.len()), (ControlsRow::Binding(0), keys::KEYS + 4), "the controller's bindings, no choice of input");
        m.set_devices(Devices::default());
        assert!(!m.controls_rows().contains(&ControlsRow::Touch), "no touch rows on a computer");
        // the controller's bindings shown on a phone with only a controller, whatever was chosen
        let mut st = Settings::default();
        m.set_devices(Devices { keyboard: false, controller: true, touch: true, touch_shown: true });
        step(&mut m, &mut st, Key::Down);
        assert_eq!(st.input, Input::Controller);
    }

    #[test]
    fn a_phone_turns_its_controls_off_and_on_in_controls() {
        let mut st = Settings::default();
        let mut m = Menu::new(GameMode::Map);
        m.set_devices(Devices { keyboard: false, controller: false, touch: true, touch_shown: true });
        m.go(Screen::Controls, 0);
        step(&mut m, &mut st, Key::Right);
        assert_eq!(st.touch, TouchShow::Always);
        step(&mut m, &mut st, Key::Right);
        assert_eq!(st.touch, TouchShow::Never);
    }

    fn cheats_menu(mode: GameMode) -> Menu {
        Menu::new(mode).with_cheats(CheatMenu { state: Cheats::default(), icons: None, secret_exit: false, levels: vec![(0, String::new()), (1, String::new()), (12, String::new()), (32, "Mine".into())] })
    }

    #[test]
    fn the_cheats_entry_is_there_only_with_the_mode_on() {
        assert!(!Menu::new(GameMode::Playing).items.contains(&IT_CHEATS));
        assert_eq!(cheats_menu(GameMode::Playing).items[1], IT_CHEATS);
        assert_eq!(cheats_menu(GameMode::Map).items[0], IT_CHEATS);
    }

    #[test]
    fn no_quit_where_the_program_is_not_quit() {
        assert_eq!(Menu::new(GameMode::Map).items.last(), Some(&IT_QUIT));
        let m = Menu::new(GameMode::Map).without_quit();
        assert!(!m.items.contains(&IT_QUIT));
        assert_eq!(m.items.last(), Some(&IT_FILES));
    }

    #[test]
    fn cheats_toggle_and_summon_in_a_level() {
        let mut st = Settings::default();
        let mut m = cheats_menu(GameMode::Playing);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Enter);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::Cheat(Cheat::Invincible)));
        assert!(m.cheats.as_ref().unwrap().state.invincible);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Down);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::None), "no secret exit in this level");
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Right);
        step(&mut m, &mut st, Key::Right);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::Summon(Item::Flower)));
        step(&mut m, &mut st, Key::Escape);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Down);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::Cheat(Cheat::MaxLives)));
        assert_eq!(m.done, vec![Cheat::MaxLives]);
    }

    #[test]
    fn go_to_a_level_from_the_list() {
        let mut st = Settings::default();
        let mut m = cheats_menu(GameMode::Map);
        step(&mut m, &mut st, Key::Enter);
        for _ in 0..3 {
            step(&mut m, &mut st, Key::Down);
        }
        step(&mut m, &mut st, Key::Enter);
        assert!(m.screen == Screen::Levels);
        step(&mut m, &mut st, Key::Down);
        step(&mut m, &mut st, Key::Down);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::GoTo(12)));
        step(&mut m, &mut st, Key::Right);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::GoTo(32)));
    }

    #[test]
    fn level_cheats_are_refused_on_the_map() {
        let mut st = Settings::default();
        let mut m = cheats_menu(GameMode::Map);
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Down);
        for _ in 0..2 {
            assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::None));
            step(&mut m, &mut st, Key::Down);
        }
        step(&mut m, &mut st, Key::Down);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::None), "summon only in a level");
        step(&mut m, &mut st, Key::Down);
        assert!(m.screen == Screen::Cheats);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::Cheat(Cheat::FreezeTimer)));
    }

    #[test]
    fn leaving_the_level_asks_first() {
        let mut st = Settings::default();
        let mut m = Menu::new(GameMode::Playing);
        for _ in 0..5 {
            step(&mut m, &mut st, Key::Down); // ... Language, Back to map
        }
        step(&mut m, &mut st, Key::Enter);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::None)); // "No" comes first
        step(&mut m, &mut st, Key::Enter);
        step(&mut m, &mut st, Key::Left);
        assert!(matches!(step(&mut m, &mut st, Key::Enter), Action::BackToMap));
    }
}
