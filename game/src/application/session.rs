//! One play session: the emulated console plus everything the port adds around it (pause
//! menu, the editor's new levels and zones, battery saves, the start-up hint, the cheats).
//! Each call to `frame` is one frame of the window.
use super::preferences::Preferences;
use super::zones::ZoneSwap;
use crate::domain::devices::{Devices, Prompts};
use crate::domain::touch::TouchShow;
use crate::domain::cheats::{draw_info, draw_note, draw_objects, Cheat, Cheats};
use crate::domain::keys::{self, Key};
use crate::domain::menu::{Action, CheatMenu, Menu};
use crate::domain::i18n::texts;
use crate::domain::overlay::draw_hint;
use crate::domain::retry::{Entry, Retry};
use crate::domain::settings::Settings;
use crate::domain::bindings::Bindings;
use crate::domain::menu::MenuInput;
use crate::ports::{AudioOut, Gamepad, Keyboard, SaveStore, ZoneProgressStore};
use cart_model::cheats::{self as cheat_rules, Item};
use cart_model::extra_levels;
use cart_model::level_start::Start;
use cart_model::secret_exits;
use cart_model::ram_map::{GAME_MODE, LEVEL_NUMBER, LIVES, MAP_POINT, POWER, SPECIAL_MAP_POINTS};
use cart_model::{map_routes, world};
use cart_model::{checkpoint, levels, start_point};
use cart_model::{GameDriver, GameMode};
use gb_core::palette::{Palette, PALETTES};
use gb_core::{Buttons, Gb, State, FRAME_SECONDS};
use std::collections::VecDeque;

#[derive(PartialEq, Debug)]
pub enum Flow {
    Continue,
    Quit,
}

/// Writes that undo a new level put in place of its base level.
type Undo = Vec<(usize, Vec<u8>)>;

/// The level to ask the game for, to play `level`: a new level (32 and up, see
/// `extra_levels`) is put in place of its base level in the console's ROM (its secret exit's
/// target too), and the writes that undo it come back. None if the copy has no such level.
fn stand_in(g: &mut Gb, level: u8) -> Option<(u8, Option<Undo>)> {
    let Some(k) = (level as usize).checked_sub(extra_levels::FIRST) else { return Some((level, None)) };
    let e = extra_levels::read(g.cart().rom()).into_iter().nth(k)?;
    let mut writes = extra_levels::writes_for(g.cart().rom(), &e);
    writes.extend(secret_exits::stand_in(g.cart().rom(), level as usize, e.base as usize));
    Some((e.base, Some(extra_levels::apply(g.cart_mut().rom_mut(), &writes))))
}

/// Enter game level `level` (0-31) from the map, whatever the project's map opens: the point
/// Mario stands on (or, when it can't take Start, another plain point) opens `level` while
/// Start is pressed, then the map's own levels are back.
fn enter_from_map(gb: &mut Gb, level: u8) -> bool {
    let own = world::point_levels(gb.cart().rom());
    let here = gb.read(MAP_POINT);
    let others = (0..map_routes::POINTS as u8).filter(|&p| p != here);
    for p in std::iter::once(here).chain(others).filter(|p| !SPECIAL_MAP_POINTS.contains(p)).take(4) {
        let mut g = gb.clone();
        g.write(MAP_POINT, p);
        let mut opens = own.clone();
        opens[p as usize] = level;
        world::write_point_levels(g.cart_mut().rom_mut(), &opens);
        let entered = g.enter_level(level);
        world::write_point_levels(g.cart_mut().rom_mut(), &own);
        if entered {
            *gb = g;
            return true;
        }
    }
    false
}

/// Enter game level `level` (0-31) from the map; with `at`, Mario stands there (left x, feet
/// y) for this entry only: the header's start and the level's restart entries (a midway bell
/// rung before) are that place while it loads, then the game's again.
fn enter_at(gb: &mut Gb, level: u8, at: Option<(u16, u16)>) -> bool {
    let Some((x, feet)) = at else { return enter_from_map(gb, level) };
    let l = level as usize;
    let rom = gb.cart().rom();
    let h = levels::header(rom, l).at;
    let own_start = rom[h..h + 8].to_vec();
    let bells: Vec<usize> = (0..checkpoint::MAX).filter(|&b| b == 0 || checkpoint::installed(rom)).collect();
    let own_restarts: Vec<[u8; 8]> = bells.iter().map(|&b| checkpoint::bell_entry(rom, l, b)).collect();
    let mut place = [0u8; 8];
    Start::standing(x, feet).write(&mut place);
    let rom = gb.cart_mut().rom_mut();
    rom[h..h + 8].copy_from_slice(&place);
    for &b in &bells {
        checkpoint::set_bell_entry(rom, l, b, &place);
    }
    let entered = enter_from_map(gb, level);
    let rom = gb.cart_mut().rom_mut();
    rom[h..h + 8].copy_from_slice(&own_start);
    for (&b, own) in bells.iter().zip(&own_restarts) {
        checkpoint::set_bell_entry(rom, l, b, own);
    }
    entered
}

/// A test game on the map: the title, the file select and the new game's way onto the map
/// run unseen (Start pressed now and then until the map is up, as `boot_to_map` does), the
/// new zones following Mario as in play, until the map has been up a while with Mario on his
/// start.
fn to_map_at_start(gb: &mut Gb, zones: &mut ZoneSwap) {
    let mut on_map = 0;
    for f in 0..3000u32 {
        let press = on_map == 0 && f % 50 < 5 && f > 120;
        gb.run_frame_with(if press { Buttons::START } else { Buttons::NONE });
        gb.clear_audio();
        zones.update(gb);
        if gb.mode() == GameMode::Map {
            on_map += 1;
            if on_map == 90 {
                break;
            }
        }
    }
    gb.set_buttons(Buttons::NONE);
}

/// After each frame: when the game is about to load a level numbered 32 or more (a map
/// point the editor set to open one of its new levels), it is given that level's base
/// level with the new one in place. Checked while it is only setting up (modes $16 and $0B,
/// before the header is read).
fn take_new_level(g: &mut Gb, extra: &mut Option<(u8, Undo)>) {
    if extra.is_some() || !matches!(g.read(GAME_MODE), 0x16 | 0x0B) {
        return;
    }
    let level = g.read(LEVEL_NUMBER);
    if (level as usize) < extra_levels::FIRST {
        return;
    }
    if let Some((base, Some(undo))) = stand_in(g, level) {
        g.write(LEVEL_NUMBER, base);
        *extra = Some((level, undo));
    }
}

/// While the timer is frozen and Mario is in a level, write back the time he had when it
/// was frozen (or when the level began).
fn hold_timer(g: &mut Gb, frozen: &mut Option<[u8; 2]>, on: bool) {
    if !on || !matches!(GameMode::from_byte(g.read(GAME_MODE)), GameMode::Playing | GameMode::Paused) {
        *frozen = None;
        return;
    }
    match frozen {
        Some(t) => cheat_rules::set_timer(g, *t),
        None => *frozen = Some(cheat_rules::timer(g)),
    }
}

/// How long the tag with the menu's key (and the cheats mode's keys) stays up.
const HINT_SECONDS: f32 = 1.5;

/// Rewind keeps a state every this many frames, as many as `REWIND_STATES` (10 seconds).
const REWIND_EVERY: u32 = 2;
const REWIND_STATES: usize = 300;
/// Seconds a save state note stays on the screen.
const NOTE_SECONDS: f32 = 2.0;

/// A game saved with F6: the console with its program as it was, and the session's own
/// state that goes with that program.
#[derive(Clone)]
struct Saved {
    gb: Gb,
    extra: Option<(u8, Undo)>,
    zones: ZoneSwap,
    frozen: Option<[u8; 2]>,
}

/// The states rewind can go back to, all taken with the same program in memory.
#[derive(Default)]
struct Rewind {
    states: VecDeque<State>,
    program: Option<(u32, Option<u8>, bool, u32)>,
    frames: u32,
}

impl Rewind {
    fn keep(&mut self, g: &mut Gb, program: (u32, Option<u8>, bool, u32)) {
        if self.program != Some(program) {
            self.states.clear();
            self.program = Some(program);
        }
        self.frames += 1;
        if self.frames % REWIND_EVERY == 0 {
            if self.states.len() == REWIND_STATES {
                self.states.pop_front();
            }
            self.states.push_back(g.state());
        }
    }

    fn back(&mut self, g: &mut Gb, program: (u32, Option<u8>, bool, u32)) {
        if self.program != Some(program) {
            self.states.clear();
            return;
        }
        if self.states.len() > 1 {
            self.states.pop_back();
        }
        if let Some(s) = self.states.back() {
            g.set_state(s);
        }
    }
}

/// What a session is started with, besides the console itself.
/// A test game from the editor: Mario gets these lives and coins once the game is on the map
/// or in a level, and this power each time the tested level is entered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TestGame {
    pub lives: u8,
    pub coins: u16,
    /// `ram_map::POWER`'s value: 0 small, 1 big, 2 carrot, 3 fire.
    pub power: u8,
}

pub struct SessionParts {
    pub prefs: Preferences,
    pub saves: Box<dyn SaveStore>,
    pub zones: Box<dyn ZoneProgressStore>,
    /// Go straight into this level (the level editor's Play button).
    pub start_level: Option<u8>,
    /// With `start_level`: Mario's left x and feet y for this start only.
    pub start_at: Option<(u16, u16)>,
    /// Unattended run for a check: no menu, faster, no hint.
    pub checking: bool,
    /// The debug / cheats mode is on.
    pub cheats: bool,
    /// A test game from the editor.
    pub test: Option<TestGame>,
    /// A test game on the map: Mario's start for this game only (the zone editor's Here);
    /// None: the copy's own.
    pub start_point: Option<start_point::Start>,
}

pub struct GameSession {
    gb: Gb,
    prefs: Preferences,
    saves: Box<dyn SaveStore>,
    menu: Option<Menu>,
    checking: bool,
    acc: f64,
    save_timer: f32,
    /// After the menu closes, the game sees no buttons until every key is let go, so the
    /// key that closed it doesn't also press a button in the game.
    input_lock: bool,
    start_tap: u32,
    hint_time: f32,
    prev_mode: GameMode,
    screen: Vec<u8>,
    /// The new level being played (its number, and how to put its base level back once
    /// Mario is on the map again).
    extra: Option<(u8, Undo)>,
    zones: ZoneSwap,
    /// The cheats that stay on (None: the cheats mode is off).
    cheats: Option<Cheats>,
    /// The time held while the timer is frozen.
    frozen: Option<[u8; 2]>,
    saved: Option<Box<Saved>>,
    rewind: Rewind,
    /// Times the session swapped the program in memory itself (loading a saved game).
    loads: u32,
    note: Option<(String, f32)>,
    /// A test game's lives and coins, until given.
    boost: Option<(u8, u16)>,
    /// A test game started in a level: it is entered again after a lost life.
    retry: Option<Retry>,
    /// A test game's power for Mario, given at each entry from the map.
    power: Option<u8>,
    /// A test game from the editor (a level or the copy's map).
    from_editor: bool,
    /// The menu offers Quit (not in the browser, where the page is closed instead).
    can_quit: bool,
    /// The inputs there are now, for the menu's Controls.
    devices: Devices,
}

impl GameSession {
    /// Where the program cannot be quit from its menu: no Quit row.
    pub fn without_quit(mut self) -> GameSession {
        self.can_quit = false;
        self
    }

    pub fn new(mut gb: Gb, parts: SessionParts) -> GameSession {
        // the zone editor's Here: a new game begins there (in the console's memory only)
        if let Some(s) = parts.start_point.filter(|_| parts.test.is_some() && parts.start_level.is_none()) {
            start_point::write(gb.cart_mut().rom_mut(), s);
        }
        let mut zones = ZoneSwap::new(&mut gb, parts.zones);
        if parts.test.is_some() && parts.start_level.is_none() && !parts.checking {
            to_map_at_start(&mut gb, &mut zones);
        }
        let mut extra = None;
        if let Some(n) = parts.start_level {
            if gb.boot_to_map() {
                if let Some((ask, undo)) = stand_in(&mut gb, n) {
                    extra = undo.map(|u| (n, u));
                    if let Some(t) = parts.test {
                        gb.write(POWER, t.power);
                    }
                    enter_at(&mut gb, ask.min(31), parts.start_at);
                }
            }
        }
        let retry = parts.test.and(parts.start_level).map(|level| Retry::new(Entry { level, at: parts.start_at }));
        let screen = gb.screen().to_vec();
        GameSession {
            gb,
            prefs: parts.prefs,
            saves: parts.saves,
            menu: None,
            checking: parts.checking,
            acc: 0.0,
            save_timer: 0.0,
            input_lock: false,
            start_tap: 0,
            hint_time: if parts.checking { 0.0 } else { HINT_SECONDS },
            prev_mode: GameMode::Other(0),
            screen,
            extra,
            zones,
            cheats: parts.cheats.then(Cheats::default),
            frozen: None,
            saved: None,
            rewind: Rewind::default(),
            loads: 0,
            note: None,
            boost: parts.test.map(|t| (t.lives, t.coins)),
            power: parts.test.map(|t| t.power),
            from_editor: parts.test.is_some(),
            retry,
            can_quit: true,
            devices: Devices::default(),
        }
    }

    /// The inputs the device has now (the adapters detect them each frame).
    pub fn set_devices(&mut self, devices: Devices) {
        self.devices = devices;
    }

    /// The on-screen controls back to Auto, after they were turned off where nothing else plays.
    pub fn show_touch_controls(&mut self) {
        let before = self.prefs.settings.clone();
        self.prefs.settings.touch = TouchShow::Auto;
        self.prefs.changed_from(&before);
    }

    /// What the program in memory depends on: zone swaps, the new level in place, invincible,
    /// loads.
    fn program(&self) -> (u32, Option<u8>, bool, u32) {
        (self.zones.swaps(), self.extra.as_ref().map(|e| e.0), self.cheats.is_some_and(|c| c.invincible), self.loads)
    }

    /// The save and load state bindings (F6 and F7) in the cheats mode.
    fn save_states(&mut self, input: &dyn Keyboard, pad: &dyn Gamepad) {
        let Some(state) = self.cheats else { return };
        let t = texts(self.prefs.settings.lang);
        let st = self.prefs.settings.clone();
        let b = Bindings::new(input, pad, &st);
        if b.pressed(keys::SAVE_STATE) {
            self.saved = Some(Box::new(Saved { gb: self.gb.clone(), extra: self.extra.clone(), zones: self.zones.clone(), frozen: self.frozen }));
            self.note = Some((self.naming_keys(t.saved), NOTE_SECONDS));
        }
        if b.pressed(keys::LOAD_STATE) {
            match self.saved.as_deref().cloned() {
                Some(s) => {
                    self.gb = s.gb;
                    self.extra = s.extra;
                    self.zones = s.zones;
                    self.frozen = s.frozen;
                    cheat_rules::set_invincible(self.gb.cart_mut().rom_mut(), state.invincible);
                    self.loads += 1;
                    self.acc = 0.0;
                    self.note = Some((t.loaded.to_string(), NOTE_SECONDS));
                }
                None => self.note = Some((self.naming_keys(t.nothing_saved), NOTE_SECONDS)),
            }
        }
    }

    /// A save state note with its keys (the texts say F6 and F7) as the input that plays names
    /// them; the on-screen controls have their own pictures, and the keys are left out.
    fn naming_keys(&self, text: &str) -> String {
        let st = &self.prefs.settings;
        let label = |i: usize| match self.devices.prompts() {
            Prompts::Controller => Some(st.pad[i].label()),
            Prompts::Keyboard => Some(st.keys[i].label(st.lang)),
            Prompts::Touch => None,
        };
        match (label(keys::SAVE_STATE), label(keys::LOAD_STATE)) {
            (Some(save), Some(load)) => text.replace("F6", save).replace("F7", load),
            _ => text.split(" (").next().unwrap_or(text).to_string(),
        }
    }

    /// Hold the rewind binding to rewind, in the cheats mode.
    fn rewinding(&self, input: &dyn Keyboard, pad: &dyn Gamepad) -> bool {
        self.cheats.is_some() && Bindings::new(input, pad, &self.prefs.settings).held(keys::REWIND)
    }

    /// The start-up tag's lines: what opens the menu (Start in a level, else the menu binding)
    /// and the cheats mode's keys, named for the input that plays (`Prompts`). The on-screen
    /// controls show their own: only Start is named.
    fn hint_lines(&self, in_level: bool) -> Vec<String> {
        let st = &self.prefs.settings;
        let t = texts(st.lang);
        let prompts = self.devices.prompts();
        let key = |i: usize| match prompts {
            Prompts::Controller => st.pad[i].label(),
            Prompts::Keyboard => st.keys[i].label(st.lang),
            Prompts::Touch => "START",
        };
        if prompts == Prompts::Touch {
            return if in_level { vec![format!("{}: {}", key(keys::START), t.hint_menu)] } else { Vec::new() };
        }
        let menu = if in_level { key(keys::START) } else { key(keys::MENU) };
        let mut lines = vec![format!("{menu}: {}", t.hint_menu)];
        if self.cheats.is_some() {
            lines.push(format!("{}: {}", key(keys::SAVE_STATE), t.hint_save));
            lines.push(format!("{}: {}", key(keys::LOAD_STATE), t.hint_load));
            lines.push(format!("{}: {}", key(keys::REWIND), t.hint_rewind));
        }
        lines
    }

    /// The port's actions the on-screen controls offer now (`keys` indexes): fast forward always;
    /// rewind and the save states in the cheats mode; restarting a level in a level of a test game
    /// or with the cheats mode. None while the menu is open.
    pub fn touch_extras(&mut self) -> Vec<usize> {
        if self.menu.is_some() || self.checking {
            return Vec::new();
        }
        let mut extras = vec![keys::FAST_FORWARD];
        if self.cheats.is_some() {
            extras.extend([keys::REWIND, keys::SAVE_STATE, keys::LOAD_STATE]);
        }
        if (self.cheats.is_some() || self.retry.is_some()) && self.gb.mode() == GameMode::Playing {
            extras.push(keys::RESTART);
        }
        extras
    }

    /// Leave for the map and enter `level` (a new level: its base level with it in place),
    /// from its own point when the maps have one.
    fn go_to(&mut self, level: u8) {
        self.close_menu();
        if !cheat_rules::to_map(&mut self.gb) {
            return;
        }
        if let Some((_, undo)) = self.extra.take() {
            extra_levels::apply(self.gb.cart_mut().rom_mut(), &undo);
        }
        let point = cart_model::world::point_levels(self.gb.cart().rom()).iter().position(|&l| l == level);
        if let Some(p) = point {
            self.gb.show_map_at(p as u8);
        }
        self.enter(Entry { level, at: None });
    }

    /// Start the level being played again, as the game's pause menu leaves it (no life lost):
    /// a test game's at the place it was played from, any other at its own start.
    fn restart_level(&mut self) {
        self.close_menu();
        let playing = self.extra.as_ref().map_or_else(|| self.gb.read(LEVEL_NUMBER), |e| e.0);
        let entry = self.retry.as_ref().map_or(Entry { level: playing, at: None }, |r| r.entry());
        if !cheat_rules::to_map(&mut self.gb) {
            return;
        }
        if let Some((_, undo)) = self.extra.take() {
            extra_levels::apply(self.gb.cart_mut().rom_mut(), &undo);
        }
        self.enter(entry);
    }

    /// Enter `entry`'s level from the map (a new level: its base level with it in place).
    fn enter(&mut self, entry: Entry) {
        let Some((ask, undo)) = stand_in(&mut self.gb, entry.level) else { return };
        self.extra = undo.map(|u| (entry.level, u));
        if let Some(p) = self.power {
            self.gb.write(POWER, p);
        }
        enter_at(&mut self.gb, ask.min(31), entry.at);
        self.acc = 0.0;
    }

    pub fn settings(&self) -> &Settings {
        &self.prefs.settings
    }

    /// What to show: the game's picture with the port's own drawing on top, in shades 0-3.
    pub fn screen(&self) -> &[u8] {
        &self.screen
    }

    pub fn palette(&self) -> &'static Palette {
        &PALETTES[self.prefs.settings.palette % PALETTES.len()]
    }

    /// Write the battery save (also done every few seconds while playing).
    pub fn finish(&mut self) {
        self.saves.save(&mut self.gb);
    }

    /// One window frame with no controller (tests).
    #[cfg(test)]
    pub fn frame(&mut self, dt: f32, input: &dyn Keyboard, audio: &dyn AudioOut) -> Flow {
        self.frame_with(dt, input, &crate::ports::NoPad, audio)
    }

    /// One window frame lasting `dt` seconds, read from the keyboard and the controller.
    pub fn frame_with(&mut self, dt: f32, input: &dyn Keyboard, pad: &dyn Gamepad, audio: &dyn AudioOut) -> Flow {
        if input.pressed(Key::F11) {
            self.prefs.toggle_fullscreen();
        }
        let g = &mut self.gb;
        // In a level the Start key opens this pause menu; elsewhere it is the game's own
        // Start. Esc opens it anywhere.
        let mode = g.mode();
        let in_level = mode == GameMode::Playing;
        // Show the "ESC: menu / Enter: pause" tag again each time a level begins.
        if in_level && self.prev_mode != GameMode::Playing && self.prev_mode != GameMode::Paused && !self.checking {
            self.hint_time = HINT_SECONDS;
        }
        self.prev_mode = mode;
        if let (Some((lives, coins)), true) = (self.boost, in_level || mode == GameMode::Map) {
            cart_model::cheats::set_lives(g, lives);
            cart_model::cheats::set_coins(g, coins);
            self.boost = None;
        }
        let playing = in_level.then(|| self.extra.as_ref().map_or_else(|| g.read(LEVEL_NUMBER), |e| e.0));
        // back on the map: the base level of a new level is itself again
        if mode == GameMode::Map {
            if let Some((_, undo)) = self.extra.take() {
                extra_levels::apply(g.cart_mut().rom_mut(), &undo);
            }
        }
        let lives = g.read(LIVES);
        if let Some(again) = self.retry.as_mut().and_then(|r| r.step(mode, playing, lives)) {
            self.enter(again);
        }

        if self.menu.is_none() && !self.checking {
            self.save_states(input, pad);
        }
        // the Restart level key: at once, in a level of a test game or with the cheats mode on
        let may_restart = self.cheats.is_some() || self.retry.is_some();
        if self.menu.is_none() && !self.checking && in_level && !self.input_lock && may_restart && Bindings::new(input, pad, &self.prefs.settings).pressed(keys::RESTART) {
            self.restart_level();
        }
        let g = &mut self.gb;
        if self.menu.is_none() {
            if !self.checking {
                let b = Bindings::new(input, pad, &self.prefs.settings);
                let start_key = b.pressed(keys::START) && in_level && !self.input_lock;
                if b.pressed(keys::MENU) || start_key || b.guide() {
                    let mut menu = Menu::new(mode);
                    if let Some(state) = self.cheats {
                        let icons = in_level.then(|| Item::ALL.map(|i| cheat_rules::item_picture(g, i)));
                        let secret_exit = cheat_rules::has_secret_exit(g);
                        let names = extra_levels::read(g.cart().rom());
                        let levels = (0..extra_levels::FIRST as u8).map(|l| (l, String::new())).chain(names.into_iter().enumerate().map(|(k, e)| ((extra_levels::FIRST + k) as u8, if e.name.is_empty() { format!("NEW {}", extra_levels::FIRST + k) } else { e.name }))).collect();
                        menu = menu.with_cheats(CheatMenu { state, icons, secret_exit, levels });
                    }
                    if self.cheats.is_some() || self.retry.is_some() {
                        menu = menu.with_restart();
                    }
                    if self.retry.is_some() {
                        menu = menu.without_back_to_map();
                    }
                    if self.from_editor {
                        menu = menu.without_file_select();
                    }
                    if !self.can_quit {
                        menu = menu.without_quit();
                    }
                    self.menu = Some(menu);
                }
            }
        } else if let Some(m) = self.menu.as_mut() {
            m.set_devices(self.devices);
            let before = self.prefs.settings.clone();
            let uses_pad = pad.connected();
            let menu_input = MenuInput::read(input, uses_pad.then_some(pad), &self.prefs.settings);
            let action = m.update(&menu_input, &mut self.prefs.settings);
            self.prefs.changed_from(&before);
            match action {
                Action::None => {}
                Action::Close => self.close_menu(),
                Action::PressStart => {
                    self.close_menu();
                    self.start_tap = 8;
                }
                Action::Quit => return Flow::Quit,
                Action::BackToMap => {
                    self.close_menu();
                    self.gb.back_to_map();
                }
                Action::RestartLevel => self.restart_level(),
                Action::FileSelect => {
                    self.close_menu();
                    self.gb.back_to_file_select();
                    self.acc = 0.0;
                }
                Action::Cheat(c) => self.cheat(c),
                Action::Summon(item) => {
                    self.close_menu();
                    cheat_rules::summon(&mut self.gb, item);
                }
                Action::GoTo(level) => self.go_to(level),
            }
        }

        if self.checking {
            // Checks: run the game a little faster so the title screen is up.
            for _ in 0..4 {
                self.gb.run_frame();
            }
            self.gb.clear_audio();
        } else if self.menu.is_some() {
            // Paused: the game is frozen and silent.
            self.gb.clear_audio();
            self.acc = 0.0;
        } else {
            self.play(dt, input, pad, audio, in_level);
        }

        let st = &self.prefs.settings;
        self.screen.copy_from_slice(self.gb.screen());
        if self.cheats.is_some_and(|c| c.objects) {
            draw_objects(&mut self.screen, &cheat_rules::active_objects(&mut self.gb));
        }
        if self.cheats.is_some_and(|c| c.info) {
            draw_info(&mut self.screen, &cheat_rules::info(&mut self.gb));
        }
        if let Some((text, left)) = self.note.take() {
            draw_note(&mut self.screen, &text);
            self.note = (left > dt).then_some((text, left - dt));
        }
        if let Some(m) = &self.menu {
            m.draw(&mut self.screen, st);
        } else if self.hint_time > 0.0 {
            self.hint_time -= dt;
            let playing = self.gb.mode() == GameMode::Playing;
            let lines = self.hint_lines(playing);
            if !lines.is_empty() {
                draw_hint(&mut self.screen, &lines);
            }
        }
        Flow::Continue
    }

    fn close_menu(&mut self) {
        self.menu = None;
        self.input_lock = true;
    }

    fn cheat(&mut self, c: Cheat) {
        let Some(state) = self.cheats.as_mut() else { return };
        state.toggle(c);
        let g = &mut self.gb;
        match c {
            Cheat::Invincible => {
                cheat_rules::set_invincible(g.cart_mut().rom_mut(), state.invincible);
            }
            Cheat::FinishLevel => {
                self.close_menu();
                cheat_rules::finish_level(&mut self.gb);
            }
            Cheat::SecretExit => {
                self.close_menu();
                cheat_rules::take_secret_exit(&mut self.gb);
            }
            Cheat::MaxLives => cheat_rules::max_lives(g),
            Cheat::MaxCoins => cheat_rules::max_coins(g),
            Cheat::GoldenCoins => cheat_rules::golden_coins(g),
            Cheat::OpenMap => cheat_rules::open_map(g),
            Cheat::GoToLevel | Cheat::Summon | Cheat::FreezeTimer | Cheat::Info | Cheat::ObjectsView => {}
        }
    }

    /// The game runs: buttons in, frames at the console's pace, sound out, periodic saves.
    fn play(&mut self, dt: f32, input: &dyn Keyboard, pad: &dyn Gamepad, audio: &dyn AudioOut, in_level: bool) {
        let program = self.program();
        if self.rewinding(input, pad) {
            self.rewind.back(&mut self.gb, program);
            self.gb.clear_audio();
            self.acc = 0.0;
            return;
        }
        let keep = self.cheats.is_some();
        let st = &self.prefs.settings;
        let g = &mut self.gb;
        let bindings = Bindings::new(input, pad, st);
        let mut b = bindings.buttons();
        let fast = bindings.held(keys::FAST_FORWARD);
        let held_any = !b.is_empty() || input.held(Key::Enter) || input.held(Key::Space) || bindings.held(keys::MENU);
        if self.input_lock {
            if held_any {
                b = Buttons::NONE;
            } else {
                self.input_lock = false;
            }
        }
        if in_level {
            b &= !Buttons::START;
        }
        if self.start_tap > 0 {
            self.start_tap -= 1;
            b |= Buttons::START;
        }
        b |= self.zones.buttons();
        g.set_buttons(b);
        self.acc += dt.min(0.1) as f64;
        // Keep the sound queue fed: nudge timing when it runs low or high.
        let q = audio.queued_seconds();
        let target = FRAME_SECONDS * if q < 0.03 { 0.9 } else if q > 0.09 { 1.1 } else { 1.0 };
        let mut ran = 0;
        let freeze = self.cheats.is_some_and(|c| c.freeze_timer);
        while self.acc >= target && ran < 4 {
            self.acc -= target;
            g.run_frame();
            take_new_level(g, &mut self.extra);
            hold_timer(g, &mut self.frozen, freeze);
            if keep {
                let program = (self.zones.swaps(), self.extra.as_ref().map(|e| e.0), program.2, program.3);
                self.rewind.keep(g, program);
            }
            ran += 1;
        }
        if fast {
            for _ in 0..3 {
                g.run_frame();
                take_new_level(g, &mut self.extra);
                hold_timer(g, &mut self.frozen, freeze);
            }
            g.clear_audio();
        }
        audio.push(g.audio(), st.volume as f32 / 10.0);
        g.clear_audio();
        self.save_timer += dt;
        if self.zones.update(g) || self.save_timer > 5.0 {
            self.save_timer = 0.0;
            self.saves.save(g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::zone_progress::{MapKey, ZoneProgress};
    use crate::ports::{SettingsStore, Window};
    use cart_model::ram_map::{PLAYER_X, PLAYER_Y};
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    /// Keys held during the current frame; a key counts as pressed on its first frame.
    #[derive(Default)]
    struct FakeKeys {
        now: Vec<Key>,
        before: Vec<Key>,
    }
    impl FakeKeys {
        fn set(&mut self, keys: &[Key]) {
            self.before = std::mem::replace(&mut self.now, keys.to_vec());
        }
    }
    impl Keyboard for FakeKeys {
        fn pressed(&self, k: Key) -> bool {
            self.now.contains(&k) && !self.before.contains(&k)
        }
        fn held(&self, k: Key) -> bool {
            self.now.contains(&k)
        }
        fn last_pressed(&self) -> Option<Key> {
            self.now.iter().copied().find(|k| !self.before.contains(k))
        }
    }

    struct Silence;
    impl AudioOut for Silence {
        fn sample_rate(&self) -> u32 {
            44100
        }
        fn push(&self, _: &[f32], _: f32) {}
        fn queued_seconds(&self) -> f32 {
            0.05
        }
    }

    #[derive(Clone, Default)]
    struct Record {
        settings_saved: Rc<RefCell<Vec<Settings>>>,
        fullscreen: Rc<Cell<bool>>,
        battery_saves: Rc<Cell<u32>>,
        zones: Rc<RefCell<Option<ZoneProgress>>>,
    }
    impl SettingsStore for Record {
        fn load(&self) -> Settings {
            Settings::default()
        }
        fn save(&self, s: &Settings) {
            self.settings_saved.borrow_mut().push(s.clone());
        }
    }
    impl Window for Record {
        fn set_fullscreen(&mut self, on: bool) {
            self.fullscreen.set(on);
        }
    }
    impl ZoneProgressStore for Record {
        fn load(&self) -> Option<ZoneProgress> {
            self.zones.borrow().clone()
        }
        fn save(&self, p: &ZoneProgress) {
            *self.zones.borrow_mut() = Some(p.clone());
        }
    }
    impl SaveStore for Record {
        fn save(&self, _: &mut Gb) {
            self.battery_saves.set(self.battery_saves.get() + 1);
        }
    }

    /// Plays from power-on into the first level with keys only, then uses the pause menu.
    #[test]
    fn play_into_a_level_and_use_the_menu() {
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let gb = host::load_gb(&rom, 44100, None).unwrap();
        let rec = Record::default();
        let prefs = Preferences::new(Box::new(rec.clone()), Box::new(rec.clone()));
        let parts = SessionParts {
            prefs,
            saves: Box::new(rec.clone()),
            zones: Box::new(rec.clone()),
            start_level: None,
            start_at: None,
            checking: false,
            cheats: false,
            test: None,
            start_point: None,
        };
        let mut s = GameSession::new(gb, parts);
        let mut keys = FakeKeys::default();
        let frame = |s: &mut GameSession, keys: &mut FakeKeys, held: &[Key]| {
            keys.set(held);
            assert_eq!(s.frame(FRAME_SECONDS as f32, keys, &Silence), Flow::Continue);
        };

        // Title, file select, map: Start (Enter) now and then until a level is playing.
        let mut f = 0;
        while s.gb.mode() != GameMode::Playing {
            assert!(f < 3000, "never reached a level");
            let start = f % 150 >= 140;
            frame(&mut s, &mut keys, if start { &[Key::Enter] } else { &[] });
            f += 1;
        }
        assert!(rec.battery_saves.get() > 0, "a battery save every 5 seconds of play");

        // Esc opens the menu over a frozen game.
        frame(&mut s, &mut keys, &[]);
        frame(&mut s, &mut keys, &[Key::Escape]);
        let frozen = s.gb.screen().to_vec();
        for _ in 0..30 {
            frame(&mut s, &mut keys, &[]);
        }
        assert_eq!(s.gb.screen().to_vec(), frozen);
        assert_ne!(s.screen(), &frozen[..], "menu drawn over the picture");

        // Display → full screen on: saved and applied to the window.
        for k in [Key::Down, Key::Down, Key::Enter, Key::Down, Key::Enter] {
            frame(&mut s, &mut keys, &[]);
            frame(&mut s, &mut keys, &[k]);
        }
        assert!(s.settings().fullscreen);
        assert!(rec.fullscreen.get());
        assert!(rec.settings_saved.borrow().last().unwrap().fullscreen);

        // Back out and close: the game runs again.
        for k in [Key::Escape, Key::Escape] {
            frame(&mut s, &mut keys, &[]);
            frame(&mut s, &mut keys, &[k]);
        }
        // (the "ESC: MENU" tag shown when the level began stays for 3 seconds of play)
        for _ in 0..200 {
            frame(&mut s, &mut keys, &[]);
        }
        assert_eq!(s.screen(), s.gb.screen().as_slice());

        let before = rec.battery_saves.get();
        s.finish();
        assert_eq!(rec.battery_saves.get(), before + 1);
    }

    /// A session in `level` with the cheats mode as given, and Mario at `start` if any.
    fn in_level(level: u8, cheats: bool, start: Option<(u16, u16)>) -> Option<(GameSession, Record)> {
        let rom = host::test_cartridge()?;
        let gb = host::load_gb(&rom, 44100, None).unwrap();
        let rec = Record::default();
        let parts = SessionParts {
            prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
            saves: Box::new(rec.clone()),
            zones: Box::new(rec.clone()),
            start_level: Some(level),
            start_at: start,
            checking: false,
            cheats,
            test: None,
            start_point: None,
        };
        Some((GameSession::new(gb, parts), rec))
    }

    fn frames(s: &mut GameSession, keys: &mut FakeKeys, held: &[Key], n: u32) {
        for _ in 0..n {
            keys.set(held);
            s.frame(FRAME_SECONDS as f32, keys, &Silence);
        }
    }

    /// F6 in level 1, a life lost, F7: back in level 1 as it was. Without anything saved F7
    /// says so; with the cheats mode off neither key does anything.
    #[test]
    fn save_and_load_the_game() {
        let Some((mut s, _)) = in_level(1, true, None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut keys = FakeKeys::default();
        frames(&mut s, &mut keys, &[], 30);
        frames(&mut s, &mut keys, &[Key::F7], 1);
        assert_eq!(s.note.as_ref().map(|n| n.0.as_str()), Some(texts(0).nothing_saved));
        frames(&mut s, &mut keys, &[Key::F6], 1);
        let (lives, x) = (s.gb.read(cart_model::ram_map::LIVES), s.gb.read(cart_model::ram_map::PLAYER_X));
        s.gb.write(0xA224, 0x80);
        frames(&mut s, &mut keys, &[], 400);
        assert_ne!(s.gb.mode(), GameMode::Playing, "the life was lost");
        frames(&mut s, &mut keys, &[Key::F7], 1);
        assert_eq!((s.gb.mode(), s.gb.read(LEVEL_NUMBER)), (GameMode::Playing, 1));
        assert_eq!((s.gb.read(cart_model::ram_map::LIVES), s.gb.read(cart_model::ram_map::PLAYER_X)), (lives, x));

        let (mut off, _) = in_level(1, false, None).unwrap();
        frames(&mut off, &mut keys, &[Key::F6], 1);
        frames(&mut off, &mut keys, &[Key::F7], 1);
        assert!(off.saved.is_none() && off.note.is_none());
    }

    /// Walking right, then holding R: Mario goes back to the left.
    #[test]
    fn rewind_goes_back() {
        let Some((mut s, _)) = in_level(1, true, None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut keys = FakeKeys::default();
        let x = |s: &mut GameSession| s.gb.read(cart_model::ram_map::PLAYER_X) as u16 | (s.gb.read(cart_model::ram_map::PLAYER_X + 1) as u16) << 8;
        frames(&mut s, &mut keys, &[], 200);
        let before = x(&mut s);
        frames(&mut s, &mut keys, &[Key::Right], 120);
        let walked = x(&mut s);
        frames(&mut s, &mut keys, &[Key::R], 60);
        assert!(walked > before && x(&mut s) < walked, "{before} {walked} {}", x(&mut s));
    }

    /// Go to level 12 from level 1, and a level started at a given place starts again at its
    /// own start after a lost life.
    #[test]
    fn go_to_a_level_and_play_from_a_place() {
        let Some((mut s, _)) = in_level(1, true, None) else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        s.go_to(12);
        assert_eq!((s.gb.mode(), s.gb.read(LEVEL_NUMBER)), (GameMode::Playing, 12));

        let (mut p, _) = in_level(3, true, Some((1200, 496))).unwrap();
        let x = |s: &mut GameSession| s.gb.read(cart_model::ram_map::PLAYER_X) as u16 | (s.gb.read(cart_model::ram_map::PLAYER_X + 1) as u16) << 8;
        assert_eq!(x(&mut p), 1200);
        let own = Start::read(&p.gb.cart().rom()[cart_model::levels::header(p.gb.cart().rom(), 3).at..]).mario_x;
        assert_ne!(own, 1200, "the header is the level's own again");
        p.gb.write(0xA224, 0x80);
        let mut keys = FakeKeys::default();
        frames(&mut p, &mut keys, &[], 600);
        p.gb.enter_level(3);
        assert_eq!(x(&mut p), own);
    }

    /// Started as the editor starts it (a level, cheats on): Invincible from the pause menu
    /// lets Mario walk through the level's first enemies, Finish the level ends it for the
    /// map, and with Invincible off again the program and the file are as they were.
    #[test]
    fn cheats_from_the_pause_menu() {
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let file = std::fs::read(&rom).unwrap();
        let gb = host::load_gb(&rom, 44100, None).unwrap();
        let original = gb.cart().rom().to_vec();
        let rec = Record::default();
        let parts = SessionParts {
            prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
            saves: Box::new(rec.clone()),
            zones: Box::new(rec.clone()),
            start_level: Some(0),
            start_at: None,
            checking: false,
            cheats: true,
            test: None,
            start_point: None,
        };
        let mut s = GameSession::new(gb, parts);
        let mut keys = FakeKeys::default();
        let press = |s: &mut GameSession, keys: &mut FakeKeys, ks: &[Key]| {
            for &k in ks {
                for held in [&[][..], &[k][..]] {
                    keys.set(held);
                    s.frame(FRAME_SECONDS as f32, keys, &Silence);
                }
            }
            keys.set(&[]);
            s.frame(FRAME_SECONDS as f32, keys, &Silence);
        };
        assert_eq!(s.gb.mode(), GameMode::Playing);
        // menu: original pause, debug / cheats > invincible
        press(&mut s, &mut keys, &[Key::Escape, Key::Down, Key::Enter, Key::Enter, Key::Escape, Key::Escape]);
        assert!(s.cheats.unwrap().invincible);
        let lives = s.gb.read(cart_model::ram_map::LIVES);
        let mut hurt = false;
        for _ in 0..900 {
            keys.set(&[Key::Right]);
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            hurt |= s.gb.read(0xA224) != 0;
        }
        assert!(!hurt && s.gb.mode() == GameMode::Playing && s.gb.read(cart_model::ram_map::LIVES) == lives, "Mario walked through the enemies");
        // invincible off again, then finish the level
        press(&mut s, &mut keys, &[Key::Escape, Key::Down, Key::Enter, Key::Enter, Key::Down, Key::Enter]);
        assert_eq!(s.gb.cart().rom(), &original[..], "the program is the game's again");
        let mut ended = false;
        for _ in 0..600 {
            keys.set(&[]);
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            ended |= s.gb.read(GAME_MODE) == 0x18;
        }
        assert!(ended && s.gb.mode() == GameMode::Map, "the level ended for the map");
        assert_eq!(std::fs::read(&rom).unwrap(), file, "the cartridge file is unchanged");
    }

    /// A new level (32) opened by a point of the game's own map (point $00, where a new game
    /// starts): the game's Start enters it, it is played in place of its base level, and the
    /// base level is back once on the map.
    #[test]
    fn a_new_level_is_played_from_the_original_map() {
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        let mut e = extra_levels::based_on(&copy, 0);
        for c in 30..90 {
            e.map[30 * cart_model::levels::COLS + c] = 1;
        }
        extra_levels::store(&mut copy, &[e.clone()]).unwrap();
        let mut points = cart_model::world::point_levels(&copy);
        points[0] = 32;
        cart_model::world::write_point_levels(&mut copy, &points);
        let gb = Gb::new(copy.clone(), 44100).unwrap();
        let rec = Record::default();
        let parts = SessionParts {
            prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
            saves: Box::new(rec.clone()),
            zones: Box::new(rec.clone()),
            start_level: None,
            start_at: None,
            checking: false,
            cheats: false,
            test: None,
            start_point: None,
        };
        let mut s = GameSession::new(gb, parts);
        let mut keys = FakeKeys::default();
        let mut f = 0;
        while s.gb.mode() != GameMode::Playing {
            assert!(f < 3000, "never reached a level");
            keys.set(if f % 150 >= 140 { &[Key::Enter] } else { &[] });
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            f += 1;
        }
        assert_eq!(s.extra.as_ref().map(|x| x.0), Some(32));
        assert_eq!(cart_model::live::map_in_memory(&s.gb), e.map, "the new level's map is played");
        keys.set(&[]);
        for _ in 0..30 {
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
        }
        s.gb.back_to_map();
        for _ in 0..600 {
            s.frame(FRAME_SECONDS as f32, &keys, &Silence);
        }
        assert_eq!(s.gb.mode(), GameMode::Map);
        assert!(s.extra.is_none() && s.gb.cart().rom() == &copy[..], "the base level is back");
    }

    /// A new level's secret exit sent into the Pumpkin Zone: played from the level editor's
    /// Play and from a map point that opens it, Mario comes out on the Pumpkin Zone's $28 and
    /// stays there.
    #[test]
    fn a_new_level_secret_exit_leads_where_chosen() {
        use cart_model::secret_exits::{self, SecretTo};
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        let e = extra_levels::based_on(&copy, 0);
        extra_levels::store(&mut copy, &[e]).unwrap();
        let mut points = cart_model::world::point_levels(&copy);
        points[0] = 32;
        cart_model::world::write_point_levels(&mut copy, &points);
        secret_exits::write(&mut copy, &[(32, SecretTo::Point(0x28))], &[]).unwrap();
        let rec = Record::default();
        let session = |start_level| {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level,
                start_at: None,
                checking: false,
                cheats: false,
                test: None,
                start_point: None,
            };
            GameSession::new(Gb::new(copy.clone(), 44100).unwrap(), parts)
        };
        let mut keys = FakeKeys::default();
        for start_level in [Some(32), None] {
            let mut s = session(start_level);
            let mut f = 0;
            while s.gb.mode() != GameMode::Playing {
                assert!(f < 3000, "never reached a level");
                keys.set(if f % 150 >= 140 { &[Key::Enter] } else { &[] });
                s.frame(FRAME_SECONDS as f32, &keys, &Silence);
                f += 1;
            }
            keys.set(&[]);
            assert_eq!(s.extra.as_ref().map(|x| x.0), Some(32));
            s.gb.write(0xA2B4, 0xFF);
            assert!(clear_level(&mut s.gb));
            for _ in 0..300 {
                s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            }
            assert_eq!((s.gb.read(cart_model::ram_map::MAP_POINT), s.gb.read(0xA68B)), (0x28, 1), "{start_level:?}: on the Pumpkin Zone's $28, staying there");
        }
    }

    /// Clear the level of the point Mario stands on as at a goal (mode $09), then play on
    /// until the map has been up for a while.
    fn clear_level(g: &mut Gb) -> bool {
        let mut playing = false;
        for f in 0..900u32 {
            g.run_frame_with(if f % 30 < 4 { Buttons::START } else { Buttons::NONE });
            if g.mode() == GameMode::Playing {
                playing = true;
                break;
            }
        }
        for _ in 0..30 {
            g.run_frame_with(Buttons::NONE);
        }
        g.write(GAME_MODE, 0x09);
        let mut on_map = 0;
        for f in 0..6000u32 {
            let mode = g.read(GAME_MODE);
            let tap = match f % 240 {
                0..=2 => Buttons::A,
                120..=122 => Buttons::START,
                _ => Buttons::NONE,
            };
            g.run_frame_with(if mode != 0x0C && mode != 0x14 { tap } else { Buttons::NONE });
            on_map = if g.read(GAME_MODE) == 0x0C { on_map + 1 } else { 0 };
            if on_map > 300 {
                break;
            }
        }
        g.clear_audio();
        playing
    }

    /// A new zone made from the Tree Zone, entered from world point $07: its own picture and
    /// a level cleared with its own flag; saved inside and resumed there; left by its exit,
    /// after which the cartridge and the Tree Zone's flags are the game's again; then entered
    /// by the Tree Zone's exit.
    #[test]
    fn a_new_zone_is_played_saved_and_left() {
        use cart_model::new_zones;
        use cart_model::ram_map::MAP_POINT;
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        let mut z = new_zones::based_on(&copy, 0).unwrap();
        for t in z.picture.iter_mut().take(60) {
            *t ^= 1;
        }
        z.points.retain(|p| p.number == 0x23);
        z.points[0].ways = Default::default();
        z.exits[0].1 = cart_model::doors::Leads::World(0x07);
        z.door = 0x07;
        new_zones::store(&mut copy, std::slice::from_ref(&z)).unwrap();
        // the Tree Zone's own exit leads into the new zone (to its door)
        let mut d = cart_model::doors::read(&copy);
        d.exits[0].1 = cart_model::doors::Leads::World(0x07);
        assert!(cart_model::doors::write(&mut copy, &d));
        let rec = Record::default();
        let session = |gb: Gb| {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level: None,
                start_at: None,
                checking: false,
                cheats: false,
                test: None,
                start_point: None,
            };
            GameSession::new(gb, parts)
        };
        let run = |s: &mut GameSession, keys: &mut FakeKeys, held: &[Key], n: u32| {
            for f in 0..n {
                keys.set(if f < 6 { held } else { &[] });
                s.frame(FRAME_SECONDS as f32, keys, &Silence);
            }
        };
        let picture = |g: &mut Gb| (0..360).map(|i| g.read(0x9800 + ((i / 20) * 32 + i % 20) as u16)).collect::<Vec<u8>>();
        let mut keys = FakeKeys::default();

        let mut s = session(Gb::new(copy.clone(), 44100).unwrap());
        assert!(s.gb.boot_to_map() && s.gb.show_map_at(0x07));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.zones.inside(), Some(0), "at the door the new zone is in place");
        run(&mut s, &mut keys, &[Key::X], 400);
        assert_eq!(s.gb.read(MAP_POINT), 0x23);
        assert_eq!(picture(&mut s.gb), z.picture, "its own picture");
        assert!(clear_level(&mut s.gb));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.gb.read(0xA849), 0x80, "its point is cleared");
        s.finish();

        // resumed: the battery save and the kept progress put Mario back in the new zone
        let ram = s.gb.cart().ram().to_vec();
        let mut gb = Gb::new(copy.clone(), 44100).unwrap();
        gb.cart_mut().set_ram(ram);
        let mut s = session(gb);
        assert_eq!(s.zones.inside(), Some(0));
        assert!(s.gb.boot_to_map());
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.gb.read(MAP_POINT), 0x23);
        assert_eq!(picture(&mut s.gb), z.picture);
        assert_eq!(s.gb.read(0xA849), 0x80);

        // out by its exit to the door, then away from it: the game's own again
        run(&mut s, &mut keys, &[Key::Down], 600);
        assert_eq!(s.gb.read(MAP_POINT), 0x07);
        assert!(s.gb.show_map_at(0x02));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(s.zones.inside(), None);
        assert!(s.gb.cart().rom() == &copy[..], "the Tree Zone is back");
        assert_eq!(s.gb.read(0xA849), 0x00, "the Tree Zone's flag is its own");
        assert_eq!(rec.zones.borrow().as_ref().unwrap().kept[&(0, 0)][0], 0x80, "the new zone's progress is kept");

        // out of the Tree Zone into the new zone: the port presses A on its door
        run(&mut s, &mut keys, &[Key::X], 400);
        assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (0x23, None), "in the Tree Zone");
        run(&mut s, &mut keys, &[Key::Down], 900);
        assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (0x23, Some(0)));
        assert_eq!(picture(&mut s.gb), z.picture, "in the new zone");

        // level 8's secret exit leads into the new zone: Mario comes back on its door and the
        // port enters it
        use cart_model::secret_exits::{self, SecretTo};
        secret_exits::write(&mut copy, &[(8, SecretTo::NewZone(0))], &[0x07]).unwrap();
        let mut s = session(Gb::new(copy.clone(), 44100).unwrap());
        assert!(s.gb.boot_to_map() && s.gb.show_map_at(0x2A) && s.gb.enter_level(8));
        run(&mut s, &mut keys, &[], 30);
        s.gb.write(0xA2B4, 0xFF);
        assert!(clear_level(&mut s.gb));
        assert_eq!(s.gb.read(MAP_POINT), 0x07, "on the new zone's door");
        run(&mut s, &mut keys, &[], 400);
        assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (0x23, Some(0)), "in the new zone");
    }

    /// Added points ($80) on the world map (beside $07) and in the Tree Zone (beside $20):
    /// walked to and cleared with their own flag bytes, kept per map; a game saved on the
    /// Tree Zone's resumes there with it cleared, and the world map's stays apart.
    #[test]
    fn added_points_are_played_kept_and_resumed() {
        use cart_model::added_points::{self, ZoneAdded};
        use cart_model::map_routes::{self, Place, Way};
        use cart_model::new_zones::ZonePoint;
        use cart_model::ram_map::MAP_POINT;
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let mut copy = cart_model::patch::expand(&std::fs::read(rom).unwrap()).unwrap();
        assert!(added_points::install(&mut copy));
        let p = added_points::FIRST;
        let step = [(24, 0), (-24, 0), (0, -24), (0, 24)];
        let keys_of = [Key::Right, Key::Left, Key::Up, Key::Down];
        let back = [1, 0, 3, 2];
        let mut r = map_routes::read(&copy);
        let free = |r: &map_routes::MapRoutes, q: u8| (0..4).find(|&d| r.ways[q as usize][d].is_none()).unwrap();
        // the world map's: in the routes as written
        let (dw, dz) = (free(&r, 0x07), free(&r, 0x20));
        let (x, y) = r.places[0x07].on_world();
        r.places[p as usize] = Place::at_world(x + step[dw].0, y + step[dw].1);
        r.ways[0x07][dw] = Some(Way::through(&[step[dw]], p));
        r.ways[p as usize][back[dw]] = Some(Way::through(&[(-step[dw].0, -step[dw].1)], 0x07));
        r.added.push(p);
        // the Tree Zone's: the way to it in the routes, the point kept for the port
        let at = r.places[0x20];
        r.ways[0x20][dz] = Some(Way::through(&[step[dz]], p));
        map_routes::write(&mut copy, &r).unwrap();
        let mut levels = cart_model::world::point_levels(&copy);
        levels[p as usize] = 1;
        cart_model::world::write_point_levels(&mut copy, &levels);
        let place = Place { x: (at.x as i32 + step[dz].0) as u8, y: (at.y as i32 + step[dz].1) as u8, ..at };
        let mut ways: [Option<Way>; 4] = Default::default();
        ways[back[dz]] = Some(Way::through(&[(-step[dz].0, -step[dz].1)], 0x20));
        let tree = ZonePoint { number: p, place, ways, level: 2, flag: p, shut: 0, pass: None };
        added_points::store_zones(&mut copy, &[ZoneAdded { zone: 0, points: vec![tree] }]).unwrap();

        let rec = Record::default();
        let session = |gb: Gb| {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level: None,
                start_at: None,
                checking: false,
                cheats: false,
                test: None,
                start_point: None,
            };
            GameSession::new(gb, parts)
        };
        let run = |s: &mut GameSession, keys: &mut FakeKeys, held: &[Key], n: u32| {
            for f in 0..n {
                keys.set(if f < 6 { held } else { &[] });
                s.frame(FRAME_SECONDS as f32, keys, &Silence);
            }
        };
        let own = |g: &mut Gb| g.read(added_points::FLAG_RAM);
        let mut keys = FakeKeys::default();

        let mut s = session(Gb::new(copy.clone(), 44100).unwrap());
        assert!(s.gb.boot_to_map() && s.gb.show_map_at(0x20));
        run(&mut s, &mut keys, &[], 10);
        run(&mut s, &mut keys, &[keys_of[dz]], 300);
        assert_eq!(s.gb.read(MAP_POINT), p, "walked to the Tree Zone's added point");
        let flags: Vec<u8> = (0xA848..0xA872).map(|a| s.gb.read(a)).collect();
        assert!(clear_level(&mut s.gb));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!((s.gb.read(MAP_POINT), own(&mut s.gb)), (p, 0x80), "back on it, cleared");
        assert_eq!((0xA848..0xA872).map(|a| s.gb.read(a)).collect::<Vec<u8>>(), flags, "no flag of the game changed");
        s.finish();

        // resumed on the Tree Zone's added point, with it cleared
        let ram = s.gb.cart().ram().to_vec();
        let mut gb = Gb::new(copy.clone(), 44100).unwrap();
        gb.cart_mut().set_ram(ram);
        let mut s = session(gb);
        let mut f = 0;
        while s.gb.mode() != GameMode::Map {
            assert!(f < 3000, "never reached the map");
            run(&mut s, &mut keys, if f % 150 == 140 { &[Key::Enter] } else { &[] }, 1);
            f += 1;
        }
        run(&mut s, &mut keys, &[], 10);
        assert_eq!((s.gb.read(MAP_POINT), s.gb.read(0xA68B), own(&mut s.gb)), (p, 2, 0x80), "in the Tree Zone");
        run(&mut s, &mut keys, &[keys_of[back[dz]]], 300);
        assert_eq!(s.gb.read(MAP_POINT), 0x20);

        // the world map's added point: its own flag, not cleared
        assert!(s.gb.show_map_at(0x07));
        run(&mut s, &mut keys, &[], 10);
        assert_eq!(own(&mut s.gb), 0, "the world map's added point is not cleared");
        run(&mut s, &mut keys, &[keys_of[dw]], 300);
        assert_eq!(s.gb.read(MAP_POINT), p, "walked to the world map's added point");
        assert!(s.gb.cart().rom() == &copy[..], "the world map's added points are the copy's");
        assert_eq!(rec.zones.borrow().as_ref().unwrap().added[&(MapKey::Zone(0), 0)][0], 0x80, "the Tree Zone's progress is kept");
    }

    /// A test game from Play from here: Mario, made to lose a life after a midway bell is rung,
    /// begins again at the chosen place; from Play, at the bell's restart place.
    #[test]
    fn a_test_game_enters_the_level_again_after_a_lost_life() {
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: session test skipped");
            return;
        };
        let pos = |s: &mut GameSession| (s.gb.read(PLAYER_X) as u16 | (s.gb.read(PLAYER_X + 1) as u16) << 8, s.gb.read(PLAYER_Y) as u16 | (s.gb.read(PLAYER_Y + 1) as u16) << 8);
        let play = |at: Option<(u16, u16)>| {
            let rec = Record::default();
            let prefs = Preferences::new(Box::new(rec.clone()), Box::new(rec.clone()));
            let parts = SessionParts { prefs, saves: Box::new(rec.clone()), zones: Box::new(rec.clone()), start_level: Some(3), start_at: at, checking: false, cheats: false, test: Some(TestGame { lives: 5, coins: 0, power: 0 }), start_point: None };
            let mut s = GameSession::new(host::load_gb(&rom, 44100, None).unwrap(), parts);
            let keys = FakeKeys::default();
            for _ in 0..30 {
                s.frame(FRAME_SECONDS as f32, &keys, &Silence);
            }
            assert_eq!(s.gb.mode(), GameMode::Playing);
            let first = pos(&mut s);
            s.gb.write(0xA2A0, 0xFF);
            s.gb.write(0xA224, 0x80);
            let mut lost = false;
            for f in 0..2000 {
                s.frame(FRAME_SECONDS as f32, &keys, &Silence);
                lost |= s.gb.mode() == GameMode::LosingLife;
                if lost && s.gb.mode() == GameMode::Playing {
                    break;
                }
                assert!(f < 1999, "never back in the level (mode {:?})", s.gb.mode());
            }
            assert_eq!((s.gb.read(LEVEL_NUMBER), s.gb.read(LIVES)), (3, 4), "level 03 again, a life less");
            (first, pos(&mut s))
        };
        let gb = host::load_gb(&rom, 44100, None).unwrap();
        let h = levels::header(gb.cart().rom(), 3).at;
        let own = Start::read(&gb.cart().rom()[h..h + 8]);
        let (first, again) = play(Some((own.mario_x + 64, own.mario_y + cart_model::level_start::FEET)));
        assert_eq!(again, first, "Play from here: the chosen place wins over the bell");
        let restart = Start::read(checkpoint::entry(gb.cart().rom(), 3).unwrap());
        let (_, again) = play(None);
        assert_eq!(again.0, restart.mario_x, "Play: the bell's restart place");
    }

    /// The zone editor's Play and Play from point: a test game is on the map from its first
    /// frame shown, a new game with Mario on his start, or on the point asked (a game zone's, a
    /// new zone's) this time only.
    #[test]
    fn a_map_test_game_begins_on_the_point_asked() {
        use cart_model::start_point::Start;
        let Some(path) = host::test_cartridge() else { return };
        let mut doc = project_model::document::EditorDocument::open(std::fs::read(path).unwrap(), None);
        let k = doc.blank_zone(1).unwrap();
        let entry = doc.new_zones[k].entry;
        doc.commit().unwrap();
        let rec = Record::default();
        let asked = [(None, 0x00, None), (Some(Start { point: 0x29, new_zone: None }), 0x29, None), (Some(Start { point: entry, new_zone: Some(k) }), entry, Some(k))];
        for (start_point, point, inside) in asked {
            let parts = SessionParts {
                prefs: Preferences::new(Box::new(rec.clone()), Box::new(rec.clone())),
                saves: Box::new(rec.clone()),
                zones: Box::new(rec.clone()),
                start_level: None,
                start_at: None,
                checking: false,
                cheats: false,
                test: Some(TestGame { lives: 5, coins: 0, power: 0 }),
                start_point,
            };
            let mut s = GameSession::new(Gb::new(doc.rom.clone(), 44100).unwrap(), parts);
            assert_eq!(s.gb.mode(), GameMode::Map, "{start_point:?}: on the map from the first frame shown");
            let mut keys = FakeKeys::default();
            frames(&mut s, &mut keys, &[], 120);
            assert_eq!((s.gb.read(MAP_POINT), s.zones.inside()), (point, inside), "{start_point:?}");
        }
        assert_eq!(cart_model::start_point::read(&doc.rom), Start::default(), "the copy keeps its own start");
    }
}
