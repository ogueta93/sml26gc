//! Level, zone and project files: plain text, one item per line, numbers in hex. The first line
//! names the kind and version, the last is `end`, so a cut file is found. Blocks are kept
//! raw (not compressed), so a file does not depend on the cartridge's codec and a diff shows
//! what changed.
//!
//! A level: `base`, `name` (from version 2), `header` (20 bytes), `map` (48 rows of 256 blocks), `objects` (x y kind
//! flag), `secret` (from version 3: `-`, `point` n or `newzone` k), `tables` (from version 4,
//! in projects from version 6: gravity, scroll, camera, animation; `level_tables`), `enemies`
//! (from version 5, in projects from version 7: `-`, or the count, each enemy's level and type,
//! and the sprite block in hex; `enemy_sets`). A new zone: `base`, `name`, `entry`, `door`, `music` (zone files from
//! version 4, projects from version 9), `picture` (18 rows of 20 tiles), `points`
//! (number, place, level, flag, shut, pass point code, the four ways) and `exits`. A project:
//! the cartridge's hash, every level as above, the world and zone pictures, the level each
//! map point opens, the paths, where Mario stands on each point, the new points, the doors,
//! (from version 2) the new zones and (from version 4) all 256 point numbers and the game
//! zones' added points (`zonepoints`: per zone, its number and its points as a new zone's),
//! (from version 5) the pad directions the world map's added points shut (`shuts`), and (from
//! version 9) the music of the world map and of each zone map (`music`).
use super::document::{EnemySet, RestorePoint, StandIn};
use cart_model::start_point::Start;
use cart_model::enemy_sets::{Enemy, BLOCK};
use cart_model::added_points::{self, ZoneAdded};
use cart_model::doors::{Doors, Leads};
use cart_model::level_tables::Tables;
use cart_model::map_music::{self, Map};
use cart_model::levels::{LEVELS, MAP_BYTES};
use cart_model::map_routes::{self, MapRoutes, Place, Way};
use cart_model::new_zones::{self, NewZone, ZonePoint};
use cart_model::objects::Obj;
use cart_model::secret_exits::SecretTo;
use cart_model::{extra_levels, world, zone_maps};
use std::fmt::Write;

const LEVEL_KIND: &str = "NMB level";
const PROJECT_KIND: &str = "NMB project";
const WORLD_KIND: &str = "NMB world";
/// Version 1 has no Mario's start, version 2 no music.
const WORLD_VERSION: u32 = 3;
const ZONE_KIND: &str = "NMB zone";
const RESTORE_KIND: &str = "NMB restore points";
const RESTORE_VERSION: u32 = 1;
/// Version 1 has no names, version 2 no secret exit target, version 3 no tables, version 4
/// no enemy set, version 5 no origin.
const LEVEL_VERSION: u32 = 6;
/// Version 1 has no Mario's start, version 2 no origin, version 3 no music.
const ZONE_VERSION: u32 = 4;
/// Version 1 has no new zones, version 2 no secret exit targets, version 5 no level tables,
/// version 6 no enemy sets.
/// Version 7 has no Mario's start (the game's own), version 8 no maps' music (the game's).
const PROJECT_VERSION: u32 = 9;
/// Point levels a project file kept before version 4.
const OLD_LEVELS: usize = 0x82;
const MAP_ROW: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    NotLevel,
    NotProject,
    NotZone,
    NotWorld,
    /// Made by a later editor.
    Newer,
    /// Cut short or changed by hand where it can't be read: the line (from 1).
    Damaged(usize),
    /// A base level that is not one of the 32.
    BaseOutside(usize),
    /// A new zone made from a zone no door enters (an index into `zone_maps::ZONES`).
    NotABase(usize),
    /// A project made over another cartridge.
    OtherCartridge,
}

/// A level as files keep it: what it plays as, its blocks and its objects.
#[derive(Clone, Debug, PartialEq)]
pub struct LevelFile {
    pub stand_in: StandIn,
    pub map: Vec<u8>,
    pub objs: Vec<Obj>,
    /// A new level's name (empty: none).
    pub name: String,
    /// Where its secret exit leads (None: the game's way).
    pub secret: Option<SecretTo>,
    /// Its gravity, auto-scroll, camera margin and animated tiles (None: an older file, so
    /// its base level's).
    pub tables: Option<Tables>,
    /// Its own enemy set (None: the game's).
    pub enemies: Option<EnemySet>,
}

/// Everything a project file holds.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectFile {
    /// `cartridge_id` of the original cartridge.
    pub cartridge: u64,
    /// The 32 levels (their own number as base), then the new ones.
    pub levels: Vec<LevelFile>,
    pub world: Vec<u8>,
    pub zones: Vec<Vec<u8>>,
    pub points: Vec<u8>,
    pub routes: MapRoutes,
    pub doors: Doors,
    pub new_zones: Vec<NewZone>,
    /// The point each zone a door enters is entered on (empty in version 1).
    pub entries: Vec<u8>,
    /// The game zones' added points (from version 4).
    pub zone_points: Vec<ZoneAdded>,
    /// The pad directions the world map's added points shut, from `added_points::FIRST`
    /// (from version 5).
    pub shuts: Vec<u8>,
    /// Mario's start (from version 8; before, the game's own).
    pub start: Start,
    /// The world map's music (from version 9; before, the game's).
    pub world_music: u8,
    /// The game zones' music, in `zone_maps::ZONES` order (from version 9; before, the game's).
    pub zone_music: Vec<u8>,
}


/// The world map as its own file: its picture, the level each point opens, the paths and
/// Mario's places (with the added points), the doors and the added points' shut directions.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldFile {
    pub cartridge: u64,
    pub world: Vec<u8>,
    pub points: Vec<u8>,
    pub routes: MapRoutes,
    pub doors: Doors,
    pub shuts: Vec<u8>,
    /// Mario's start, when it is on the world map (from version 2).
    pub start: Option<u8>,
    /// Its music (from version 3; before, the game's).
    pub music: u8,
}

/// Tells cartridges apart (64-bit FNV-1a of the whole image).
pub fn cartridge_id(rom: &[u8]) -> u64 {
    rom.iter().fold(0xCBF2_9CE4_8422_2325, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01B3))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

fn rows(out: &mut String, cells: &[u8], width: usize) {
    for row in cells.chunks(width) {
        out.push_str(&hex(row));
        out.push('\n');
    }
}

fn level_block(out: &mut String, l: &LevelFile) {
    let _ = writeln!(out, "base {:02x}\nname {}", l.stand_in.base, l.name);
    let _ = writeln!(out, "header {}", hex(&l.stand_in.header));
    out.push_str("map\n");
    rows(out, &l.map, MAP_ROW);
    let _ = writeln!(out, "objects {}", l.objs.len());
    for o in &l.objs {
        let _ = writeln!(out, "{:04x} {:04x} {:02x} {}", o.x, o.y, o.kind, o.flag as u8);
    }
    let _ = match l.secret {
        None => writeln!(out, "secret -"),
        Some(SecretTo::Point(p)) => writeln!(out, "secret point {p:02x}"),
        Some(SecretTo::NewZone(k)) => writeln!(out, "secret newzone {k:02x}"),
    };
    if let Some(t) = l.tables {
        let _ = writeln!(out, "tables {}", hex(&t.bytes()));
    }
    let _ = match &l.enemies {
        None => writeln!(out, "enemies -"),
        Some(set) => writeln!(out, "enemies {} {} {}", set.enemies.len(), set.enemies.iter().map(|e| format!("{:02x} {:02x}", e.level, e.kind)).collect::<Vec<_>>().join(" "), hex(&set.block)),
    };
}

/// Where a level or zone file was exported from, as its `origin` line keeps it:
/// `project|level|nn|name`, `project|zone|z` (a game zone, `zone_maps::ZONES`) or
/// `project|newzone|k|name` (numbers in hex, k from 0).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Origin {
    pub project: String,
    pub of: OriginOf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OriginOf {
    Level(usize, String),
    Zone(usize),
    NewZone(usize, String),
}

impl Origin {
    pub fn text(&self) -> String {
        match &self.of {
            OriginOf::Level(l, name) => format!("{}|level|{l:02x}|{name}", self.project),
            OriginOf::Zone(z) => format!("{}|zone|{z:02x}", self.project),
            OriginOf::NewZone(k, name) => format!("{}|newzone|{k:02x}|{name}", self.project),
        }
    }

    /// None for an empty or unknown origin.
    pub fn parse(text: &str) -> Option<Origin> {
        let f: Vec<&str> = text.splitn(4, '|').collect();
        let n = |s: &str| usize::from_str_radix(s, 16).ok();
        let of = match f[..] {
            [_, "level", l, name] => OriginOf::Level(n(l)?, name.to_string()),
            [_, "zone", z] => OriginOf::Zone(n(z)?),
            [_, "newzone", k, name] => OriginOf::NewZone(n(k)?, name.to_string()),
            _ => return None,
        };
        Some(Origin { project: f[0].to_string(), of })
    }
}

/// A level file; `origin`: where it was exported from (`Origin::text`, empty: unknown).
pub fn level_text(l: &LevelFile, origin: &str) -> String {
    let mut out = format!("{LEVEL_KIND} {LEVEL_VERSION}\n");
    origin_line(&mut out, origin);
    level_block(&mut out, l);
    out.push_str("end\n");
    out
}

/// A level file and its origin (empty: unknown).
pub fn read_level(text: &str) -> Result<(LevelFile, String), FileError> {
    let mut r = Reader::new(text);
    let version = r.kind(LEVEL_KIND, LEVEL_VERSION, FileError::NotLevel)?;
    let origin = if version >= 6 { r.origin()? } else { String::new() };
    let l = r.level(version, version >= 4, version >= 5)?;
    r.end()?;
    Ok((l, origin))
}

fn origin_line(out: &mut String, origin: &str) {
    let _ = writeln!(out, "origin {}", clean_origin(origin));
}

/// An origin as files keep it: one line of at most 80 printable characters.
fn clean_origin(origin: &str) -> String {
    origin.chars().map(|c| if c.is_control() { ' ' } else { c }).take(80).collect::<String>().trim().to_string()
}

fn way_field(w: &Option<Way>) -> String {
    match w {
        None => "-".into(),
        Some(w) => format!("{:02x}:{}", w.to, hex(&w.steps)),
    }
}

fn exit_line(out: &mut String, at: u8, leads: Leads) {
    let _ = match leads {
        Leads::World(to) => writeln!(out, "{at:02x} world {to:02x}"),
        Leads::Zone(z) => writeln!(out, "{at:02x} zone {z:02x}"),
        Leads::NewZone(k) => writeln!(out, "{at:02x} newzone {k:02x}"),
    };
}

fn zone_block(out: &mut String, z: &NewZone) {
    let _ = writeln!(out, "base {:02x}\nname {}\nentry {:02x}\ndoor {:02x}\nmusic {:02x}\npicture", z.base, z.name, z.entry, z.door, z.music);
    rows(out, &z.picture, zone_maps::W);
    point_lines(out, &z.points);
    let _ = writeln!(out, "exits {}", z.exits.len());
    for &(at, leads) in &z.exits {
        exit_line(out, at, leads);
    }
}

fn point_lines(out: &mut String, points: &[ZonePoint]) {
    let _ = writeln!(out, "points {}", points.len());
    for p in points {
        let pl = &p.place;
        let pass = p.pass.map_or("-".to_string(), |a| format!("{a:04x}"));
        let ways = p.ways.iter().map(way_field).collect::<Vec<_>>().join(" ");
        let _ = writeln!(out, "{:02x} {:04x} {:04x} {:02x} {:02x} {} {:02x} {:02x} {:02x} {pass} {ways}", p.number, pl.cam_x, pl.cam_y, pl.x, pl.y, hex(&pl.rest), p.level, p.flag, p.shut);
    }
}

/// A zone file; `start`: Mario's start, when it is on this zone; `origin`: where it was
/// exported from (free text, empty: unknown).
pub fn zone_text(z: &NewZone, start: Option<u8>, origin: &str) -> String {
    let mut out = format!("{ZONE_KIND} {ZONE_VERSION}\n");
    origin_line(&mut out, origin);
    zone_block(&mut out, z);
    start_line(&mut out, start);
    out.push_str("end\n");
    out
}

/// A zone file, Mario's start if the zone holds it, and its origin (empty: unknown).
pub fn read_zone(text: &str) -> Result<(NewZone, Option<u8>, String), FileError> {
    let mut r = Reader::new(text);
    let version = r.kind(ZONE_KIND, ZONE_VERSION, FileError::NotZone)?;
    let origin = if version >= 3 { r.origin()? } else { String::new() };
    let z = r.zone(version >= 4)?;
    let start = r.start_line()?;
    r.end()?;
    Ok((z, start, origin))
}

/// A project's restore points (`document::RestorePoint`), kept beside its cartridge: what its
/// new levels (by number) and new zones (by index from 0) were imported or replaced from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RestorePoints {
    pub levels: Vec<(usize, RestorePoint<LevelFile>)>,
    pub zones: Vec<(usize, RestorePoint<NewZone>)>,
}

/// The restore points file: `level n` or `zone k` (hex), each followed by its level or zone
/// file whole, then `end`.
pub fn restore_points_text(p: &RestorePoints) -> String {
    let mut out = format!("{RESTORE_KIND} {RESTORE_VERSION}
");
    for (l, point) in &p.levels {
        let _ = writeln!(out, "level {l:02x}");
        out.push_str(&level_text(&point.file, &point.origin));
    }
    for (k, point) in &p.zones {
        let _ = writeln!(out, "zone {k:02x}");
        out.push_str(&zone_text(&point.file, None, &point.origin));
    }
    out.push_str("end
");
    out
}

pub fn read_restore_points(text: &str) -> Result<RestorePoints, FileError> {
    let mut r = Reader::new(text);
    r.kind(RESTORE_KIND, RESTORE_VERSION, FileError::Damaged(1))?;
    let mut points = RestorePoints::default();
    loop {
        let line = r.line()?;
        let (what, n) = line.split_once(' ').unwrap_or((line, ""));
        let n = usize::from_str_radix(n, 16);
        match (what, n) {
            ("end", _) => return Ok(points),
            ("level", Ok(l)) => {
                let version = r.kind(LEVEL_KIND, LEVEL_VERSION, r.damaged())?;
                let origin = if version >= 6 { r.origin()? } else { String::new() };
                let file = r.level(version, version >= 4, version >= 5)?;
                r.end()?;
                points.levels.push((l, RestorePoint { file, origin }));
            }
            ("zone", Ok(k)) => {
                let version = r.kind(ZONE_KIND, ZONE_VERSION, r.damaged())?;
                let origin = if version >= 3 { r.origin()? } else { String::new() };
                let file = r.zone(version >= 4)?;
                r.start_line()?;
                r.end()?;
                points.zones.push((k, RestorePoint { file, origin }));
            }
            _ => return Err(r.damaged()),
        }
    }
}

fn start_line(out: &mut String, start: Option<u8>) {
    if let Some(p) = start {
        let _ = writeln!(out, "start {p:02x}");
    }
}

pub fn project_text(p: &ProjectFile) -> String {
    let mut out = format!("{PROJECT_KIND} {PROJECT_VERSION}\ncartridge {:016x}\nlevels {}\n", p.cartridge, p.levels.len());
    for l in &p.levels {
        level_block(&mut out, l);
    }
    out.push_str("world\n");
    rows(&mut out, &p.world, world::W);
    let _ = writeln!(out, "zones {}", p.zones.len());
    for z in &p.zones {
        rows(&mut out, z, zone_maps::W);
    }
    map_lines(&mut out, &p.points, &p.routes, &p.doors);
    let _ = writeln!(out, "entries {}", hex(&p.entries));
    let _ = writeln!(out, "newzones {}", p.new_zones.len());
    for z in &p.new_zones {
        zone_block(&mut out, z);
    }
    let _ = writeln!(out, "zonepoints {}", p.zone_points.len());
    for z in &p.zone_points {
        let _ = writeln!(out, "zone {:02x}", z.zone);
        point_lines(&mut out, &z.points);
    }
    let _ = writeln!(out, "shuts {}", hex(&p.shuts));
    let _ = writeln!(out, "start {:02x} {}", p.start.point, p.start.new_zone.map_or("-".to_string(), |k| format!("{k:02x}")));
    let _ = writeln!(out, "music {:02x} {}", p.world_music, hex(&p.zone_music));
    out.push_str("end\n");
    out
}

/// The point levels, paths, places, added points and doors, as projects and world map files
/// keep them.
fn map_lines(out: &mut String, points: &[u8], routes: &MapRoutes, doors: &Doors) {
    let _ = writeln!(out, "points {}", hex(points));
    let _ = writeln!(out, "ways {}", routes.ways.len());
    for w in &routes.ways {
        out.push_str(&w.iter().map(way_field).collect::<Vec<_>>().join(" "));
        out.push('\n');
    }
    let _ = writeln!(out, "places {}", routes.places.len());
    for pl in &routes.places {
        let _ = writeln!(out, "{:04x} {:04x} {:02x} {:02x} {}", pl.cam_x, pl.cam_y, pl.x, pl.y, hex(&pl.rest));
    }
    let _ = writeln!(out, "added {}", hex(&routes.added));
    let _ = writeln!(out, "into {}", doors.into.len());
    for slots in &doors.into {
        let _ = writeln!(out, "{}", hex(slots));
    }
    let _ = writeln!(out, "exits {}", doors.exits.len());
    for &(at, leads) in &doors.exits {
        exit_line(out, at, leads);
    }
}

pub fn world_text(w: &WorldFile) -> String {
    let mut out = format!("{WORLD_KIND} {WORLD_VERSION}\ncartridge {:016x}\nworld\n", w.cartridge);
    rows(&mut out, &w.world, world::W);
    map_lines(&mut out, &w.points, &w.routes, &w.doors);
    let _ = writeln!(out, "shuts {}", hex(&w.shuts));
    let _ = writeln!(out, "music {:02x}", w.music);
    start_line(&mut out, w.start);
    out.push_str("end\n");
    out
}

pub fn read_world(text: &str) -> Result<WorldFile, FileError> {
    let mut r = Reader::new(text);
    let version = r.kind(WORLD_KIND, WORLD_VERSION, FileError::NotWorld)?;
    let cartridge = r.hex::<u64>("cartridge")?;
    r.word("world")?;
    let world = r.rows(world::H, world::W)?;
    let (points, routes, doors) = r.map_parts(world::POINT_COUNT, map_routes::ALL)?;
    let mut shuts = r.bytes_field("shuts")?;
    if shuts.len() > added_points::MAX {
        return Err(r.damaged());
    }
    shuts.resize(added_points::MAX, 0);
    let music = if version >= 3 { r.hex::<u8>("music")? } else { map_music::game_music(Map::World) };
    let start = r.start_line()?;
    r.end()?;
    Ok(WorldFile { cartridge, world, points, routes, doors, shuts, start, music })
}

pub fn read_project(text: &str) -> Result<ProjectFile, FileError> {
    let mut r = Reader::new(text);
    let version = r.kind(PROJECT_KIND, PROJECT_VERSION, FileError::NotProject)?;
    let cartridge = r.hex::<u64>("cartridge")?;
    let count = r.number("levels")?;
    if !(LEVELS..=LEVELS + extra_levels::MAX).contains(&count) {
        return Err(r.damaged());
    }
    let levels = (0..count).map(|_| r.level(version, version >= 6, version >= 7)).collect::<Result<Vec<_>, _>>()?;
    r.word("world")?;
    let world = r.rows(world::H, world::W)?;
    let zones = (0..r.count("zones", zone_maps::ZONES.len())?).map(|_| r.rows(zone_maps::H, zone_maps::W)).collect::<Result<Vec<_>, _>>()?;
    // before version 4: the game's numbers only ($82 levels, $80 ways and places)
    let (n_levels, n_points) = if version >= 4 { (world::POINT_COUNT, map_routes::ALL) } else { (OLD_LEVELS, map_routes::POINTS) };
    let (points, routes, doors) = r.map_parts(n_levels, n_points)?;
    let entries = if version >= 2 { r.bytes_field("entries")? } else { Vec::new() };
    let new_zones = if version >= 2 {
        let n = r.number("newzones")?;
        if n > new_zones::MAX {
            return Err(r.damaged());
        }
        (0..n).map(|_| r.zone(version >= 9)).collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let zone_points = if version >= 4 {
        let n = r.number("zonepoints")?;
        (0..n).map(|_| r.zone_added()).collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let mut shuts = if version >= 5 { r.bytes_field("shuts")? } else { Vec::new() };
    if shuts.len() > added_points::MAX {
        return Err(r.damaged());
    }
    shuts.resize(added_points::MAX, 0);
    let start = if version >= 8 { r.start()? } else { Start::default() };
    let (world_music, zone_music) = if version >= 9 { r.music()? } else { game_music() };
    r.end()?;
    Ok(ProjectFile { cartridge, levels, world, zones, points, routes, doors, new_zones, entries, zone_points, shuts, start, world_music, zone_music })
}

/// The game's own music of the world map and of each zone map.
pub fn game_music() -> (u8, Vec<u8>) {
    (map_music::game_music(Map::World), (0..zone_maps::ZONES.len()).map(|z| map_music::game_music(Map::Zone(z))).collect())
}

struct Reader<'a> {
    lines: std::iter::Peekable<std::str::Lines<'a>>,
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Reader<'a> {
        Reader { lines: text.lines().peekable(), at: 0 }
    }

    fn damaged(&self) -> FileError {
        FileError::Damaged(self.at)
    }

    /// `start pp` when the map holds Mario's start.
    fn start_line(&mut self) -> Result<Option<u8>, FileError> {
        if !self.lines.peek().is_some_and(|l| l.trim().starts_with("start ")) {
            return Ok(None);
        }
        self.hex::<u8>("start").map(Some)
    }

    /// A project's `music ww zz..`: the world map's, then each game zone's.
    fn music(&mut self) -> Result<(u8, Vec<u8>), FileError> {
        let line = self.line()?;
        let mut parts = line.strip_prefix("music ").ok_or_else(|| self.damaged())?.split_whitespace();
        let world = parts.next().and_then(|p| u8::from_str_radix(p, 16).ok()).ok_or_else(|| self.damaged())?;
        let zones = self.bytes(parts.next().unwrap_or(""))?;
        if zones.len() != zone_maps::ZONES.len() {
            return Err(self.damaged());
        }
        Ok((world, zones))
    }

    /// A project's `start pp zz` (`-`: not in a new zone).
    fn start(&mut self) -> Result<Start, FileError> {
        let line = self.line()?;
        let mut parts = line.strip_prefix("start ").ok_or_else(|| self.damaged())?.split_whitespace();
        let point = parts.next().and_then(|p| u8::from_str_radix(p, 16).ok()).ok_or_else(|| self.damaged())?;
        let new_zone = match parts.next() {
            Some("-") => None,
            Some(k) => Some(usize::from_str_radix(k, 16).map_err(|_| self.damaged())?),
            None => return Err(self.damaged()),
        };
        Ok(Start { point, new_zone })
    }

    /// `map_lines`: `n_levels` point levels and `n_points` ways and places (fewer in older
    /// projects), grown to every point.
    fn map_parts(&mut self, n_levels: usize, n_points: usize) -> Result<(Vec<u8>, MapRoutes, Doors), FileError> {
        let mut points = self.bytes_field("points")?;
        if points.len() != n_levels {
            return Err(self.damaged());
        }
        points.resize(world::POINT_COUNT, world::NO_LEVEL);
        let mut ways = (0..self.count("ways", n_points)?).map(|_| self.ways()).collect::<Result<Vec<_>, _>>()?;
        let mut places = (0..self.count("places", n_points)?).map(|_| self.place()).collect::<Result<Vec<_>, _>>()?;
        ways.resize(map_routes::ALL, Default::default());
        places.resize(map_routes::ALL, Place { cam_x: 0, cam_y: 0, x: 0, y: 0, rest: [0, 0] });
        let added = self.bytes_field("added")?;
        let into_n = self.number("into")?;
        let into = (0..into_n).map(|_| self.bytes_line()).collect::<Result<Vec<_>, _>>()?;
        let exits_n = self.number("exits")?;
        let exits = (0..exits_n).map(|_| self.exit()).collect::<Result<Vec<_>, _>>()?;
        Ok((points, MapRoutes { ways, places, added }, Doors { into, exits }))
    }

    fn line(&mut self) -> Result<&'a str, FileError> {
        self.at += 1;
        self.lines.next().map(str::trim).ok_or(FileError::Damaged(self.at))
    }

    /// The first line: this kind of file, at a version this editor reads (up to `newest`).
    fn kind(&mut self, kind: &str, newest: u32, wrong: FileError) -> Result<u32, FileError> {
        let line = self.line().map_err(|_| wrong.clone())?.trim_start_matches('\u{feff}');
        let version = line.strip_prefix(kind).and_then(|v| v.trim().parse::<u32>().ok()).ok_or(wrong)?;
        if version > newest {
            return Err(FileError::Newer);
        }
        Ok(version)
    }

    fn word(&mut self, key: &str) -> Result<(), FileError> {
        if self.line()? == key {
            Ok(())
        } else {
            Err(self.damaged())
        }
    }

    fn field(&mut self, key: &str) -> Result<&'a str, FileError> {
        let line = self.line()?;
        match line.split_once(' ') {
            Some((k, v)) if k == key => Ok(v.trim()),
            None if line == key => Ok(""),
            _ => Err(self.damaged()),
        }
    }

    fn hex<T: Hex>(&mut self, key: &str) -> Result<T, FileError> {
        let v = self.field(key)?;
        T::from_hex(v).ok_or(self.damaged())
    }

    /// A count (decimal).
    fn number(&mut self, key: &str) -> Result<usize, FileError> {
        let v = self.field(key)?;
        v.parse().map_err(|_| self.damaged())
    }

    /// A count that must be exactly `n`.
    fn count(&mut self, key: &str, n: usize) -> Result<usize, FileError> {
        let v = self.number(key)?;
        if v == n {
            Ok(v)
        } else {
            Err(self.damaged())
        }
    }

    fn bytes(&self, s: &str) -> Result<Vec<u8>, FileError> {
        if s.len() % 2 != 0 || !s.is_ascii() {
            return Err(self.damaged());
        }
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| self.damaged())).collect()
    }

    fn bytes_line(&mut self) -> Result<Vec<u8>, FileError> {
        let line = self.line()?;
        self.bytes(line)
    }

    fn bytes_field(&mut self, key: &str) -> Result<Vec<u8>, FileError> {
        let v = self.field(key)?;
        self.bytes(v)
    }

    fn rows(&mut self, h: usize, w: usize) -> Result<Vec<u8>, FileError> {
        let mut cells = Vec::with_capacity(w * h);
        for _ in 0..h {
            let row = self.bytes_line()?;
            if row.len() != w {
                return Err(self.damaged());
            }
            cells.extend(row);
        }
        Ok(cells)
    }

    fn level(&mut self, version: u32, tabled: bool, enemied: bool) -> Result<LevelFile, FileError> {
        let named = version >= 2;
        let base = self.hex::<u8>("base")?;
        if base as usize >= LEVELS {
            return Err(FileError::BaseOutside(base as usize));
        }
        let name = if named { self.name()? } else { String::new() };
        let header: [u8; 20] = self.bytes_field("header")?.try_into().map_err(|_| self.damaged())?;
        self.word("map")?;
        let map = self.rows(MAP_BYTES / MAP_ROW, MAP_ROW)?;
        let n = self.number("objects")?;
        let objs = (0..n).map(|_| self.object()).collect::<Result<Vec<_>, _>>()?;
        let secret = if version >= 3 { self.secret()? } else { None };
        let tables = if tabled { Some(Tables::from_bytes(self.bytes_field("tables")?.try_into().map_err(|_| self.damaged())?)) } else { None };
        let enemies = if enemied { self.enemies()? } else { None };
        Ok(LevelFile { stand_in: StandIn { base, header }, map, objs, name, secret, tables, enemies })
    }

    fn enemies(&mut self) -> Result<Option<EnemySet>, FileError> {
        let v = self.field("enemies")?;
        let f: Vec<&str> = v.split_whitespace().collect();
        if f.first() == Some(&"-") {
            return Ok(None);
        }
        let n: usize = f.first().and_then(|n| n.parse().ok()).ok_or_else(|| self.damaged())?;
        let byte = |s: &str| u8::from_str_radix(s, 16).ok();
        let enemies = (0..n).map(|i| Some(Enemy { level: byte(f.get(1 + i * 2)?)?, kind: byte(f.get(2 + i * 2)?)? })).collect::<Option<Vec<_>>>().ok_or_else(|| self.damaged())?;
        let block = self.bytes(f.get(1 + n * 2).copied().unwrap_or(""))?;
        if block.len() != BLOCK || enemies.iter().any(|e| e.level as usize >= LEVELS) {
            return Err(self.damaged());
        }
        Ok(Some(EnemySet { enemies, block }))
    }

    fn secret(&mut self) -> Result<Option<SecretTo>, FileError> {
        let v = self.field("secret")?;
        let f: Vec<&str> = v.split_whitespace().collect();
        let to = match f[..] {
            ["-"] => Some(None),
            ["point", p] => u8::from_str_radix(p, 16).ok().map(|p| Some(SecretTo::Point(p))),
            ["newzone", k] => usize::from_str_radix(k, 16).ok().filter(|&k| k < new_zones::MAX).map(|k| Some(SecretTo::NewZone(k))),
            _ => None,
        };
        to.ok_or(self.damaged())
    }

    fn origin(&mut self) -> Result<String, FileError> {
        let v = self.field("origin")?;
        Ok(clean_origin(v))
    }

    fn name(&mut self) -> Result<String, FileError> {
        let v = self.field("name")?;
        Ok(cart_model::extra_levels::clean_name(v))
    }

    fn object(&mut self) -> Result<Obj, FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        let obj = match f[..] {
            [x, y, kind, flag] => (|| Some(Obj { x: u16::from_str_radix(x, 16).ok()?, y: u16::from_str_radix(y, 16).ok()?, kind: u8::from_str_radix(kind, 16).ok()?, flag: matches!(flag, "1") }))(),
            _ => None,
        };
        obj.ok_or(self.damaged())
    }

    fn ways(&mut self) -> Result<[Option<Way>; 4], FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 4 {
            return Err(self.damaged());
        }
        self.way_fields(&f)
    }

    fn way_fields(&self, f: &[&str]) -> Result<[Option<Way>; 4], FileError> {
        let mut ways: [Option<Way>; 4] = Default::default();
        for (slot, &field) in ways.iter_mut().zip(f) {
            if field == "-" {
                continue;
            }
            let (to, steps) = field.split_once(':').ok_or(self.damaged())?;
            *slot = Some(Way { to: u8::from_str_radix(to, 16).map_err(|_| self.damaged())?, steps: self.bytes(steps)? });
        }
        Ok(ways)
    }

    fn place(&mut self) -> Result<Place, FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        let place = match f[..] {
            [cx, cy, x, y, rest] => (|| {
                let rest: [u8; 2] = self.bytes(rest).ok()?.try_into().ok()?;
                Some(Place { cam_x: u16::from_str_radix(cx, 16).ok()?, cam_y: u16::from_str_radix(cy, 16).ok()?, x: u8::from_str_radix(x, 16).ok()?, y: u8::from_str_radix(y, 16).ok()?, rest })
            })(),
            _ => None,
        };
        place.ok_or(self.damaged())
    }

    /// A new zone: its points within its base's numbers, one of them the entry, an exit for
    /// each of its base's exit sites.
    /// Without `music` (older files) it has its base's game music.
    fn zone(&mut self, music: bool) -> Result<NewZone, FileError> {
        let base = self.hex::<u8>("base")? as usize;
        let b = new_zones::base_of(base).ok_or(FileError::NotABase(base))?;
        let name = self.name()?;
        let entry = self.hex::<u8>("entry")?;
        let door = self.hex::<u8>("door")?;
        let music = if music { self.hex::<u8>("music")? } else { map_music::game_music(Map::Zone(base)) };
        self.word("picture")?;
        let picture = self.rows(zone_maps::H, zone_maps::W)?;
        let n = self.number("points")?;
        let points = (0..n).map(|_| self.zone_point()).collect::<Result<Vec<_>, _>>()?;
        let mut numbers: Vec<u8> = points.iter().map(|p| p.number).collect();
        numbers.sort();
        numbers.dedup();
        if numbers.len() != n || !numbers.iter().all(|&p| b.holds(p)) || !numbers.contains(&entry) {
            return Err(self.damaged());
        }
        let exits = (0..self.count("exits", new_zones::exit_sites(b).len())?).map(|_| self.exit()).collect::<Result<Vec<_>, _>>()?;
        Ok(NewZone { base, picture, points, entry, exits, door, name, music })
    }

    /// A game zone's added points: numbered from $80, at most `added_points::MAX`.
    fn zone_added(&mut self) -> Result<ZoneAdded, FileError> {
        let zone = self.hex::<u8>("zone")? as usize;
        let n = self.number("points")?;
        let points = (0..n).map(|_| self.zone_point()).collect::<Result<Vec<_>, _>>()?;
        if zone >= added_points::ZONES || n > added_points::MAX || !points.iter().all(|p| added_points::index(p.number).is_some()) {
            return Err(self.damaged());
        }
        Ok(ZoneAdded { zone, points })
    }

    fn zone_point(&mut self) -> Result<ZonePoint, FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        if f.len() != 14 {
            return Err(self.damaged());
        }
        let ways = self.way_fields(&f[10..])?;
        let point = (|| {
            let b = |s: &str| u8::from_str_radix(s, 16).ok();
            let w = |s: &str| u16::from_str_radix(s, 16).ok();
            let rest: [u8; 2] = self.bytes(f[5]).ok()?.try_into().ok()?;
            let pass = if f[9] == "-" { None } else { Some(w(f[9])?) };
            let place = Place { cam_x: w(f[1])?, cam_y: w(f[2])?, x: b(f[3])?, y: b(f[4])?, rest };
            Some(ZonePoint { number: b(f[0])?, place, ways, level: b(f[6])?, flag: b(f[7])?, shut: b(f[8])?, pass })
        })();
        point.ok_or(self.damaged())
    }

    fn exit(&mut self) -> Result<(u8, Leads), FileError> {
        let line = self.line()?;
        let f: Vec<&str> = line.split_whitespace().collect();
        let exit = match f[..] {
            [at, kind, to] => (|| {
                let at = u8::from_str_radix(at, 16).ok()?;
                let to = u8::from_str_radix(to, 16).ok()?;
                match kind {
                    "world" => Some((at, Leads::World(to))),
                    "zone" => Some((at, Leads::Zone(to as usize))),
                    "newzone" => Some((at, Leads::NewZone(to as usize))),
                    _ => None,
                }
            })(),
            _ => None,
        };
        exit.ok_or(self.damaged())
    }

    fn end(&mut self) -> Result<(), FileError> {
        self.word("end")
    }
}

trait Hex: Sized {
    fn from_hex(s: &str) -> Option<Self>;
}

impl Hex for u8 {
    fn from_hex(s: &str) -> Option<u8> {
        u8::from_str_radix(s, 16).ok()
    }
}

impl Hex for u64 {
    fn from_hex(s: &str) -> Option<u64> {
        u64::from_str_radix(s, 16).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(base: u8) -> LevelFile {
        let map = (0..MAP_BYTES).map(|i| (i % 0x80) as u8).collect();
        let objs = vec![Obj { x: 0x40, y: 0x18, kind: 0x25, flag: true }, Obj { x: 0xFF8, y: 0x3F8, kind: 0x7F, flag: false }];
        let secret = [None, Some(SecretTo::Point(0x07)), Some(SecretTo::NewZone(1))][base as usize % 3];
        let tables = Some(Tables { gravity: base & 8, scroll: 1, camera: 0, animation: 0x4D });
        let enemies = (base % 2 == 1).then(|| EnemySet { enemies: vec![Enemy { level: 3, kind: 0x21 }, Enemy { level: base, kind: 0x09 }], block: (0..BLOCK).map(|i| i as u8).collect() });
        LevelFile { stand_in: StandIn { base, header: core::array::from_fn(|i| i as u8 * 3) }, map, objs, name: format!("Cave {base}"), secret, tables, enemies }
    }

    fn project() -> ProjectFile {
        let way = Way { steps: vec![1, 1, 0x18], to: 7 };
        ProjectFile {
            cartridge: 0x0123_4567_89AB_CDEF,
            levels: (0..LEVELS as u8 + 2).map(|l| level(l % LEVELS as u8)).collect(),
            world: (0..world::W * world::H).map(|i| i as u8).collect(),
            zones: (0..zone_maps::ZONES.len()).map(|z| vec![z as u8; zone_maps::W * zone_maps::H]).collect(),
            points: (0..world::POINT_COUNT).map(|p| p as u8).collect(),
            routes: MapRoutes {
                ways: (0..map_routes::ALL).map(|p| if p % 3 == 0 { [Some(way.clone()), None, Some(Way { steps: vec![], to: 2 }), None] } else { Default::default() }).collect(),
                places: (0..map_routes::ALL).map(|p| Place { cam_x: p as u16 * 3, cam_y: 0x150, x: 0x4B, y: p as u8, rest: [1, 2] }).collect(),
                added: vec![0x6D, 0x7B, 0x80],
            },
            doors: Doors { into: vec![vec![0x02, 0x19], vec![0xFF]], exits: vec![(0x23, Leads::World(0x05)), (0x28, Leads::Zone(2))] },
            new_zones: vec![zone()],
            entries: vec![0x23, 0x28, 0x32, 0x3C, 0x50],
            zone_points: vec![ZoneAdded { zone: 1, points: vec![ZonePoint { number: 0x80, flag: 0x80, ..zone().points[0].clone() }] }],
            shuts: (0..added_points::MAX).map(|j| if j == 0 { 0x90 } else { 0 }).collect(),
            start: Start { point: 0x29, new_zone: None },
            world_music: 0x14,
            zone_music: vec![0x10, 0x28, 0x05, 0x1C, 0x12, 0x1B, 0x0E, 0x0D],
        }
    }

    fn zone() -> NewZone {
        let place = |x: u8| Place { cam_x: 0, cam_y: 0, x, y: 0x60, rest: [1, 2] };
        let way = |to: u8| Some(Way { steps: vec![0x10, 0x02], to });
        let points = vec![
            ZonePoint { number: 0x23, place: place(0x40), ways: [way(0x25), None, None, None], level: 1, flag: 5, shut: 0x10, pass: None },
            ZonePoint { number: 0x25, place: place(0x60), ways: [None, way(0x23), None, None], level: world::NO_LEVEL, flag: 6, shut: 0, pass: Some(0x429A) },
        ];
        let sites = new_zones::exit_sites(&new_zones::BASES[0]).len();
        let exits = (0..sites).map(|i| (0x25, if i == 0 { Leads::World(0x19) } else { Leads::NewZone(0) })).collect();
        NewZone { base: 0, picture: (0..zone_maps::W * zone_maps::H).map(|i| i as u8).collect(), points, entry: 0x23, exits, door: 0x19, name: "Big tree".into(), music: 0x28 }
    }

    #[test]
    fn a_zone_comes_back_as_it_was_written() {
        let z = zone();
        let text = zone_text(&z, None, "Demo|newzone 1 Big tree");
        assert!(text.starts_with("NMB zone 4\norigin Demo|newzone 1 Big tree\nbase 00\nname Big tree\nentry 23\ndoor 19\nmusic 28\n") && text.ends_with("end\n"));
        assert_eq!(read_zone(&text), Ok((z.clone(), None, "Demo|newzone 1 Big tree".into())));
        assert_eq!(read_zone(&zone_text(&z, Some(0x21), "")), Ok((z.clone(), Some(0x21), String::new())), "with Mario's start");
        let v3 = text.replace("NMB zone 4", "NMB zone 3").replace("music 28\n", "");
        let z = NewZone { music: 0x10, ..z };
        assert_eq!(read_zone(&v3), Ok((z.clone(), None, "Demo|newzone 1 Big tree".into())), "a version 3 file has its base's music");
        let v2 = v3.replace("NMB zone 3", "NMB zone 2").replace("origin Demo|newzone 1 Big tree\n", "");
        assert_eq!(read_zone(&v2), Ok((z.clone(), None, String::new())), "a version 2 file has no origin");
        assert_eq!(read_zone(&v2.replace("NMB zone 2", "NMB zone 1")), Ok((z, None, String::new())), "a version 1 file");
    }

    #[test]
    fn unusable_zone_files_are_refused() {
        let text = zone_text(&zone(), None, "");
        assert!(matches!(read_zone(&text[..text.len() / 2]), Err(FileError::Damaged(_))));
        assert_eq!(read_zone(&text.replace("base 00", "base 04")), Err(FileError::NotABase(4)));
        assert!(matches!(read_zone(&text.replace("entry 23", "entry 24")), Err(FileError::Damaged(_))), "the entry is one of its points");
        assert!(matches!(read_zone(&text.replace("\n25 ", "\n32 ")), Err(FileError::Damaged(_))), "points stay in the base's numbers");
        assert_eq!(read_zone(&text.replace("NMB zone 4", "NMB zone 5")), Err(FileError::Newer));
        assert_eq!(read_zone(&level_text(&level(0), "")), Err(FileError::NotZone));
    }

    #[test]
    fn a_level_comes_back_as_it_was_written() {
        let l = level(13);
        let text = level_text(&l, "Demo|level 05");
        assert!(text.starts_with("NMB level 6\norigin Demo|level 05\nbase 0d\nname Cave 13\n") && text.contains("secret point 07\ntables 0801004d\nenemies 2 03 21 0d 09 0001"));
        assert_eq!(read_level(&text), Ok((l.clone(), "Demo|level 05".into())));
        let v5 = text.replacen("NMB level 6", "NMB level 5", 1).replacen("origin Demo|level 05\n", "", 1);
        assert_eq!(read_level(&v5), Ok((l.clone(), String::new())), "version 5 has no origin");
        let read_level = |t: &str| read_level(t).map(|f| f.0);
        let v4 = v5.replacen("NMB level 5", "NMB level 4", 1);
        let v4 = format!("{}end\n", &v4[..v4.find("enemies").unwrap()]);
        assert_eq!(read_level(&v4), Ok(LevelFile { enemies: None, ..l.clone() }), "version 4 has no enemy set");
        for b in [12, 14] {
            assert_eq!(read_level(&level_text(&level(b), "")), Ok(level(b)));
        }
        let v3 = v4.replacen("NMB level 4", "NMB level 3", 1).replacen("tables 0801004d\n", "", 1);
        assert_eq!(read_level(&v3), Ok(LevelFile { tables: None, enemies: None, ..l.clone() }), "version 3 has no tables");
        let v2 = v3.replacen("NMB level 3", "NMB level 2", 1).replacen("secret point 07\n", "", 1);
        assert_eq!(read_level(&v2), Ok(LevelFile { secret: None, tables: None, enemies: None, ..l.clone() }), "version 2 has no secret exit target");
        let old = v2.replacen("NMB level 2", "NMB level 1", 1).replacen("name Cave 13\n", "", 1);
        assert_eq!(read_level(&old), Ok(LevelFile { name: String::new(), secret: None, tables: None, enemies: None, ..l }), "version 1 has no names");
        assert!(matches!(read_level(&text.replace("secret point 07", "secret door 07")), Err(FileError::Damaged(_))));
    }

    #[test]
    fn restore_points_come_back_as_they_were_written() {
        let points = RestorePoints {
            levels: vec![(0x21, RestorePoint { file: level(13), origin: "Demo|level|05|".into() }), (0x23, RestorePoint { file: level(3), origin: String::new() })],
            zones: vec![(1, RestorePoint { file: zone(), origin: "Demo|zone|00".into() })],
        };
        let text = restore_points_text(&points);
        assert_eq!(read_restore_points(&text), Ok(points));
        assert_eq!(read_restore_points(&restore_points_text(&RestorePoints::default())), Ok(RestorePoints::default()));
        assert!(read_restore_points(&text[..text.len() / 2]).is_err());
        assert!(read_restore_points(&level_text(&level(0), "")).is_err());
    }

    #[test]
    fn origins_come_back_as_they_were_written() {
        for of in [OriginOf::Level(0x21, "Ice|cave".into()), OriginOf::Zone(3), OriginOf::NewZone(1, String::new())] {
            let o = Origin { project: "Demo".into(), of };
            assert_eq!(Origin::parse(&o.text()), Some(o));
        }
        assert_eq!(Origin::parse(""), None);
        assert_eq!(Origin::parse("Demo|door|05"), None);
    }

    #[test]
    fn unusable_level_files_are_refused() {
        let text = level_text(&level(3), "");
        assert!(matches!(read_level(&text[..text.len() / 2]), Err(FileError::Damaged(_))));
        assert_eq!(read_level(&text.replace("base 03", "base 20")), Err(FileError::BaseOutside(32)));
        assert_eq!(read_level(&text.replace("NMB level 6", "NMB level 7")), Err(FileError::Newer));
        assert_eq!(read_level("hello"), Err(FileError::NotLevel));
        assert_eq!(read_level(""), Err(FileError::NotLevel));
        let short_row = text.replacen("\n0001", "\n01", 1);
        assert!(matches!(read_level(&short_row), Err(FileError::Damaged(_))));
        assert_eq!(read_level(&project_text(&project())), Err(FileError::NotLevel));
    }

    #[test]
    fn a_project_comes_back_as_it_was_written() {
        let p = project();
        let text = project_text(&p);
        assert!(text.contains("\nmusic 14 1028051c121b0e0d\nend\n") && text.contains("door 19\nmusic 28\n"));
        assert_eq!(read_project(&text), Ok(p.clone()));
        let v8 = text.replacen("NMB project 9", "NMB project 8", 1).replacen("music 14 1028051c121b0e0d\n", "", 1).replacen("music 28\n", "", 1);
        let (world_music, zone_music) = game_music();
        let new_zones = vec![NewZone { music: 0x10, ..zone() }];
        assert_eq!(read_project(&v8), Ok(ProjectFile { world_music, zone_music, new_zones, ..p }), "version 8 has the game's music");
    }

    #[test]
    fn world_files_round_trip_and_are_not_projects() {
        let p = project();
        let w = WorldFile { cartridge: p.cartridge, world: p.world.clone(), points: p.points.clone(), routes: p.routes.clone(), doors: p.doors.clone(), shuts: p.shuts.clone(), start: Some(0x07), music: 0x28 };
        assert_eq!(read_world(&world_text(&w)), Ok(w.clone()));
        let v2 = world_text(&w).replacen("NMB world 3", "NMB world 2", 1).replacen("music 28\n", "", 1);
        assert_eq!(read_world(&v2), Ok(WorldFile { music: 0x06, ..w.clone() }), "a version 2 file has the game's music");
        assert_eq!(read_world(&project_text(&p)), Err(FileError::NotWorld));
        assert_eq!(read_project(&world_text(&w)), Err(FileError::NotProject));
    }

    #[test]
    fn a_version_1_project_has_no_new_zones() {
        // the game's point numbers only
        let mut p = project();
        p.points.truncate(OLD_LEVELS);
        p.routes.ways.truncate(map_routes::POINTS);
        p.routes.places.truncate(map_routes::POINTS);
        p.routes.added.retain(|&a| a < 0x80);
        let text = project_text(&p);
        let old = format!("{}end\n", &text[..text.find("entries").unwrap()]).replacen("NMB project 9", "NMB project 1", 1);
        let old: String = old.lines().filter(|l| !l.starts_with("name ") && !l.starts_with("secret ") && !l.starts_with("tables ") && !l.starts_with("enemies ")).map(|l| format!("{l}\n")).collect();
        let levels = p.levels.iter().map(|l| LevelFile { name: String::new(), secret: None, tables: None, enemies: None, ..l.clone() }).collect();
        p.points.resize(world::POINT_COUNT, world::NO_LEVEL);
        p.routes.ways.resize(map_routes::ALL, Default::default());
        p.routes.places.resize(map_routes::ALL, Place { cam_x: 0, cam_y: 0, x: 0, y: 0, rest: [0, 0] });
        assert_eq!(read_project(&old), Ok(ProjectFile { new_zones: Vec::new(), entries: Vec::new(), zone_points: Vec::new(), shuts: vec![0; added_points::MAX], start: Start::default(), world_music: 0x06, zone_music: game_music().1, levels, ..p }));
    }

    #[test]
    fn unusable_project_files_are_refused() {
        let text = project_text(&project());
        let cut = &text[..text.find("zones").unwrap()];
        assert!(matches!(read_project(cut), Err(FileError::Damaged(_))));
        assert_eq!(read_project(&level_text(&level(0), "")), Err(FileError::NotProject));
        assert!(matches!(read_project(&text.replace("levels 34", "levels 3")), Err(FileError::Damaged(_))));
    }

    #[test]
    fn cartridges_are_told_apart() {
        assert_ne!(cartridge_id(&[1, 2, 3]), cartridge_id(&[1, 2, 4]));
        assert_eq!(cartridge_id(&[9; 64]), cartridge_id(&[9; 64]));
    }
}
