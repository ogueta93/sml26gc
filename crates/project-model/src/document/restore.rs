//! Restore: the map being edited as the original cartridge has it, its picture and its points
//! (the levels they open, their paths, places, doors and exits); a new zone, as its base is
//! there (it keeps its name and its door), or as the zone file it was imported from had it (it
//! keeps its door). The map's music too. Undoable in both tabs.
use super::{zones::{self, ZonePoints}, EditorDocument};
use cart_model::doors::{self, NONE};
use cart_model::map_routes::{self, ZONE_EXITS};
use cart_model::new_zones::{self, NewZone};
use cart_model::original_points::{OnMap, SPOTS};
use cart_model::map_music::{self, Map};
use cart_model::{world, zone_maps};

impl EditorDocument {
    /// The points of the world map: drawn there, where Mario walks out of a zone, or new.
    pub(super) fn world_points(&self) -> Vec<u8> {
        let mut v: Vec<u8> = SPOTS.iter().filter(|s| s.map == OnMap::World).map(|s| s.point).chain(ZONE_EXITS.iter().map(|z| z.1)).chain(self.routes.added.iter().copied()).collect();
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Point `p`'s level, ways and place as the original has them (and its twin's ways).
    fn restore_point(&mut self, p: u8, original: &map_routes::MapRoutes, levels: &[u8]) {
        let i = p as usize;
        self.points[i] = levels[i];
        self.routes.ways[i] = original.ways[i].clone();
        self.routes.places[i] = original.places[i];
        if let Some(t) = map_routes::twin(p) {
            self.routes.ways[t as usize] = original.ways[t as usize].clone();
        }
    }

    /// Restore map `map` (see the module). False when there is nothing to restore it from.
    pub fn restore_map(&mut self, map: OnMap) -> bool {
        let original = map_routes::read(&self.original);
        let levels = world::point_levels(&self.original);
        let restored = match map {
            OnMap::NewZone(k) if k < self.new_zones.len() && self.new_zones[k].restore.is_some() => {
                let point = self.new_zones[k].restore.clone();
                let door = self.new_zones[k].door;
                let file = point.as_ref().map(|p| p.file.clone()).unwrap_or_else(|| self.new_zone(k));
                self.map_snapshot();
                let (points, picture) = ZonePoints::from(NewZone { door, ..zones::settled(file, self.new_zones.len()) });
                self.new_zones[k] = ZonePoints { restore: point, ..points };
                self.zones[zone_maps::ZONES.len() + k] = picture;
                self.new_zones_dirty = true;
                true
            }
            OnMap::NewZone(k) if k < self.new_zones.len() => {
                let base = self.new_zones[k].base;
                let picture = zone_maps::read(&self.original, base);
                let Some(z) = new_zones::from_parts(&self.original, base, &picture, &original, &levels, &doors::read(&self.original)) else { return false };
                let (door, name) = (self.new_zones[k].door, self.new_zones[k].name.clone());
                self.map_snapshot();
                let (points, picture) = ZonePoints::from(z);
                self.new_zones[k] = ZonePoints { door, name, ..points };
                self.zones[zone_maps::ZONES.len() + k] = picture;
                self.new_zones_dirty = true;
                true
            }
            OnMap::NewZone(_) => false,
            OnMap::World => {
                self.map_snapshot();
                self.world = world::read(&self.original);
                self.world_music = map_music::read(&self.original, Map::World);
                self.music_dirty = true;
                for p in self.world_points() {
                    self.restore_point(p, &original, &levels);
                }
                self.routes.added.clear();
                if !self.doors.into.is_empty() {
                    self.doors.into = doors::read(&self.original).into;
                }
                for z in &mut self.new_zones {
                    z.door = NONE;
                }
                self.new_zones_dirty |= !self.new_zones.is_empty();
                true
            }
            OnMap::Zone(z) => {
                self.map_snapshot();
                self.zones[z] = zone_maps::read(&self.original, z);
                self.zone_music[z] = map_music::read(&self.original, Map::Zone(z));
                self.music_dirty = true;
                self.show_added(map);
                self.take_added();
                let world_points = self.world_points();
                let own: Vec<u8> = SPOTS.iter().filter(|s| s.map == map && !world_points.contains(&s.point)).map(|s| s.point).collect();
                for &p in &own {
                    self.restore_point(p, &original, &levels);
                }
                if let Some(i) = doors::INTO.iter().position(|d| d.zone == z) {
                    self.entries[i] = new_zones::entry_point(&self.original, z).unwrap_or(NONE);
                }
                if !self.doors.exits.is_empty() {
                    let first = doors::read(&self.original).exits;
                    for (i, e) in doors::EXITS.iter().enumerate() {
                        if own.contains(&e.point) {
                            self.doors.exits[i] = first[i];
                        }
                    }
                }
                true
            }
        };
        if restored {
            self.world_dirty = true;
            self.points_dirty = true;
            self.routes_dirty = true;
            self.doors_dirty = true;
        }
        restored
    }
}
