//! The editor's new zones while the game is played (`cart_model::new_zones`): when Mario
//! stands on a new zone's door on the world map, the zone is written over its base zone in
//! the console's ROM and its cleared flags take the base zone's place in the file being
//! played; it stays while he is on its points, and the base zone comes back once he is on
//! the world map away from the door. Which zone is in, and the flags not in the game's
//! memory, are kept by a `ZoneProgressStore`, so a game saved inside a new zone resumes there.
//! An exit into a new zone leads to its door (`new_zones::resolve`): arriving there from
//! another zone, the port presses A for Mario; so does a secret exit into a new zone
//! (`secret_exits`), arriving there from a level played elsewhere. A new game whose start is
//! in a new zone (`cart_model::start_point`) gets the zone once the file is chosen: the game
//! sets Mario's point before it loads the map (research `newstartframes`).
//!
//! Added points (`cart_model::added_points`): the copy has the world map's in place; while a
//! game zone is shown its own are written in (a new zone's come with the zone), and the flag
//! bytes of the shown map's are kept per map and file, as is the map each file was on (a
//! file is resumed on its map).
use crate::domain::zone_progress::{Inside, MapKey, ZoneProgress};
use crate::ports::ZoneProgressStore;
use cart_model::added_points::{self, ZoneAdded};
use cart_model::new_zones::{self, NewZone, CURRENT_FILE};
use cart_model::secret_exits::{self, SecretTo};
use cart_model::start_point;
use cart_model::ram_map::{GAME_MODE, MAP_POINT};
use gb_core::{Buttons, Gb};
use std::rc::Rc;

/// The map shown ($00: the world map).
const CURRENT_MAP: u16 = 0xA68B;
const MAP_MODE: u8 = 0x0C;
const LEVEL_MODE: u8 = 0x04;
const FILE_SELECT: u8 = 0x1A;
/// Window frames to wait on a door before pressing A, and for how long.
const ENTER_WAIT: u32 = 40;
const ENTER_PRESS: u32 = 6;

/// Cartridge RAM as kept, whether the game has it switched on or not.
fn sram(g: &Gb, a: u16) -> u8 {
    g.cart().ram().get((a - 0xA000) as usize).copied().unwrap_or(0)
}

fn set_sram(g: &mut Gb, a: u16, v: u8) {
    if let Some(b) = g.cart_mut().ram_mut().get_mut((a - 0xA000) as usize) {
        *b = v;
    }
}

#[derive(Clone)]
pub struct ZoneSwap {
    zones: Vec<NewZone>,
    progress: ZoneProgress,
    store: Rc<dyn ZoneProgressStore>,
    undo: Vec<(usize, Vec<u8>)>,
    /// The map shown when last on a map.
    last_map: u8,
    /// Frames left until A is let go after arriving on a door from another zone.
    enter_in: u32,
    /// The doors of the new zones a level's secret exit leads into.
    secret_doors: Vec<u8>,
    /// The point Mario was on when he last played a level (None: none since the map).
    played_from: Option<u8>,
    last_point: u8,
    /// The copy has added points.
    added: bool,
    added_zones: Vec<ZoneAdded>,
    /// The map and file whose added points' flags are in the game's memory.
    added_in: Option<(MapKey, u8)>,
    /// The `added_zones` entry written in, and the bytes to undo it.
    added_rom: Option<usize>,
    added_undo: Vec<(usize, Vec<u8>)>,
    /// How many times the program in memory has been changed.
    swaps: u32,
    /// Mario's start, when it is in a new zone: the zone and the point.
    start: Option<(usize, u8)>,
    last_mode: u8,
}

impl ZoneSwap {
    /// The new zones of the console's cartridge; the one the progress was saved in is put
    /// back in place (its flags are already in the battery save).
    pub fn new(g: &mut Gb, store: Box<dyn ZoneProgressStore>) -> ZoneSwap {
        let zones = new_zones::read(g.cart().rom());
        let progress = store.load().unwrap_or_default();
        let secret_doors = secret_exits::read(g.cart().rom()).into_iter().filter_map(|(_, to)| match to {
            SecretTo::NewZone(k) => zones.get(k).map(|z| z.door),
            SecretTo::Point(_) => None,
        });
        let secret_doors = secret_doors.collect();
        let added = added_points::installed(g.cart().rom());
        let added_zones = added_points::read_zones(g.cart().rom());
        let store: Rc<dyn ZoneProgressStore> = Rc::from(store);
        let start = start_point::read(g.cart().rom());
        let start = start.new_zone.filter(|&k| k < zones.len()).map(|k| (k, start.point));
        let mut s = ZoneSwap { start, last_mode: 0, swaps: 0, zones, progress, store, undo: Vec::new(), last_map: 0, enter_in: 0, secret_doors, played_from: None, last_point: 0, added, added_zones, added_in: None, added_rom: None, added_undo: Vec::new() };
        if let Some(i) = s.progress.inside.clone() {
            match new_zones::writes_for(g.cart().rom(), &s.zones, i.zone).ok() {
                Some(w) => s.undo = new_zones::apply(g.cart_mut().rom_mut(), &w),
                None => {
                    // the zone is gone from the copy: give the base zone its flags back
                    let (flags, values): (Vec<u8>, Vec<u8>) = i.base.into_iter().unzip();
                    new_zones::set_progress(g, i.file, &flags, &values);
                    s.progress.inside = None;
                    s.store.save(&s.progress);
                }
            }
        }
        // the added points of the map the file was on
        s.update_added(g);
        s
    }

    /// Changes so far to the program in memory (a state taken before one belongs to another
    /// program).
    pub fn swaps(&self) -> u32 {
        self.swaps
    }

    /// The new zone in place, if any.
    pub fn inside(&self) -> Option<usize> {
        self.progress.inside.as_ref().map(|i| i.zone)
    }

    /// After the game's frames: put a new zone in or out as Mario's place asks, and the shown
    /// map's added points. True if it changed (the battery save should be written now, to
    /// match the kept progress).
    pub fn update(&mut self, g: &mut Gb) -> bool {
        let zone = self.update_zones(g);
        self.update_added(g) || zone
    }

    fn update_zones(&mut self, g: &mut Gb) -> bool {
        if self.zones.is_empty() {
            return false;
        }
        let mode = g.read(GAME_MODE);
        if std::mem::replace(&mut self.last_mode, mode) == FILE_SELECT && mode != FILE_SELECT && self.new_game_start(g) {
            return true;
        }
        if mode != MAP_MODE {
            if mode == LEVEL_MODE {
                self.played_from = self.played_from.or(Some(self.last_point));
            }
            return false;
        }
        let point = g.read(MAP_POINT);
        let from_level = self.played_from.take();
        if from_level.is_some_and(|p| p != point) && self.secret_doors.contains(&point) {
            self.enter_in = ENTER_WAIT + ENTER_PRESS;
        }
        self.last_point = point;
        let map = g.read(CURRENT_MAP);
        let from_zone = std::mem::replace(&mut self.last_map, map) != 0 && map == 0;
        let now = self.inside();
        // the zone in place stays on its door and its points (entering, the point changes
        // while the world map is still up)
        let stays = now.filter(|&k| point == self.zones[k].door || new_zones::base_of(self.zones[k].base).is_some_and(|b| b.holds(point)));
        let want = stays.or_else(|| (map == 0).then(|| self.zones.iter().position(|z| z.door == point)).flatten());
        if want == now {
            return false;
        }
        self.leave(g);
        if let Some(k) = want {
            self.enter(g, k);
            if from_zone {
                self.enter_in = ENTER_WAIT + ENTER_PRESS;
            }
        }
        self.store.save(&self.progress);
        true
    }

    /// Buttons the port presses for Mario this window frame (A, to enter a new zone an exit
    /// led into).
    pub fn buttons(&mut self) -> Buttons {
        self.enter_in = self.enter_in.saturating_sub(1);
        if self.enter_in > 0 && self.enter_in <= ENTER_PRESS {
            Buttons::A
        } else {
            Buttons::NONE
        }
    }

    /// The map whose added points should be in: the one shown, or out of the maps the one
    /// the file was on.
    fn added_map(&self, g: &mut Gb, file: u8) -> Option<MapKey> {
        if g.read(GAME_MODE) == MAP_MODE {
            return Some(match (self.inside(), added_points::zone_shown(g.read(CURRENT_MAP))) {
                (Some(k), _) => MapKey::NewZone(k),
                (None, Some(z)) => MapKey::Zone(z),
                (None, None) => MapKey::World,
            });
        }
        match self.added_in {
            Some((_, f)) if f == file => None,
            _ => Some(self.progress.shown.get(&file).copied().unwrap_or(MapKey::World)),
        }
    }

    fn update_added(&mut self, g: &mut Gb) -> bool {
        if !self.added {
            return false;
        }
        let file = sram(g, CURRENT_FILE);
        if file > 2 {
            return false;
        }
        if g.read(GAME_MODE) == FILE_SELECT {
            // the game is (re)started: the flags go back in once a file is played
            self.added_in = None;
            return false;
        }
        let flags = |g: &mut Gb| (0..added_points::MAX as u16).map(|j| sram(g, added_points::FLAG_RAM + j)).collect::<Vec<u8>>();
        let Some(map) = self.added_map(g, file).or(self.added_in.map(|a| a.0)) else { return false };
        if self.added_in == Some((map, file)) {
            // the game sets them on the map, and wipes them when it is switched on
            if g.read(GAME_MODE) != MAP_MODE {
                return false;
            }
            let now = flags(g);
            if self.progress.added.get(&(map, file)) == Some(&now) {
                return false;
            }
            self.progress.added.insert((map, file), now);
            self.store.save(&self.progress);
            return true;
        }
        if let Some(was) = self.added_in {
            self.progress.added.insert(was, flags(g));
        }
        let zone = match map {
            MapKey::Zone(z) => self.added_zones.iter().position(|a| a.zone == z),
            _ => None,
        };
        if zone != self.added_rom {
            self.swaps += 1;
            new_zones::apply(g.cart_mut().rom_mut(), &std::mem::take(&mut self.added_undo));
            if let Some(w) = zone.and_then(|i| added_points::writes_for_zone(g.cart().rom(), self.added_zones[i].zone, &self.added_zones[i].points).ok()) {
                self.added_undo = new_zones::apply(g.cart_mut().rom_mut(), &w);
            }
            self.added_rom = zone;
        }
        let own = self.progress.added.get(&(map, file)).cloned().unwrap_or_default();
        for j in 0..added_points::MAX {
            set_sram(g, added_points::FLAG_RAM + j as u16, own.get(j).copied().unwrap_or(0));
        }
        self.progress.added.insert((map, file), flags(g));
        self.progress.shown.insert(file, map);
        self.added_in = Some((map, file));
        self.store.save(&self.progress);
        true
    }

    /// A file just chosen on Mario's start in a new zone, never in it before (no progress of
    /// it kept): the zone is put in before the map is drawn.
    fn new_game_start(&mut self, g: &mut Gb) -> bool {
        let Some((k, point)) = self.start else { return false };
        let file = g.read(CURRENT_FILE);
        if self.inside().is_some() || g.read(MAP_POINT) != point || self.progress.kept.contains_key(&(k, file)) {
            return false;
        }
        self.enter(g, k);
        self.store.save(&self.progress);
        true
    }

    fn enter(&mut self, g: &mut Gb, k: usize) {
        let Some(b) = new_zones::base_of(self.zones[k].base) else { return };
        let Ok(w) = new_zones::writes_for(g.cart().rom(), &self.zones, k) else { return };
        let flags = new_zones::flags(g.cart().rom(), b);
        self.swaps += 1;
        self.undo = new_zones::apply(g.cart_mut().rom_mut(), &w);
        let file = g.read(CURRENT_FILE);
        let base = new_zones::progress(g, file, &flags);
        let own = self.progress.kept.remove(&(k, file)).unwrap_or_else(|| vec![0; flags.len()]);
        new_zones::set_progress(g, file, &flags, &own);
        self.progress.inside = Some(Inside { zone: k, file, base: flags.into_iter().zip(base).collect() });
    }

    fn leave(&mut self, g: &mut Gb) {
        let Some(i) = self.progress.inside.take() else { return };
        self.swaps += 1;
        new_zones::apply(g.cart_mut().rom_mut(), &std::mem::take(&mut self.undo));
        let (flags, values): (Vec<u8>, Vec<u8>) = i.base.into_iter().unzip();
        let own = new_zones::progress(g, i.file, &flags);
        self.progress.kept.insert((i.zone, i.file), own);
        new_zones::set_progress(g, i.file, &flags, &values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cart_model::start_point::Start;
    use std::cell::RefCell;

    #[derive(Default)]
    struct Kept(RefCell<Option<ZoneProgress>>);
    impl ZoneProgressStore for Kept {
        fn load(&self) -> Option<ZoneProgress> {
            self.0.borrow().clone()
        }
        fn save(&self, p: &ZoneProgress) {
            *self.0.borrow_mut() = Some(p.clone());
        }
    }

    /// A new game of `rom` played with no save, until the map has been shown for two seconds.
    fn new_game_on_the_map(rom: Vec<u8>) -> (Gb, ZoneSwap) {
        let mut g = Gb::new(rom, 1000).unwrap();
        let mut swap = ZoneSwap::new(&mut g, Box::new(Kept::default()));
        let mut on_map = 0;
        for f in 0..3000u32 {
            g.run_frame_with(if f % 50 < 5 && f > 120 && on_map == 0 { Buttons::START } else { Buttons::NONE });
            g.clear_audio();
            swap.update(&mut g);
            if g.read(GAME_MODE) == MAP_MODE {
                on_map += 1;
                if on_map == 120 {
                    break;
                }
            }
        }
        assert_eq!(on_map, 120, "the map is reached");
        (g, swap)
    }

    /// A project whose Mario's start is in an empty new zone, played with no save: the new
    /// game is in the new zone, on its point (SHOT=file.png keeps the screen).
    #[test]
    fn a_new_game_begins_in_the_new_zone_holding_the_start() {
        let Some(path) = host::test_cartridge() else { return };
        let mut doc = project_model::document::EditorDocument::open(std::fs::read(path).unwrap(), None);
        let k = doc.blank_zone(1).unwrap();
        let point = doc.new_zones[k].entry;
        doc.set_start(Start { point, new_zone: Some(k) });
        doc.commit().unwrap();
        let (mut g, swap) = new_game_on_the_map(doc.rom.clone());
        assert_eq!((swap.inside(), g.read(MAP_POINT)), (Some(k), point));
        if let Ok(shot) = std::env::var("SHOT") {
            host::png_out::save_png(&g, std::path::Path::new(&shot));
        }
    }

    /// A new zone made from the Pumpkin Zone with the Space Zone's music plays it; the Pumpkin
    /// Zone keeps its own.
    #[test]
    fn a_new_zone_plays_its_own_music() {
        use cart_model::ram_map::MUSIC_PLAYING;
        use cart_model::original_points::OnMap;
        let Some(path) = host::test_cartridge() else { return };
        let mut doc = project_model::document::EditorDocument::open(std::fs::read(path).unwrap(), None);
        let k = doc.blank_zone(1).unwrap();
        doc.set_map_music(OnMap::NewZone(k), 0x1C);
        let point = doc.new_zones[k].entry;
        doc.set_start(Start { point, new_zone: Some(k) });
        doc.commit().unwrap();
        let (mut g, swap) = new_game_on_the_map(doc.rom.clone());
        assert_eq!((swap.inside(), g.read(MUSIC_PLAYING)), (Some(k), 0x1C));
        doc.set_start(Start { point, new_zone: None });
        doc.commit().unwrap();
        let (mut g, swap) = new_game_on_the_map(doc.rom.clone());
        assert_eq!((swap.inside(), g.read(MUSIC_PLAYING)), (None, 0x1E), "the Pumpkin Zone");
    }
}
