//! Level editor: works on projects over the person's base cartridge (checked first). A
//! project shows each level with the game's own blocks and graphics, lets them paint blocks,
//! place objects and edit the world map and zones, and saves into the project's own edited
//! cartridge. The base cartridge is never written.
//!
//! Layers: `domain` (document, undo history, tools) ← `application` (saving, captured
//! graphics) ← `ports` ← `adapters` (files, the game) and `ui` (macroquad panels). This
//! file is the composition root.
#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
mod adapters;
mod application;
mod domain;
mod ports;
mod texts;
mod ui;

use adapters::fs_repository;
use adapters::menu_bar::WindowMenu;
use adapters::platform::{self, Dialogs, Files, Launcher, Sound};
use cart_model::original_points::OnMap;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;
use i18n::Lang;
use ports::{FileDialogs, MenuBar, PrefsStore};
use std::path::PathBuf;
use ui::drawn_menu_bar::DrawnMenuBar;
use ui::inset;
use ui::{language_menu, BaseScreen, Editor, Leave, Mode, Ports, ProjectsChoice, ProjectsScreen, TAB_POINTS};

const TITLE: &str = "SML2 6GC Editor";

fn conf() -> Conf {
    platform::window_conf(TITLE)
}

#[macroquad::main(conf)]
async fn main() {
    host::console::attach();
    let args = platform::arguments();
    application::look_loader::set_capture(platform::CAPTURE);
    platform::claim_keys(&ui::shortcuts::command_keys());
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1));
    let shot = value("--screenshot").map(PathBuf::from);
    let level = value("--level").and_then(|v| v.parse::<usize>().ok());
    macroquad::input::prevent_quit();
    let mut lang = fs_repository::language();
    ui::set_text_language(lang);
    let mut dialogs = Dialogs::new(lang, TITLE);
    clear_background(ui::col(3));
    next_frame().await;
    let mut menu: Box<dyn MenuBar> = match WindowMenu::attach(TITLE) {
        Some(m) => Box::new(m),
        None => Box::<DrawnMenuBar>::default(),
    };
    language_menu::add(&mut *menu, lang, fs_repository::PrefsFile.load().language);

    // the base cartridge, asked for until a valid one is chosen
    let base = match host::base_cartridge() {
        Ok(p) => p,
        Err(problem) => {
            let mut screen = BaseScreen::new(lang, problem);
            let mut frames = 0;
            loop {
                inset::begin(&mut *menu);
                if choose_language(&mut *menu, &mut lang, &mut dialogs) {
                    screen.set_lang(lang);
                }
                let chosen = screen.frame(&dialogs);
                inset::end(&mut *menu);
                if let Some(p) = chosen {
                    break p;
                }
                if macroquad::input::is_quit_requested() || shoot(&shot, &mut frames) {
                    return;
                }
                next_frame().await;
            }
        }
    };
    // --project NAME: open that project straight away (the game's "edit in the editor")
    let mut straight = value("--project").cloned().filter(|n| host::storage::is_dir(host::project_dir(n)));
    // --new-project: the project screen typing a new project's name
    let mut new_project = args.iter().any(|a| a == "--new-project");
    let mut last = Some(fs_repository::PrefsFile.load().last_project).filter(|n| !n.is_empty());
    let mut first = true;
    loop {
        let name = match straight.take() {
            Some(n) => n,
            None => {
                let mut screen = ProjectsScreen::new(lang, base.clone(), last.as_deref());
                if std::mem::take(&mut new_project) {
                    screen.start_new();
                }
                let mut frames = 0;
                loop {
                    inset::begin(&mut *menu);
                    if choose_language(&mut *menu, &mut lang, &mut dialogs) {
                        screen.set_lang(lang);
                    }
                    let chosen = screen.frame(&dialogs);
                    inset::end(&mut *menu);
                    match chosen {
                        Some(ProjectsChoice::Open(n)) => break n,
                        Some(ProjectsChoice::Quit) => return,
                        None => {}
                    }
                    if macroquad::input::is_quit_requested() || shoot(&shot, &mut frames) {
                        return;
                    }
                    next_frame().await;
                }
            }
        };
        let Ok(opened) = fs_repository::open_project(&base, &name) else { continue };
        menu.clear();
        let mut prefs = fs_repository::PrefsFile.load();
        prefs.last_project = name.clone();
        fs_repository::PrefsFile.save(&prefs);
        last = Some(name.clone());
        let looks = fs_repository::LookFile::for_cartridge(&opened.doc.original);
        let ports = Ports {
            store: Box::new(opened.store),
            launcher: Box::new(Launcher::default()),
            prefs: Box::new(fs_repository::PrefsFile),
            dialogs: Box::new(Dialogs::new(lang, TITLE)),
            text_files: Box::new(Files),
            menu,
            sound: Box::new(Sound::new()),
            looks: Box::new(looks),
        };
        let mut ed = Editor::new(opened.doc, ports, name, lang);
        // --level N: that level; else where the project was last saved
        match level.filter(|_| first) {
            Some(l) => ed.select_level(l.min(ed.level_count() - 1)),
            None => {
                ed.select_level(0);
                ed.open_at_saved_place();
            }
        }
        ed.drop_other_catalogue();
        if first && !command_line(&mut ed, &args) {
            return;
        }
        if first && shot.is_some() {
            ed.capture_level_looks();
        }
        first = false;
        let mut frames = 0;
        loop {
            ed.frame();
            platform::set_unsaved(ed.unsaved());
            if shoot(&shot, &mut frames) {
                return;
            }
            if macroquad::input::is_quit_requested() {
                ed.ask_leave(Leave::Quit);
            }
            match ed.leaving() {
                Some(Leave::Quit) => return,
                Some(Leave::Projects) => break,
                None => {}
            }
            next_frame().await;
        }
        platform::set_unsaved(false);
        menu = ed.take_menu();
        lang = fs_repository::language();
        dialogs.set_lang(lang);
        language_menu::add(&mut *menu, lang, fs_repository::PrefsFile.load().language);
        next_frame().await;
    }
}

/// A language chosen in the Language menu outside the editor (the project and cartridge
/// screens): kept in `editor.cfg` and applied to the texts, the dialogs and the menu. True when
/// it changed.
fn choose_language(menu: &mut dyn MenuBar, lang: &mut Lang, dialogs: &mut Dialogs) -> bool {
    let Some(choice) = menu.chosen().into_iter().find_map(language_menu::chosen) else { return false };
    let mut prefs = fs_repository::PrefsFile.load();
    prefs.language = choice;
    fs_repository::PrefsFile.save(&prefs);
    *lang = language_menu::resolve(choice);
    ui::set_text_language(*lang);
    dialogs.set_lang(*lang);
    menu.clear();
    language_menu::add(menu, *lang, choice);
    true
}

/// With `--screenshot`: after 20 frames, the window saved as a picture; true then (stop).
fn shoot(shot: &Option<PathBuf>, frames: &mut u32) -> bool {
    let Some(p) = shot else { return false };
    *frames += 1;
    if *frames < 20 {
        return false;
    }
    platform::save_screenshot(p);
    true
}

/// The options that show a part of the editor at start (for the check pictures). False when
/// the editor should stop (`--capture-all` has done its job).
fn command_line(ed: &mut Editor, args: &[String]) -> bool {
    let flag = |f: &str| args.iter().any(|a| a == f);
    let value = |f: &str| args.iter().position(|a| a == f).and_then(|i| args.get(i + 1));
    if flag("--objects") {
        ed.set_mode(Mode::Objects);
    }
    if flag("--world") {
        ed.enter_world(true);
        if flag("--points") {
            ed.world_tab = TAB_POINTS;
        }
        // --point HEX: choose that map point (its map is shown)
        if let Some(p) = value("--point").and_then(|v| usize::from_str_radix(v, 16).ok()).filter(|&p| p < cart_model::world::POINT_COUNT) {
            ed.choose_point(p);
        }
        // --map N: zone map N-1 (0 = the world map)
        if let Some(z) = value("--map").and_then(|v| v.parse::<usize>().ok()).filter(|&z| (1..=ZONES.len()).contains(&z)) {
            ed.show_map(OnMap::Zone(z - 1));
        }
        // --new-zone: a new zone made from the map shown (not saved)
        if flag("--new-zone") {
            ed.new_zone();
        }
        // --maps-open: the map list unfolded; --blank-zone: the blank zone chooser
        if flag("--maps-open") {
            ed.open_map_list();
        }
        if flag("--go-to-start") {
            ed.go_to_start();
        }
        if flag("--blank-zone") {
            ed.show_blank_zone_chooser();
        }
        // --choose: choose on the maps where the chosen point's door or exit leads
        if flag("--choose") {
            ed.choose_target();
        }
    }
    // --test-game: the test game's settings open
    if flag("--test-game") {
        ed.show_test_game();
    }
    // --copy-level: a copy of the level shown (not saved)
    if flag("--copy-level") {
        ed.copy_level();
    }
    // --inspector: the Level tab shown (with --levels-open, the level grid unfolded)
    if flag("--inspector") {
        ed.show_inspector(flag("--levels-open"));
    }
    // --place-start: "Place start" chosen
    if flag("--place-start") {
        ed.place_start();
    }
    // --place-finish: "Place finish" chosen
    if flag("--place-finish") {
        ed.place_finish();
    }
    // --secret-leads: choose where the level's secret exit leads
    if flag("--secret-leads") {
        ed.choose_secret();
    }
    // --kind HEX: that object type chosen in the palette
    if let Some(k) = value("--kind").and_then(|v| u8::from_str_radix(v, 16).ok()) {
        ed.pick_kind(k);
    }
    // --all-objects: the object palette shows every type
    if flag("--all-objects") {
        ed.show_all_objects();
    }
    // --capture-all: capture every object picture and tile, keep them and stop
    if flag("--capture-all") {
        ed.capture_all();
        return false;
    }
    // --catalogue: the object catalogue open (--power-ups: only power-ups)
    if flag("--catalogue") {
        ed.show_catalogue(flag("--power-ups"));
    }
    // --catalogue-pick L:K: that object chosen in the open catalogue (level L's type K, hex)
    if let Some((l, k)) = value("--catalogue-pick").and_then(|v| v.split_once(':')).and_then(|(l, k)| Some((l.parse::<usize>().ok()?, u8::from_str_radix(k, 16).ok()?))) {
        ed.choose_in_catalogue(l, k);
    }
    // --blank N: the New blank level chooser on game level N
    if let Some(n) = value("--blank").and_then(|v| v.parse::<usize>().ok()) {
        ed.show_blank_chooser(n);
    }
    true
}
