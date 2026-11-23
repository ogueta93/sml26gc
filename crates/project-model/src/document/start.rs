//! Mario's start in the document (`cart_model::start_point`): one point of the project, on the
//! world map, a game zone or a new zone, where every new game begins. It is set in the map
//! undo history; a project whose start is missing is not saved.
use super::EditorDocument;
use cart_model::original_points::{OnMap, SPOTS};
use cart_model::start_point::Start;

impl EditorDocument {
    /// The map Mario's start is on (None: missing).
    pub fn start_map(&self) -> Option<OnMap> {
        match self.start.new_zone {
            Some(k) => (k < self.new_zones.len() && self.new_zones[k].point(self.start.point).is_some()).then_some(OnMap::NewZone(k)),
            None => SPOTS.iter().find(|s| s.point == self.start.point).map(|s| s.map),
        }
    }

    /// Is Mario's start on a point the project has?
    pub fn start_ok(&self) -> bool {
        self.start_map().is_some()
    }

    /// Mario's start's point, if it is on `map`.
    pub fn map_start(&self, map: OnMap) -> Option<u8> {
        (self.start_map() == Some(map)).then_some(self.start.point)
    }

    /// Mario's start were it on point `p` of `map` (None: it cannot be there: a game point of
    /// another map, or an added point).
    pub fn start_at(&self, map: OnMap, p: u8) -> Option<Start> {
        match map {
            OnMap::NewZone(k) => self.new_zones.get(k)?.point(p).map(|_| Start { point: p, new_zone: Some(k) }),
            _ => SPOTS.iter().any(|s| s.point == p && s.map == map).then_some(Start { point: p, new_zone: None }),
        }
    }

    /// Put Mario's start there (an undo step); the old one is gone, as there is one.
    pub fn set_start(&mut self, s: Start) {
        if s != self.start {
            self.map_snapshot();
            self.start = s;
            self.start_dirty = true;
        }
    }

    /// New zone `k` deleted: a start in a later one follows it down; one in `k` is missing.
    pub(super) fn start_after_removal(&mut self, k: usize) {
        if let Some(j) = self.start.new_zone {
            if j > k {
                self.start.new_zone = Some(j - 1);
                self.start_dirty = true;
            } else if j == k {
                // left pointing nowhere: saving says so until it is set again
                self.start.new_zone = Some(usize::MAX);
                self.start_dirty = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> Option<EditorDocument> {
        let rom = host::test_cartridge()?;
        Some(EditorDocument::open(std::fs::read(rom).ok()?, None))
    }

    #[test]
    fn one_start_set_saved_and_refused_when_missing() {
        let Some(mut doc) = open() else { return };
        assert_eq!(doc.start, Start::default());
        assert_eq!(doc.start_map(), Some(OnMap::World));
        // a point of the Pumpkin Zone (zone 1)
        let s = doc.start_at(OnMap::Zone(1), 0x29).expect("a Pumpkin Zone point");
        assert_eq!(doc.start_at(OnMap::World, 0x29), None, "not a world point");
        doc.set_start(s);
        assert_eq!(doc.map_start(OnMap::Zone(1)), Some(0x29));
        assert_eq!(doc.map_start(OnMap::World), None, "only one start");
        doc.commit().unwrap();
        assert_eq!(cart_model::start_point::read(&doc.rom), s);
        assert_eq!(EditorDocument::open(doc.original.clone(), Some(doc.rom.clone())).start, s);
        assert!(doc.map_undo());
        assert_eq!(doc.start, Start::default());
        // a new zone holding the start, deleted: not saved
        let k = doc.add_zone(OnMap::Zone(1)).unwrap();
        doc.set_start(Start { point: 0x29, new_zone: Some(k) });
        assert!(doc.start_ok());
        doc.remove_zone(k);
        assert!(!doc.start_ok());
        assert_eq!(doc.commit().err(), Some(cart_model::CartError::StartMissing));
    }
}
