//! New zones in the document (`cart_model::new_zones`): made from a zone as it is being
//! edited, their points within the base zone's numbers, the paths between them, the levels
//! they open, their exits and the world point that is their door. A new zone's picture is
//! `zones[ZONES.len() + k]`, in the world history with the other pictures; the rest is in the
//! points history. Making or deleting one empties both histories.
use super::{first_dir, EditorDocument, RestorePoint, ZONE_POINTS};
use cart_model::added_points;
use cart_model::doors::{self, Leads, NONE};
use cart_model::map_routes::{Place, Way};
use cart_model::new_zones::{self, NewZone, ZonePoint};
use cart_model::original_points::OnMap;
use cart_model::ram_map::SPECIAL_MAP_POINTS;
use cart_model::{world, zone_maps};

/// A new zone's points, paths, levels, exits and door (its picture is kept with the others).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZonePoints {
    /// Index into `zone_maps::ZONES`.
    pub base: usize,
    pub points: Vec<ZonePoint>,
    pub entry: u8,
    /// Per exit site of the base (`new_zones::exit_sites`).
    pub exits: Vec<(u8, Leads)>,
    /// The world point that enters it (`doors::NONE`: none).
    pub door: u8,
    /// Empty: none.
    pub name: String,
    /// The music its map plays (`cart_model::map_music`).
    pub music: u8,
    /// The zone file it was imported or replaced from, if it was.
    pub restore: Option<Box<RestorePoint<NewZone>>>,
}

impl ZonePoints {
    pub fn from(z: NewZone) -> (ZonePoints, Vec<u8>) {
        (ZonePoints { base: z.base, points: z.points, entry: z.entry, exits: z.exits, door: z.door, name: z.name, music: z.music, restore: None }, z.picture)
    }

    pub fn point(&self, p: u8) -> Option<&ZonePoint> {
        self.points.iter().find(|x| x.number == p)
    }

    pub(super) fn point_mut(&mut self, p: u8) -> Option<&mut ZonePoint> {
        self.points.iter_mut().find(|x| x.number == p)
    }

    /// How many points a zone made from this base can have.
    pub fn room(&self) -> usize {
        new_zones::base_of(self.base).map_or(0, |b| b.room().min(ZONE_POINTS))
    }
}

/// Why a new zone or point was not made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneRefusal {
    /// As many new zones as there can be.
    TooManyZones(usize),
    /// New zones are made from the zones a door enters (and from new zones).
    NotABase,
    /// The zone has as many points as its base's numbers allow.
    NoNumberLeft(usize),
}

/// `(x, y)` on the picture moved to the 4-pixel grid of the zone's entry point, where paths
/// (steps of 4 and 8 pixels) can reach it.
fn snapped(z: &ZonePoints, x: i32, y: i32) -> (u8, u8) {
    let at = z.point(z.entry).or(z.points.first()).map_or((0, 0), |p| (p.place.x as i32, p.place.y as i32));
    let snap = |v: i32, from: i32, max: i32| (from + ((v - from) as f32 / 4.0).round() as i32 * 4).clamp(from.rem_euclid(4), max - (max - from).rem_euclid(4)) as u8;
    (snap(x, at.0, 159), snap(y, at.1, 143))
}

/// A zone from a file or a project among `count` new zones: exits into a new zone that is not
/// there lead nowhere.
pub(super) fn settled(mut z: NewZone, count: usize) -> NewZone {
    for e in &mut z.exits {
        if matches!(e.1, Leads::NewZone(j) if j >= count) {
            e.1 = Leads::World(NONE);
        }
    }
    z
}

fn leads_after_removal(l: &mut Leads, k: usize) {
    match *l {
        Leads::NewZone(j) if j == k => *l = Leads::World(NONE),
        Leads::NewZone(j) if j > k => *l = Leads::NewZone(j - 1),
        _ => {}
    }
}

impl EditorDocument {
    fn zone_picture(k: usize) -> usize {
        zone_maps::ZONES.len() + k
    }

    /// Zone `z` (one a door enters) as being edited, as a new zone.
    fn game_zone(&self, z: usize) -> Option<NewZone> {
        let mut n = new_zones::from_parts(&self.rom, z, &self.zones[z], &self.routes, &self.points, &self.doors)?;
        n.music = self.zone_music[z];
        let entry = self.entries[doors::INTO.iter().position(|d| d.zone == z)?];
        if n.points.iter().any(|p| p.number == entry) {
            n.entry = entry;
        }
        Some(n)
    }

    /// The music map `map` plays.
    pub fn map_music(&self, map: OnMap) -> u8 {
        match map {
            OnMap::World => self.world_music,
            OnMap::Zone(z) => self.zone_music.get(z).copied().unwrap_or(0),
            OnMap::NewZone(k) => self.new_zones.get(k).map_or(0, |z| z.music),
        }
    }

    /// Give map `map` music `music` (undoable).
    pub fn set_map_music(&mut self, map: OnMap, music: u8) {
        if self.map_music(map) == music {
            return;
        }
        self.map_snapshot();
        match map {
            OnMap::World => {
                self.world_music = music;
                self.music_dirty = true;
            }
            OnMap::Zone(z) if z < self.zone_music.len() => {
                self.zone_music[z] = music;
                self.music_dirty = true;
            }
            OnMap::NewZone(k) if k < self.new_zones.len() => {
                self.new_zones[k].music = music;
                self.new_zones_dirty = true;
            }
            _ => {}
        }
    }

    /// Make the doors into zone `z` (one a door enters) enter it on its point `p` (undoable).
    pub fn set_game_entry(&mut self, z: usize, p: u8) {
        let Some(i) = doors::INTO.iter().position(|d| d.zone == z) else { return };
        if self.entries[i] != p && new_zones::base_of(z).is_some_and(|b| b.has(p)) {
            self.map_snapshot();
            self.entries[i] = p;
            self.doors_dirty = true;
        }
    }

    /// The point zone `z` is entered on, if a door enters it.
    pub fn game_entry(&self, z: usize) -> Option<u8> {
        doors::INTO.iter().position(|d| d.zone == z).map(|i| self.entries[i])
    }

    /// New zone `k` as the edited copy keeps it.
    pub fn new_zone(&self, k: usize) -> NewZone {
        let z = &self.new_zones[k];
        NewZone { base: z.base, picture: self.zones[Self::zone_picture(k)].clone(), points: z.points.clone(), entry: z.entry, exits: z.exits.clone(), door: z.door, name: z.name.clone(), music: z.music }
    }

    fn zones_changed(&mut self) {
        self.new_zones_dirty = true;
        self.doors_dirty = true;
        self.map_history.clear();
    }

    /// Add a new zone after the others; empties the map undo histories.
    pub fn push_zone(&mut self, z: NewZone) -> Result<usize, ZoneRefusal> {
        if self.new_zones.len() >= new_zones::MAX {
            return Err(ZoneRefusal::TooManyZones(new_zones::MAX));
        }
        new_zones::base_of(z.base).ok_or(ZoneRefusal::NotABase)?;
        let (points, picture) = ZonePoints::from(z);
        self.new_zones.push(points);
        self.zones.push(picture);
        self.zones_changed();
        Ok(self.new_zones.len() - 1)
    }

    /// A new zone that starts as a copy of map `from` (a zone a door enters, or a new zone)
    /// as it is being edited, with no door yet.
    pub fn add_zone(&mut self, from: OnMap) -> Result<usize, ZoneRefusal> {
        let z = match from {
            OnMap::Zone(z) => self.game_zone(z).ok_or(ZoneRefusal::NotABase)?,
            OnMap::NewZone(k) if k < self.new_zones.len() => NewZone { door: NONE, ..self.new_zone(k) },
            _ => return Err(ZoneRefusal::NotABase),
        };
        self.push_zone(z)
    }

    /// An empty new zone with zone `z`'s graphics: its picture all its most common tile, one
    /// point (where `z` is entered, with no paths or level), no exits and no door.
    pub fn blank_zone(&mut self, z: usize) -> Result<usize, ZoneRefusal> {
        let from = self.game_zone(z).ok_or(ZoneRefusal::NotABase)?;
        let mut count = [0usize; 256];
        for &t in &self.zones[z] {
            count[t as usize] += 1;
        }
        let ground = (0..=255u8).max_by_key(|&t| count[t as usize]).unwrap_or(0);
        let entry = from.points.iter().find(|p| p.number == from.entry).or(from.points.first()).ok_or(ZoneRefusal::NotABase)?;
        let point = ZonePoint { ways: Default::default(), level: world::NO_LEVEL, shut: 0, ..entry.clone() };
        let blank = NewZone {
            base: z,
            picture: vec![ground; from.picture.len()],
            entry: point.number,
            points: vec![point],
            exits: from.exits.iter().map(|_| (NONE, Leads::World(NONE))).collect(),
            door: NONE,
            name: String::new(),
            music: from.music,
        };
        self.push_zone(blank)
    }

    /// Delete new zone `k`: doors and exits into it lead nowhere, the ones after it move down
    /// a number. Empties the map undo histories.
    pub fn remove_zone(&mut self, k: usize) -> bool {
        if k >= self.new_zones.len() {
            return false;
        }
        self.new_zones.remove(k);
        self.zones.remove(Self::zone_picture(k));
        for e in &mut self.doors.exits {
            leads_after_removal(&mut e.1, k);
        }
        for z in &mut self.new_zones {
            for e in &mut z.exits {
                leads_after_removal(&mut e.1, k);
            }
        }
        self.start_after_removal(k);
        self.zones_changed();
        true
    }

    /// Map `map` as a zone file keeps it: a new zone, or a zone a door enters as a new zone
    /// made from it. With no door, and exits into new zones lead nowhere.
    pub fn zone_file(&self, map: OnMap) -> Option<NewZone> {
        let z = match map {
            OnMap::Zone(z) => self.game_zone(z)?,
            OnMap::NewZone(k) if k < self.new_zones.len() => self.new_zone(k),
            _ => return None,
        };
        Some(NewZone { door: NONE, ..settled(z, 0) })
    }

    /// A new zone from a zone file exported from `origin`, after the others, with no door;
    /// Restore puts the file's zone back.
    pub fn import_zone(&mut self, f: NewZone, origin: &str) -> Result<usize, ZoneRefusal> {
        let count = self.new_zones.len() + 1;
        let z = NewZone { door: NONE, ..settled(f, count) };
        let k = self.push_zone(z.clone())?;
        self.new_zones[k].restore = Some(Box::new(RestorePoint { file: z, origin: origin.to_string() }));
        Ok(k)
    }

    /// Where new zone `k` was imported from, when Restore puts it back as its file had it
    /// (`files::Origin::text`, empty: unknown).
    pub fn zone_restore_origin(&self, k: usize) -> Option<&str> {
        self.new_zones.get(k)?.restore.as_ref().map(|r| r.origin.as_str())
    }

    /// Replace new zone `k` with a zone file exported from `origin`, which Restore then puts
    /// back; it keeps its door. The game's zones are never replaced. Empties the map undo
    /// histories.
    pub fn replace_zone(&mut self, k: usize, f: NewZone, origin: &str) -> bool {
        if k >= self.new_zones.len() || new_zones::base_of(f.base).is_none() {
            return false;
        }
        let door = self.new_zones[k].door;
        let z = settled(f, self.new_zones.len());
        let (points, picture) = ZonePoints::from(NewZone { door, ..z.clone() });
        self.new_zones[k] = ZonePoints { restore: Some(Box::new(RestorePoint { file: NewZone { door: NONE, ..z }, origin: origin.to_string() })), ..points };
        self.zones[Self::zone_picture(k)] = picture;
        self.zones_changed();
        true
    }

    /// The new zone world point `p` enters, if any.
    pub fn zone_door_of(&self, p: u8) -> Option<usize> {
        self.new_zones.iter().position(|z| z.door == p)
    }

    /// Make world point `p` enter new zone `k` or no new zone (undoable); a door into a game
    /// zone there is taken out.
    pub fn set_zone_door(&mut self, p: u8, k: Option<usize>) {
        if self.zone_door_of(p) == k || k.is_some_and(|k| k >= self.new_zones.len()) {
            return;
        }
        self.map_snapshot();
        for z in &mut self.new_zones {
            if z.door == p {
                z.door = NONE;
            }
        }
        if let Some(k) = k {
            self.new_zones[k].door = p;
            if !self.doors.into.is_empty() {
                self.doors.set_door(p, None);
            }
        }
        self.new_zones_dirty = true;
        self.doors_dirty = true;
    }

    /// Name new zone `k` (undoable).
    pub fn rename_zone(&mut self, k: usize, name: &str) {
        let name = cart_model::extra_levels::clean_name(name);
        if k < self.new_zones.len() && self.new_zones[k].name != name {
            self.zone_edit(k).name = name;
        }
    }

    pub(super) fn zone_edit(&mut self, k: usize) -> &mut ZonePoints {
        self.map_snapshot();
        self.new_zones_dirty = true;
        &mut self.new_zones[k]
    }

    /// A new point on new zone `k` near `(x, y)` of its picture (pixels, Mario's place), opening
    /// no level yet, with a cleared flag of its base no other point of the zone uses.
    pub fn zone_add_point(&mut self, k: usize, x: i32, y: i32) -> Result<u8, ZoneRefusal> {
        let z = &self.new_zones[k];
        let b = new_zones::base_of(z.base).ok_or(ZoneRefusal::NotABase)?;
        let number = b.usable().find(|&n| z.point(n).is_none()).filter(|_| z.points.len() < z.room()).ok_or(ZoneRefusal::NoNumberLeft(z.room()))?;
        let pool = new_zones::flags(&self.rom, b);
        let free = pool.iter().copied().find(|f| !SPECIAL_MAP_POINTS.contains(f) && !z.points.iter().any(|p| p.flag == *f));
        // an added number's flag is the number itself
        let flag = if added_points::index(number).is_some() { number } else { free.or_else(|| pool.first().copied()).unwrap_or(0) };
        let cam = z.points.first().map_or(Place { cam_x: 0, cam_y: 0, x: 0, y: 0, rest: [0, 0] }, |p| p.place);
        let (x, y) = snapped(z, x, y);
        let place = Place { x, y, ..cam };
        self.zone_edit(k).points.push(ZonePoint { number, place, ways: Default::default(), level: world::NO_LEVEL, flag, shut: 0, pass: None });
        Ok(number)
    }

    /// Move point `p` of new zone `k` near `(x, y)`; the paths to and from it become straight.
    pub fn zone_move_point(&mut self, k: usize, p: u8, x: i32, y: i32) {
        let Some(at) = self.new_zones[k].point(p).map(|q| q.place) else { return };
        let (x, y) = snapped(&self.new_zones[k], x, y);
        if (at.x, at.y) == (x, y) {
            return;
        }
        let z = self.zone_edit(k);
        if let Some(q) = z.point_mut(p) {
            q.place.x = x;
            q.place.y = y;
        }
        let places: Vec<(u8, Place)> = z.points.iter().map(|q| (q.number, q.place)).collect();
        let place_of = |n: u8| places.iter().find(|q| q.0 == n).map(|q| q.1);
        for q in &mut z.points {
            for w in q.ways.iter_mut().flatten() {
                if q.number != p && w.to != p {
                    continue;
                }
                if let Some(to) = place_of(w.to) {
                    *w = Way::through(&[(to.x as i32 - q.place.x as i32, to.y as i32 - q.place.y as i32)], w.to);
                }
            }
        }
    }

    /// Take out point `p` of new zone `k` with its paths; its exits are on no point. The last
    /// point stays.
    pub fn zone_remove_point(&mut self, k: usize, p: u8) -> bool {
        if self.new_zones[k].point(p).is_none() || self.new_zones[k].points.len() < 2 {
            return false;
        }
        let z = self.zone_edit(k);
        z.points.retain(|q| q.number != p);
        for q in &mut z.points {
            for w in q.ways.iter_mut() {
                if w.as_ref().is_some_and(|w| w.to == p) {
                    *w = None;
                }
            }
        }
        for e in &mut z.exits {
            if e.0 == p {
                e.0 = NONE;
            }
        }
        if z.entry == p {
            z.entry = z.points[0].number;
        }
        true
    }

    /// Mario enters new zone `k` on point `p`.
    pub fn zone_set_entry(&mut self, k: usize, p: u8) {
        if self.new_zones[k].entry != p && self.new_zones[k].point(p).is_some() {
            self.zone_edit(k).entry = p;
        }
    }

    /// Step the level point `p` of new zone `k` opens: no level, 0, 1, ... the last level.
    pub fn zone_step_level(&mut self, k: usize, p: u8, step: i32) {
        let count = self.levels.len() as i32;
        let Some(cur) = self.new_zones[k].point(p).map(|q| q.level) else { return };
        if step == 0 {
            return;
        }
        let i = if cur == world::NO_LEVEL || cur as i32 >= count { count } else { cur as i32 };
        let n = (i + step).rem_euclid(count + 1);
        if let Some(q) = self.zone_edit(k).point_mut(p) {
            q.level = if n == count { world::NO_LEVEL } else { n as u8 };
        }
    }

    /// Set or take out the way from point `p` of new zone `k` in direction `dir`.
    pub fn zone_set_way(&mut self, k: usize, p: u8, dir: usize, way: Option<Way>) {
        if self.new_zones[k].point(p).is_none_or(|q| q.ways[dir] == way) {
            return;
        }
        if let Some(q) = self.zone_edit(k).point_mut(p) {
            q.ways[dir] = way;
        }
    }

    /// A path drawn on new zone `k`, as `draw_path` on the game's maps.
    pub fn zone_draw_path(&mut self, k: usize, from: u8, to: u8, corners: &[(i32, i32)]) -> Option<(usize, Option<usize>)> {
        let z = &self.new_zones[k];
        z.point(to)?;
        z.point(from)?;
        let way = Way::through(corners, to);
        let dir = first_dir(&way)?;
        let (back, back_dir) = super::way_back(&way, corners, from).filter(|(_, d)| z.point(to).is_some_and(|q| q.ways[*d].is_none())).unzip();
        let z = self.zone_edit(k);
        if let (Some(back), Some(d)) = (back, back_dir) {
            if let Some(q) = z.point_mut(to) {
                q.ways[d] = Some(back);
            }
        }
        if let Some(q) = z.point_mut(from) {
            q.ways[dir] = Some(way);
        }
        Some((dir, back_dir))
    }

    /// Set where exit `i` of new zone `k` is and where it leads.
    pub fn zone_set_exit(&mut self, k: usize, i: usize, at: u8, leads: Leads) {
        if self.new_zones[k].exits.get(i).is_none_or(|e| *e == (at, leads)) {
            return;
        }
        self.zone_edit(k).exits[i] = (at, leads);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cart_model::doors;

    fn open() -> Option<EditorDocument> {
        let rom = host::find_rom(None)?;
        Some(EditorDocument::open(std::fs::read(rom).ok()?, None))
    }

    #[test]
    fn a_game_zone_entry_is_saved_undone_and_restored() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let first = doc.game_entry(0);
        doc.set_game_entry(0, 0x30);
        assert_eq!(doc.game_entry(0), first, "only the zone's own points");
        doc.set_game_entry(0, 0x21);
        assert_eq!(doc.game_entry(0), Some(0x21));
        assert_eq!(doc.add_zone(OnMap::Zone(0)).map(|k| doc.new_zone(k).entry), Ok(0x21), "a copy enters where it does");
        doc.commit().unwrap();
        assert_eq!(new_zones::entry_point(&doc.rom, 0), Some(0x21));
        assert!(doc.restore_map(OnMap::Zone(0)));
        assert_eq!(doc.game_entry(0), first);
    }

    #[test]
    fn a_new_zone_is_restored_as_its_base_keeping_its_name_and_door() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let k = doc.add_zone(OnMap::Zone(0)).unwrap();
        let first = doc.new_zone(k);
        doc.rename_zone(k, "Forest");
        doc.set_zone_door(0x07, Some(k));
        doc.zone_step_level(k, 0x23, 1);
        doc.zone_add_point(k, 40, 40).unwrap();
        doc.map_snapshot();
        doc.set_tile(OnMap::NewZone(k), 2, 2, doc.tile(OnMap::NewZone(k), 2, 2) ^ 1);
        assert!(doc.restore_map(OnMap::NewZone(k)));
        assert_eq!(doc.new_zone(k), NewZone { name: "Forest".into(), door: 0x07, ..first.clone() });
        assert!(doc.map_undo());
        assert_eq!(doc.new_zones[k].points.len(), first.points.len() + 1, "undo brings the points back");
    }

    #[test]
    fn zone_files_make_and_replace_new_zones_only() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        assert_eq!(doc.zone_file(OnMap::World), None);
        let tree = doc.zone_file(OnMap::Zone(0)).unwrap();
        assert_eq!(doc.import_zone(tree.clone(), ""), Ok(0));
        assert_eq!(doc.new_zone(0), tree);
        doc.set_zone_door(0x07, Some(0));
        doc.zone_set_exit(0, 0, 0x23, Leads::NewZone(0));
        let (f, _, _) = super::super::files::read_zone(&super::super::files::zone_text(&doc.zone_file(OnMap::NewZone(0)).unwrap(), None, "")).unwrap();
        assert_eq!((f.door, f.exits[0].1), (NONE, Leads::World(NONE)), "a file has no door and no new zones to go to");
        let pumpkin = doc.zone_file(OnMap::Zone(1)).unwrap();
        assert!(doc.replace_zone(0, pumpkin.clone(), ""));
        assert_eq!(doc.new_zone(0), NewZone { door: 0x07, ..pumpkin }, "it keeps its door");
        assert!(!doc.replace_zone(1, tree, ""), "only new zones are replaced");
    }

    #[test]
    fn an_imported_zone_restores_as_its_file_had_it() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let made = doc.add_zone(OnMap::Zone(1)).unwrap();
        let tree = doc.zone_file(OnMap::Zone(0)).unwrap();
        let k = doc.import_zone(tree.clone(), "Demo|zone|00").unwrap();
        assert_eq!(doc.zone_restore_origin(made), None, "a zone made from a game zone has none");
        doc.set_zone_door(0x07, Some(k));
        doc.zone_add_point(k, 40, 40).unwrap();
        assert!(doc.restore_map(OnMap::NewZone(k)));
        assert_eq!(doc.new_zone(k), NewZone { door: 0x07, ..tree.clone() }, "the file's zone, keeping its door");
        assert!(doc.remove_zone(made));
        let k = k - 1;
        doc.zone_add_point(k, 40, 40).unwrap();
        assert!(doc.restore_map(OnMap::NewZone(k)));
        assert_eq!((doc.new_zone(k).points, doc.zone_restore_origin(k)), (tree.points, Some("Demo|zone|00")), "it follows the zone when an earlier one is removed");
    }

    #[test]
    fn new_zones_are_made_saved_read_back_and_deleted() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        assert_eq!(doc.add_zone(OnMap::World), Err(ZoneRefusal::NotABase));
        assert_eq!(doc.add_zone(OnMap::Zone(4)), Err(ZoneRefusal::NotABase), "no door enters the Turtle Zone");
        let tree = OnMap::Zone(0);
        assert_eq!(doc.add_zone(tree), Ok(0));
        let made = doc.new_zone(0);
        assert_eq!(made.picture, doc.zones[0]);
        assert_eq!(made.points.iter().map(|p| p.number).collect::<Vec<_>>(), (0x1F..=0x27).collect::<Vec<u8>>());
        let new = OnMap::NewZone(0);
        doc.map_snapshot();
        doc.set_tile(new, 3, 4, doc.tile(new, 3, 4) ^ 1);
        let picture = doc.zones[zone_maps::ZONES.len()].clone();
        assert_ne!(picture, doc.zones[0], "painting a new zone leaves its base alone");
        doc.set_zone_door(0x07, Some(0));
        doc.set_exit(0, 0x23, Leads::NewZone(0));
        doc.commit().unwrap();

        let mut again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.new_zones, doc.new_zones);
        assert_eq!(again.zones[zone_maps::ZONES.len()], picture);
        assert_eq!((again.zone_door_of(0x07), again.doors.exits[0]), (Some(0), (0x23, Leads::NewZone(0))));
        // in the copy the exit leads to the new zone's door
        assert_eq!(doors::read(&again.rom).exits[0].1, Leads::World(0x07));

        assert_eq!(again.add_zone(new), Ok(1));
        assert_eq!(again.new_zones[1].door, doors::NONE, "a copy has no door");
        assert!(again.remove_zone(0));
        assert_eq!(again.new_zones.len(), 1);
        assert_eq!(again.zone_door_of(0x07), None);
        assert_eq!(again.doors.exits[0].1, Leads::World(doors::NONE), "the exit into it leads nowhere");
        assert_eq!(again.zones.len(), zone_maps::ZONES.len() + 1);
        assert!(!again.remove_zone(5));
    }

    #[test]
    fn points_of_a_new_zone_take_its_bases_numbers_then_spare_then_added_ones() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let k = doc.add_zone(OnMap::Zone(0)).unwrap();
        let base = new_zones::base_of(0).unwrap();
        let spare: Vec<u8> = (0..new_zones::SPARE.len() as i32).map(|i| doc.zone_add_point(k, 30 + 8 * i, 50).unwrap()).collect();
        assert_eq!(spare, new_zones::SPARE.to_vec(), "the Tree Zone uses all its numbers; then come the spare ones");
        assert!(spare.iter().all(|&p| new_zones::flags(&doc.rom, base).contains(&doc.new_zones[k].point(p).unwrap().flag)));
        let added: Vec<u8> = (0..20).map(|i| doc.zone_add_point(k, 30 + 4 * (i % 10), 60 + 8 * (i / 10)).unwrap()).collect();
        assert_eq!(added, (0x80..0x94).collect::<Vec<u8>>(), "then numbers from $80, up to 40 points");
        assert!(added.iter().all(|&p| doc.new_zones[k].point(p).unwrap().flag == p), "an added number's flag is its own");
        assert_eq!(doc.zone_add_point(k, 50, 50), Err(ZoneRefusal::NoNumberLeft(ZONE_POINTS)));
        for p in added.into_iter().chain(spare) {
            assert!(doc.zone_remove_point(k, p));
        }
        assert!(doc.zone_remove_point(k, 0x24));
        assert!(doc.new_zones[k].points.iter().all(|p| p.ways.iter().flatten().all(|w| w.to != 0x24)));
        let before = doc.new_zones[k].clone();
        let p = doc.zone_add_point(k, 60, 40).unwrap();
        assert_eq!(p, 0x24);
        assert!(new_zones::flags(&doc.rom, base).contains(&doc.new_zones[k].point(p).unwrap().flag), "one of its base's flags");

        // a path from the entry to it, then the point moved: the paths follow
        let entry = doc.new_zones[k].entry;
        let e = doc.new_zones[k].point(entry).unwrap().place;
        let at = doc.new_zones[k].point(p).unwrap().place;
        let (dx, dy) = (at.x as i32 - e.x as i32, at.y as i32 - e.y as i32);
        doc.zone_set_way(k, entry, 2, None);
        assert!(doc.zone_draw_path(k, entry, p, &[(0, dy), (dx, dy)]).is_some());
        doc.zone_move_point(k, p, 70, 41);
        let to = doc.new_zones[k].point(p).unwrap().place;
        assert!((to.x as i32 - 70).abs() <= 2 && (to.y as i32 - 41).abs() <= 2);
        let way = doc.new_zones[k].point(entry).unwrap().ways.iter().flatten().find(|w| w.to == p).unwrap().clone();
        assert_eq!(*way.track().last().unwrap(), (to.x as i32 - e.x as i32, to.y as i32 - e.y as i32), "the path ends on the point");

        doc.zone_step_level(k, p, 1);
        assert_eq!(doc.new_zones[k].point(p).unwrap().level, 0);
        assert!(doc.map_undo());
        assert_eq!(doc.new_zones[k].point(p).unwrap().level, world::NO_LEVEL);
        let mut steps = 0;
        while doc.new_zones[k] != before && steps < 6 {
            doc.map_undo();
            steps += 1;
        }
        assert_eq!(doc.new_zones[k], before, "undo goes back step by step to before the point was added");
        assert!(!doc.zone_remove_point(k, 0x30), "not one of its points");
    }

    #[test]
    fn a_blank_zone_has_its_base_graphics_one_point_and_nothing_else() {
        let Some(mut doc) = open() else { return };
        let k = doc.blank_zone(1).unwrap();
        let z = doc.new_zone(k);
        assert_eq!(z.base, 1);
        assert_eq!(z.points.len(), 1);
        assert_eq!(z.entry, z.points[0].number);
        assert!(z.points[0].ways.iter().all(|w| w.is_none()) && z.points[0].level == world::NO_LEVEL);
        assert!(z.picture.iter().all(|&t| t == z.picture[0]), "one tile all over");
        assert!(z.exits.iter().all(|e| e.0 == NONE) && z.door == NONE);
        assert_eq!(doc.blank_zone(4), Err(ZoneRefusal::NotABase), "the castle is no base");
        doc.commit().unwrap();
    }
}
