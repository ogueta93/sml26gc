//! Checks of the game's own additions, run from the command line without a window:
//!   sml26gc --research menushots DIR      the pause menu's screens in every language (Controls
//!                                         with the keyboard and with the controller)
use crate::domain::menu::Menu;
use crate::domain::settings::Settings;
use gb_core::palette::PALETTES;
use host::load_gb;
use gb_core::script::{buttons_at, parse_script};
use host::png_out::write_screen;
use std::path::Path;

/// Save the pause menu's screens over a level frame as PNGs into `dir`.
pub fn menu_shots(rom: &Path, dir: &Path) {
    let Ok(mut gb) = load_gb(rom, 44100, None) else { return };
    let sc = parse_script("S@400-405;S@600-605;S@800-805;R@960-1000;");
    for f in 0..1010 {
        gb.set_buttons(buttons_at(&sc, f));
        gb.run_frame();
        gb.clear_audio();
    }
    let _ = host::storage::create_dir_all(dir);
    let mut st = Settings::default();
    let icons = cart_model::cheats::Item::ALL.map(|i| cart_model::cheats::item_picture(&mut gb, i));
    for (lang, tag) in crate::domain::i18n::LANG_IDS.iter().enumerate() {
        st.lang = lang;
        for name in ["main", "controls", "wait", "display", "sound", "language", "confirm", "cheats", "summon", "levels"] {
            let mut screen = *gb.screen();
            Menu::preview(name, Some(icons)).draw(&mut screen, &st);
            if let Err(e) = write_screen(&dir.join(format!("{tag}-{name}.png")), &screen, &PALETTES[0]) {
                eprintln!("{e}");
            }
        }
        // Controls with the controller chosen
        let pad = Settings { input: crate::domain::pad::Input::Controller, ..st.clone() };
        let mut screen = *gb.screen();
        Menu::preview("controls", Some(icons)).draw(&mut screen, &pad);
        if let Err(e) = write_screen(&dir.join(format!("{tag}-controls-pad.png")), &screen, &PALETTES[0]) {
            eprintln!("{e}");
        }
    }
}

pub fn research_main(rom: &Path, v: &[String]) {
    let g = |i: usize| v.get(i).cloned().unwrap_or_default();
    match g(0).as_str() {
        "menushots" => menu_shots(rom, Path::new(&g(1))),
        _ => eprintln!("sml26gc --research menushots DIR   (other research commands: the research tool, tools/research)"),
    }
}
