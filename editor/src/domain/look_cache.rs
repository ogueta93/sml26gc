//! Object pictures captured from the game, kept as text so the next start has them at once:
//! a first line `NMB looks 5`, then one line per (level, object type): `level kind -` when the
//! object draws nothing, else `level kind w h dx dy sets pixels` (`sets`: the sprite tile sets
//! it uses, `level_graphics`) with one character per pixel (`.` see-through, `0`-`3` a shade);
//! and `t level kind tiles` (hex, no spaces) for the sprite tiles an object shows over its
//! animation (`art::object_tiles`). An older cache is not read, so it is captured again.
use cart_model::art::Look;
use look_capture::{parse_picture, parse_tiles, picture_line, tiles_line};

const HEAD: &str = "NMB looks 5";

/// A captured picture: the level it was captured in, the object type, the picture.
pub type Entry = (usize, u8, Option<Look>);
/// The sprite tiles an object type shows in a level over its animation.
pub type Tiles = (usize, u8, Vec<u8>);

pub fn to_text(entries: &[Entry], tiles: &[Tiles]) -> String {
    let mut out = format!("{HEAD}\n");
    for (l, k, t) in tiles {
        out += &tiles_line(*l, *k, t);
        out.push('\n');
    }
    for (l, k, look) in entries {
        out += &picture_line(*l, *k, look.as_ref());
        out.push('\n');
    }
    out
}

/// The pictures and tiles of a cache text; a text of another version gives none, a damaged
/// line is skipped.
pub fn from_text(text: &str) -> (Vec<Entry>, Vec<Tiles>) {
    let mut lines = text.lines();
    if lines.next() != Some(HEAD) {
        return (Vec::new(), Vec::new());
    }
    let (tiles, pictures): (Vec<&str>, Vec<&str>) = lines.partition(|l| l.starts_with("t "));
    (pictures.into_iter().filter_map(parse_picture).collect(), tiles.into_iter().filter_map(parse_tiles).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pictures_go_to_text_and_back() {
        let look = Look { w: 2, h: 2, px: vec![None, Some(1), Some(3), Some(0)], dx: -4, dy: 8, sets: 5 };
        let entries = vec![(3, 0x1F, Some(look)), (3, 0x20, None)];
        let tiles = vec![(3, 0x1F, vec![0xB0, 0xB1, 0x0A]), (4, 0x11, vec![])];
        let text = to_text(&entries, &tiles);
        assert_eq!(text, "NMB looks 5\nt 3 31 b0b10a\nt 4 17 \n3 31 2 2 -4 8 5 .130\n3 32 -\n");
        assert_eq!(from_text(&text), (entries, tiles));
        assert_eq!(from_text("NMB looks 3\n3 32 -\n"), (vec![], vec![]), "an older cache is captured again");
        assert_eq!(from_text("NMB looks 5\n3 31 2 2 0 0 0 .1\n3 32 -\nt 1 2 zz\n"), (vec![(3, 0x20, None)], vec![]));
    }
}
