//! Everything being edited: the cartridge being built (the edited copy), the untouched
//! original it is checked against, the 32 levels (blocks and objects) and the new ones
//! after them (`extra_levels`) with their undo history, the world map picture, the level
//! each map point opens and the paths between points (`map_routes`), and the new zones
//! (`zones`). What has changed since the last save is tracked here; the files are not.
mod added;
pub mod check;
mod mods;
pub use mods::SavedMods;
mod points;
mod restore;
mod start;
mod zones;

pub use added::ZONE_POINTS;

pub use zones::{ZonePoints, ZoneRefusal};
use super::files::{self, FileError, LevelFile, ProjectFile, RestorePoints, WorldFile};
use super::history::History;
use super::stamp::{Area, Stamp};
use cart_model::error::{BankUsage, ObjectsUsage};
use cart_model::extra_levels::{self, Extra};
use cart_model::level_goals::{self, Goal};
use cart_model::level_settings::Settings;
use cart_model::blank_level;
use cart_model::enemy_sets::{self, Enemy};
use cart_model::level_tables::Tables;
use cart_model::rom::offset as rom_offset;
use cart_model::level_start::Start;
use cart_model::secret_exits::{self, SecretTo};
use cart_model::levels::{self, COLS, LEVELS, ROWS};
use cart_model::doors::{self, Doors, Leads};
use cart_model::map_routes::{self, MapRoutes, Way};
use cart_model::objects::{self, Obj};
use cart_model::original_notes::{self, Note};
use cart_model::map_music::{self, Map};
use cart_model::new_zones::ZonePoint;
use cart_model::original_points::OnMap;
use cart_model::start_point::{self, Start as GameStart};
use cart_model::{added_points, checkpoint, layout, level_graphics, new_zones, patch, world, zone_maps, CartError};

/// What a painting tool works on: a level's blocks or a map picture's tiles.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Grid {
    Level(usize),
    Map(OnMap),
}

/// What a new level is played as: one of the 32 (its graphics, music and time) with the new
/// level's own header.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StandIn {
    pub base: u8,
    pub header: [u8; 20],
}

/// What a new level or new zone was when it was imported or replaced from a file, to restore
/// it to; `origin` as the file said it (`files::Origin::text`, empty: unknown).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestorePoint<T> {
    pub file: T,
    pub origin: String,
}

/// A level as it can be undone: its blocks, its objects and, for a new level, its stand-in.
#[derive(Clone, PartialEq, Debug)]
pub struct LevelContent {
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
    /// None for the 32 levels of the cartridge.
    pub stand_in: Option<StandIn>,
    /// A new level's name (empty: none).
    pub name: String,
    /// Where Mario begins the level (header bytes $00-$07).
    pub start: Start,
    /// Start time, music and background shades (header bytes $13, $0E, $0F).
    pub settings: Settings,
    /// Gravity, auto-scroll, camera margin and animated tiles (the game's per-level tables).
    pub tables: Tables,
    /// Its own enemy set (None: the game's enemies of the level it plays as).
    pub enemies: Option<EnemySet>,
    /// What the cells under the finish and the secret exit had before they were placed
    /// there (while editing; not saved).
    pub under: [Vec<u8>; 2],
    /// Where the secret exit leads (None: where the game's leads, or nowhere).
    pub secret: Option<SecretTo>,
    /// The file it was imported or replaced from, if it was.
    pub restore: Option<Box<RestorePoint<LevelFile>>>,
}

pub struct LevelDoc {
    pub content: LevelContent,
    pub blocks_dirty: bool,
    pub objects_dirty: bool,
    pub start_dirty: bool,
    pub secret_dirty: bool,
    history: History<LevelContent>,
}

impl LevelDoc {
    fn new(content: LevelContent, dirty: bool) -> LevelDoc {
        LevelDoc { content, blocks_dirty: dirty, objects_dirty: dirty, start_dirty: dirty, secret_dirty: dirty, history: History::default() }
    }

    pub fn dirty(&self) -> bool {
        self.blocks_dirty || self.objects_dirty || self.start_dirty || self.secret_dirty
    }

    fn all_dirty(&mut self) {
        self.blocks_dirty = true;
        self.objects_dirty = true;
        self.start_dirty = true;
        self.secret_dirty = true;
    }
}

/// A level's own enemies and the sprite block built for them (`enemy_sets`).
#[derive(Clone, PartialEq, Debug)]
pub struct EnemySet {
    pub enemies: Vec<Enemy>,
    pub block: Vec<u8>,
}

/// Level `l`'s enemy set as the copy `rom` keeps it.
fn stored_set(rom: &[u8], l: usize) -> Option<EnemySet> {
    let s = enemy_sets::read(rom).into_iter().find(|s| s.level == l)?;
    let at = rom_offset(s.block.0 as usize, s.block.1);
    Some(EnemySet { enemies: s.enemies, block: rom.get(at..at + enemy_sets::BLOCK)?.to_vec() })
}

/// A level from a file or a stored new level: its start is in its header; a file without
/// tables gets its base level's in `original`.
fn content_with(f: LevelFile, original: &[u8]) -> LevelContent {
    let tables = f.tables.unwrap_or_else(|| Tables::read(original, f.stand_in.base as usize));
    LevelContent { map: f.map, objs: f.objs, start: Start::read(&f.stand_in.header), settings: Settings::read(&f.stand_in.header), tables, enemies: f.enemies, stand_in: Some(f.stand_in), name: f.name, under: Default::default(), secret: f.secret, restore: None }
}

/// `objs` with each checkpoint bell where the game finds it (x order within its page), as a
/// bell dragged elsewhere would not come to life.
fn bells_in_order(objs: &[Obj]) -> Vec<Obj> {
    let mut out: Vec<Obj> = objs.iter().filter(|o| o.kind != checkpoint::BELL).copied().collect();
    for b in objs.iter().filter(|o| o.kind == checkpoint::BELL) {
        checkpoint::insert_bell(&mut out, b.x, b.y);
    }
    out
}

/// Why a level cannot get another checkpoint.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CheckpointRefused {
    /// Level 31 (or a new level played as it): the game keeps no restart place for it.
    NoRoom,
    /// It has `checkpoint::MAX` already.
    Full,
}

/// What a save wrote into the cartridge (shown to the person).
pub enum Saved {
    Bank(BankUsage),
    WorldMap,
    LevelPoints,
    /// The map's paths and points: bytes the changed paths take.
    Routes(usize),
    /// Which points enter which zone and where the zones' exits lead.
    Doors,
    Objects(ObjectsUsage),
    /// The new levels: how many, bytes used.
    NewLevels(usize, usize),
    /// The new zones: how many, bytes used.
    NewZones(usize, usize),
    /// Levels of the 32 whose start or settings changed.
    Starts(usize),
    /// The levels whose secret exit leads where the user chose.
    SecretExits(usize),
}

pub struct EditorDocument {
    pub original: Vec<u8>,
    pub rom: Vec<u8>,
    /// The 32 levels, then the new ones (number 32 and up).
    pub levels: Vec<LevelDoc>,
    /// A new level was added or removed.
    pub extras_dirty: bool,
    /// The world map picture (64 x 64 tiles) and the zone maps' (20 x 18 each): the game's
    /// eight, then the new zones'.
    pub world: Vec<u8>,
    pub zones: Vec<Vec<u8>>,
    /// Any map picture changed.
    pub world_dirty: bool,
    /// The music of the game's zone maps (`zone_maps::ZONES` order) and of the world map; the
    /// new zones' are in `new_zones`.
    pub zone_music: Vec<u8>,
    pub world_music: u8,
    pub music_dirty: bool,
    /// Which level each of the game's own map points opens.
    pub points: Vec<u8>,
    pub points_dirty: bool,
    /// The paths between map points, where Mario stands on each, and the new points.
    pub routes: MapRoutes,
    pub routes_dirty: bool,
    /// Doors between the world map and the zones.
    pub doors: Doors,
    pub doors_dirty: bool,
    /// The new zones' points, paths, exits and doors (`zones`).
    pub new_zones: Vec<ZonePoints>,
    pub new_zones_dirty: bool,
    /// The point each zone a door enters is entered on (`doors::INTO` order).
    pub entries: Vec<u8>,
    /// Where every new game begins (`cart_model::start_point`).
    pub start: GameStart,
    pub start_dirty: bool,
    /// The map's paths and places as the original has them.
    original_routes: MapRoutes,
    /// What the original game does with each of its points besides level, paths and doors.
    original_notes: Vec<Vec<Note>>,
    /// Undo of the maps: every picture, and the levels the points open, the paths, the
    /// doors, the new zones' points, the zones' entry points and the added points, together,
    /// so steps come back in the order made whatever the tab.
    map_history: History<MapState>,
    /// Which map's added points are in `routes` and `points` (`added`), and the others'.
    added_on: usize,
    added_kept: Vec<Vec<ZonePoint>>,
    /// By point number, the pad directions an added point in `routes` shuts until its level
    /// is cleared (`added_points::shuts`).
    shuts: Vec<u8>,
    /// What the copy as last saved (or opened) changes in the game's levels and maps.
    pub mods: SavedMods,
}

type PointsState = (Vec<u8>, MapRoutes, Doors, Vec<ZonePoints>, Vec<u8>, usize, Vec<Vec<ZonePoint>>, Vec<u8>);
type MapState = ((Vec<u8>, Vec<Vec<u8>>), PointsState, GameStart, (u8, Vec<u8>));

/// `points` with the game's special values (the castle door's $FF) back where a copy saved
/// by an editor that lost them on deleting a level has "no level" (the editor cannot set
/// either on those points).
fn with_specials(original: &[u8], mut points: Vec<u8>) -> Vec<u8> {
    for (p, &o) in world::point_levels(original).iter().enumerate().take(map_routes::POINTS) {
        if o == 0xFF && points[p] == world::NO_LEVEL {
            points[p] = o;
        }
    }
    points
}

/// The point each zone a door enters is entered on in `rom`.
fn entries_of(rom: &[u8]) -> Vec<u8> {
    doors::INTO.iter().map(|d| new_zones::entry_point(rom, d.zone).unwrap_or(doors::NONE)).collect()
}

fn level_from(rom: &[u8], l: usize) -> LevelContent {
    let (_, start) = levels::map_location(rom, levels::header(rom, l));
    let h = levels::header(rom, l).at;
    LevelContent { map: levels::decode(rom, start).0, objs: objects::read(rom, l).0, stand_in: None, name: String::new(), start: Start::read(&rom[h..h + 8]), settings: Settings::read(&rom[h..h + 20]), tables: Tables::read(rom, l), enemies: stored_set(rom, l), under: Default::default(), secret: None, restore: None }
}

impl EditorDocument {
    /// Start from the edited copy if there is a usable one; otherwise from a new copy of the
    /// original, made 1 MB so every zone has room for bigger levels (`patch::expand`).
    pub fn open(original: Vec<u8>, edited: Option<Vec<u8>>) -> EditorDocument {
        let mut rom = edited
            .filter(|r| r.len() == original.len() || patch::is_expanded(r))
            .unwrap_or_else(|| patch::expand(&original).unwrap_or_else(|_| original.clone()));
        patch::upgrade(&mut rom); // copies made before the object patch get it now
        let mut routes = map_routes::read(&rom);
        // copies saved before zone exits followed their entrance get it now
        let routes_dirty = routes.sync_exits();
        let doc = |content| LevelDoc::new(content, false);
        let mut levels: Vec<LevelDoc> = (0..LEVELS).map(|l| doc(level_from(&rom, l))).collect();
        let stored = extra_levels::read(&rom);
        levels.extend(stored.into_iter().map(|e| doc(content_with(LevelFile { stand_in: StandIn { base: e.base, header: e.header }, map: e.map, objs: e.objs, name: e.name, secret: None, tables: Some(e.tables), enemies: None }, &original))));
        for (l, lv) in levels.iter_mut().enumerate().skip(LEVELS) {
            lv.content.enemies = stored_set(&rom, l);
        }
        for (l, to) in secret_exits::read(&rom) {
            if let Some(lv) = levels.get_mut(l) {
                lv.content.secret = Some(to);
            }
        }
        let mut zones: Vec<Vec<u8>> = (0..zone_maps::ZONES.len()).map(|z| zone_maps::read(&rom, z)).collect();
        let (new_zones, pictures): (Vec<ZonePoints>, Vec<Vec<u8>>) = new_zones::read(&rom).into_iter().map(ZonePoints::from).unzip();
        zones.extend(pictures);
        // the copy keeps an exit into a new zone as one to its door
        let mut doors = doors::read(&rom);
        for e in &mut doors.exits {
            if let Leads::World(p) = e.1 {
                if let Some(k) = new_zones.iter().position(|z| z.door == p && p != doors::NONE) {
                    e.1 = Leads::NewZone(k);
                }
            }
        }
        let points = with_specials(&original, world::point_levels(&rom));
        let mut doc = EditorDocument {
            extras_dirty: false,
            world: world::read(&rom),
            zones,
            world_dirty: false,
            zone_music: (0..zone_maps::ZONES.len()).map(|z| map_music::read(&rom, Map::Zone(z))).collect(),
            world_music: map_music::read(&rom, Map::World),
            music_dirty: false,
            points_dirty: points != world::point_levels(&rom),
            points,
            routes,
            routes_dirty,
            doors,
            doors_dirty: false,
            new_zones,
            new_zones_dirty: false,
            entries: entries_of(&rom),
            start: start_point::read(&rom),
            start_dirty: false,
            original_routes: map_routes::read(&original),
            original_notes: (0..map_routes::POINTS as u8).map(|p| original_notes::notes(&original, p)).collect(),
            map_history: History::default(),
            added_on: 0,
            added_kept: added::kept_in(&rom),
            shuts: added::shuts_in(&rom),
            mods: SavedMods::default(),
            original,
            rom,
            levels,
        };
        doc.mods = doc.saved_mods();
        doc
    }

    /// Remember level `l` before a change, for undo.
    pub fn snapshot(&mut self, l: usize) {
        let lv = &mut self.levels[l];
        lv.history.record(lv.content.clone());
    }

    pub fn undo(&mut self, l: usize) -> bool {
        let lv = &mut self.levels[l];
        let done = lv.history.undo(&mut lv.content);
        if done {
            lv.all_dirty();
        }
        done
    }

    pub fn redo(&mut self, l: usize) -> bool {
        let lv = &mut self.levels[l];
        let done = lv.history.redo(&mut lv.content);
        if done {
            lv.all_dirty();
        }
        done
    }

    /// Put block `b` at column `c`, row `r`. True if something changed.
    pub fn set_block(&mut self, l: usize, c: i32, r: i32, b: u8) -> bool {
        let i = r as usize * COLS + c as usize;
        let lv = &mut self.levels[l];
        if lv.content.map[i] == b {
            return false;
        }
        lv.content.map[i] = b;
        lv.blocks_dirty = true;
        true
    }

    pub fn block(&self, l: usize, c: i32, r: i32) -> u8 {
        self.levels[l].content.map[r as usize * COLS + c as usize]
    }

    /// The cells of level `l`'s finish or secret exit, top first.
    pub fn goal_cells(&self, l: usize, goal: Goal) -> Vec<(usize, usize)> {
        level_goals::cells(&self.levels[l].content.map, goal)
    }

    /// Where level `l`'s secret exit leads when the user chose it.
    pub fn level_secret(&self, l: usize) -> Option<SecretTo> {
        self.levels.get(l).and_then(|lv| lv.content.secret)
    }

    /// Send level `l`'s secret exit to `to` (None: the game's way). Undoable.
    pub fn set_level_secret(&mut self, l: usize, to: Option<SecretTo>) {
        if self.levels.get(l).is_none_or(|lv| lv.content.secret == to) {
            return;
        }
        self.snapshot(l);
        let lv = &mut self.levels[l];
        lv.content.secret = to;
        lv.secret_dirty = true;
    }

    /// Level `l` ends with its boss: it has no finish to place.
    pub fn ends_with_boss(&self, l: usize) -> bool {
        level_goals::BOSS_LEVELS.contains(&self.base_of(l)) && self.goal_cells(l, Goal::Finish).is_empty()
    }

    /// Put level `l`'s finish or secret exit at cell `to` (the finish takes its bell along).
    /// The caller takes the undo snapshot, once per drag.
    pub fn place_goal(&mut self, l: usize, goal: Goal, to: (usize, usize)) -> bool {
        let lv = &mut self.levels[l];
        let c = &mut lv.content;
        let under = &mut c.under[goal as usize];
        if !level_goals::place(&mut c.map, &mut c.objs, goal, to, under) {
            return false;
        }
        lv.blocks_dirty = true;
        lv.objects_dirty |= goal == Goal::Finish;
        true
    }

    /// The level as the original cartridge has it (undoable); a new level, as its base level
    /// is there (header too), keeping its name; one imported or replaced from a file, as the
    /// file had it.
    pub fn restore_level(&mut self, l: usize) {
        self.snapshot(l);
        if let Some(point) = self.levels[l].content.restore.clone() {
            let mut content = content_with(point.file.clone(), &self.original);
            content.restore = Some(point);
            let lv = &mut self.levels[l];
            lv.content = content;
            lv.all_dirty();
            self.extras_dirty = true;
            return;
        }
        let base = self.base_of(l);
        let mut content = level_from(&self.original, base);
        let at = levels::header(&self.original, base).at;
        let lv = &mut self.levels[l];
        content.stand_in = lv.content.stand_in.map(|s| StandIn { header: self.original[at..at + 20].try_into().unwrap_or(s.header), ..s });
        content.name = std::mem::take(&mut lv.content.name);
        if let Some(s) = content.stand_in {
            content.start = Start::read(&s.header);
            content.settings = Settings::read(&s.header);
        }
        lv.content = content;
        lv.all_dirty();
    }

    /// Where level `l` was imported from, when Restore puts it back as its file had it
    /// (`files::Origin::text`, empty: unknown).
    pub fn level_restore_origin(&self, l: usize) -> Option<&str> {
        self.levels.get(l)?.content.restore.as_ref().map(|r| r.origin.as_str())
    }

    /// What the new levels and new zones were imported or replaced from, to keep with the
    /// project.
    pub fn restore_points(&self) -> RestorePoints {
        let levels = self.levels.iter().enumerate().skip(LEVELS).filter_map(|(l, lv)| Some((l, (**lv.content.restore.as_ref()?).clone()))).collect();
        let zones = self.new_zones.iter().enumerate().filter_map(|(k, z)| Some((k, (**z.restore.as_ref()?).clone()))).collect();
        RestorePoints { levels, zones }
    }

    /// Give the new levels and new zones the restore points kept with the project; one for a
    /// level or zone the document does not have is dropped. Changes nothing to save.
    pub fn attach_restore_points(&mut self, points: RestorePoints) {
        for (l, point) in points.levels {
            if let Some(lv) = self.levels.get_mut(l).filter(|_| l >= LEVELS) {
                lv.content.restore = Some(Box::new(point));
            }
        }
        for (k, point) in points.zones {
            if let Some(z) = self.new_zones.get_mut(k) {
                z.restore = Some(Box::new(point));
            }
        }
    }

    /// Where Mario begins level `l`.
    pub fn level_start(&self, l: usize) -> Start {
        self.levels[l].content.start
    }

    pub fn level_settings(&self, l: usize) -> Settings {
        self.levels[l].content.settings
    }

    pub fn level_tables(&self, l: usize) -> Tables {
        self.levels[l].content.tables
    }

    /// Level `l`'s tables as the game has them for its base level.
    pub fn game_tables(&self, l: usize) -> Tables {
        Tables::read(&self.original, self.base_of(l))
    }

    /// Change level `l`'s gravity, auto-scroll, camera margin or animated tiles, with an undo
    /// step.
    pub fn set_level_tables(&mut self, l: usize, t: Tables) {
        if self.levels[l].content.tables != t {
            self.snapshot(l);
            let lv = &mut self.levels[l];
            lv.content.tables = t;
            lv.start_dirty = true;
        }
    }

    pub fn level_enemies(&self, l: usize) -> Option<&EnemySet> {
        self.levels[l].content.enemies.as_ref()
    }

    /// Give level `l` its own enemy set, or the game's back (None), with an undo step.
    pub fn set_level_enemies(&mut self, l: usize, set: Option<EnemySet>) {
        if self.levels[l].content.enemies != set {
            self.snapshot(l);
            let lv = &mut self.levels[l];
            lv.content.enemies = set;
            lv.start_dirty = true;
        }
    }

    /// Level `l`'s set becomes `set` and every object of type `kind` goes (an enemy taken out
    /// of the set would look broken), with one undo step. Returns how many objects went.
    /// Take out of every level's enemy set the members of the other object catalogue (a level
    /// plays its own catalogue's objects only), with their objects, each an undo step: the
    /// levels changed and the objects taken out.
    pub fn drop_other_catalogue(&mut self) -> (Vec<usize>, usize) {
        let (mut levels, mut objects) = (Vec::new(), 0);
        for l in 0..self.levels.len() {
            let own = level_graphics::catalogue(self.base_of(l));
            let Some(set) = self.levels[l].content.enemies.clone() else { continue };
            for e in set.enemies.iter().filter(|e| level_graphics::catalogue(e.level as usize) != own) {
                let mut kept = self.levels[l].content.enemies.clone().unwrap_or(set.clone());
                kept.enemies.retain(|m| m != e);
                objects += self.drop_set_enemy(l, kept, e.kind);
                if !levels.contains(&l) {
                    levels.push(l);
                }
            }
        }
        (levels, objects)
    }

    pub fn drop_set_enemy(&mut self, l: usize, set: EnemySet, kind: u8) -> usize {
        self.snapshot(l);
        let lv = &mut self.levels[l];
        let before = lv.content.objs.len();
        lv.content.objs.retain(|o| o.kind != kind);
        lv.content.enemies = Some(set);
        lv.start_dirty = true;
        lv.objects_dirty = true;
        before - lv.content.objs.len()
    }

    /// Change level `l`'s settings, with an undo step.
    pub fn set_level_settings(&mut self, l: usize, s: Settings) {
        if self.levels[l].content.settings != s {
            self.snapshot(l);
            let lv = &mut self.levels[l];
            lv.content.settings = s;
            lv.start_dirty = true;
        }
    }

    /// Move level `l`'s start (the caller takes the undo snapshot, once per drag).
    pub fn set_level_start(&mut self, l: usize, s: Start) {
        let lv = &mut self.levels[l];
        if lv.content.start != s {
            lv.content.start = s;
            lv.start_dirty = true;
        }
    }

    /// Levels whose map is the same one as level `l`'s (a new level has its own).
    pub fn sharing(&self, l: usize) -> Vec<usize> {
        if l >= LEVELS {
            return Vec::new();
        }
        levels::sharing(&self.rom, l)
    }

    /// The level of the 32 that level `l` is played as (itself for those): its graphics,
    /// blocks, music and time.
    pub fn base_of(&self, l: usize) -> usize {
        self.levels.get(l).and_then(|lv| lv.content.stand_in).map(|s| s.base as usize).unwrap_or(l.min(LEVELS - 1))
    }

    /// How many new levels there are.
    pub fn new_levels(&self) -> usize {
        self.levels.len() - LEVELS
    }

    /// A new level after the others, starting as a copy of level `from` as it is being
    /// edited (a new level copies its base). Returns its number, or None when there are as
    /// many as there can be.
    pub fn add_level(&mut self, from: usize) -> Option<usize> {
        let mut content = self.levels[from].content.clone();
        let base = self.base_of(from) as u8;
        content.stand_in = content.stand_in.or_else(|| Some(StandIn { base, header: extra_levels::based_on(&self.rom, base).header }));
        self.push_level(content)
    }

    /// A new blank level after the others that plays as game level `base` (its header,
    /// settings and tables as the game has them): its sky, a floor and Mario's start, no
    /// objects (`blank_level`). None when there is no room.
    pub fn add_blank_level(&mut self, base: usize) -> Option<usize> {
        let mut content = level_from(&self.original, base);
        let at = levels::header(&self.original, base).at;
        let header: [u8; 20] = self.original[at..at + 20].try_into().ok()?;
        let (map, start) = blank_level::blank(&content.map, &content.start);
        (content.map, content.start, content.objs) = (map, start, Vec::new());
        content.stand_in = Some(StandIn { base: base as u8, header });
        self.push_level(content)
    }

    /// Add `content` (a new level's) after the others, or None when there is no room.
    fn push_level(&mut self, content: LevelContent) -> Option<usize> {
        if self.new_levels() >= extra_levels::MAX {
            return None;
        }
        self.levels.push(LevelDoc::new(content, true));
        self.extras_dirty = true;
        Some(self.levels.len() - 1)
    }

    /// Level `l` as a file keeps it; one of the 32 is its own base, with its header.
    pub fn level_file(&self, l: usize) -> LevelFile {
        let c = &self.levels[l].content;
        let mut stand_in = c.stand_in.unwrap_or_else(|| {
            let at = levels::header(&self.rom, l).at;
            StandIn { base: l as u8, header: self.rom[at..at + 20].try_into().unwrap_or([0; 20]) }
        });
        c.start.write(&mut stand_in.header);
        c.settings.write(&mut stand_in.header);
        LevelFile { stand_in, map: c.map.clone(), objs: c.objs.clone(), name: c.name.clone(), secret: c.secret, tables: Some(c.tables), enemies: c.enemies.clone() }
    }

    /// A new level after the others from a level file exported from `origin`, which Restore
    /// puts back; None when there is no room.
    pub fn import_level(&mut self, f: LevelFile, origin: &str) -> Option<usize> {
        let mut content = content_with(f.clone(), &self.original);
        content.restore = Some(Box::new(RestorePoint { file: f, origin: origin.to_string() }));
        self.push_level(content)
    }

    /// Replace new level `l` whole with a level file exported from `origin`, which Restore then
    /// puts back (undoable). The 32 are never replaced.
    pub fn replace_level(&mut self, l: usize, f: LevelFile, origin: &str) -> bool {
        if l < LEVELS || l >= self.levels.len() {
            return false;
        }
        self.snapshot(l);
        let mut content = content_with(f.clone(), &self.original);
        content.restore = Some(Box::new(RestorePoint { file: f, origin: origin.to_string() }));
        let lv = &mut self.levels[l];
        lv.content = content;
        lv.all_dirty();
        true
    }

    /// Name new level `l` (undoable); the 32 have the game's numbers only.
    pub fn rename_level(&mut self, l: usize, name: &str) {
        let name = cart_model::extra_levels::clean_name(name);
        if l < LEVELS || l >= self.levels.len() || self.levels[l].content.name == name {
            return;
        }
        self.snapshot(l);
        self.levels[l].content.name = name;
        self.extras_dirty = true;
    }

    /// The name of level `l` (empty: none).
    pub fn level_name(&self, l: usize) -> &str {
        self.levels.get(l).map_or("", |lv| &lv.content.name)
    }

    /// Everything being edited, as a project file keeps it.
    pub fn project_file(&self) -> ProjectFile {
        ProjectFile {
            cartridge: files::cartridge_id(&self.original),
            levels: (0..self.levels.len()).map(|l| self.level_file(l)).collect(),
            world: self.world.clone(),
            zones: self.zones[..zone_maps::ZONES.len()].to_vec(),
            points: self.world_routes().1,
            routes: self.world_routes().0,
            shuts: self.world_shuts(),
            doors: self.doors.clone(),
            new_zones: (0..self.new_zones.len()).map(|k| self.new_zone(k)).collect(),
            entries: self.entries.clone(),
            zone_points: self.zones_added(),
            start: self.start,
            world_music: self.world_music,
            zone_music: self.zone_music.clone(),
        }
    }

    /// Put a project file in place of everything being edited: all of it marked changed,
    /// nothing written until the next save, every undo history emptied. Refused when the
    /// file was made over another cartridge.
    pub fn replace_project(&mut self, p: ProjectFile) -> Result<(), FileError> {
        if p.cartridge != files::cartridge_id(&self.original) {
            return Err(FileError::OtherCartridge);
        }
        self.levels = p
            .levels
            .into_iter()
            .enumerate()
            .map(|(l, f)| {
                let mut content = content_with(f, &self.original);
                if l < LEVELS {
                    (content.stand_in, content.name) = (None, String::new());
                }
                LevelDoc::new(content, true)
            })
            .collect();
        self.extras_dirty = true;
        (self.world, self.zones) = (p.world, p.zones);
        self.world_dirty = true;
        (self.points, self.routes, self.doors) = (with_specials(&self.original, p.points), p.routes, p.doors);
        self.added_on = 0;
        self.added_kept = added::kept_from(p.zone_points);
        self.shuts = added::shuts_from(&p.shuts);
        self.entries = if p.entries.len() == doors::INTO.len() { p.entries } else { entries_of(&self.original) };
        self.start = p.start;
        self.start_dirty = true;
        (self.world_music, self.zone_music) = (p.world_music, p.zone_music);
        self.music_dirty = true;
        self.new_zones_dirty = !self.new_zones.is_empty() || !p.new_zones.is_empty();
        let count = p.new_zones.len();
        let (zones, pictures): (Vec<ZonePoints>, Vec<Vec<u8>>) = p.new_zones.into_iter().map(|z| ZonePoints::from(zones::settled(z, count))).unzip();
        self.new_zones = zones;
        self.zones.extend(pictures);
        for e in &mut self.doors.exits {
            if matches!(e.1, Leads::NewZone(j) if j >= count) {
                e.1 = Leads::World(doors::NONE);
            }
        }
        self.points_dirty = true;
        self.routes_dirty = true;
        self.doors_dirty = true;
        self.map_history = History::default();
        Ok(())
    }

    /// The world map as a world map file (`files::WorldFile`).
    pub fn world_file(&self) -> WorldFile {
        let (routes, points) = self.world_routes();
        WorldFile { cartridge: files::cartridge_id(&self.original), world: self.world.clone(), points, routes, doors: self.doors.clone(), shuts: self.world_shuts(), start: self.map_start(OnMap::World), music: self.world_music }
    }

    /// Put a world map file in place of the world map (picture, point levels, paths, places,
    /// added points, doors), marked changed, nothing written until the next save, the map undo
    /// history emptied. Points opening a new level the project does not have open none, and
    /// exits into a missing new zone lead nowhere: how many of each. Refused when the file was
    /// made over another cartridge.
    pub fn replace_world(&mut self, w: WorldFile) -> Result<(usize, usize), FileError> {
        if w.cartridge != files::cartridge_id(&self.original) {
            return Err(FileError::OtherCartridge);
        }
        self.show_added(OnMap::World);
        let mut points = with_specials(&self.original, w.points);
        let mut cleared = 0;
        for p in points.iter_mut() {
            if (*p as usize) >= self.levels.len() && *p != world::NO_LEVEL && *p != 0xFF {
                *p = world::NO_LEVEL;
                cleared += 1;
            }
        }
        let mut doors = w.doors;
        let mut exits = 0;
        for e in &mut doors.exits {
            if matches!(e.1, Leads::NewZone(j) if j >= self.new_zones.len()) {
                e.1 = Leads::World(doors::NONE);
                exits += 1;
            }
        }
        self.world = w.world;
        (self.points, self.routes, self.doors) = (points, w.routes, doors);
        self.shuts = added::shuts_from(&w.shuts);
        self.world_music = w.music;
        self.music_dirty = true;
        self.world_dirty = true;
        self.points_dirty = true;
        self.routes_dirty = true;
        self.doors_dirty = true;
        self.map_history = History::default();
        Ok((cleared, exits))
    }

    /// Delete new level `l`; the ones after it move down a number, and the points of every
    /// map follow (those that opened it open nothing; special values such as the castle
    /// door's $FF stay).
    pub fn remove_level(&mut self, l: usize) -> bool {
        let count = self.levels.len();
        if l < LEVELS || l >= count {
            return false;
        }
        self.levels.remove(l);
        self.extras_dirty = true;
        let follow = |p: &mut u8| match *p as usize {
            v if v == l => *p = world::NO_LEVEL,
            v if v > l && v < count => *p -= 1,
            _ => {}
        };
        self.points.iter_mut().for_each(follow);
        self.added_kept.iter_mut().flatten().for_each(|q| follow(&mut q.level));
        if self.new_zones.iter().any(|z| z.points.iter().any(|q| q.level as usize >= l && (q.level as usize) < count)) {
            self.new_zones.iter_mut().flat_map(|z| z.points.iter_mut()).for_each(|q| follow(&mut q.level));
            self.new_zones_dirty = true;
        }
        self.points_dirty = true;
        true
    }

    fn extras_now(&self) -> Vec<Extra> {
        self.levels[LEVELS..]
            .iter()
            .filter_map(|lv| {
                let mut s = lv.content.stand_in?;
                lv.content.start.write(&mut s.header);
                lv.content.settings.write(&mut s.header);
                Some(Extra { base: s.base, header: s.header, map: lv.content.map.clone(), objs: bells_in_order(&lv.content.objs), name: lv.content.name.clone(), tables: lv.content.tables, enemy_block: None })
            })
            .collect()
    }

    /// Why level `l` cannot get another checkpoint, if it cannot (the game keeps no restart
    /// place for level 31, and a level has at most `checkpoint::MAX`).
    pub fn checkpoint_refused(&self, l: usize) -> Option<CheckpointRefused> {
        if self.base_of(l) >= checkpoint::LEVELS {
            Some(CheckpointRefused::NoRoom)
        } else if checkpoint::bells(&self.levels[l].content.objs).len() >= checkpoint::MAX {
            Some(CheckpointRefused::Full)
        } else {
            None
        }
    }

    /// Add a checkpoint bell at level pixel (`x`, `y`) where the game finds it (the caller
    /// takes the undo snapshot); returns its index.
    pub fn add_bell(&mut self, l: usize, x: u16, y: u16) -> Option<usize> {
        if self.checkpoint_refused(l).is_some() {
            return None;
        }
        let lv = &mut self.levels[l];
        lv.objects_dirty = true;
        Some(checkpoint::insert_bell(&mut lv.content.objs, x, y))
    }

    /// Add an object; returns its index.
    pub fn add_object(&mut self, l: usize, o: Obj) -> usize {
        let lv = &mut self.levels[l];
        lv.content.objs.push(o);
        lv.objects_dirty = true;
        lv.content.objs.len() - 1
    }

    pub fn move_object(&mut self, l: usize, i: usize, x: u16, y: u16) {
        let lv = &mut self.levels[l];
        if let Some(o) = lv.content.objs.get_mut(i) {
            if (x, y) != (o.x, o.y) {
                o.x = x;
                o.y = y;
                lv.objects_dirty = true;
            }
        }
    }

    pub fn remove_object(&mut self, l: usize, i: usize) {
        let lv = &mut self.levels[l];
        if i < lv.content.objs.len() {
            lv.content.objs.remove(i);
            lv.objects_dirty = true;
        }
    }

    /// Give object `i` another type (undoable). False if it already had it.
    pub fn set_object_kind(&mut self, l: usize, i: usize, kind: u8) -> bool {
        if !self.levels[l].content.objs.get(i).map(|o| o.kind != kind).unwrap_or(false) {
            return false;
        }
        self.snapshot(l);
        let lv = &mut self.levels[l];
        lv.content.objs[i].kind = kind;
        lv.objects_dirty = true;
        true
    }

    /// Remember the maps (pictures and points) before a change, for undo.
    pub fn map_snapshot(&mut self) {
        let points = (self.points.clone(), self.routes.clone(), self.doors.clone(), self.new_zones.clone(), self.entries.clone(), self.added_on, self.added_kept.clone(), self.shuts.clone());
        self.map_history.record(((self.world.clone(), self.zones.clone()), points, self.start, (self.world_music, self.zone_music.clone())));
    }

    fn map_swap(&mut self, undo: bool) -> bool {
        let pictures = (std::mem::take(&mut self.world), std::mem::take(&mut self.zones));
        let points = (std::mem::take(&mut self.points), std::mem::take(&mut self.routes), std::mem::take(&mut self.doors), std::mem::take(&mut self.new_zones), std::mem::take(&mut self.entries), self.added_on, std::mem::take(&mut self.added_kept), std::mem::take(&mut self.shuts));
        let mut now = (pictures, points, self.start, (self.world_music, std::mem::take(&mut self.zone_music)));
        let done = if undo { self.map_history.undo(&mut now) } else { self.map_history.redo(&mut now) };
        let (pictures, points, start, music) = now;
        self.start = start;
        (self.world_music, self.zone_music) = music;
        (self.world, self.zones) = pictures;
        (self.points, self.routes, self.doors, self.new_zones, self.entries, self.added_on, self.added_kept, self.shuts) = points;
        let left = if undo { self.map_history.next_redo() } else { self.map_history.next_undo() };
        if let (true, Some(((world, zones), (points, routes, doors, new_zones, entries, added_on, added_kept, shuts), start, music))) = (done, left) {
            self.start_dirty |= *start != self.start;
            self.music_dirty |= music.0 != self.world_music || music.1 != self.zone_music;
            self.world_dirty |= *world != self.world || *zones != self.zones;
            self.points_dirty |= *points != self.points || *shuts != self.shuts;
            self.routes_dirty |= *routes != self.routes || *added_on != self.added_on || *added_kept != self.added_kept;
            self.doors_dirty |= *doors != self.doors || *entries != self.entries;
            self.new_zones_dirty |= *new_zones != self.new_zones;
        }
        done
    }

    pub fn map_undo(&mut self) -> bool {
        self.map_swap(true)
    }

    pub fn map_redo(&mut self) -> bool {
        self.map_swap(false)
    }

    /// Size of a map picture in tiles.
    pub fn map_size(map: OnMap) -> (usize, usize) {
        match map {
            OnMap::World => (world::W, world::H),
            OnMap::Zone(_) | OnMap::NewZone(_) => (zone_maps::W, zone_maps::H),
        }
    }

    fn picture(&self, map: OnMap) -> &Vec<u8> {
        match map {
            OnMap::World => &self.world,
            OnMap::Zone(z) => &self.zones[z],
            OnMap::NewZone(k) => &self.zones[zone_maps::ZONES.len() + k],
        }
    }

    pub fn set_tile(&mut self, map: OnMap, x: usize, y: usize, t: u8) {
        let i = y * Self::map_size(map).0 + x;
        let pic = match map {
            OnMap::World => &mut self.world,
            OnMap::Zone(z) => &mut self.zones[z],
            OnMap::NewZone(k) => &mut self.zones[zone_maps::ZONES.len() + k],
        };
        if pic[i] != t {
            pic[i] = t;
            self.world_dirty = true;
        }
    }

    pub fn tile(&self, map: OnMap, x: usize, y: usize) -> u8 {
        self.picture(map)[y * Self::map_size(map).0 + x]
    }

    pub fn grid_size(&self, g: Grid) -> (usize, usize) {
        match g {
            Grid::Level(_) => (COLS, ROWS),
            Grid::Map(m) => Self::map_size(m),
        }
    }

    fn inside(&self, g: Grid, x: i32, y: i32) -> bool {
        let (w, h) = self.grid_size(g);
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h
    }

    /// The block or tile at `(x, y)`, if it is inside.
    pub fn cell(&self, g: Grid, x: i32, y: i32) -> Option<u8> {
        self.inside(g, x, y).then(|| match g {
            Grid::Level(l) => self.block(l, x, y),
            Grid::Map(m) => self.tile(m, x as usize, y as usize),
        })
    }

    /// Put a block or tile; outside the grid nothing happens. True if something changed.
    pub fn set_cell(&mut self, g: Grid, x: i32, y: i32, v: u8) -> bool {
        if !self.inside(g, x, y) {
            return false;
        }
        match g {
            Grid::Level(l) => self.set_block(l, x, y, v),
            Grid::Map(m) => {
                let changed = self.tile(m, x as usize, y as usize) != v;
                self.set_tile(m, x as usize, y as usize, v);
                changed
            }
        }
    }

    /// Remember the grid before a change, for undo.
    pub fn grid_snapshot(&mut self, g: Grid) {
        match g {
            Grid::Level(l) => self.snapshot(l),
            Grid::Map(_) => self.map_snapshot(),
        }
    }

    /// The block the level uses most, or the tile the map uses most.
    pub fn background(&self, g: Grid) -> u8 {
        let mut count = [0usize; 256];
        match g {
            Grid::Level(l) => self.levels[l].content.map.iter().for_each(|&b| count[b as usize & 0x7F] += 1),
            Grid::Map(m) => self.picture(m).iter().for_each(|&t| count[t as usize] += 1),
        }
        (0..256).max_by_key(|&i| count[i]).unwrap_or(0) as u8
    }

    /// The part of `a` inside the grid, if any.
    pub fn clip(&self, g: Grid, a: Area) -> Option<Area> {
        let (w, h) = self.grid_size(g);
        let c = Area { x0: a.x0.max(0), y0: a.y0.max(0), x1: a.x1.min(w as i32 - 1), y1: a.y1.min(h as i32 - 1) };
        (c.x0 <= c.x1 && c.y0 <= c.y1).then_some(c)
    }

    /// The cells of `a` (the part inside the grid).
    pub fn copy_rect(&self, g: Grid, a: Area) -> Option<Stamp> {
        let a = self.clip(g, a)?;
        let cells = a.cells().map(|(x, y)| self.cell(g, x, y).unwrap_or(0)).collect();
        Some(Stamp { w: a.w(), h: a.h(), cells })
    }

    /// Fill `a` with the background, undoable.
    pub fn clear_rect(&mut self, g: Grid, a: Area) {
        let Some(a) = self.clip(g, a) else { return };
        let bg = self.background(g);
        self.grid_snapshot(g);
        for (x, y) in a.cells() {
            self.set_cell(g, x, y, bg);
        }
    }

    /// Does map point `point` open a level (one of the 32 or a new one)?
    #[cfg(test)]
    pub fn point_level(&self, point: usize) -> Option<usize> {
        let l = self.points[point] as usize;
        (l < self.levels.len()).then_some(l)
    }

    /// What the original game does with point `p` besides its level, paths and doors.
    pub fn point_notes(&self, p: u8) -> &[Note] {
        self.original_notes.get(p as usize).map_or(&[], |n| n.as_slice())
    }

    /// Is game point `p`'s level, paths or place other than the original's?
    pub fn point_changed(&self, p: u8) -> bool {
        let i = p as usize;
        i < map_routes::POINTS && (self.points[i] != world::point_levels(&self.original)[i] || self.routes.ways[i] != self.original_routes.ways[i] || self.routes.places[i] != self.original_routes.places[i])
    }

    /// Step the level a game map point opens: no level, 0, 1, ... the last level (the new
    /// ones too), no level (undoable). Points the game uses for something else ($FF...) are
    /// left alone. A zone entrance's twin (`map_routes::twin`) opening the same level changes
    /// with it, as the game keeps the pair as one place.
    pub fn step_point_level(&mut self, point: usize, step: i32) {
        let cur = self.points[point];
        let count = self.levels.len() as i32;
        if step == 0 || !((cur as i32) < count || cur == world::NO_LEVEL) {
            return;
        }
        self.map_snapshot();
        let i = if cur == world::NO_LEVEL { count } else { cur as i32 };
        let n = (i + step).rem_euclid(count + 1);
        self.points[point] = if n == count { world::NO_LEVEL } else { n as u8 };
        if let Some(t) = map_routes::twin(point as u8).map(usize::from).filter(|&t| self.points.get(t) == Some(&cur)) {
            self.points[t] = self.points[point];
        }
        self.points_dirty = true;
    }

    /// Make world point `p` enter zone `zone` (`doors::INTO` index) or nothing, undoable; a
    /// door into a new zone there is taken out.
    pub fn set_door(&mut self, p: u8, zone: Option<usize>) {
        if self.doors.into.is_empty() || self.doors.door_of(p) == zone {
            return;
        }
        self.map_snapshot();
        self.doors.set_door(p, zone);
        self.doors_dirty = true;
        if zone.is_some() {
            for z in self.new_zones.iter_mut().filter(|z| z.door == p) {
                z.door = doors::NONE;
                self.new_zones_dirty = true;
            }
        }
    }

    /// Set where exit `i` (`doors::EXITS` index) is and leads, undoable.
    pub fn set_exit(&mut self, i: usize, at: u8, leads: Leads) {
        if self.doors.exits.get(i) == Some(&(at, leads)) || i >= self.doors.exits.len() {
            return;
        }
        self.map_snapshot();
        self.doors.exits[i] = (at, leads);
        self.doors_dirty = true;
    }

    /// Set or take out the way from `point` in direction `dir` (0 right, 1 left, 2 up,
    /// 3 down), undoable. A zone's entrance and the point Mario leaves the zone on change
    /// together.
    pub fn set_way(&mut self, point: usize, dir: usize, way: Option<Way>) {
        if point >= map_routes::ALL || self.routes.ways[point][dir] == way {
            return;
        }
        self.map_snapshot();
        self.put_way(point, dir, way);
    }

    fn put_way(&mut self, point: usize, dir: usize, way: Option<Way>) {
        if let Some(t) = map_routes::twin(point as u8) {
            self.routes.ways[t as usize][dir] = way.clone();
        }
        self.routes.ways[point][dir] = way;
        self.routes_dirty = true;
    }

    /// A path drawn from `from` through `corners` (pixels from `from`) to point `to`: the
    /// way out of `from` in the direction it starts in, and the way back the same road if
    /// `to` has none in the direction it would start in, or has one along the same road
    /// (`from` stands on it, so Mario stops there now). Returns the directions used.
    pub fn draw_path(&mut self, from: usize, to: u8, corners: &[(i32, i32)]) -> Option<(usize, Option<usize>)> {
        let way = Way::through(corners, to);
        let dir = first_dir(&way)?;
        self.map_snapshot();
        let mut back_dir = None;
        if let Some((back, bd)) = way_back(&way, corners, from as u8) {
            let t = to as usize;
            if t < map_routes::ALL && self.routes.ways[t][bd].as_ref().is_none_or(|old| on_road(&back, old)) {
                self.put_way(t, bd, Some(back));
                back_dir = Some(bd);
            }
        }
        self.put_way(from, dir, Some(way));
        Some((dir, back_dir))
    }

    /// Take out a new point and every path to it.
    pub fn remove_point(&mut self, p: u8) -> bool {
        if !self.routes.added.contains(&p) {
            return false;
        }
        self.map_snapshot();
        self.routes.remove_point(p);
        self.routes.places[p as usize] = map_routes::read(&self.original).places[p as usize];
        self.points[p as usize] = world::NO_LEVEL;
        self.routes_dirty = true;
        self.points_dirty = true;
        true
    }

    /// The edited maps of one bank: (map number, blocks), one per map.
    pub fn bank_edits(&self, bank: u8) -> Vec<(u8, &[u8])> {
        let mut v: Vec<(u8, &[u8])> = Vec::new();
        for l in 0..LEVELS {
            let h = levels::header(&self.rom, l);
            if h.bank == bank && self.levels[l].blocks_dirty && !v.iter().any(|e| e.0 == h.map) {
                v.push((h.map, self.levels[l].content.map.as_slice()));
            }
        }
        v
    }

    /// Bytes level `l`'s zone needs and has (a new level: its own map and the room it gets).
    pub fn zone_space(&self, l: usize) -> (usize, usize) {
        if l >= LEVELS {
            return (levels::encode(&self.levels[l].content.map).len(), extra_levels::MAP_ROOM);
        }
        let bank = levels::header(&self.rom, l).bank;
        layout::bank_space(&self.rom, &self.original, bank, &self.bank_edits(bank))
    }

    fn object_lists(&self) -> Vec<Vec<Obj>> {
        self.levels[..LEVELS].iter().map(|l| if l.objects_dirty { bells_in_order(&l.content.objs) } else { l.content.objs.clone() }).collect()
    }

    /// Bytes the object lists need and have: all those of the 32 levels together, or a new
    /// level's own (`l`).
    pub fn object_space(&self, l: usize) -> (usize, usize) {
        if l >= LEVELS {
            return (self.levels[l].content.objs.len() * 3 + 1, extra_levels::OBJECTS_ROOM);
        }
        objects::space(&self.rom, &self.original, &self.object_lists())
    }

    /// Write every change into the cartridge image (`rom`). Zones already written stay
    /// written if a later one doesn't fit.
    pub fn commit(&mut self) -> Result<Vec<Saved>, CartError> {
        if !self.start_ok() {
            return Err(CartError::StartMissing);
        }
        if !start_point::write(&mut self.rom, self.start) {
            return Err(CartError::UnknownVersion);
        }
        self.start_dirty = false;
        let mut notes = Vec::new();
        let pictures_changed = self.world_dirty;
        let mut banks: Vec<u8> = (0..LEVELS).filter(|&l| self.levels[l].blocks_dirty).map(|l| levels::header(&self.rom, l).bank).collect();
        banks.sort();
        banks.dedup();
        for bank in banks {
            let edits: Vec<(u8, Vec<u8>)> = self.bank_edits(bank).into_iter().map(|(m, d)| (m, d.to_vec())).collect();
            let refs: Vec<(u8, &[u8])> = edits.iter().map(|(m, d)| (*m, d.as_slice())).collect();
            let mut rom = self.rom.clone();
            notes.push(Saved::Bank(layout::store_bank(&mut rom, &self.original, bank, &refs)?));
            self.rom = rom;
            for l in 0..LEVELS {
                let h = levels::header(&self.rom, l);
                if h.bank != bank {
                    continue;
                }
                // Levels that share an edited map see the change too.
                if let Some((_, d)) = edits.iter().find(|e| e.0 == h.map) {
                    self.levels[l].content.map = d.clone();
                }
                self.levels[l].blocks_dirty = false;
            }
        }
        let mut moved = 0;
        for l in 0..LEVELS {
            let at = levels::header(&self.rom, l).at;
            let c = &self.levels[l].content;
            if Start::read(&self.rom[at..at + 8]) != c.start || Settings::read(&self.rom[at..at + 20]) != c.settings || Tables::read(&self.rom, l) != c.tables {
                c.start.write(&mut self.rom[at..at + 8]);
                c.settings.write(&mut self.rom[at..at + 20]);
                let tables = c.tables;
                tables.write(&mut self.rom, l);
                moved += 1;
            }
            self.levels[l].start_dirty = false;
        }
        if moved > 0 {
            notes.push(Saved::Starts(moved));
        }
        if self.music_dirty {
            map_music::write(&mut self.rom, Map::World, self.world_music);
            for (z, &m) in self.zone_music.iter().enumerate() {
                map_music::write(&mut self.rom, Map::Zone(z), m);
            }
            self.music_dirty = false;
        }
        if self.world_dirty {
            world::write(&mut self.rom, &self.world);
            for (z, pic) in self.zones.iter().enumerate().take(zone_maps::ZONES.len()) {
                zone_maps::write(&mut self.rom, z, pic);
            }
            self.world_dirty = false;
            notes.push(Saved::WorldMap);
        }
        // the world map's added points in place, as the copy keeps them
        let shown = self.added_on();
        self.show_added(OnMap::World);
        let saved = self.commit_points(&mut notes, pictures_changed);
        self.show_added(shown);
        saved?;
        self.mods = self.saved_mods();
        Ok(notes)
    }

    /// Every level's own enemy set into the copy (`enemy_sets`); nothing when no level has one
    /// and the copy never had.
    fn commit_enemy_sets(&mut self) -> Result<(), CartError> {
        let sets: Vec<(usize, Vec<Enemy>, Vec<u8>)> = self.levels.iter().enumerate().filter_map(|(l, lv)| lv.content.enemies.as_ref().map(|s| (l, s.enemies.clone(), s.block.clone()))).collect();
        if sets.is_empty() && enemy_sets::read(&self.rom).is_empty() {
            return Ok(());
        }
        let mut rom = self.rom.clone();
        enemy_sets::store(&mut rom, &self.original, &sets)?;
        self.rom = rom;
        Ok(())
    }

    /// The restart places of levels whose bells moved (as stored), the game's own for the
    /// others; the several-bells hook when a level has more than one bell. New levels carry
    /// theirs when played (`extra_levels::writes_for`).
    fn commit_checkpoints(&mut self) -> Result<(), CartError> {
        let several = self.levels.iter().enumerate().any(|(l, lv)| (l < checkpoint::LEVELS || l >= LEVELS) && checkpoint::bells(&lv.content.objs).len() > 1);
        if several && !checkpoint::installed(&self.rom) && !checkpoint::install(&mut self.rom) {
            return Err(CartError::UnknownVersion);
        }
        for l in 0..checkpoint::LEVELS {
            let bells = |rom: &[u8]| checkpoint::bell_places(&objects::read(rom, l).0);
            if bells(&self.rom) != bells(&self.original) {
                let c = &self.levels[l].content;
                extra_levels::apply(&mut self.rom, &checkpoint::writes(l, &c.objs, &c.map));
            } else if let Some(own) = checkpoint::entry(&self.original, l).map(<[u8]>::to_vec) {
                checkpoint::set_bell_entry(&mut self.rom, l, 0, &own);
            }
        }
        Ok(())
    }

    fn commit_points(&mut self, notes: &mut Vec<Saved>, pictures_changed: bool) -> Result<(), CartError> {
        if self.uses_added() && !added_points::installed(&self.rom) {
            if !added_points::install(&mut self.rom) {
                return Err(CartError::UnknownVersion);
            }
            self.points_dirty = true;
            self.routes_dirty = true;
        }
        if added_points::installed(&self.rom) {
            let mut rom = self.rom.clone();
            added_points::store_zones(&mut rom, &self.zones_added())?;
            self.rom = rom;
        }
        if self.points_dirty {
            world::write_point_levels(&mut self.rom, &self.points);
            if added_points::installed(&self.rom) {
                added_points::write(&mut self.rom, &self.points[added_points::FIRST as usize..], &self.shuts[added_points::FIRST as usize..], added_points::WORLD_REP);
            }
            self.points_dirty = false;
            notes.push(Saved::LevelPoints);
        }
        let stored_zones: Vec<new_zones::NewZone> = (0..self.new_zones.len()).map(|k| self.new_zone(k)).collect();
        if self.doors_dirty {
            let mut d = self.doors.clone();
            for e in &mut d.exits {
                e.1 = new_zones::resolve(e.1, &stored_zones);
            }
            if doors::write(&mut self.rom, &d) {
                notes.push(Saved::Doors);
            }
            for (i, &p) in self.entries.iter().enumerate() {
                if p != doors::NONE && new_zones::entry_point(&self.rom, doors::INTO[i].zone) != Some(p) {
                    new_zones::set_entry(&mut self.rom, doors::INTO[i].zone, p, self.routes.places[p as usize])?;
                }
            }
            self.doors_dirty = false;
        }
        if self.routes_dirty {
            let mut rom = self.rom.clone();
            notes.push(Saved::Routes(map_routes::write(&mut rom, &self.routes)?));
            self.rom = rom;
            self.routes_dirty = false;
        }
        if self.levels[..LEVELS].iter().any(|l| l.objects_dirty) {
            let mut rom = self.rom.clone();
            notes.push(Saved::Objects(objects::store(&mut rom, &self.original, &self.object_lists())?));
            self.rom = rom;
            for l in 0..LEVELS {
                // what the game will read: page by page, as stored
                self.levels[l].content.objs = objects::read(&self.rom, l).0;
                self.levels[l].objects_dirty = false;
            }
        }
        if self.extras_dirty || self.levels[LEVELS..].iter().any(|l| l.dirty()) {
            let mut rom = self.rom.clone();
            let used = extra_levels::store(&mut rom, &self.extras_now())?;
            self.rom = rom;
            for (k, e) in extra_levels::read(&self.rom).into_iter().enumerate() {
                let lv = &mut self.levels[LEVELS + k];
                lv.content.objs = e.objs; // stored left to right, as the game keeps them
                lv.blocks_dirty = false;
                lv.objects_dirty = false;
                lv.start_dirty = false;
            }
            self.extras_dirty = false;
            notes.push(Saved::NewLevels(self.new_levels(), used));
        }
        self.commit_checkpoints()?;
        self.commit_enemy_sets()?;
        let targets: Vec<(usize, SecretTo)> = self.levels.iter().enumerate().filter_map(|(l, lv)| Some((l, lv.content.secret?))).collect();
        let before = self.rom.clone();
        secret_exits::write(&mut self.rom, &targets, &stored_zones.iter().map(|z| z.door).collect::<Vec<u8>>())?;
        if self.rom != before {
            notes.push(Saved::SecretExits(targets.len()));
        }
        for lv in &mut self.levels {
            lv.secret_dirty = false;
        }
        if self.new_zones_dirty || (pictures_changed && !stored_zones.is_empty()) {
            let mut rom = self.rom.clone();
            let used = new_zones::store(&mut rom, &stored_zones)?;
            self.rom = rom;
            self.new_zones_dirty = false;
            notes.push(Saved::NewZones(stored_zones.len(), used));
        }
        Ok(())
    }
}

/// The way back from the end of `way` (drawn from `from` through `corners`) the same road,
/// and the direction it starts in.
fn way_back(way: &Way, corners: &[(i32, i32)], from: u8) -> Option<(Way, usize)> {
    let end = *way.track().last()?;
    let back_corners: Vec<(i32, i32)> = corners.iter().rev().skip(1).map(|c| (c.0 - end.0, c.1 - end.1)).chain(std::iter::once((-end.0, -end.1))).collect();
    let back = Way::through(&back_corners, from);
    let d = first_dir(&back)?;
    Some((back, d))
}

/// Does `way` go along `road` (both from the same point), every step of it on the road?
fn on_road(way: &Way, road: &Way) -> bool {
    let road = road.track();
    let on = |p: (i32, i32)| {
        road.windows(2).any(|s| {
            let ((ax, ay), (bx, by)) = (s[0], s[1]);
            (bx - ax) * (p.1 - ay) == (by - ay) * (p.0 - ax) && p.0 >= ax.min(bx) && p.0 <= ax.max(bx) && p.1 >= ay.min(by) && p.1 <= ay.max(by)
        })
    };
    way.track().into_iter().all(on)
}

/// The pad direction a way starts in (0 right, 1 left, 2 up, 3 down): its first step's;
/// a diagonal one goes by where the way ends up most.
fn first_dir(w: &Way) -> Option<usize> {
    use map_routes::{DOWN, LEFT, RIGHT, UP};
    let s = *w.steps.first()?;
    let end = *w.track().last()?;
    let h = if s & RIGHT != 0 { Some(0) } else if s & LEFT != 0 { Some(1) } else { None };
    let v = if s & UP != 0 { Some(2) } else if s & DOWN != 0 { Some(3) } else { None };
    match (h, v) {
        (Some(h), Some(v)) => Some(if end.0.abs() >= end.1.abs() { h } else { v }),
        (h, v) => h.or(v),
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
    fn a_world_map_moves_to_another_project() {
        let Some(mut a) = open() else { return };
        let Some(mut b) = open() else { return };
        a.world[0] ^= 1;
        let w = a.world_file();
        assert_eq!(b.replace_world(w.clone()), Ok((0, 0)));
        assert_eq!(b.world, a.world);
        assert!(b.world_dirty);
        // a point opening a new level the project does not have opens none
        let mut far = w;
        far.points[0] = 40;
        assert_eq!(b.replace_world(far.clone()), Ok((1, 0)));
        assert_eq!(b.points[0], world::NO_LEVEL);
        far.cartridge ^= 1;
        assert_eq!(b.replace_world(far), Err(FileError::OtherCartridge));
    }

    #[test]
    fn copy_cut_and_undo_on_levels_and_maps() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        for g in [Grid::Level(3), Grid::Map(OnMap::World), Grid::Map(OnMap::Zone(1))] {
            let a = Area::between((2, 3), (5, 4));
            let before = doc.copy_rect(g, a).unwrap();
            assert_eq!((before.w, before.h, before.at(1, 0)), (4, 2, doc.cell(g, 3, 3).unwrap()));
            let bg = doc.background(g);
            doc.clear_rect(g, a);
            assert!(a.cells().all(|(x, y)| doc.cell(g, x, y) == Some(bg)));
            match g {
                Grid::Level(l) => assert!(doc.undo(l)),
                Grid::Map(_) => assert!(doc.map_undo()),
            }
            assert_eq!(doc.copy_rect(g, a), Some(before));
            let (w, h) = doc.grid_size(g);
            let edge = doc.copy_rect(g, Area::between((w as i32 - 2, h as i32 - 1), (w as i32 + 3, h as i32 + 3))).unwrap();
            assert_eq!((edge.w, edge.h), (2, 1), "clipped to the grid");
            assert_eq!(doc.copy_rect(g, Area::between((-5, -5), (-1, -1))), None);
            assert!(!doc.set_cell(g, -1, 0, 1) && doc.cell(g, w as i32, 0).is_none());
        }
    }

    #[test]
    fn paint_undo_redo_and_save_into_the_cartridge() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let before = doc.levels[3].content.clone();
        let old = doc.block(3, 10, 5);
        let new = (old + 1) & 0x7F;
        doc.snapshot(3);
        assert!(doc.set_block(3, 10, 5, new));
        assert!(!doc.set_block(3, 10, 5, new), "same block twice is no change");
        assert!(doc.levels[3].dirty());
        assert!(doc.undo(3));
        assert_eq!(doc.levels[3].content, before);
        assert!(doc.redo(3));
        assert_eq!(doc.block(3, 10, 5), new);

        let i = doc.add_object(3, Obj { x: 64, y: 32, kind: 5, flag: false });
        doc.move_object(3, i, 72, 40);
        doc.set_tile(OnMap::World, 1, 1, doc.tile(OnMap::World, 1, 1) ^ 1);
        let tile = doc.tile(OnMap::World, 1, 1);
        let zone = OnMap::Zone(2);
        doc.map_snapshot();
        doc.set_tile(zone, 19, 17, doc.tile(zone, 19, 17) ^ 1);
        let zone_tile = doc.tile(zone, 19, 17);
        let parts = doc.commit().unwrap_or_else(|e| panic!("{e}"));
        assert!(matches!(parts[0], Saved::Bank(_)));
        assert!(parts.iter().any(|p| matches!(p, Saved::WorldMap)));
        assert!(parts.iter().any(|p| matches!(p, Saved::Objects(_))));
        assert!(!doc.levels.iter().any(|l| l.dirty()) && !doc.world_dirty);

        // Read back from the cartridge image, as the game will.
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.block(3, 10, 5), new);
        assert!(again.levels[3].content.objs.iter().any(|o| (o.x, o.y, o.kind) == (72, 40, 5)));
        assert_eq!(again.tile(OnMap::World, 1, 1), tile);
        assert_eq!(again.tile(zone, 19, 17), zone_tile);
        assert_eq!(again.zones[0], doc.zones[0], "other zone maps untouched");
        assert_eq!(again.levels[4].content, doc.levels[4].content, "other levels untouched");
    }

    #[test]
    fn restoring_brings_back_the_original_and_can_be_undone() {
        let Some(mut doc) = open() else { return };
        let original = doc.levels[0].content.clone();
        doc.snapshot(0);
        doc.remove_object(0, 0);
        let edited = doc.levels[0].content.clone();
        doc.restore_level(0);
        assert_eq!(doc.levels[0].content, original);
        assert!(doc.undo(0));
        assert_eq!(doc.levels[0].content, edited);

        doc.step_point_level(0, 1);
        assert!(doc.points_dirty);
        let zone = OnMap::Zone(5);
        let before = doc.tile(zone, 3, 3);
        doc.map_snapshot();
        doc.set_tile(zone, 3, 3, before ^ 1);
        doc.map_undo();
        assert_eq!(doc.tile(zone, 3, 3), before, "zone maps are undone with the world map");
        doc.map_redo();
        let world_tile = doc.tile(OnMap::World, 5, 5);
        doc.map_snapshot();
        doc.set_tile(OnMap::World, 5, 5, world_tile ^ 1);
        assert!(doc.restore_map(zone));
        assert_eq!(doc.tile(zone, 3, 3), before);
        assert_eq!(doc.points[0], world::point_levels(&doc.original)[0] + 1, "a zone's restore leaves the world map's points");
        assert_eq!(doc.tile(OnMap::World, 5, 5), world_tile ^ 1, "and its picture");
        assert!(doc.restore_map(OnMap::World));
        assert_eq!(doc.points, world::point_levels(&doc.original));
        assert_eq!(doc.tile(OnMap::World, 5, 5), world_tile);
    }

    #[test]
    fn point_levels_undo() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let before = doc.points.clone();
        doc.step_point_level(0, 2);
        assert_eq!(doc.points[0], before[0] + 2);
        assert!(doc.map_undo());
        assert_eq!(doc.points, before);
        assert!(doc.map_redo() && doc.points[0] == before[0] + 2);
    }

    /// Gravity on a game level and on a new one: undone, saved into the copy (the game level's
    /// tables, the new level's store), kept when the copy is opened again, filed, restored.
    #[test]
    fn level_tables_are_saved_filed_and_restored() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let own = doc.level_tables(1);
        let mut space = own;
        space.set_space_gravity(true, &doc.game_tables(1));
        doc.set_level_tables(1, space);
        assert!(doc.undo(1) && doc.level_tables(1) == own);
        assert!(doc.redo(1));
        let n = doc.add_blank_level(1).unwrap();
        doc.set_level_tables(n, space);
        assert!(doc.levels[n].content.objs.is_empty() && !doc.levels[n].content.start.blocked(&doc.levels[n].content.map), "a blank level, Mario free");
        doc.commit().unwrap();
        assert_eq!(Tables::read(&doc.rom, 1), space);
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.level_tables(1), again.level_tables(n)), (space, space));
        assert!(again.mods.level(1), "the game level differs from the game's");
        let filed = doc.import_level(files::read_level(&files::level_text(&doc.level_file(n), "")).unwrap().0, "").unwrap();
        assert_eq!(doc.level_tables(filed), space);
        doc.restore_level(1);
        assert_eq!(doc.level_tables(1), own);
    }

    /// An enemy set on a game level and on a new one: undone, saved (blocks, directory, the
    /// game level's entry), kept when the copy is opened again, filed, restored.
    #[test]
    fn enemy_sets_are_saved_filed_and_restored() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let set = EnemySet { enemies: vec![Enemy { level: 0, kind: 0x01 }, Enemy { level: 1, kind: 0x21 }], block: vec![0x5A; enemy_sets::BLOCK] };
        doc.set_level_enemies(1, Some(set.clone()));
        assert!(doc.undo(1) && doc.level_enemies(1).is_none());
        assert!(doc.redo(1));
        let n = doc.add_blank_level(3).unwrap();
        doc.set_level_enemies(n, Some(set.clone()));
        doc.commit().unwrap();
        assert_ne!(enemy_sets::own_block(&doc.rom, 1), enemy_sets::own_block(&doc.original, 1), "level 01 loads its own block");
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.level_enemies(1), again.level_enemies(n)), (Some(&set), Some(&set)));
        assert!(again.mods.level(1));
        let filed = doc.import_level(files::read_level(&files::level_text(&doc.level_file(n), "")).unwrap().0, "").unwrap();
        assert_eq!(doc.level_enemies(filed), Some(&set));
        doc.restore_level(1);
        assert!(doc.level_enemies(1).is_none());
        doc.commit().unwrap();
        assert_eq!(enemy_sets::own_block(&doc.rom, 1), enemy_sets::own_block(&doc.original, 1), "the game's enemies again");
    }

    /// The original game's special points, and a zone entrance's twin that changes with it.
    #[test]
    fn the_original_notes_and_the_twins_level() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        assert_eq!(doc.point_notes(0x13), [Note::UnusedDoor]);
        assert_eq!(doc.point_notes(0x05), [Note::FlightOut, Note::Entrance { zone: 3, twin: 0x1A }]);
        assert_eq!(doc.point_notes(0x1A), [Note::FlightOut, Note::ZoneReturn { zone: 3, twin: 0x05 }]);
        assert_eq!(doc.point_notes(0x3C), [Note::FlightIn]);
        for p in [0x0E, 0x10, 0x3E] {
            assert_eq!(doc.point_notes(p), [Note::NeverStood], "{p:02x}");
        }
        assert!(doc.point_notes(0x01).is_empty() && !doc.point_changed(0x05));
        doc.step_point_level(0x05, 1);
        assert_eq!((doc.points[0x05], doc.points[0x1A]), (18, 18), "Hippo's twin goes along");
        assert!(doc.point_changed(0x05) && doc.point_changed(0x1A));
    }

    /// Pictures and points share one history: undo goes back in the order the changes
    /// were made, whatever the tab.
    #[test]
    fn map_steps_undo_in_the_order_made() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let (tile, level) = (doc.tile(OnMap::World, 3, 3), doc.points[0]);
        doc.map_snapshot();
        doc.set_tile(OnMap::World, 3, 3, tile ^ 1);
        doc.step_point_level(0, 1);
        doc.map_snapshot();
        doc.set_tile(OnMap::World, 3, 3, tile ^ 2);
        assert!(doc.map_undo());
        assert_eq!((doc.tile(OnMap::World, 3, 3), doc.points[0] != level), (tile ^ 1, true), "the last tile first");
        assert!(doc.map_undo());
        assert_eq!((doc.tile(OnMap::World, 3, 3), doc.points[0]), (tile ^ 1, level), "then the point");
        assert!(doc.map_undo());
        assert_eq!(doc.tile(OnMap::World, 3, 3), tile, "then the first tile");
        assert!(doc.map_redo() && doc.map_redo() && doc.map_redo());
        assert_eq!(doc.tile(OnMap::World, 3, 3), tile ^ 2);
    }

    #[test]
    fn new_levels_are_added_saved_and_removed() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(5, 10, 5, 3);
        let n = doc.add_level(5).unwrap();
        assert_eq!((n, doc.base_of(n)), (LEVELS, 5));
        assert_eq!(doc.block(n, 10, 5), 3, "a copy of the level as it is being edited");
        let m = doc.add_level(n).unwrap();
        assert_eq!(doc.base_of(m), 5, "a copy of a new level has the same base");
        doc.set_block(m, 20, 5, 4);
        doc.add_object(m, Obj { x: 80, y: 40, kind: 5, flag: false });
        doc.points[0] = m as u8; // the game's first point opens the second new level
        doc.points[1] = n as u8;
        doc.points_dirty = true;
        doc.commit().unwrap();
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.points[0], again.point_level(1)), (m as u8, Some(n)));
        assert_eq!(again.levels.len(), LEVELS + 2);
        assert_eq!(again.levels[m].content, doc.levels[m].content);
        assert_eq!(again.base_of(m), 5);
        assert!(doc.remove_level(n) && !doc.remove_level(3));
        assert_eq!(doc.points[0], n as u8, "the point follows its level");
        assert_eq!(doc.points[0x5C], 0xFF, "the castle door's special value stays");
        assert_eq!(doc.points[1], world::NO_LEVEL, "a point to a deleted level opens nothing");
        assert_eq!(doc.levels[n].content, again.levels[m].content);
    }

    #[test]
    fn a_new_levels_stand_in_is_undone_with_it() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let n = doc.add_level(2).unwrap();
        let before = doc.levels[n].content.clone();
        let other = doc.levels[20].content.clone();
        doc.snapshot(n);
        doc.levels[n].content = LevelContent { stand_in: Some(StandIn { base: 20, header: [7; 20] }), ..other };
        assert_eq!(doc.base_of(n), 20);
        assert!(doc.undo(n));
        assert_eq!((doc.base_of(n), &doc.levels[n].content), (2, &before));
        assert!(doc.redo(n));
        assert_eq!(doc.levels[n].content.stand_in.map(|s| s.header), Some([7; 20]));
        doc.restore_level(n);
        assert_eq!(doc.base_of(n), 20, "restoring a new level keeps what it plays as");
    }

    #[test]
    fn levels_go_out_to_files_and_come_back() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(9, 4, 4, 0x11);
        let f = files::read_level(&files::level_text(&doc.level_file(9), "")).unwrap().0;
        assert_eq!(f.stand_in.base, 9);
        let n = doc.import_level(f.clone(), "").unwrap();
        assert_eq!((doc.base_of(n), doc.block(n, 4, 4)), (9, 0x11));
        assert_eq!(doc.levels[n].content.objs, doc.levels[9].content.objs);
        assert!(!doc.replace_level(9, f.clone(), ""), "the 32 levels are never replaced");
        let m = doc.add_level(2).unwrap();
        let before = doc.levels[m].content.clone();
        assert!(doc.replace_level(m, f, ""));
        assert_eq!((doc.base_of(m), doc.block(m, 4, 4)), (9, 0x11));
        assert!(doc.undo(m));
        assert_eq!(doc.levels[m].content, before);
        while doc.add_level(0).is_some() {}
        assert_eq!(doc.new_levels(), extra_levels::MAX);
        assert_eq!(doc.import_level(doc.level_file(0), ""), None, "no room");
    }

    #[test]
    fn an_imported_level_restores_as_its_file_had_it() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(5, 4, 4, 0x11);
        let a = doc.level_file(5);
        let copy = doc.add_level(2).unwrap();
        let n = doc.import_level(a.clone(), "Demo|level|05|").unwrap();
        doc.set_block(n, 4, 4, 0x22);
        doc.restore_level(n);
        assert_eq!((doc.base_of(n), doc.block(n, 4, 4), doc.level_restore_origin(n)), (5, 0x11, Some("Demo|level|05|")));
        assert_eq!(doc.level_file(n), a, "as the file had it, not as base level 05");
        doc.set_block(20, 4, 4, 0x33);
        assert!(doc.replace_level(n, doc.level_file(20), "B"));
        assert!(doc.undo(n));
        assert_eq!(doc.level_restore_origin(n), Some("Demo|level|05|"), "undo brings the earlier restore point back");
        assert!(doc.remove_level(copy));
        let n = n - 1;
        doc.set_block(n, 4, 4, 0x44);
        doc.restore_level(n);
        assert_eq!(doc.block(n, 4, 4), 0x11, "it follows the level when an earlier one is removed");
        let kept = doc.restore_points();
        assert_eq!(kept.levels.iter().map(|p| p.0).collect::<Vec<_>>(), vec![n]);
        let mut again = open().unwrap();
        let mut odd = kept.clone();
        odd.levels.push((7, odd.levels[0].1.clone()));
        odd.zones.push((0, RestorePoint { file: again.zone_file(OnMap::Zone(0)).unwrap(), origin: String::new() }));
        again.attach_restore_points(odd);
        assert_eq!(again.restore_points(), RestorePoints::default(), "no new level or zone to attach to, and 7 is a game level");
        again.add_level(2);
        again.attach_restore_points(kept.clone());
        assert_eq!(again.restore_points(), kept, "attached by number when the level is there");
        let c = doc.add_level(2).unwrap();
        assert_eq!(doc.level_restore_origin(c), None, "a copied level has none");
        doc.restore_level(c);
        assert_eq!(doc.base_of(c), 2);
    }

    #[test]
    fn a_level_start_is_moved_undone_saved_filed_and_restored() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let first = doc.level_start(3);
        let moved = Start::standing(0x800, 0x1F0);
        doc.snapshot(3);
        doc.set_level_start(3, moved);
        assert!(doc.levels[3].dirty());
        assert!(doc.undo(3));
        assert_eq!(doc.level_start(3), first);
        assert!(doc.redo(3));
        assert_eq!(doc.level_start(3), moved);
        let n = doc.add_level(3).unwrap();
        let other = Start::standing(0x300, 0x100);
        doc.set_level_start(n, other);
        let filed = doc.import_level(files::read_level(&files::level_text(&doc.level_file(3), "")).unwrap().0, "").unwrap();
        assert_eq!(doc.level_start(filed), moved);
        doc.commit().unwrap();
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.level_start(3), again.level_start(n), again.level_start(filed)), (moved, other, moved));
        let at = levels::header(&doc.rom, 3).at;
        assert_eq!(Start::read(&doc.rom[at..at + 8]), moved);
        let text = files::project_text(&doc.project_file());
        let mut fresh = open().unwrap();
        fresh.replace_project(files::read_project(&text).unwrap()).unwrap();
        assert_eq!((fresh.level_start(3), fresh.level_start(n)), (moved, other));
        doc.restore_level(3);
        doc.restore_level(n);
        assert_eq!((doc.level_start(3), doc.level_start(n)), (first, first));
    }

    /// Settings and checkpoints: undone, saved (restart entries, the hook only with several
    /// bells), filed in level and project files, restored.
    #[test]
    fn settings_and_checkpoints_are_saved_filed_and_restored() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let own = doc.level_settings(1);
        let changed = Settings { time: 2, music: 0x28, palette: 0x93 };
        doc.set_level_settings(1, changed);
        assert!(doc.undo(1));
        assert_eq!(doc.level_settings(1), own);
        assert!(doc.redo(1));
        doc.commit().unwrap();
        assert!(!checkpoint::installed(&doc.rom), "one bell per level: no hook");
        let at = levels::header(&doc.rom, 1).at;
        assert_eq!(Settings::read(&doc.rom[at..at + 20]), changed);
        doc.snapshot(1);
        doc.add_bell(1, 2600, 464).unwrap();
        doc.commit().unwrap();
        assert_eq!(checkpoint::bells(&doc.levels[1].content.objs).len(), 2, "the game's bell and the new one");
        assert!(checkpoint::installed(&doc.rom), "several bells: the hook");
        assert_ne!(checkpoint::bell_entry(&doc.rom, 1, 1), [0; 8], "the second bell's restart");
        let filed = doc.import_level(files::read_level(&files::level_text(&doc.level_file(1), "")).unwrap().0, "").unwrap();
        assert_eq!(doc.level_settings(filed), changed);
        assert_eq!(checkpoint::bell_places(&doc.levels[filed].content.objs), checkpoint::bell_places(&doc.levels[1].content.objs));
        let text = files::project_text(&doc.project_file());
        let mut fresh = open().unwrap();
        fresh.replace_project(files::read_project(&text).unwrap()).unwrap();
        assert_eq!(fresh.level_settings(1), changed);
        doc.restore_level(1);
        assert_eq!(doc.level_settings(1), own);
        assert_eq!(doc.checkpoint_refused(31), Some(CheckpointRefused::NoRoom));
    }

    #[test]
    fn a_project_goes_out_to_a_file_and_comes_back() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        doc.set_block(1, 6, 6, 0x22);
        let n = doc.add_level(4).unwrap();
        doc.set_tile(OnMap::Zone(3), 2, 2, 0x44);
        doc.step_point_level(0, 1);
        let text = files::project_text(&doc.project_file());
        let mut other = open().unwrap();
        other.snapshot(1);
        other.replace_project(files::read_project(&text).unwrap()).unwrap();
        assert_eq!(other.levels.len(), n + 1);
        assert_eq!((other.block(1, 6, 6), other.base_of(n), other.tile(OnMap::Zone(3), 2, 2)), (0x22, 4, 0x44));
        assert_eq!((&other.points, &other.routes, &other.doors), (&doc.points, &doc.routes, &doc.doors));
        assert!(other.levels.iter().all(|l| l.dirty()) && other.world_dirty && other.points_dirty);
        assert!(!other.undo(1), "no undo across a project import");
        other.commit().unwrap();
        let again = EditorDocument::open(other.original.clone(), Some(other.rom.clone()));
        assert_eq!((again.block(1, 6, 6), again.base_of(n)), (0x22, 4));
        let mut p = doc.project_file();
        p.cartridge ^= 1;
        assert_eq!(other.replace_project(p), Err(FileError::OtherCartridge));
    }

    #[test]
    fn maps_music_is_changed_undone_restored_saved_and_filed() {
        let Some(mut doc) = open() else { return };
        let pumpkin = OnMap::Zone(1);
        assert_eq!((doc.map_music(OnMap::World), doc.map_music(pumpkin)), (0x06, 0x1E));
        doc.set_map_music(pumpkin, 0x28);
        doc.set_map_music(OnMap::World, 0x14);
        assert!(doc.music_dirty);
        assert!(doc.map_undo());
        assert_eq!((doc.map_music(OnMap::World), doc.map_music(pumpkin)), (0x06, 0x28));
        assert!(doc.map_redo());
        assert_eq!(doc.map_music(OnMap::World), 0x14);
        let k = doc.blank_zone(0).unwrap();
        assert_eq!(doc.map_music(OnMap::NewZone(k)), 0x10, "a blank zone has its base's music");
        doc.set_map_music(OnMap::NewZone(k), 0x1C);

        let text = files::project_text(&doc.project_file());
        let world = files::world_text(&doc.world_file());
        doc.commit().unwrap_or_else(|e| panic!("{e}"));
        assert!(!doc.music_dirty);
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.map_music(OnMap::World), again.map_music(pumpkin), again.map_music(OnMap::NewZone(k))), (0x14, 0x28, 0x1C));
        assert_eq!(again.map_music(OnMap::Zone(0)), 0x10, "the other zones keep theirs");

        let mut other = open().unwrap();
        other.replace_project(files::read_project(&text).unwrap()).unwrap();
        assert_eq!((other.map_music(OnMap::World), other.map_music(pumpkin), other.map_music(OnMap::NewZone(0))), (0x14, 0x28, 0x1C));
        let mut other = open().unwrap();
        other.replace_world(files::read_world(&world).unwrap()).unwrap();
        assert_eq!((other.map_music(OnMap::World), other.map_music(pumpkin)), (0x14, 0x1E));

        doc.restore_map(pumpkin);
        assert_eq!((doc.map_music(pumpkin), doc.map_music(OnMap::World)), (0x1E, 0x14), "a zone's restore leaves the world map's music");
        doc.restore_map(OnMap::World);
        assert_eq!(doc.map_music(OnMap::World), 0x06);
        assert!(doc.map_undo());
        assert_eq!(doc.map_music(OnMap::World), 0x14);
    }

    #[test]
    fn paths_and_new_points_are_drawn_saved_and_undone() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let original = doc.routes.clone();
        // a new point 32 pixels above point 04, a path up to it (and back down), level 7
        let (x, y) = doc.routes.places[4].on_world();
        let p = doc.add_point(OnMap::World, x, y - 32).unwrap();
        assert_eq!(doc.draw_path(4, p, &[(0, -32)]), Some((2, Some(3))));
        doc.step_point_level(p as usize, 8); // no level -> 7
        assert_eq!(doc.point_level(p as usize), Some(7));
        assert_eq!(doc.routes.ways[p as usize][3].as_ref().map(|w| (w.to, w.track().last().copied())), Some((4, Some((0, 32)))));
        let parts = doc.commit().unwrap();
        assert!(parts.iter().any(|s| matches!(s, Saved::Routes(_))));
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.routes, doc.routes);
        assert_eq!(again.point_level(p as usize), Some(7));
        // undo the level, the path and the point
        assert!(doc.map_undo() && doc.map_undo() && doc.map_undo());
        assert_eq!(doc.routes, original);
        assert!(doc.map_redo() && doc.remove_point(p));
        assert_eq!(doc.routes, original, "taking the point out takes its paths");
        // a point put on that path 20 pixels up, joined to 04: 04 now stops at it
        let far = doc.add_point(OnMap::World, x, y - 48).unwrap();
        doc.draw_path(4, far, &[(0, -48)]);
        let mid = doc.add_point(OnMap::World, x, y - 20).unwrap();
        assert_eq!(doc.draw_path(mid as usize, 4, &[(0, 20)]), Some((3, Some(2))));
        assert_eq!(doc.routes.ways[4][2].as_ref().map(|w| w.to), Some(mid), "the way along the same road is replaced");
        assert!(doc.remove_point(mid) && doc.remove_point(far));
        // a path out of a zone's entrance is also the path out of where Mario leaves the zone
        let (x, y) = doc.routes.places[2].on_world();
        let q = doc.add_point(OnMap::World, x, y + 16).unwrap();
        doc.set_way(2, 3, None);
        assert!(doc.draw_path(2, q, &[(0, 16)]).is_some());
        assert_eq!(doc.routes.ways[0x19], doc.routes.ways[2]);
        assert_eq!(doc.routes.ways[0x19][3].as_ref().map(|w| w.to), Some(q));
        // doors: the new point opens the Pumpkin Zone, the Tree Zone's exit leads into the
        // Macro Zone; saved, read back, undone
        doc.set_door(q, Some(1));
        doc.set_exit(0, 0x23, Leads::Zone(2));
        doc.commit().unwrap();
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.doors.door_of(q), again.doors.exits[0]), (Some(1), (0x23, Leads::Zone(2))));
        assert!(doc.map_undo() && doc.map_undo());
        assert_eq!(doc.doors, doors::read(&doc.original));
    }

    #[test]
    fn a_finish_is_placed_with_its_bell_saved_and_undone() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let before = doc.levels[1].content.clone();
        let goal = doc.goal_cells(1, Goal::Finish);
        assert_eq!(goal, vec![(253, 30)]);
        let bell = level_goals::bell(&before.objs, goal[0]).unwrap();
        doc.snapshot(1);
        assert!(doc.place_goal(1, Goal::Finish, (120, 28)) && doc.place_goal(1, Goal::Finish, (128, 28)));
        assert_eq!(doc.goal_cells(1, Goal::Finish), vec![(128, 28)]);
        assert_eq!(doc.block(1, 253, 30), before.map[30 * COLS + 252], "the cell left gets the block to its left");
        assert_eq!(doc.block(1, 120, 28), before.map[28 * COLS + 120], "a later move gives back what the cell had");
        let o = (before.objs[bell].x as i32 - 125 * 16, before.objs[bell].y as i32 - 2 * 16);
        assert_eq!((doc.levels[1].content.objs[bell].x as i32, doc.levels[1].content.objs[bell].y as i32), o, "the bell goes along");
        assert!(doc.ends_with_boss(5) && !doc.ends_with_boss(1));
        doc.commit().unwrap();
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!(again.goal_cells(1, Goal::Finish), vec![(128, 28)]);
        assert_eq!(again.levels[1].content.objs[bell], doc.levels[1].content.objs[bell]);
        assert!(doc.undo(1));
        assert_eq!(doc.levels[1].content, before, "one drag, one undo step");
    }

    #[test]
    fn a_secret_exit_is_placed_sent_elsewhere_saved_filed_and_undone() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        assert_eq!(doc.goal_cells(1, Goal::Secret), vec![]);
        doc.snapshot(1);
        assert!(doc.place_goal(1, Goal::Secret, (20, 28)));
        doc.set_level_secret(1, Some(SecretTo::Point(0x0A)));
        doc.set_level_secret(7, Some(SecretTo::Point(0x34)));
        let n = doc.add_level(12).unwrap();
        doc.set_level_secret(n, Some(SecretTo::Point(0x07)));
        let parts = doc.commit().unwrap();
        assert!(parts.iter().any(|s| matches!(s, Saved::SecretExits(3))));
        let again = EditorDocument::open(doc.original.clone(), Some(doc.rom.clone()));
        assert_eq!((again.level_secret(1), again.level_secret(7), again.level_secret(n), again.level_secret(8)), (Some(SecretTo::Point(0x0A)), Some(SecretTo::Point(0x34)), Some(SecretTo::Point(0x07)), None));
        assert_eq!(again.goal_cells(1, Goal::Secret), vec![(20, 28)]);
        let f = doc.level_file(7);
        assert_eq!(files::read_level(&files::level_text(&f, "")).map(|f| f.0.secret), Ok(Some(SecretTo::Point(0x34))));
        assert!(doc.undo(7));
        assert_eq!(doc.level_secret(7), None, "undone");
        assert!(doc.undo(1) && doc.undo(1));
        assert_eq!(doc.goal_cells(1, Goal::Secret), vec![]);
        doc.set_level_secret(1, None);
        doc.set_level_secret(n, None);
        doc.commit().unwrap();
        assert_eq!(&doc.rom[cart_model::rom::offset(0x0F, 0x6F62)..][..3], &[0xFA, 0x40, 0xA8], "no targets: the game's code");
    }

    #[test]
    fn world_and_zone_points_are_dragged_with_straight_paths_and_undone() {
        let Some(mut doc) = open() else {
            eprintln!("no cartridge found: document test skipped");
            return;
        };
        let original = doc.routes.clone();
        // world point $07, 40 pixels right: its place and the paths to and from it
        let (x, y) = doc.routes.places[7].on_world();
        doc.map_snapshot();
        doc.move_point(OnMap::World, 7, doc.routes.places[7], 41, 0);
        assert_eq!(doc.routes.places[7].on_world(), (x + 40, y));
        assert_eq!(doc.point_moved(OnMap::World, 7), (40, 0));
        for q in 0..map_routes::POINTS {
            for w in doc.routes.ways[q].iter().flatten().filter(|w| q == 7 || w.to == 7) {
                let (a, b) = (doc.routes.places[q].on_world(), doc.routes.places[w.to as usize].on_world());
                assert_eq!(w.track().last().copied(), Some((b.0 - a.0, b.1 - a.1)), "way {q:02X} > {:02X}", w.to);
            }
        }
        // a zone's entrance takes the point Mario walks out of the zone on with it
        doc.move_point(OnMap::World, 2, doc.routes.places[2], 0, -16);
        assert_eq!((doc.routes.places[0x19], doc.routes.ways[0x19].clone()), (doc.routes.places[2], doc.routes.ways[2].clone()));
        let parts = doc.commit().unwrap();
        assert!(parts.iter().any(|s| matches!(s, Saved::Routes(_))));
        assert_eq!(EditorDocument::open(doc.original.clone(), Some(doc.rom.clone())).routes, doc.routes);
        assert!(doc.map_undo());
        assert_eq!(doc.routes, original, "one drag, one undo step");
        // a point of the Macro Zone
        let p = cart_model::original_points::on(OnMap::Zone(2)).map(|s| s.point).find(|&p| map_routes::twin(p).is_none()).unwrap();
        doc.map_snapshot();
        doc.move_point(OnMap::Zone(2), p, doc.routes.places[p as usize], -8, 8);
        assert_eq!(doc.point_moved(OnMap::Zone(2), p), (-8, 8));
        assert!(doc.map_undo());
        assert_eq!(doc.routes, original);
    }
}
