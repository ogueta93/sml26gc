//! Mistakes in a level that only show when it is played, found while it is edited. The
//! limits are the game's (research `crowdlist`, `crowdprobe`): its own levels never put more
//! than 6 objects within one screen width, and a level's records hold 83 objects.
use super::{EditorDocument, LevelContent};
use cart_model::level_goals::{self, Goal};
use cart_model::levels::COLS;
use cart_model::{checkpoint, objects, world};

/// Objects the game's levels put within one screen width at most.
pub const SCREEN_MOST: usize = 6;
pub const SCREEN_W: u16 = 160;
/// Objects a level's records hold ($AB06-$AD00, 6 bytes each, one end record).
pub const OBJECTS_MOST: usize = 83;

/// A warning, with the level pixel it is about when it is about one place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Warning {
    NoFinish,
    FinishWithoutBell((u16, u16)),
    BellWithoutFinish((u16, u16)),
    CrowdedScreen((u16, u16), usize),
    TooManyObjects(usize),
    StartBlocked((u16, u16)),
    RestartBlocked((u16, u16)),
    /// A map point (its number) opens a level the copy does not have.
    MissingLevel(u8, u8),
    /// Objects on the screen the level begins on, which the game never brings to life: the
    /// first one's place, how many.
    OnFirstScreen((u16, u16), usize),
    /// A block content over a block that does not release it: its place.
    IdleBlockContent((u16, u16)),
}

impl Warning {
    pub fn place(&self) -> Option<(u16, u16)> {
        match *self {
            Warning::FinishWithoutBell(p) | Warning::BellWithoutFinish(p) | Warning::CrowdedScreen(p, _) | Warning::StartBlocked(p) | Warning::RestartBlocked(p) | Warning::OnFirstScreen(p, _) | Warning::IdleBlockContent(p) => Some(p),
            Warning::NoFinish | Warning::TooManyObjects(_) | Warning::MissingLevel(..) => None,
        }
    }
}

fn cell_px((c, r): (usize, usize)) -> (u16, u16) {
    ((c * 16) as u16, (r * 16) as u16)
}

/// The warnings of a level's content; `ends_with_boss`: it ends with its boss (no finish);
/// `bell_expected`: the game's level it is played as has a bell over its finish (levels 25-31
/// have none).
pub fn check(c: &LevelContent, ends_with_boss: bool, bell_expected: bool) -> Vec<Warning> {
    let mut w = Vec::new();
    let finishes = level_goals::cells(&c.map, Goal::Finish);
    if finishes.is_empty() && !ends_with_boss {
        w.push(Warning::NoFinish);
    }
    let mut bells_used = Vec::new();
    if let Some(&top) = finishes.first() {
        match level_goals::bell(&c.objs, top) {
            Some(b) => bells_used.push(b),
            None if bell_expected => w.push(Warning::FinishWithoutBell(cell_px(top))),
            None => {}
        }
    }
    for (i, o) in c.objs.iter().enumerate() {
        if o.kind == level_goals::BELL && !bells_used.contains(&i) {
            w.push(Warning::BellWithoutFinish((o.x, o.y)));
        }
    }
    let mut xs: Vec<(u16, u16)> = c.objs.iter().map(|o| (o.x, o.y)).collect();
    xs.sort();
    let mut from = 0;
    while from < xs.len() {
        let n = xs[from..].iter().take_while(|p| p.0 < xs[from].0 + SCREEN_W).count();
        if n > SCREEN_MOST {
            w.push(Warning::CrowdedScreen(xs[from], n));
            from += n;
        } else {
            from += 1;
        }
    }
    if c.objs.len() > OBJECTS_MOST {
        w.push(Warning::TooManyObjects(c.objs.len()));
    }
    if c.start.blocked(&c.map) {
        w.push(Warning::StartBlocked(c.start.sprite_at()));
    }
    let first: Vec<(u16, u16)> = c.objs.iter().filter(|o| c.start.passed_at_start(o.x)).map(|o| (o.x, o.y)).collect();
    if let Some(&p) = first.first() {
        w.push(Warning::OnFirstScreen(p, first.len()));
    }
    for o in c.objs.iter().filter(|o| objects::BLOCK_CONTENTS.contains(&o.kind)) {
        let (col, row) = objects::block_of(o);
        let block = c.map.get(row * COLS + col).map_or(0xFF, |b| b & 0x7F);
        if !objects::RELEASED_BY.contains(&block) {
            w.push(Warning::IdleBlockContent((o.x, o.y)));
        }
    }
    for i in checkpoint::bells(&c.objs) {
        let r = checkpoint::restart_for(&c.map, &c.objs[i]);
        if r.blocked(&c.map) {
            w.push(Warning::RestartBlocked(r.sprite_at()));
        }
    }
    w
}

impl EditorDocument {
    /// The warnings of level `l`, and the map points that open levels the copy lacks. The
    /// game's own block contents that never come out (level 05's heart inside a hill, those
    /// over block $06 in levels 10 and 31) are left as the game has them, unwarned.
    pub fn warnings(&self, l: usize) -> Vec<Warning> {
        let base = self.base_of(l);
        let own = objects::read(&self.original, base).0;
        let bell_expected = own.iter().any(|o| o.kind == level_goals::BELL);
        let mut w = check(&self.levels[l].content, self.ends_with_boss(l), bell_expected);
        w.retain(|w| !matches!(*w, Warning::IdleBlockContent((x, y)) if own.iter().any(|o| (o.x, o.y) == (x, y) && objects::BLOCK_CONTENTS.contains(&o.kind))));
        let count = self.levels.len();
        for (p, &v) in self.points.iter().enumerate() {
            if v != world::NO_LEVEL && v != 0xFF && v as usize >= count {
                w.push(Warning::MissingLevel(p as u8, v));
            }
        }
        w
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cart_model::level_start::Start;
    use cart_model::levels::ROWS;
    use cart_model::objects::Obj;

    fn level() -> LevelContent {
        let mut map = vec![0x60u8; COLS * ROWS];
        for c in 0..COLS {
            map[30 * COLS + c] = 0x10;
        }
        map[29 * COLS + 200] = 0x4A;
        let bell = Obj { x: 200 * 16 - 48, y: 29 * 16 - 120, kind: level_goals::BELL, flag: false };
        LevelContent { map, objs: vec![bell], stand_in: None, name: String::new(), start: Start::standing(64, 30 * 16), settings: Default::default(), tables: Default::default(), enemies: None, under: Default::default(), secret: None, restore: None }
    }

    #[test]
    fn the_game_levels_have_no_warnings_but_their_objects_missing_at_the_start() {
        let Some(rom) = host::test_cartridge() else {
            eprintln!("no cartridge found: test skipped");
            return;
        };
        let rom = std::fs::read(rom).unwrap();
        // the game's own levels with objects far above or below the first screen that do not
        // appear at the start either (research `firstscreen`, `startobjects`): level, how many
        let missing = [(4, 1), (15, 3), (17, 2), (24, 2)];
        for l in 0..cart_model::levels::LEVELS {
            let c = super::super::level_from(&rom, l);
            let boss = level_goals::BOSS_LEVELS.contains(&l) && level_goals::cells(&c.map, Goal::Finish).is_empty();
            let bell = c.objs.iter().any(|o| o.kind == level_goals::BELL);
            // the game's own idle block contents are not warned about (`warnings`)
            let mut found: Vec<Warning> = check(&c, boss, bell);
            found.retain(|w| !matches!(w, Warning::IdleBlockContent(_)));
            match missing.iter().find(|m| m.0 == l) {
                Some(&(_, n)) => assert!(matches!(found[..], [Warning::OnFirstScreen(_, k)] if k == n), "level {l}: {found:?}"),
                None => assert_eq!(found, vec![], "level {l}"),
            }
        }
    }

    #[test]
    fn the_game_idle_block_contents_are_not_warned_but_a_placed_one_is() {
        let Some(rom) = host::test_cartridge() else { return };
        let mut doc = EditorDocument::open(std::fs::read(rom).unwrap(), None);
        assert!(!doc.warnings(5).iter().any(|w| matches!(w, Warning::IdleBlockContent(_))), "level 05's heart inside a hill");
        // a hidden mushroom put next to the game's heart, inside the same hill
        let mushroom = Obj { x: 648 + 16, y: 616, kind: 0x11, flag: false };
        let (col, row) = objects::block_of(&mushroom);
        assert!(!objects::RELEASED_BY.contains(&(doc.levels[5].content.map[row * COLS + col] & 0x7F)));
        doc.levels[5].content.objs.push(mushroom);
        assert!(doc.warnings(5).contains(&Warning::IdleBlockContent((mushroom.x, mushroom.y))));
    }

    #[test]
    fn a_good_level_has_no_warnings() {
        assert_eq!(check(&level(), false, true), vec![]);
    }

    #[test]
    fn objects_on_the_first_screen_are_found() {
        let mut c = level();
        let near = Obj { x: c.start.mario_x + 48, y: c.start.mario_y, kind: 9, flag: false };
        c.objs.push(near);
        c.objs.push(Obj { x: c.start.mario_x + 48, y: c.start.mario_y + 8, kind: 9, flag: false });
        c.objs.push(Obj { x: c.start.mario_x + 120, y: 0, kind: 9, flag: false });
        c.objs.push(Obj { x: c.start.mario_x + 240, y: c.start.mario_y, kind: 9, flag: false });
        assert_eq!(check(&c, false, true), vec![Warning::OnFirstScreen((near.x, near.y), 3)], "whatever their height; further on they come in as usual");
    }

    #[test]
    fn a_block_content_only_counts_over_a_block_that_releases_it() {
        let mut c = level();
        // a hidden mushroom half a block above row 30 (the ground, block $10)
        let mushroom = Obj { x: 100 * 16, y: 30 * 16 - 8, kind: 0x11, flag: false };
        c.objs.push(mushroom);
        assert_eq!(check(&c, false, true), vec![Warning::IdleBlockContent((mushroom.x, mushroom.y))]);
        c.map[30 * COLS + 100] = 0x01;
        assert_eq!(check(&c, false, true), vec![], "over a ? block");
    }

    #[test]
    fn each_mistake_is_found() {
        let mut c = level();
        c.map[29 * COLS + 200] = 0x60;
        assert_eq!(check(&c, false, true), vec![Warning::NoFinish, Warning::BellWithoutFinish((200 * 16 - 48, 29 * 16 - 120))]);
        assert_eq!(check(&c, true, true), vec![Warning::BellWithoutFinish((200 * 16 - 48, 29 * 16 - 120))]);
        let mut c = level();
        c.objs.clear();
        assert_eq!(check(&c, false, true), vec![Warning::FinishWithoutBell((200 * 16, 29 * 16))]);
        assert_eq!(check(&c, false, false), vec![], "the game's level has no bell either");
        let mut c = level();
        c.objs.extend((0..7).map(|k| Obj { x: 1000 + k * 16, y: 400, kind: 1, flag: false }));
        assert_eq!(check(&c, false, true), vec![Warning::CrowdedScreen((1000, 400), 7)]);
        let mut c = level();
        c.objs.extend((0..84).map(|k| Obj { x: k * 48, y: 400, kind: 1, flag: false }));
        assert!(check(&c, false, true).contains(&Warning::TooManyObjects(85)));
        let mut c = level();
        c.map[29 * COLS + 4] = 0x10;
        assert_eq!(check(&c, false, true), vec![Warning::StartBlocked(c.start.sprite_at())]);
        let mut c = level();
        c.objs.push(Obj { x: 600, y: 31 * 16, kind: checkpoint::BELL, flag: false });
        for r in 31..ROWS {
            for col in 35..40 {
                c.map[r * COLS + col] = 0x10;
            }
        }
        assert!(matches!(check(&c, false, true)[..], [Warning::RestartBlocked(_)]));
    }
}
