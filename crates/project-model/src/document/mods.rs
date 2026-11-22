//! Which of the game's levels and maps the saved copy has changed from the original (shown as
//! "mod"): a level's blocks, objects, start, settings or secret exit target; a map's picture, its own
//! points (paths, places, levels), points added to it or its music. A zone's exit point ($19, ...) has
//! the ways of its world entrance (`map_routes::twin`): those count for the world map.
use super::{level_from, EditorDocument};
use cart_model::levels::LEVELS;
use cart_model::original_points::{self, OnMap};
use cart_model::map_music::{self, Map};
use cart_model::{added_points, map_routes, secret_exits, world, zone_maps};

/// Per game level, and per game map (the world map, then `zone_maps::ZONES`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SavedMods {
    pub levels: Vec<bool>,
    pub maps: Vec<bool>,
}

impl SavedMods {
    pub fn level(&self, l: usize) -> bool {
        self.levels.get(l).copied().unwrap_or(false)
    }

    pub fn map(&self, map: OnMap) -> bool {
        let i = match map {
            OnMap::World => 0,
            OnMap::Zone(z) => 1 + z,
            OnMap::NewZone(_) => return false,
        };
        self.maps.get(i).copied().unwrap_or(false)
    }
}

impl EditorDocument {
    /// What the copy as saved (`rom`) changes from the original.
    pub(super) fn saved_mods(&self) -> SavedMods {
        let (rom, original) = (&self.rom, &self.original);
        let secrets = secret_exits::read(rom);
        let levels = (0..LEVELS)
            .map(|l| {
                let (a, b) = (level_from(rom, l), level_from(original, l));
                a.map != b.map || a.objs != b.objs || a.start != b.start || a.settings != b.settings || a.tables != b.tables || a.enemies != b.enemies || secrets.iter().any(|&(s, _)| s == l)
            })
            .collect();
        let (routes, first) = (map_routes::read(rom), map_routes::read(original));
        let (now, was) = (world::point_levels(rom), world::point_levels(original));
        let points_differ = |map: OnMap| {
            original_points::on(map).any(|s| {
                let p = s.point as usize;
                let world_ways = map != OnMap::World && map_routes::ZONE_EXITS.iter().any(|&(_, exit)| exit == s.point);
                (!world_ways && (routes.ways[p] != first.ways[p] || routes.places[p] != first.places[p])) || now[p] != was[p]
            })
        };
        let added = added_points::read_zones(rom);
        let music_differs = |map: Map| map_music::read(rom, map) != map_music::read(original, map);
        let mut maps = vec![world::read(rom) != world::read(original) || points_differ(OnMap::World) || routes.added.iter().any(|&p| !first.added.contains(&p)) || music_differs(Map::World)];
        maps.extend((0..zone_maps::ZONES.len()).map(|z| zone_maps::read(rom, z) != zone_maps::read(original, z) || points_differ(OnMap::Zone(z)) || added.iter().any(|a| a.zone == z && !a.points.is_empty()) || music_differs(Map::Zone(z))));
        SavedMods { levels, maps }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new copy changes nothing; a saved block, a saved point and an added point do.
    #[test]
    fn saved_changes_mark_their_level_and_map() {
        let Some(path) = host::test_cartridge() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let Ok(rom) = std::fs::read(path) else { return };
        let mut doc = EditorDocument::open(rom, None);
        assert_eq!(doc.mods, SavedMods { levels: vec![false; LEVELS], maps: vec![false; 1 + zone_maps::ZONES.len()] }, "a new copy is the game's");
        let b = doc.block(3, 10, 5);
        doc.set_block(3, 10, 5, b ^ 1);
        doc.step_point_level(0x07, 1);
        doc.add_point(OnMap::Zone(1), 80, 60).unwrap();
        assert!(!doc.mods.level(3), "only saved changes count");
        doc.commit().unwrap();
        let marked: Vec<usize> = (0..LEVELS).filter(|&l| doc.mods.level(l)).collect();
        assert!(marked.contains(&3) && !marked.contains(&0), "{marked:?}");
        assert!(doc.mods.map(OnMap::World) && doc.mods.map(OnMap::Zone(1)) && !doc.mods.map(OnMap::Zone(0)));
        assert_eq!(EditorDocument::open(doc.original.clone(), Some(doc.rom.clone())).mods, doc.mods);
        // a new way out of the Tree Zone's entrance $02 (and so of its exit $19) is the world map's
        doc.set_way(0x02, 3, None);
        doc.commit().unwrap();
        assert!(!doc.mods.map(OnMap::Zone(0)), "the Tree Zone is as the game's");
        doc.set_map_music(OnMap::Zone(2), 0x28);
        doc.commit().unwrap();
        assert!(doc.mods.map(OnMap::Zone(2)), "a saved music");
    }
}
