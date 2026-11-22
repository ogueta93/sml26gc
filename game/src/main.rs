//! Native Windows port of a Game Boy cartridge: the original program runs on a
//! Game Boy implemented in Rust, drawn and heard through macroquad and cpal.
//!
//! Layers: `domain` (rules, no I/O) ← `application` (session use case) ← `ports` (traits)
//! ← `adapters` (macroquad, cpal, files, Win32, the browser's page). This file is the
//! composition root: it reads the arguments, picks the adapters (`adapters::platform` for what
//! differs between the desktop and the browser) and starts the window loop. Opened without
//! a cartridge file, it first shows the start screen: the base cartridge, then what to play
//! (`macroquad_picker`).
#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
mod adapters;
mod application;
mod checks;
mod domain;
mod ports;
mod selftest;

use adapters::fs_stores::{BatteryFile, DefaultSettings, NoBattery, NoZoneProgress, SettingsFile, ZoneFile};
use adapters::macroquad_display::{save_window_png, MacroquadDisplay, MacroquadWindow};
use adapters::macroquad_keyboard::MacroquadKeyboard;
use adapters::macroquad_picker::{Choice, Picker};
use adapters::macroquad_touch::Touch;
use adapters::platform::{self, Audio, Pad};
use adapters::system_window;
use application::preferences::Preferences;
use application::session::{Flow, GameSession, SessionParts, TestGame};
use domain::keys::Key;
use ports::{AudioOut, Gamepad, Keyboard, SaveStore, SettingsStore, ZoneProgressStore};
use std::path::PathBuf;

#[derive(Default)]
struct Args {
    rom: Option<PathBuf>,
    screenshot: Option<PathBuf>,
    self_test: Option<PathBuf>,
    dump: Option<PathBuf>,
    script: String,
    frames: u32,
    every: u32,
    /// Start straight in this level (32 and up: the editor's new levels), as the level
    /// editor's Play button asks.
    level: Option<u8>,
    /// The debug / cheats mode for this run, whatever the setting says (the editor's Play).
    cheats: bool,
    /// With `level`: Mario's left x and feet y in the level, for this start only (the level
    /// editor's Play from here).
    start: Option<(u16, u16)>,
    /// The editor's test game: no saves, Mario with `lives` and `coins` (the most when not given).
    test: bool,
    lives: Option<u8>,
    coins: Option<u16>,
    /// The language of this game only (an `i18n::Lang` id): the editor's, for its test games.
    lang: Option<i18n::Lang>,
    /// Mario's power in the test game (0 small, 1 big, 2 carrot, 3 fire).
    power: Option<u8>,
    /// A test game on the map beginning on this point (`HEX`, or `HEX@K` in new zone K).
    point: Option<cart_model::start_point::Start>,
    /// The project whose saves the game on the command line uses (none: the original's).
    project: Option<String>,
    /// With `--screenshot`: the start screen (a check picture of it).
    start_screen: bool,
}

fn parse_args(v: &[String]) -> Args {
    let mut a = Args { frames: 600, every: 60, ..Default::default() };
    let mut i = 0;
    while i < v.len() {
        let next = v.get(i + 1).cloned().unwrap_or_default();
        match v[i].as_str() {
            "--screenshot" => a.screenshot = Some(next.into()),
            "--self-test" => a.self_test = Some(next.into()),
            "--dump" => a.dump = Some(next.into()),
            "--script" => a.script = next,
            "--frames" => a.frames = next.parse().unwrap_or(600),
            "--every" => a.every = next.parse().unwrap_or(60),
            "--rom" => a.rom = Some(next.into()),
            "--level" => a.level = next.parse().ok(),
            "--start" => a.start = next.split_once(',').and_then(|(x, y)| Some((x.trim().parse().ok()?, y.trim().parse().ok()?))),
            "--project" => a.project = Some(next),
            "--lives" => a.lives = next.parse().ok(),
            "--coins" => a.coins = next.parse().ok(),
            "--power" => a.power = next.parse().ok().filter(|&p| p <= 3),
            "--point" => a.point = map_point(&next),
            "--lang" => a.lang = i18n::Lang::from_id(&next),
            "--cheats" | "--test" | "--start-screen" => {
                match v[i].as_str() {
                    "--test" => a.test = true,
                    "--start-screen" => a.start_screen = true,
                    _ => a.cheats = true,
                }
                i += 1;
                continue;
            }
            s => {
                if !s.starts_with("--") {
                    a.rom = Some(s.into());
                }
                i += 1;
                continue;
            }
        }
        i += 2; // option and its value
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cheats_flag_takes_no_value() {
        let a = parse_args(&["copy.gb".into(), "--cheats".into(), "--level".into(), "3".into()]);
        assert!(a.cheats);
        assert_eq!(a.rom, Some(PathBuf::from("copy.gb")));
        assert_eq!(a.level, Some(3));
        assert!(!parse_args(&["copy.gb".into()]).cheats);
    }

    #[test]
    fn a_start_place_is_two_numbers() {
        let a = parse_args(&["copy.gb".into(), "--level".into(), "3".into(), "--start".into(), "1200,496".into()]);
        assert_eq!(a.start, Some((1200, 496)));
        assert_eq!(parse_args(&["--start".into(), "12".into()]).start, None);
    }
}

const TITLE: &str = "SML2 6GC";

fn main() {
    host::console::attach();
    let raw = platform::arguments();
    if raw.first().map(|s| s == "--research").unwrap_or(false) {
        let rom = host::require_rom(None).unwrap_or_else(|e| {
            eprintln!("{e}");
            std::process::exit(1)
        });
        checks::research_main(&rom, &raw[1..]);
        return;
    }
    let args = parse_args(&raw);
    let found = host::require_rom(args.rom.clone());

    if let Some(out) = &args.self_test {
        let res = selftest::run(found.as_deref().ok());
        if let Err(e) = host::storage::write(out, res) {
            eprintln!("could not write {}: {e}", out.display());
            std::process::exit(1);
        }
        return;
    }
    if let Some(dir) = &args.dump {
        selftest::dump(found.as_deref().ok(), dir, &args.script, args.frames, args.every);
        return;
    }

    // A check run (--screenshot) is unattended: silent, default settings, no menu.
    let checking = args.screenshot.is_some();
    let title = if checking { format!("{TITLE} [check {}]", platform::run_tag()) } else { TITLE.to_string() };
    if checking {
        system_window::remember_foreground();
    }
    macroquad::Window::from_config(platform::window_conf(title.clone()), run(args, found, checking, title));
}

/// Plug the adapters into a session, or keep the error to show it. `saves`: the folder of the
/// game's saves (none: nothing read or written, for checks and the editor's test games).
fn start(args: &Args, found: Result<PathBuf, String>, saves: Option<PathBuf>, checking: bool, audio: &dyn AudioOut) -> Result<GameSession, (Preferences, String)> {
    let settings: Box<dyn SettingsStore> = if checking { Box::new(DefaultSettings) } else { Box::new(SettingsFile) };
    let mut prefs = Preferences::new(settings, Box::new(MacroquadWindow));
    if let Some(lang) = args.lang {
        prefs.speak_for_this_game(lang as usize);
    }
    let rom = match found {
        Ok(r) => r,
        Err(e) => return Err((prefs, e)),
    };
    let battery = saves.as_deref().map(host::battery_file);
    let gb = match host::load_gb(&rom, audio.sample_rate(), battery.as_deref()) {
        Ok(g) => g,
        Err(e) => return Err((prefs, e)),
    };
    let cheats = !checking && (args.cheats || prefs.settings.cheats);
    let (zones, store): (Box<dyn ZoneProgressStore>, Box<dyn SaveStore>) = match saves {
        Some(saves) => (Box::new(ZoneFile { saves: saves.clone() }), Box::new(BatteryFile { saves })),
        None => (Box::new(NoZoneProgress), Box::new(NoBattery)),
    };
    let parts = SessionParts {
        prefs,
        zones,
        saves: store,
        start_level: args.level,
        start_at: args.start,
        checking,
        cheats,
        test: args.test.then(|| TestGame { lives: args.lives.unwrap_or(99), coins: args.coins.unwrap_or(999), power: args.power.unwrap_or(0) }),
        start_point: args.point,
    };
    let session = GameSession::new(gb, parts);
    Ok(if platform::CAN_QUIT { session } else { session.without_quit() })
}

/// `--point`'s value: a map point in hexadecimal, `@` and a new zone's index when in one.
fn map_point(v: &str) -> Option<cart_model::start_point::Start> {
    let (p, zone) = match v.split_once('@') {
        Some((p, k)) => (p, Some(k.parse().ok()?)),
        None => (v, None),
    };
    Some(cart_model::start_point::Start { point: u8::from_str_radix(p, 16).ok()?, new_zone: zone })
}

/// Where the saves of the cartridge on the command line go.
fn command_line_saves(args: &Args, checking: bool) -> Option<PathBuf> {
    (!checking && !args.test).then(|| host::saves_dir(args.project.as_deref()))
}

/// Make the cartridge file `p` the base cartridge, if it is the supported one, and show what it
/// plays; else say why not.
fn take_cartridge(picker: &mut Picker, p: &std::path::Path) {
    let last = SettingsFile.load().last_play;
    match host::base_cartridge::choose(p) {
        Ok(()) => {
            picker.error = None;
            picker.reload(&last);
        }
        Err(e) => picker.error = Some(e.text(picker.lang())),
    }
}

/// What the window shows: the start screen, the game, or why it could not start.
enum App {
    Pick(Picker),
    Run(GameSession),
    Fail(Preferences, String),
}

async fn run(args: Args, found: Result<PathBuf, String>, checking: bool, title: String) {
    macroquad::input::prevent_quit();
    let audio = if checking { Audio::silent() } else { Audio::new() };
    let keyboard = MacroquadKeyboard;
    let mut pad = Pad::new();
    let mut display = MacroquadDisplay::new();
    let mut touch = Touch::new();
    // no cartridge on the command line: the start screen first (checks find it themselves)
    let mut app = if args.rom.is_none() && (!checking || args.start_screen) {
        App::Pick({
            let mut picker = Picker::new(&SettingsFile.load());
            picker.error = host::paths::data_dir_problem(picker.lang()).map(String::from);
            picker
        })
    } else {
        match start(&args, found, command_line_saves(&args, checking), checking, &audio) {
            Ok(s) => App::Run(s),
            Err((prefs, e)) => App::Fail(prefs, e),
        }
    };
    let mut render_frames = 0u32;

    loop {
        if checking && render_frames == 1 {
            system_window::send_to_back(&title);
        }
        let dt = macroquad::time::get_frame_time();
        // the page's own buttons are only where this frame's screen puts them again
        platform::place_page_button(platform::PageButton::Browse, None);
        platform::place_page_button(platform::PageButton::Fullscreen, None);
        let mut next = None;
        match &mut app {
            App::Pick(picker) => {
                let chosen = match picker.frame() {
                    Choice::Wait => None,
                    Choice::Quit => break,
                    Choice::Play(p) => Some(p),
                    Choice::Edit(project) => {
                        if let Err(e) = platform::open_editor(project.as_deref()) {
                            picker.error = Some(e);
                        }
                        None
                    }
                    Choice::Cheats => {
                        picker.cheats = !picker.cheats;
                        let mut s = SettingsFile.load();
                        s.cheats = picker.cheats;
                        SettingsFile.save(&s);
                        None
                    }
                    Choice::Language(lang) => {
                        let mut s = SettingsFile.load();
                        (s.lang, s.lang_auto) = (lang as usize, false);
                        SettingsFile.save(&s);
                        None
                    }
                    Choice::Browse => {
                        let (t, kind) = picker.dialog_texts();
                        if let Some(p) = system_window::pick_cartridge(&title, picker.start_dir().as_deref(), t, kind) {
                            take_cartridge(picker, &p);
                        } else if !system_window::dialog_available() {
                            picker.error = Some(host::file_dialog::unavailable_text(picker.lang()).into());
                        }
                        None
                    }
                };
                if let Some(p) = system_window::arrived_cartridge() {
                    take_cartridge(picker, &p);
                }
                let chosen = chosen.and_then(|p| match picker.ready(&p) {
                    Ok(p) => Some(p),
                    Err(e) => {
                        picker.error = Some(e);
                        None
                    }
                });
                if let Some(play) = chosen {
                    match start(&args, Ok(play.rom.clone()), Some(play.saves()), checking, &audio) {
                        Ok(s) => {
                            // it starts: chosen first next time
                            let mut settings = SettingsFile.load();
                            settings.last_play = play.project.clone().unwrap_or_default();
                            SettingsFile.save(&settings);
                            next = Some(App::Run(s));
                        }
                        Err((_, e)) => picker.error = Some(e),
                    }
                }
            }
            App::Run(session) => {
                pad.poll(checking || system_window::in_front(&title));
                let extras = session.touch_extras();
                if touch.update(session.settings(), keyboard.last_pressed().is_some(), pad.last_pressed().is_some(), pad.connected(), &extras) {
                    session.show_touch_controls();
                }
                session.set_devices(touch.devices());
                let bindings = session.settings().pad;
                if session.frame_with(dt, &keyboard, &touch.pad(&pad, bindings), &audio) == Flow::Quit {
                    break;
                }
                display.present(session.screen(), session.palette(), session.settings().scale, touch.picture());
                touch.draw();
            }
            App::Fail(prefs, error) => {
                if keyboard.pressed(Key::F11) {
                    prefs.toggle_fullscreen();
                }
                display.show_error(error);
            }
        }
        if let Some(n) = next {
            app = n;
        }

        if let Some(out) = &args.screenshot {
            render_frames += 1;
            if render_frames >= 120 {
                save_window_png(out);
                return;
            }
        }

        if macroquad::input::is_quit_requested() {
            break;
        }
        macroquad::window::next_frame().await;
    }
    if let App::Run(session) = &mut app {
        session.finish();
    }
}
