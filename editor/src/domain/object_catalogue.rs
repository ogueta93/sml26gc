//! The game's objects once each: the (level, type) pairs the game's levels use, grouped when
//! their captured pictures have the same pixels (the same object in several levels, or in
//! both object catalogues). A pair that draws nothing groups with the same type of its
//! catalogue; one not captured yet stays apart until it is.
use cart_model::art::Look;
use cart_model::levels::LEVELS;
use cart_model::objects;

/// One object of the game.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CatalogueObject {
    /// Every (level, type) the game's levels use it as, in level order.
    pub uses: Vec<(usize, u8)>,
}

impl CatalogueObject {
    /// The (level, type) whose picture and name stand for it.
    pub fn first(&self) -> (usize, u8) {
        self.uses[0]
    }

    pub fn levels(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.uses.iter().map(|u| u.0).collect();
        v.dedup();
        v
    }
}

/// The first level of the second object catalogue.
const SECOND: usize = 10;

/// What a pair's picture is: not captured yet, captured drawing nothing, or a picture.
pub enum Seen<'a> {
    NotYet,
    Nothing,
    Picture(&'a Look),
}

/// The objects of the game's levels in `original`, grouped by `seen` pictures.
pub fn objects<'a>(original: &[u8], seen: impl Fn((usize, u8)) -> Seen<'a>) -> Vec<CatalogueObject> {
    let mut uses: Vec<(usize, u8)> = (0..LEVELS).flat_map(|l| objects::read(original, l).0.into_iter().map(move |o| (l, o.kind))).collect();
    uses.sort_unstable();
    uses.dedup();
    let mut out: Vec<(CatalogueObject, Option<&'a Look>)> = Vec::new();
    for u in uses {
        let s = seen(u);
        let found = out.iter_mut().find(|(o, look)| match (&s, look) {
            (Seen::Picture(a), Some(b)) => (a.w, a.h, &a.px) == (b.w, b.h, &b.px),
            (Seen::Nothing, None) => {
                let f = o.first();
                matches!(seen(f), Seen::Nothing) && f.1 == u.1 && (f.0 >= SECOND) == (u.0 >= SECOND)
            }
            _ => false,
        });
        match found {
            Some((o, _)) => o.uses.push(u),
            None => out.push((CatalogueObject { uses: vec![u] }, if let Seen::Picture(l) = s { Some(l) } else { None })),
        }
    }
    out.into_iter().map(|(o, _)| o).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_picture_is_one_object_and_markers_join_by_type() {
        let Some(path) = host::test_cartridge() else {
            eprintln!("no cartridge found: test skipped");
            return;
        };
        let rom = std::fs::read(path).unwrap();
        let mushroom = Look { w: 2, h: 1, px: vec![Some(1), Some(2)], dx: 0, dy: 0, sets: 2 };
        let other = Look { w: 1, h: 1, px: vec![Some(3)], dx: 0, dy: 0, sets: 4 };
        let seen = |(l, k): (usize, u8)| match k {
            0x1B if l != 19 => Seen::Picture(&mushroom),
            0x11 | 0x1B => Seen::Nothing,
            _ => Seen::Picture(&other),
        };
        let all = objects(&rom, seen);
        let with = |k: u8| all.iter().filter(|o| o.uses.iter().any(|u| u.1 == k)).count();
        assert_eq!(all.iter().filter(|o| o.uses.contains(&(7, 0x1B))).count(), 1);
        let shroom = all.iter().find(|o| o.uses.contains(&(7, 0x1B))).unwrap();
        assert!(shroom.uses.contains(&(11, 0x1B)), "the mushroom of both catalogues is one object");
        assert!(!shroom.uses.contains(&(19, 0x1B)), "level 19's $1B draws nothing: another object");
        assert_eq!(all.iter().filter(|o| o.uses.iter().all(|u| u.1 == 0x11)).count(), 2, "the marker once per catalogue");
        assert!(with(0x11) >= 2);
    }
}
