//! The menu's texts in every language (`i18n::Lang` order): the English in `en.rs`, which keys
//! the catalogue `game/i18n/menu.txt` that holds every language's (drawn with the pixel font,
//! `overlay`; the catalogue's header says what it can draw).
mod en;

use i18n::{Dictionary, Lang};
use std::sync::OnceLock;

/// The menu's catalogue, and the keys' names.
const MENU_TEXTS: &str = include_str!("../../../i18n/menu.txt");
const KEY_TEXTS: &str = include_str!("../../../i18n/keys.txt");

pub const LANG_EN: usize = 0;
#[cfg(test)]
pub const LANG_ES: usize = 1;
#[allow(dead_code)] // named for completeness: the game picks Japanese by number only
pub const LANG_JA: usize = 2;
/// How many languages the menu speaks.
pub const LANGS: usize = i18n::LANGS;
/// Each language's name written in itself, as the Language screen lists them.
pub const LANG_NAMES: [&str; LANGS] = ["ENGLISH", "ESPAÑOL", "ニホンゴ", "PORTUGUES", "FRANCAIS", "DEUTSCH", "ITALIANO", "INDONESIA", "РУССКИЙ", "中文", "한국어"];
/// The ids kept in `settings.cfg`, as `i18n::Lang::id` gives them.
pub const LANG_IDS: [&str; LANGS] = ["en", "es", "ja", "pt", "fr", "de", "it", "id", "ru", "zh", "ko"];

pub struct Texts {
    pub pause: &'static str,
    /// Continue, Controls, Display, Sound, Language, Original pause, Quit, Back to map, File select, Debug / cheats, Restart level.
    pub main: [&'static str; 11],
    pub sure: &'static str,
    /// The start-up tag: the menu, then what the cheats mode's keys do.
    pub hint_menu: &'static str,
    pub hint_save: &'static str,
    pub hint_load: &'static str,
    pub hint_rewind: &'static str,
    pub footer_main: &'static str,
    pub controls: &'static str,
    /// The bindings, in `keys` order.
    pub buttons: [&'static str; 14],
    pub restore: &'static str,
    /// The Controls screen's touch rows: when the on-screen controls show (in `TouchShow::ALL`'s
    /// order), their size (in `TouchSize::ALL`'s), vibration.
    pub touch: &'static str,
    pub touch_modes: [&'static str; 3],
    pub touch_size: &'static str,
    pub touch_sizes: [&'static str; 3],
    pub vibration: &'static str,
    pub footer_controls: &'static str,
    pub press_key: &'static str,
    pub press_button: &'static str,
    /// The Controls screen's first row: what plays, keyboard or controller.
    pub input: &'static str,
    pub keyboard: &'static str,
    pub controller: &'static str,
    pub now: &'static str,
    pub cancel: &'static str,
    pub display: &'static str,
    pub colours: &'static str,
    pub palettes: [&'static str; 3],
    pub fullscreen: &'static str,
    pub yes: &'static str,
    pub no: &'static str,
    pub scale: &'static str,
    pub sharp: &'static str,
    pub fitted: &'static str,
    pub stretched: &'static str,
    pub footer_change: &'static str,
    pub sound: &'static str,
    pub volume: &'static str,
    pub language: &'static str,
    pub footer_language: &'static str,
    /// In `Cheat::ALL`'s order.
    pub cheats: [&'static str; 12],
    /// "Level" before a level's number in the level list.
    pub level: &'static str,
    pub saved: &'static str,
    pub loaded: &'static str,
    pub nothing_saved: &'static str,
    /// In `Item::ALL`'s order.
    pub items: [&'static str; 5],
    pub on: &'static str,
    pub off: &'static str,
    pub done: &'static str,
    pub only_level: &'static str,
    pub no_secret: &'static str,
    pub footer_summon: &'static str,
}

fn catalogue() -> &'static Dictionary {
    static CATALOGUE: OnceLock<Dictionary> = OnceLock::new();
    CATALOGUE.get_or_init(|| Dictionary::parse(MENU_TEXTS).expect("the menu's catalogue"))
}

fn key_catalogue() -> &'static Dictionary {
    static CATALOGUE: OnceLock<Dictionary> = OnceLock::new();
    CATALOGUE.get_or_init(|| Dictionary::parse(KEY_TEXTS).expect("the keys' catalogue"))
}

/// The name of the key called `en` in English, in language `lang`.
pub fn key_label(lang: usize, en: &'static str) -> &'static str {
    Lang::ALL.get(lang).map_or(en, |&l| key_catalogue().get(l, en))
}

/// Every language's texts, made once from the catalogue (a text it lacks shows in English).
fn all() -> &'static [Texts] {
    static ALL: OnceLock<Vec<Texts>> = OnceLock::new();
    ALL.get_or_init(|| Lang::ALL.iter().map(|&l| in_language(l)).collect())
}

/// The English texts, each looked up in the catalogue for `lang`.
fn in_language(lang: Lang) -> Texts {
    let e = &en::TEXTS;
    let tr = |s: &'static str| catalogue().get(lang, s);
    Texts {
        pause: tr(e.pause),
        main: e.main.map(tr),
        sure: tr(e.sure),
        hint_menu: tr(e.hint_menu),
        hint_save: tr(e.hint_save),
        hint_load: tr(e.hint_load),
        hint_rewind: tr(e.hint_rewind),
        footer_main: tr(e.footer_main),
        controls: tr(e.controls),
        buttons: e.buttons.map(tr),
        restore: tr(e.restore),
        touch: tr(e.touch),
        touch_modes: e.touch_modes.map(tr),
        touch_size: tr(e.touch_size),
        touch_sizes: e.touch_sizes.map(tr),
        vibration: tr(e.vibration),
        footer_controls: tr(e.footer_controls),
        press_key: tr(e.press_key),
        press_button: tr(e.press_button),
        input: tr(e.input),
        keyboard: tr(e.keyboard),
        controller: tr(e.controller),
        now: tr(e.now),
        cancel: tr(e.cancel),
        display: tr(e.display),
        colours: tr(e.colours),
        palettes: e.palettes.map(tr),
        fullscreen: tr(e.fullscreen),
        yes: tr(e.yes),
        no: tr(e.no),
        scale: tr(e.scale),
        sharp: tr(e.sharp),
        fitted: tr(e.fitted),
        stretched: tr(e.stretched),
        footer_change: tr(e.footer_change),
        sound: tr(e.sound),
        volume: tr(e.volume),
        language: tr(e.language),
        footer_language: tr(e.footer_language),
        cheats: e.cheats.map(tr),
        level: tr(e.level),
        saved: tr(e.saved),
        loaded: tr(e.loaded),
        nothing_saved: tr(e.nothing_saved),
        items: e.items.map(tr),
        on: tr(e.on),
        off: tr(e.off),
        done: tr(e.done),
        only_level: tr(e.only_level),
        no_secret: tr(e.no_secret),
        footer_summon: tr(e.footer_summon),
    }
}

/// Language `lang`'s texts (English for an unknown number).
pub fn texts(lang: usize) -> &'static Texts {
    all().get(lang).unwrap_or(&all()[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::overlay::text_width;

    /// Every text of `t` by field name and index.
    fn fields(t: &Texts) -> Vec<(&'static str, usize, &'static str)> {
        let one = |n: &'static str, s: &'static str| vec![(n, 0, s)];
        let many = |n: &'static str, v: &[&'static str]| v.iter().enumerate().map(|(i, &s)| (n, i, s)).collect::<Vec<_>>();
        [
            one("pause", t.pause), many("main", &t.main), one("sure", t.sure), one("hint_menu", t.hint_menu), one("hint_save", t.hint_save),
            one("hint_load", t.hint_load), one("hint_rewind", t.hint_rewind), one("footer_main", t.footer_main), one("controls", t.controls),
            many("buttons", &t.buttons), one("restore", t.restore), one("touch", t.touch), many("touch_modes", &t.touch_modes),
            one("touch_size", t.touch_size), many("touch_sizes", &t.touch_sizes), one("vibration", t.vibration), one("footer_controls", t.footer_controls),
            one("press_key", t.press_key), one("press_button", t.press_button), one("input", t.input), one("keyboard", t.keyboard),
            one("controller", t.controller), one("now", t.now), one("cancel", t.cancel), one("display", t.display), one("colours", t.colours),
            many("palettes", &t.palettes), one("fullscreen", t.fullscreen), one("yes", t.yes), one("no", t.no), one("scale", t.scale),
            one("sharp", t.sharp), one("fitted", t.fitted), one("stretched", t.stretched), one("footer_change", t.footer_change), one("sound", t.sound), one("volume", t.volume),
            one("language", t.language), one("footer_language", t.footer_language), many("cheats", &t.cheats), one("level", t.level),
            one("saved", t.saved), one("loaded", t.loaded), one("nothing_saved", t.nothing_saved), many("items", &t.items), one("on", t.on),
            one("off", t.off), one("done", t.done), one("only_level", t.only_level), one("no_secret", t.no_secret), one("footer_summon", t.footer_summon),
        ]
        .concat()
    }

    /// The room a field's text has in its box (pixels), from the menu's layout: a box's width
    /// less its margins, or a row's less the value beside it.
    fn room(field: &str, i: usize) -> i32 {
        // the Controls rows: 132 pixels for a binding's name and its value, the longest
        // default value of each binding taken off (arrows, Backspace, Ctrl)
        const BUTTONS: [i32; 14] = [62, 62, 62, 62, 84, 84, 84, 74, 84, 84, 84, 80, 84, 84];
        match field {
            "buttons" => BUTTONS[i],
            "main" | "language" | "footer_language" => 108,
            "pause" | "sure" | "footer_main" | "press_key" | "press_button" | "cancel" | "sound" | "items" | "footer_summon" => 124,
            "restore" => 132,
            // a touch row's name beside its widest value
            "touch" | "touch_size" | "vibration" => 84,
            "touch_modes" | "touch_sizes" => 44,
            "controls" | "footer_controls" => 148,
            "display" | "footer_change" => 140,
            "saved" | "loaded" | "nothing_saved" | "only_level" | "no_secret" => 150,
            "cheats" => 96,
            "input" | "keyboard" | "fullscreen" | "volume" => 60,
            "controller" => 66,
            "colours" | "scale" | "level" => 54,
            "now" | "palettes" | "sharp" | "fitted" | "stretched" => 48,
            "hint_menu" | "hint_save" | "hint_load" | "hint_rewind" => 90,
            _ => 30,
        }
    }

    /// Every character of every language's texts and names has a glyph in the pixel font.
    #[test]
    fn the_pixel_font_draws_every_character() {
        let mut missing = std::collections::BTreeSet::new();
        for t in all() {
            for (_, _, text) in fields(t) {
                missing.extend(text.chars().filter(|&c| !crate::domain::overlay::has_glyph(c)));
            }
        }
        missing.extend(LANG_NAMES.iter().flat_map(|n| n.chars()).filter(|&c| !crate::domain::overlay::has_glyph(c)));
        assert!(missing.is_empty(), "no glyph for {missing:?}");
    }

    /// Every language has every text, and each fits its box, or is no wider than the widest of
    /// English, Spanish and Japanese, which the menu was laid out for.
    #[test]
    fn every_text_is_there_and_fits_its_box() {
        let first: Vec<Vec<(&str, usize, &str)>> = all()[..3].iter().map(fields).collect();
        let mut over = Vec::new();
        for (l, t) in all().iter().enumerate() {
            for (k, (field, i, text)) in fields(t).into_iter().enumerate() {
                assert!(!text.trim().is_empty(), "{} {field}[{i}] is empty", LANG_IDS[l]);
                let allowed = first.iter().map(|f| text_width(f[k].2)).max().unwrap_or(0).max(room(field, i));
                if text_width(text) > allowed {
                    over.push(format!("{} {field}[{i}] {text:?}: {} > {allowed}", LANG_IDS[l], text_width(text)));
                }
            }
        }
        assert!(over.is_empty(), "texts wider than their box:
{}", over.join("
"));
    }

    /// Every catalogue entry has every language and is an English text of the menu.
    #[test]
    fn the_catalogue_is_complete_and_every_entry_used() {
        let used: Vec<&str> = fields(&en::TEXTS).into_iter().map(|f| f.2).collect();
        let mut wrong: Vec<String> = catalogue().entries(Lang::En).filter(|(en, _)| !catalogue().missing(en).is_empty()).map(|(en, _)| format!("{en:?}: incomplete")).collect();
        wrong.extend(catalogue().entries(Lang::En).filter(|(en, _)| !used.contains(en)).map(|(en, _)| format!("{en:?}: not used")));
        wrong.extend(used.iter().filter(|en| catalogue().entries(Lang::En).all(|(e, _)| e != **en)).map(|en| format!("{en:?}: not in the catalogue")));
        assert!(wrong.is_empty(), "{wrong:?}");
    }

    #[test]
    fn every_key_name_has_every_language_and_is_a_key() {
        let keys: Vec<&str> = crate::domain::keys::BINDABLE.iter().map(|e| e.2).collect();
        let wrong: Vec<&str> = key_catalogue().entries(Lang::En).map(|(en, _)| en).filter(|en| !keys.contains(en) || !key_catalogue().missing(en).is_empty()).collect();
        assert!(wrong.is_empty(), "{wrong:?}");
        for t in key_catalogue().entries(Lang::En).flat_map(|(en, _)| Lang::ALL.map(|l| key_catalogue().get(l, en))) {
            assert!(t.chars().all(crate::domain::overlay::has_glyph), "{t:?}");
        }
    }

    #[test]
    fn the_ids_are_the_editor_s_in_the_same_order() {
        assert!(i18n::Lang::ALL.iter().enumerate().all(|(i, l)| LANG_IDS[i] == l.id()));
    }
}
