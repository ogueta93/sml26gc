//! The game's arguments for a test game, the same from the desktop's process and the browser's
//! tab: the cartridge, then the cheats mode, the editor's language, Mario's lives, coins and
//! power, and where it begins.
use crate::ports::TestGame;
use cart_model::start_point::Start;

/// Which cartridge the game plays: a file, or a project's own (the browser names it so).
#[allow(dead_code)] // each platform names it one way
pub enum Cartridge<'a> {
    File(&'a std::path::Path),
    Project(&'a str),
}

pub fn test_game_args(cartridge: Cartridge, level: Option<usize>, start: Option<(u16, u16)>, point: Option<Start>, game: TestGame) -> Vec<String> {
    let mut a = match cartridge {
        Cartridge::File(p) => vec![p.to_string_lossy().into_owned()],
        Cartridge::Project(name) => vec!["--project".into(), name.into()],
    };
    a.extend(["--cheats".into(), "--lang".into(), game.lang.id().into(), "--test".into()]);
    a.extend(["--lives".into(), game.lives.to_string(), "--coins".into(), game.coins.to_string(), "--power".into(), game.power.to_string()]);
    if let Some(l) = level {
        a.extend(["--level".into(), l.to_string()]);
    }
    if let Some((x, feet)) = start {
        a.extend(["--start".into(), format!("{x},{feet}")]);
    }
    if let Some(p) = point {
        a.extend([
            "--point".into(),
            match p.new_zone {
                Some(k) => format!("{:02X}@{k}", p.point),
                None => format!("{:02X}", p.point),
            },
        ]);
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_test_game_s_arguments() {
        let game = TestGame { lives: 5, coins: 30, power: 1, lang: i18n::Lang::Es };
        let a = test_game_args(Cartridge::Project("Sky"), Some(3), Some((1200, 496)), None, game);
        assert_eq!(a, ["--project", "Sky", "--cheats", "--lang", "es", "--test", "--lives", "5", "--coins", "30", "--power", "1", "--level", "3", "--start", "1200,496"]);
        let on_map = test_game_args(Cartridge::File(std::path::Path::new("x.gb")), None, None, Some(Start { point: 0x1A, new_zone: Some(2) }), game);
        assert_eq!(on_map[0], "x.gb");
        assert_eq!(on_map[on_map.len() - 2..], ["--point", "1A@2"]);
    }
}
