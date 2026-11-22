//! Points numbered from $80 (`cart_model::added_points`) on the world map and the game's
//! zones. As in the game, the routes and the point levels hold one map's at a time
//! (`added_on`); the other maps' are kept in `added_kept`. Both are in the points history, so
//! an undo puts back the map that was in with its points.
use super::EditorDocument;
use cart_model::added_points::{self, ZoneAdded};
use cart_model::map_routes::{MapRoutes, Place};
use cart_model::new_zones::ZonePoint;
use cart_model::original_points::{self, OnMap};
use cart_model::world;

/// Points a game zone can have in all.
pub const ZONE_POINTS: usize = 40;

/// `added_kept` index of a map with added points of its own (the world map and the game's
/// zones but the bonus game).
fn slot(map: OnMap) -> Option<usize> {
    match map {
        OnMap::World => Some(0),
        OnMap::Zone(z) if z < added_points::ZONES => Some(1 + z),
        _ => None,
    }
}

fn map_of(slot: usize) -> OnMap {
    if slot == 0 {
        OnMap::World
    } else {
        OnMap::Zone(slot - 1)
    }
}

impl EditorDocument {
    /// The map whose added points are in the routes.
    pub fn added_on(&self) -> OnMap {
        map_of(self.added_on)
    }

    /// How many added points map `map` has while it is not in the routes.
    pub(super) fn added_kept_on(&self, map: OnMap) -> usize {
        slot(map).map_or(0, |s| self.added_kept[s].len())
    }

    /// The added points of the map in the routes, taken out of them.
    pub(super) fn take_added(&mut self) -> Vec<ZonePoint> {
        let v = added_of(&self.routes, &self.points, &self.shuts);
        for q in &v {
            self.routes.ways[q.number as usize] = Default::default();
            self.points[q.number as usize] = world::NO_LEVEL;
            self.shuts[q.number as usize] = 0;
        }
        self.routes.added.retain(|&p| added_points::index(p).is_none());
        v
    }

    fn put_added(&mut self, v: &[ZonePoint]) {
        for q in v {
            let i = q.number as usize;
            self.routes.ways[i] = q.ways.clone();
            self.routes.places[i] = q.place;
            self.points[i] = q.level;
            self.shuts[i] = q.shut;
            self.routes.added.push(q.number);
        }
    }

    /// Put map `map`'s added points in the routes (the world map's or a game zone's; other
    /// maps change nothing). Not an edit.
    pub fn show_added(&mut self, map: OnMap) {
        let Some(s) = slot(map) else { return };
        if s == self.added_on {
            return;
        }
        let out = self.take_added();
        self.added_kept[self.added_on] = out;
        let now = std::mem::take(&mut self.added_kept[s]);
        self.put_added(&now);
        self.added_on = s;
    }

    /// The game zones' added points as the copy keeps them.
    pub fn zones_added(&self) -> Vec<ZoneAdded> {
        (1..=added_points::ZONES)
            .map(|s| ZoneAdded { zone: s - 1, points: if s == self.added_on { added_of(&self.routes, &self.points, &self.shuts) } else { self.added_kept[s].clone() } })
            .filter(|z| !z.points.is_empty())
            .collect()
    }

    /// Does any map use numbers from $80 (the copy then needs the hooks)?
    pub fn uses_added(&self) -> bool {
        self.routes.added.iter().any(|&p| added_points::index(p).is_some())
            || self.added_kept.iter().any(|v| !v.is_empty())
            || self.new_zones.iter().any(|z| z.points.iter().any(|p| added_points::index(p.number).is_some()))
    }

    /// A new point on map `map` (the world map or a game zone) at `(x, y)` of its picture,
    /// opening no level yet. None when the map has no room left.
    pub fn add_point(&mut self, map: OnMap, x: i32, y: i32) -> Option<u8> {
        slot(map)?;
        self.show_added(map);
        let (used, max) = self.point_count(map);
        if used >= max {
            return None;
        }
        let free = added_points::numbers().find(|p| !self.routes.added.contains(p));
        let p = match map {
            OnMap::World => free.or_else(|| self.routes.free_point())?,
            _ => free?,
        };
        let place = match map {
            OnMap::World => Place::at_world(x, y),
            _ => {
                let cam = original_points::on(map).next().map_or(self.routes.places[0], |s| self.routes.places[s.point as usize]);
                Place { x: x.clamp(0, 159) as u8, y: y.clamp(8, 143) as u8, rest: [0, 0], ..cam }
            }
        };
        self.map_snapshot();
        self.routes.added.push(p);
        self.routes.ways[p as usize] = Default::default();
        self.routes.places[p as usize] = place;
        self.points[p as usize] = world::NO_LEVEL;
        self.shuts[p as usize] = 0;
        self.routes_dirty = true;
        self.points_dirty = true;
        Some(p)
    }

    /// Can point `p` of the map in the routes (or of new zone `zone`) shut its ways until its
    /// level is cleared? Only added points can: the game's keep the game's.
    pub fn can_shut(&self, zone: Option<usize>, p: u8) -> bool {
        added_points::index(p).is_some() && zone.map_or(self.routes.added.contains(&p), |k| self.new_zones[k].point(p).is_some())
    }

    /// The pad directions point `p` shuts until its level is cleared (`added_points::DIR_BITS`).
    pub fn shut_of(&self, zone: Option<usize>, p: u8) -> u8 {
        match zone {
            Some(k) => self.new_zones[k].point(p).map_or(0, |q| q.shut),
            None => self.shuts[p as usize],
        }
    }

    /// Shut or open the way of point `p` in direction `dir` (0 right, 1 left, 2 up, 3 down)
    /// until its level is cleared, undoably.
    pub fn toggle_shut(&mut self, zone: Option<usize>, p: u8, dir: usize) {
        if !self.can_shut(zone, p) {
            return;
        }
        let bit = added_points::DIR_BITS[dir];
        match zone {
            Some(k) => {
                if let Some(q) = self.zone_edit(k).point_mut(p) {
                    q.shut ^= bit;
                }
            }
            None => {
                self.map_snapshot();
                self.shuts[p as usize] ^= bit;
                self.points_dirty = true;
            }
        }
    }
}

/// By point number, the shut directions of the world map's added points in `rom`.
pub(super) fn shuts_in(rom: &[u8]) -> Vec<u8> {
    shuts_from(&added_points::shuts(rom))
}

/// By point number, from the world map's added points' shut directions (from `FIRST`).
pub(super) fn shuts_from(added: &[u8]) -> Vec<u8> {
    let mut v = vec![0; cart_model::map_routes::ALL];
    for (j, &s) in added.iter().take(added_points::MAX).enumerate() {
        v[added_points::FIRST as usize + j] = s;
    }
    v
}

/// The added points in `routes` (their levels in `points`).
fn added_of(routes: &MapRoutes, points: &[u8], shuts: &[u8]) -> Vec<ZonePoint> {
    let numbers = routes.added.iter().copied().filter(|&p| added_points::index(p).is_some());
    numbers.map(|p| ZonePoint { number: p, place: routes.places[p as usize], ways: routes.ways[p as usize].clone(), level: points[p as usize], flag: p, shut: shuts[p as usize], pass: None }).collect()
}

/// `added_kept` as a copy keeps it (the world map's are in its routes).
pub(super) fn kept_in(rom: &[u8]) -> Vec<Vec<ZonePoint>> {
    kept_from(added_points::read_zones(rom))
}

pub(super) fn kept_from(zones: Vec<ZoneAdded>) -> Vec<Vec<ZonePoint>> {
    let mut v = vec![Vec::new(); 1 + added_points::ZONES];
    for z in zones.into_iter().filter(|z| z.zone < added_points::ZONES) {
        v[1 + z.zone] = z.points;
    }
    v
}

impl EditorDocument {
    /// The routes and point levels with the world map's added points in.
    pub fn world_routes(&self) -> (MapRoutes, Vec<u8>) {
        let (mut routes, mut points) = (self.routes.clone(), self.points.clone());
        if self.added_on != 0 {
            for q in added_of(&routes, &points, &self.shuts) {
                routes.ways[q.number as usize] = Default::default();
                points[q.number as usize] = world::NO_LEVEL;
            }
            routes.added.retain(|&p| added_points::index(p).is_none());
            for q in &self.added_kept[0] {
                let i = q.number as usize;
                routes.ways[i] = q.ways.clone();
                routes.places[i] = q.place;
                points[i] = q.level;
                routes.added.push(q.number);
            }
        }
        (routes, points)
    }

    /// The shut directions of the world map's added points, from `FIRST`.
    pub fn world_shuts(&self) -> Vec<u8> {
        let mut v = vec![0; added_points::MAX];
        let world: Vec<ZonePoint> = if self.added_on == 0 { added_of(&self.routes, &self.points, &self.shuts) } else { self.added_kept[0].clone() };
        for q in world {
            if let Some(j) = added_points::index(q.number) {
                v[j] = q.shut;
            }
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cart_model::map_routes::{Way, RIGHT};

    fn open() -> Option<EditorDocument> {
        let rom = std::fs::read(host::test_cartridge()?).ok()?;
        Some(EditorDocument::open(rom, None))
    }

    /// Points from $80 on the world map and the Tree Zone: each map shows its own, the copy
    /// keeps them (the world map's in its routes, the zone's for the port), undo puts back the
    /// map that was in, and a zone takes 40 points in all.
    #[test]
    fn added_points_are_kept_per_map_saved_and_undone() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let w = doc.add_point(OnMap::World, 200, 300).unwrap();
        assert_eq!(w, added_points::FIRST, "new world points take numbers from $80");
        doc.set_way(0x07, 0, Some(Way { steps: vec![RIGHT; 3], to: w }));
        let t = doc.add_point(OnMap::Zone(0), 80, 60).unwrap();
        assert_eq!((t, doc.added_on()), (added_points::FIRST, OnMap::Zone(0)), "the Tree Zone's own $80");
        doc.step_point_level(t as usize, 3);
        let level = doc.points[t as usize];
        doc.toggle_shut(None, t, 1);
        doc.show_added(OnMap::World);
        doc.toggle_shut(None, w, 0);
        assert!(!doc.can_shut(None, 0x07), "the game's points keep the game's");
        assert!(doc.routes.added.contains(&w) && doc.routes.ways[0x07][0].as_ref().is_some_and(|x| x.to == w));
        assert_eq!(doc.points[w as usize], world::NO_LEVEL, "the world map's $80 opens nothing");
        assert_eq!(doc.point_count(OnMap::Zone(0)).0, original_points::on(OnMap::Zone(0)).count() + 1);

        doc.commit().unwrap();
        assert!(added_points::installed(&doc.rom));
        let z = added_points::read_zones(&doc.rom);
        assert_eq!((z.len(), z[0].zone, z[0].points[0].number, z[0].points[0].level, z[0].points[0].shut), (1, 0, t, level, added_points::DIR_BITS[1]));
        assert_eq!(added_points::shuts(&doc.rom)[0], added_points::DIR_BITS[0], "the world map's $80 shuts its way right");
        assert!(cart_model::map_routes::read(&doc.rom).added.contains(&w), "the world map's in the copy's routes");
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.zones_added(), z);
        let p = again.project_file();
        assert_eq!(crate::files::read_project(&crate::files::project_text(&p)), Ok(p.clone()));
        assert_eq!(p.zone_points, z);

        // undo across maps: back to before the zone's point, with the world map's kept
        doc.show_added(OnMap::Zone(0));
        assert!(doc.map_undo() && doc.map_undo() && doc.map_undo() && doc.map_undo());
        doc.show_added(OnMap::Zone(0));
        assert!(!doc.routes.added.contains(&t), "the Tree Zone's point is undone");
        doc.show_added(OnMap::World);
        assert!(doc.routes.added.contains(&w));

        let room = ZONE_POINTS - original_points::on(OnMap::Zone(0)).count();
        for i in 0..room {
            assert!(doc.add_point(OnMap::Zone(0), 8 + 4 * (i as i32 % 30), 40 + 8 * (i as i32 / 30)).is_some());
        }
        assert_eq!(doc.point_count(OnMap::Zone(0)), (ZONE_POINTS, ZONE_POINTS));
        assert_eq!(doc.add_point(OnMap::Zone(0), 50, 50), None, "40 points in all");
    }
}
