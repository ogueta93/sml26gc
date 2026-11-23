//! What the player chose: key bindings, colours, volume, screen and language, and their
//! `key=value` text form. Where the text is kept is a port (`SettingsStore`).
use super::i18n::{LANGS, LANG_EN, LANG_IDS};
use super::keys::{Key, DEFAULT_KEYS, KEYS};
use super::pad::{Input, PadButton, DEFAULT_PAD};
use super::touch::{TouchShow, TouchSize};

/// How the picture fills its room: whole pixels (sharp, with borders), as large as fits at its
/// shape, or all the room (out of shape).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Scale {
    #[default]
    Sharp,
    Fitted,
    Stretched,
}

impl Scale {
    pub const ALL: [Scale; 3] = [Scale::Sharp, Scale::Fitted, Scale::Stretched];

    pub fn id(self) -> &'static str {
        match self {
            Scale::Sharp => "sharp",
            Scale::Fitted => "fitted",
            Scale::Stretched => "stretched",
        }
    }

    pub fn from_id(id: &str) -> Option<Scale> {
        Scale::ALL.into_iter().find(|s| s.id() == id)
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    /// Up, Down, Left, Right, A, B, Start, Select, save state, load state, rewind, fast forward,
    /// menu, restart level.
    pub keys: [Key; KEYS],
    /// What plays: the keyboard or the controller.
    pub input: Input,
    /// The controller's bindings, in `keys` order.
    pub pad: [PadButton; KEYS],
    pub palette: usize,
    pub volume: u8,
    pub fullscreen: bool,
    pub scale: Scale,
    pub lang: usize,
    /// No language chosen yet: `lang` is the system's (`SettingsFile` sets it), and the file
    /// keeps `language=auto`.
    pub lang_auto: bool,
    /// The debug / cheats mode for games started from the start screen.
    pub cheats: bool,
    /// What the start screen played last: a project's name, or empty for the original.
    pub last_play: String,
    /// When the on-screen controls show, their size, and whether pressing one vibrates.
    pub touch: TouchShow,
    pub touch_size: TouchSize,
    pub vibrate: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { keys: DEFAULT_KEYS, input: Input::Keyboard, pad: DEFAULT_PAD, palette: 0, volume: 8, fullscreen: false, scale: Scale::Sharp, lang: LANG_EN, lang_auto: true, cheats: false, last_play: String::new(), touch: TouchShow::Auto, touch_size: TouchSize::Medium, vibrate: true }
    }
}

impl Settings {
    /// Read `key=value` lines; anything missing or unknown keeps its default.
    pub fn from_text(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            if let Some(i) = k.strip_prefix("key").and_then(|n| n.parse::<usize>().ok()) {
                if i < KEYS {
                    if let Some(code) = Key::from_id(v) {
                        s.keys[i] = code;
                    }
                }
            }
            if let Some(i) = k.strip_prefix("pad").and_then(|n| n.parse::<usize>().ok()) {
                if let (true, Some(b)) = (i < KEYS, PadButton::from_id(v)) {
                    s.pad[i] = b;
                }
            }
            match k {
                "input" => s.input = Input::from_id(v).unwrap_or_default(),
                "palette" => s.palette = v.parse().unwrap_or(0) % 3,
                "volume" => s.volume = v.parse::<u8>().unwrap_or(8).min(10),
                "fullscreen" => s.fullscreen = v == "1",
                // before `scale`: 1 sharp, 0 fitted
                "integer_scale" => s.scale = if v == "0" { Scale::Fitted } else { Scale::Sharp },
                "scale" => s.scale = Scale::from_id(v).unwrap_or_default(),
                "language" => {
                    s.lang_auto = v == "auto";
                    s.lang = LANG_IDS.iter().position(|&id| id == v).unwrap_or(LANG_EN);
                }
                "cheats" => s.cheats = v == "1",
                "last_play" => s.last_play = v.to_string(),
                "touch" => s.touch = TouchShow::from_id(v).unwrap_or_default(),
                "touch_size" => s.touch_size = TouchSize::from_id(v).unwrap_or_default(),
                "vibrate" => s.vibrate = v != "0",
                _ => {}
            }
        }
        s
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (i, k) in self.keys.iter().enumerate() {
            out += &format!("key{i}={}\n", k.id().unwrap_or("Unknown"));
        }
        out += &format!("input={}\n", self.input.id());
        for (i, b) in self.pad.iter().enumerate() {
            out += &format!("pad{i}={}\n", b.id().unwrap_or("Unknown"));
        }
        out += &format!(
            "palette={}\nvolume={}\nfullscreen={}\nscale={}\nlanguage={}\ncheats={}\nlast_play={}\ntouch={}\ntouch_size={}\nvibrate={}\n",
            self.palette,
            self.volume,
            self.fullscreen as u8,
            self.scale.id(),
            if self.lang_auto { "auto" } else { LANG_IDS[self.lang.min(LANGS - 1)] },
            self.cheats as u8,
            self.last_play,
            self.touch.id(),
            self.touch_size.id(),
            self.vibrate as u8
        );
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trip() {
        let s = Settings::from_text("key4=Space\nkey5=LeftShift\npalette=1\nfullscreen=1\nlanguage=es\nvolume=99\n");
        assert_eq!(s.keys[4], Key::Space);
        assert_eq!(s.keys[5], Key::LeftShift);
        assert_eq!((s.palette, s.fullscreen, s.lang, s.volume), (1, true, 1, 10));
        assert_eq!(Settings::from_text(&s.to_text()), s);
        assert_eq!(Settings::from_text(""), Settings::default());
    }

    #[test]
    fn the_scale_and_an_old_files_integer_scale() {
        assert_eq!(Settings::from_text("integer_scale=0\n").scale, Scale::Fitted);
        assert_eq!(Settings::from_text("integer_scale=1\n").scale, Scale::Sharp);
        let s = Settings::from_text("scale=stretched\n");
        assert_eq!(s.scale, Scale::Stretched);
        assert_eq!(Settings::from_text(&s.to_text()), s);
    }

    #[test]
    fn the_touch_controls_settings() {
        let s = Settings::from_text("touch=never\ntouch_size=large\nvibrate=0\n");
        assert_eq!((s.touch, s.touch_size, s.vibrate), (TouchShow::Never, TouchSize::Large, false));
        assert_eq!(Settings::from_text(&s.to_text()), s);
        let old = Settings::from_text("palette=1\n");
        assert_eq!((old.touch, old.touch_size, old.vibrate), (TouchShow::Auto, TouchSize::Medium, true), "a file from before");
    }

    #[test]
    fn the_language_is_automatic_until_chosen() {
        assert!(Settings::from_text("palette=1\n").lang_auto, "an old file without a language");
        assert!(Settings::default().to_text().contains("language=auto\n"));
        let chosen = Settings::from_text("language=ko\n");
        assert_eq!((chosen.lang, chosen.lang_auto), (10, false));
        assert!(chosen.to_text().contains("language=ko\n"));
    }

    #[test]
    fn the_controller_is_kept_and_an_old_file_plays_from_the_keyboard() {
        let mut s = Settings { input: Input::Controller, ..Settings::default() };
        s.pad[4] = PadButton::North;
        assert_eq!(Settings::from_text(&s.to_text()), s);
        let old = Settings::from_text("key4=Space\npalette=1\n");
        assert_eq!((old.input, old.pad), (Input::Keyboard, DEFAULT_PAD));
    }

    #[test]
    fn cheats_are_off_unless_saved_on() {
        assert!(!Settings::from_text("palette=1\n").cheats);
        let on = Settings { cheats: true, ..Settings::default() };
        assert!(on.to_text().contains("cheats=1\n"));
        assert_eq!(Settings::from_text(&on.to_text()), on);
    }
}
