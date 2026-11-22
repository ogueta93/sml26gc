//! Zones beyond the game's eight, kept in the 1 MB edited copy and played in place of one of
//! the game's zones (the base), as new levels are (`extra_levels`).
//!
//! What the map code (bank $0F) reads for a zone, found with `research zonetry`:
//! * The map shown is picked from the point number ($A840, 0F:7003-0F:7146): every zone owns
//!   a block of ten numbers. A new zone's points take numbers of its base's block (`BASES`)
//!   and the numbers no code of the game uses (`SPARE`, in the Turtle and Castle blocks); while
//!   one is played, 0F:7003 calls `HOOK`, which tells the map code a spare number is the
//!   base's. Mario never goes from a new zone into the Turtle or Castle Zone (exits lead to
//!   world points or into zones a door enters), so their numbers are free there.
//! * A door enters a zone's code (`doors::INTO`), which puts Mario's start in $A842/$A843 and
//!   the entry point in $A840 (`ld a,n / ld ($A84x),a`, `entry_sites`); Mario then walks 48
//!   pixels onto the entry point (up in the Tree and Pumpkin zones, right in the others).
//! * Ways, places and levels by point (`map_routes`, `world`); the cleared flag is
//!   $A848 + 0F:650E[point], and 0F:6591[flag] the pad directions it shuts (flags up to $29).
//!   A new zone's points use its base's flags and `EXTRA_FLAGS`, which no point of the game
//!   has and no code reads by address.
//! * Pass points: the list at 0F:438D-0F:44C8 (`ld a,($A840) / cp point / call z,code`)
//!   presses a direction for some points each frame (Tree: $25 right, $27 left), so they
//!   ignore the pad and Mario walks on.
//! * Points whose flag is $05, $09, $0E, $17, $1D or $11 never open their level again once
//!   cleared (0F:418F-0F:41A1).
//! * Exits: the base's sites of the exit chain (`doors::EXITS`), each with its own pad
//!   direction.
//!
//! While a new zone is played the port writes it over its base (`writes_for`, `apply`) and
//! keeps the base's cleared flags (`flags`) apart: `progress` and `set_progress` read and
//! write them for one of the three files, in the game's working copy ($A848 + flag) and in
//! the file's save slot. The slots (0:2934 writes the one of file $A2A3) are at $A000, $A050
//! and $A0A0: a copy of $A840-$A87F, five more bytes, their sum, then 12 34 56 78 (0:28EC
//! wipes a slot whose sum or mark is wrong). Storage: bank $31, the twin of $11, which
//! the game reads only with the upper bank bit clear: "XZN1", the count, then each zone.
use crate::doors::{self, Leads, NONE};
use crate::error::CartError;
use crate::map_routes::{self, Place, Way};
use crate::patch::{is_expanded, TWIN};
use crate::rom::{offset, word, BANK_SIZE};
use crate::map_music::{self, Map};
use crate::{added_points, world, zone_maps};
use gb_core::Gb;
use std::ops::RangeInclusive;

/// A zone a new zone can be made from: one a door enters.
pub struct Base {
    /// Index into `zone_maps::ZONES`.
    pub zone: usize,
    /// Index into `doors::INTO`.
    pub door: usize,
    pub first: u8,
    pub last: u8,
    /// How far Mario walks from his start onto the entry point.
    walk: (i16, i16),
}

pub const BASES: [Base; 5] = [
    Base { zone: 0, door: 0, first: 0x1F, last: 0x27, walk: (0, -48) }, // $1E is a world point
    Base { zone: 1, door: 1, first: 0x28, last: 0x31, walk: (0, -48) },
    Base { zone: 2, door: 2, first: 0x32, last: 0x3B, walk: (48, 0) },
    Base { zone: 3, door: 3, first: 0x3C, last: 0x45, walk: (48, 0) },
    Base { zone: 5, door: 4, first: 0x50, last: 0x59, walk: (48, 0) },
];

/// Point numbers no code of the game singles out and no point of it uses (no level, no flag,
/// not in the pass point list, the door or exit chains or the map code's special cases).
pub const SPARE: [u8; 11] = [0x4C, 0x4D, 0x4E, 0x4F, 0x5D, 0x5E, 0x5F, 0x60, 0x61, 0x62, 0x63];
/// Cleared flags no point of the game has and no code reads by address.
const EXTRA_FLAGS: [u8; 9] = [0x0A, 0x0F, 0x12, 0x14, 0x18, 0x19, 0x1E, 0x21, 0x22];

impl Base {
    /// The base zone's own block.
    pub fn numbers(&self) -> RangeInclusive<u8> {
        self.first..=self.last
    }

    pub fn has(&self, p: u8) -> bool {
        self.numbers().contains(&p)
    }

    /// The numbers a new zone of this base can give its points: its block, `SPARE`, then the
    /// added points' (`added_points`).
    pub fn usable(&self) -> impl Iterator<Item = u8> + use<> {
        self.numbers().chain(SPARE).chain(added_points::numbers())
    }

    pub fn holds(&self, p: u8) -> bool {
        self.has(p) || SPARE.contains(&p) || added_points::index(p).is_some()
    }

    /// How many points a new zone of this base can have.
    pub fn room(&self) -> usize {
        self.numbers().count() + SPARE.len() + added_points::MAX
    }
}

/// The base for zone `zone` (`zone_maps::ZONES` index), if new zones can be made from it.
pub fn base_of(zone: usize) -> Option<&'static Base> {
    BASES.iter().find(|b| b.zone == zone)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZonePoint {
    pub number: u8,
    pub place: Place,
    /// Right, left, up, down.
    pub ways: [Option<Way>; 4],
    pub level: u8,
    /// Its cleared flag ($A848 + flag), one of its base's (`flags`).
    pub flag: u8,
    /// The pad directions its flag shuts (0F:6591).
    pub shut: u8,
    /// A pass point: the code its entry in the pass point list calls.
    pub pass: Option<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewZone {
    /// Index into `zone_maps::ZONES`.
    pub base: usize,
    /// 20 x 18 tiles, row by row.
    pub picture: Vec<u8>,
    pub points: Vec<ZonePoint>,
    /// The point Mario enters on.
    pub entry: u8,
    /// Per exit site of the base (`exit_sites`): the point it is on and where it leads.
    pub exits: Vec<(u8, Leads)>,
    /// The world point that enters it (`doors::NONE`: none).
    pub door: u8,
    /// Given in the editor; empty: none.
    pub name: String,
    /// The music its map plays (`map_music`).
    pub music: u8,
}

const FLAGS: u16 = 0x650E;
const SHUT: u16 = 0x6591;
const PASS_FROM: u16 = 0x438D;
const PASS_TO: u16 = 0x44C9;
const ENTRY_CODE: usize = 32;
/// 0F:7003 `ld a,($A840)`, where the map code picks the map from the point number.
const MAP_PICK: u16 = 0x7003;
/// Unused space at the end of bank $0F.
const HOOK: u16 = 0x7E00;
const STORE: usize = 0x31;
/// "XZN2": the same without music, "XZN1" without names either.
const MAGIC: &[u8; 4] = b"XZN3";
const MAGIC_UNMUSICAL: &[u8; 4] = b"XZN2";
const MAGIC_UNNAMED: &[u8; 4] = b"XZN1";
/// How many new zones there can be.
pub const MAX: usize = 32;

fn set(rom: &mut [u8], o: usize, v: u8) {
    rom[o] = v;
    if rom.len() >= 2 * TWIN && o < TWIN {
        rom[o + TWIN] = v;
    }
}

/// The cleared flags a new zone of this base can use, the bytes ($A848 + flag) kept apart
/// while it is played: its base's points' flags, then `EXTRA_FLAGS`.
pub fn flags(rom: &[u8], b: &Base) -> Vec<u8> {
    let mut v: Vec<u8> = b.numbers().map(|p| rom[offset(0x0F, FLAGS) + p as usize]).filter(|&f| f < 0x80).collect();
    v.sort_unstable();
    v.dedup();
    let extra: Vec<u8> = EXTRA_FLAGS.into_iter().filter(|f| !v.contains(f)).collect();
    v.extend(extra);
    v
}

/// The map code's hook: a spare number is read as the base's first.
fn write_hook(n: &mut [u8], b: &Base) -> Result<(), CartError> {
    let at = offset(0x0F, MAP_PICK);
    let read: [u8; 3] = n[at..at + 3].try_into().unwrap_or_default();
    if read != [0xFA, 0x40, 0xA8] && read != added_points::read_point() {
        return Err(CartError::UnknownVersion);
    }
    let list = HOOK + 19;
    let mut code = vec![read[0], read[1], read[2], 0xE5, 0x21, list as u8, (list >> 8) as u8, 0xCB, 0x7E, 0x20, 0x06, 0xBE, 0x23, 0x20, 0xF8, 0x3E, b.first, 0xE1, 0xC9];
    code.extend(SPARE);
    code.push(0xFF);
    for (i, &c) in code.iter().enumerate() {
        set(n, offset(0x0F, HOOK) + i, c);
    }
    for (i, c) in [0xCD, HOOK as u8, (HOOK >> 8) as u8].into_iter().enumerate() {
        set(n, at + i, c);
    }
    Ok(())
}

/// File offsets of the `cp` operands of the base's entries in the pass point list.
fn pass_slots(rom: &[u8], b: &Base) -> Vec<usize> {
    (offset(0x0F, PASS_FROM)..offset(0x0F, PASS_TO)).filter(|&o| rom[o - 4..o] == [0xFA, 0x40, 0xA8, 0xFE] && rom[o + 1] == 0xCC && b.has(rom[o])).collect()
}

/// File offsets of the operands that set $A842, $A843 and $A840 in the base's zone code.
fn entry_sites(rom: &[u8], b: &Base) -> Option<[usize; 3]> {
    let code = offset(0x0F, doors::INTO[b.door].code);
    let find = |lo: u8| (code..code + ENTRY_CODE).find(|&o| rom[o] == 0x3E && rom[o + 2..o + 5] == [0xEA, lo, 0xA8]).map(|o| o + 1);
    Some([find(0x42)?, find(0x43)?, find(0x40)?])
}

/// The point a door enters zone `zone` on (a zone a door enters).
pub fn entry_point(rom: &[u8], zone: usize) -> Option<u8> {
    let [_, _, entry] = entry_sites(rom, base_of(zone)?)?;
    Some(rom[entry])
}

/// `doors::EXITS` indices of the base's exits.
pub fn exit_sites(b: &Base) -> Vec<usize> {
    doors::EXITS.iter().enumerate().filter(|(_, e)| b.has(e.point)).map(|(i, _)| i).collect()
}

/// A new zone that starts as a copy of zone `zone` as this copy has it.
pub fn based_on(rom: &[u8], zone: usize) -> Option<NewZone> {
    from_parts(rom, zone, &zone_maps::read(rom, zone), &map_routes::read(rom), &world::point_levels(rom), &doors::read(rom))
}

/// A new zone that starts as a copy of zone `zone` with this picture, routes, point levels
/// and doors (as being edited); its flags, shut directions and pass points are the copy's.
pub fn from_parts(rom: &[u8], zone: usize, picture: &[u8], routes: &map_routes::MapRoutes, levels: &[u8], d: &doors::Doors) -> Option<NewZone> {
    let b = base_of(zone)?;
    let [_, _, entry] = entry_sites(rom, b)?;
    let slots = pass_slots(rom, b);
    let pass = |p: u8| slots.iter().find(|&&o| rom[o] == p).map(|&o| word(rom, o + 2));
    let reached = |p: u8| b.numbers().any(|q| routes.ways[q as usize].iter().flatten().any(|w| w.to == p));
    let points = b
        .numbers()
        .filter(|&p| routes.ways[p as usize].iter().any(|w| w.is_some()) || levels[p as usize] != world::NO_LEVEL || reached(p) || pass(p).is_some())
        .map(|p| {
            let flag = rom[offset(0x0F, FLAGS) + p as usize];
            ZonePoint { number: p, place: routes.places[p as usize], ways: routes.ways[p as usize].clone(), level: levels[p as usize], flag, shut: rom[offset(0x0F, SHUT) + flag as usize], pass: pass(p) }
        })
        .collect();
    let exits = exit_sites(b).into_iter().filter_map(|i| d.exits.get(i).copied()).collect();
    let music = map_music::read(rom, Map::Zone(zone));
    Some(NewZone { base: zone, picture: picture.to_vec(), points, entry: rom[entry], exits, door: NONE, name: String::new(), music })
}

/// The pass point list and the entry code with `z` in place of its base.
fn write_code(n: &mut [u8], rom: &[u8], b: &Base, z: &NewZone) -> Result<(), CartError> {
    let slots = pass_slots(rom, b);
    let passes: Vec<(u8, u16)> = z.points.iter().filter_map(|p| p.pass.map(|c| (p.number, c))).collect();
    if passes.len() > slots.len() {
        return Err(CartError::TooManyPassPoints { max: slots.len() });
    }
    for (k, &o) in slots.iter().enumerate() {
        let (p, code) = passes.get(k).copied().unwrap_or((NONE, word(rom, o + 2)));
        set(n, o, p);
        set(n, o + 2, code as u8);
        set(n, o + 3, (code >> 8) as u8);
    }
    let place = z.points.iter().find(|p| p.number == z.entry).ok_or(CartError::NoEntryPoint)?.place;
    write_entry(n, rom, b, z.entry, place)
}

/// The base's zone code with Mario entering on `entry` (at `place`): he starts `walk` away.
fn write_entry(n: &mut [u8], rom: &[u8], b: &Base, entry: u8, place: Place) -> Result<(), CartError> {
    let [x, y, point] = entry_sites(rom, b).ok_or(CartError::UnknownVersion)?;
    set(n, x, (place.x as i16 - b.walk.0) as u8);
    set(n, y, (place.y as i16 - b.walk.1) as u8);
    set(n, point, entry);
    Ok(())
}

/// Make the doors into zone `zone` (one a door enters) enter it on point `entry`, standing
/// at `place`.
pub fn set_entry(rom: &mut [u8], zone: usize, entry: u8, place: Place) -> Result<(), CartError> {
    let b = base_of(zone).ok_or(CartError::UnknownVersion)?;
    let original = rom.to_vec();
    write_entry(rom, &original, b, entry, place)
}

/// Where an exit leads in the copy: into a new zone is to its door point.
pub fn resolve(leads: Leads, zones: &[NewZone]) -> Leads {
    match leads {
        Leads::NewZone(k) => Leads::World(zones.get(k).map_or(NONE, |z| z.door)),
        l => l,
    }
}

/// Bytes to write over the base zone's (file offset, bytes) so that the game plays new zone
/// `k` of `zones` where it plays its base. `rom` is the copy as the game has it.
pub fn writes_for(rom: &[u8], zones: &[NewZone], k: usize) -> Result<Vec<(usize, Vec<u8>)>, CartError> {
    let z = zones.get(k).ok_or(CartError::UnknownVersion)?;
    let b = base_of(z.base).ok_or(CartError::UnknownVersion)?;
    if let Some(p) = z.points.iter().find(|p| !b.holds(p.number)) {
        return Err(CartError::PointOutsideZone { point: p.number });
    }
    let mut n = rom.to_vec();
    zone_maps::write(&mut n, b.zone, &z.picture);
    map_music::write(&mut n, Map::Zone(b.zone), z.music);
    let mut routes = map_routes::read(rom);
    let mut levels = world::point_levels(rom);
    for p in b.usable() {
        routes.ways[p as usize] = Default::default();
        levels[p as usize] = world::NO_LEVEL;
    }
    let own = flags(rom, b);
    let mut shuts = [0; added_points::MAX];
    for p in &z.points {
        routes.ways[p.number as usize] = p.ways.clone();
        routes.places[p.number as usize] = p.place;
        levels[p.number as usize] = p.level;
        if let Some(j) = added_points::index(p.number) {
            shuts[j] = p.shut;
            continue;
        }
        set(&mut n, offset(0x0F, FLAGS) + p.number as usize, p.flag);
        if own.contains(&p.flag) {
            set(&mut n, offset(0x0F, SHUT) + p.flag as usize, p.shut);
        }
    }
    map_routes::write(&mut n, &routes)?;
    world::write_point_levels(&mut n, &levels);
    if added_points::installed(&n) {
        let first = added_points::FIRST as usize;
        added_points::write(&mut n, &levels[first..], &shuts, zone_maps::ZONES[b.zone].point);
    } else if z.points.iter().any(|p| added_points::index(p.number).is_some()) {
        return Err(CartError::UnknownVersion);
    }
    write_code(&mut n, rom, b, z)?;
    write_hook(&mut n, b)?;
    let mut d = doors::read(rom);
    if d.exits.is_empty() {
        return Err(CartError::UnknownVersion);
    }
    for (j, i) in exit_sites(b).into_iter().enumerate() {
        d.exits[i] = z.exits.get(j).map_or((NONE, Leads::World(NONE)), |&(at, leads)| (at, resolve(leads, zones)));
    }
    if z.door != NONE {
        d.set_door(z.door, Some(b.door));
    }
    doors::write(&mut n, &d);
    Ok(runs(rom, &n))
}

pub use crate::extra_levels::apply;

/// The file being played (0-2).
pub const CURRENT_FILE: u16 = 0xA2A3;
const FLAG_BYTES: u16 = 0xA848;
const SLOTS: [u16; 3] = [0xA000, 0xA050, 0xA0A0];
const SLOT_SUM: u16 = 0x45;
const SLOT_MARK: [u8; 4] = [0x12, 0x34, 0x56, 0x78];

fn slot(g: &mut Gb, file: u8) -> Option<u16> {
    let s = *SLOTS.get(file as usize)?;
    (0..4).all(|i| g.read(s + SLOT_SUM + 1 + i) == SLOT_MARK[i as usize]).then_some(s)
}

/// The bytes of cleared `flags` of file `file`: the working copy if it is the file being
/// played, else its save slot (zeros if it has none).
pub fn progress(g: &mut Gb, file: u8, flags: &[u8]) -> Vec<u8> {
    let at = if g.read(CURRENT_FILE) == file { Some(FLAG_BYTES) } else { slot(g, file).map(|s| s + 8) };
    flags.iter().map(|&f| at.map_or(0, |a| g.read(a + f as u16))).collect()
}

/// Write the bytes of cleared `flags` of file `file` into its save slot (keeping its sum
/// right) and, if it is the file being played, into the working copy.
pub fn set_progress(g: &mut Gb, file: u8, flags: &[u8], values: &[u8]) {
    if g.read(CURRENT_FILE) == file {
        for (&f, &v) in flags.iter().zip(values) {
            g.write(FLAG_BYTES + f as u16, v);
        }
    }
    if let Some(s) = slot(g, file) {
        for (&f, &v) in flags.iter().zip(values) {
            g.write(s + 8 + f as u16, v);
        }
        let sum = (0..SLOT_SUM).fold(0u8, |c, i| c.wrapping_add(g.read(s + i)));
        g.write(s + SLOT_SUM, sum);
    }
}

/// The bytes of `b` that differ from `a`, as runs (file offset, bytes).
pub(crate) fn runs(a: &[u8], b: &[u8]) -> Vec<(usize, Vec<u8>)> {
    let mut v: Vec<(usize, Vec<u8>)> = Vec::new();
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        if x == y {
            continue;
        }
        match v.last_mut() {
            Some((at, bytes)) if *at + bytes.len() + 4 > i => bytes.extend_from_slice(&b[*at + bytes.len()..=i]),
            _ => v.push((i, vec![*y])),
        }
    }
    v
}

fn encode(z: &NewZone) -> Result<Vec<u8>, CartError> {
    let name = crate::extra_levels::clean_name(&z.name);
    let mut v = vec![name.len() as u8];
    v.extend_from_slice(name.as_bytes());
    v.extend_from_slice(&[z.base as u8, z.door, z.entry, z.music]);
    v.extend_from_slice(&z.picture[..zone_maps::W * zone_maps::H]);
    v.push(z.exits.len() as u8);
    for &(p, leads) in &z.exits {
        v.extend_from_slice(&match leads {
            Leads::World(w) => [p, 0, w],
            Leads::Zone(i) => [p, 1, i as u8],
            Leads::NewZone(k) => [p, 2, k as u8],
        });
    }
    v.push(z.points.len() as u8);
    for p in &z.points {
        encode_point(p, &mut v)?;
    }
    Ok(v)
}

pub(crate) fn encode_point(p: &ZonePoint, v: &mut Vec<u8>) -> Result<(), CartError> {
    v.push(p.number);
    v.extend_from_slice(&p.place.bytes());
    let pass = p.pass.unwrap_or(0xFFFF);
    v.extend_from_slice(&[p.level, p.flag, p.shut, pass as u8, (pass >> 8) as u8]);
    for w in &p.ways {
        match w {
            Some(w) if w.steps.len() < 0xFF => {
                v.push(w.steps.len() as u8);
                v.extend_from_slice(&w.steps);
                v.push(w.to);
            }
            Some(w) => return Err(CartError::RoutesFull { need: w.steps.len(), capacity: 0xFE }),
            None => v.push(0xFF),
        }
    }
    Ok(())
}

pub(crate) fn decode_point(r: &[u8], i: &mut usize) -> Option<ZonePoint> {
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = r.get(*i..*i + n)?;
        *i += n;
        Some(s)
    };
    let number = take(1)?[0];
    let place = Place::from(take(8)?);
    let f = take(5)?.to_vec();
    let mut ways: [Option<Way>; 4] = Default::default();
    for w in ways.iter_mut() {
        let len = take(1)?[0];
        if len != 0xFF {
            let steps = take(len as usize)?.to_vec();
            *w = Some(Way { steps, to: take(1)?[0] });
        }
    }
    let pass = f[3] as u16 | (f[4] as u16) << 8;
    Some(ZonePoint { number, place, ways, level: f[0], flag: f[1], shut: f[2], pass: (pass != 0xFFFF).then_some(pass) })
}

/// One stored zone; `music` (its base's music in the copy) when the format has none.
fn decode(r: &[u8], i: &mut usize, named: bool, music: Option<&dyn Fn(usize) -> u8>) -> Option<NewZone> {
    let mut take = |n: usize| -> Option<&[u8]> {
        let s = r.get(*i..*i + n)?;
        *i += n;
        Some(s)
    };
    let name = if named {
        let n = take(1)?[0] as usize;
        crate::extra_levels::name_from(take(n)?)
    } else {
        String::new()
    };
    let mut h = take(3)?.to_vec();
    h.push(match music {
        Some(base_music) => base_music(h[0] as usize),
        None => take(1)?[0],
    });
    let picture = take(zone_maps::W * zone_maps::H)?.to_vec();
    let n = take(1)?[0] as usize;
    let mut exits = Vec::new();
    for _ in 0..n {
        let e = take(3)?;
        exits.push((e[0], match e[1] {
            1 => Leads::Zone(e[2] as usize),
            2 => Leads::NewZone(e[2] as usize),
            _ => Leads::World(e[2]),
        }));
    }
    let n = take(1)?[0] as usize;
    let mut points = Vec::new();
    for _ in 0..n {
        points.push(decode_point(r, i)?);
    }
    base_of(h[0] as usize)?;
    Some(NewZone { base: h[0] as usize, picture, points, entry: h[2], exits, door: h[1], name, music: h[3] })
}

/// The new zones kept in this copy (none in a copy that has no room for them).
pub fn read(rom: &[u8]) -> Vec<NewZone> {
    let at = offset(STORE, 0x4000);
    if !is_expanded(rom) {
        return Vec::new();
    }
    let magic = &rom[at..at + 4];
    if magic != MAGIC && magic != MAGIC_UNMUSICAL && magic != MAGIC_UNNAMED {
        return Vec::new();
    }
    let bank = &rom[at..at + BANK_SIZE];
    let base_music = |base: usize| map_music::read(rom, Map::Zone(base));
    let music: Option<&dyn Fn(usize) -> u8> = (magic != MAGIC).then_some(&base_music);
    let mut i = 5;
    (0..(bank[4] as usize).min(MAX)).map_while(|_| decode(bank, &mut i, magic != MAGIC_UNNAMED, music)).collect()
}

/// Write the new zones into the copy (replacing the ones it had). Returns the bytes used.
pub fn store(rom: &mut [u8], zones: &[NewZone]) -> Result<usize, CartError> {
    if !is_expanded(rom) {
        return Err(CartError::UnknownVersion);
    }
    if zones.len() > MAX {
        return Err(CartError::TooManyZones { max: MAX });
    }
    let mut data = vec![];
    data.extend_from_slice(MAGIC);
    data.push(zones.len() as u8);
    for z in zones {
        data.extend(encode(z)?);
    }
    if data.len() > BANK_SIZE {
        return Err(CartError::ZonesFull { need: data.len(), capacity: BANK_SIZE });
    }
    let at = offset(STORE, 0x4000);
    rom[at..at + BANK_SIZE].fill(0xFF);
    rom[at..at + data.len()].copy_from_slice(&data);
    Ok(data.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_routes::{LEFT, RIGHT};
    use crate::patch::EXPANDED_SIZE;

    fn place(x: u8, y: u8) -> Place {
        Place { cam_x: 17, cam_y: 242, x, y, rest: [0, 0] }
    }

    fn zone(steps: usize) -> NewZone {
        let way = |d: u8, to: u8| Some(Way { steps: vec![d; steps], to });
        NewZone {
            base: 0,
            picture: (0..360).map(|i| i as u8).collect(),
            points: vec![
                ZonePoint { number: 0x23, place: place(48, 110), ways: [way(RIGHT, 0x25), None, None, None], level: 1, flag: 1, shut: 0x10, pass: None },
                ZonePoint { number: 0x25, place: place(72, 110), ways: [None, way(LEFT, 0x23), None, None], level: 40, flag: 36, shut: 0, pass: Some(0x429A) },
            ],
            entry: 0x23,
            exits: vec![(0x23, Leads::World(0x07))],
            door: 0x07,
            name: "Bosque".into(),
            music: 0x28,
        }
    }

    fn expanded() -> Vec<u8> {
        let mut r = vec![0u8; EXPANDED_SIZE];
        r[0x037C] = 0x01;
        r[0x39BD..0x39BD + 11].copy_from_slice(&[0x7C, 0xFE, 0xE0, 0xC2, 0x86, 0x03, 0xAF, 0xEA, 0x00, 0x40, 0xC9]);
        r
    }

    /// An expanded-looking copy with the Tree Zone's entry code and pass point entries.
    fn fake() -> Vec<u8> {
        let mut r = expanded();
        let code = offset(0x0F, 0x50D7);
        r[code..code + 15].copy_from_slice(&[0x3E, 0x30, 0xEA, 0x42, 0xA8, 0x3E, 0x9E, 0xEA, 0x43, 0xA8, 0x3E, 0x23, 0xEA, 0x40, 0xA8]);
        for (k, (p, call)) in [(0x25u8, 0x429Au16), (0x14, 0x42AC), (0x27, 0x42A0)].into_iter().enumerate() {
            let o = offset(0x0F, PASS_FROM) + k * 8;
            r[o..o + 8].copy_from_slice(&[0xFA, 0x40, 0xA8, 0xFE, p, 0xCC, call as u8, (call >> 8) as u8]);
        }
        r
    }

    #[test]
    fn stored_zones_read_back() {
        let mut rom = expanded();
        assert!(read(&rom).is_empty());
        let (a, mut b) = (zone(3), zone(5));
        b.base = 2;
        b.door = NONE;
        b.exits[0].1 = Leads::NewZone(0);
        let used = store(&mut rom, &[a.clone(), b.clone()]).unwrap();
        assert!(used > 2 * 360);
        assert_eq!(read(&rom), vec![a, b]);
    }

    #[test]
    fn zones_stored_without_music_play_their_bases() {
        let mut rom = expanded();
        map_music::write(&mut rom, Map::Zone(2), 0x05);
        let (mut a, mut b) = (zone(3), zone(4));
        b.base = 2;
        store(&mut rom, &[a.clone(), b.clone()]).unwrap();
        let at = offset(STORE, 0x4000);
        let mut old = MAGIC_UNMUSICAL.to_vec();
        old.push(2);
        for z in [&a, &b] {
            let mut e = encode(z).unwrap();
            e.remove(1 + z.name.len() + 3);
            old.extend(e);
        }
        rom[at..at + old.len()].copy_from_slice(&old);
        (a.music, b.music) = (map_music::read(&rom, Map::Zone(0)), 0x05);
        assert_eq!(read(&rom), vec![a, b]);
    }

    #[test]
    fn a_full_bank_is_refused() {
        let mut rom = expanded();
        let big = zone(200);
        assert!(matches!(store(&mut rom, &vec![big; 25]), Err(CartError::ZonesFull { .. })));
        assert_eq!(store(&mut rom, &vec![zone(1); MAX + 1]), Err(CartError::TooManyZones { max: MAX }));
    }

    #[test]
    fn entry_and_pass_points_take_the_bases_slots() {
        let rom = fake();
        let mut n = rom.clone();
        let mut z = zone(3);
        write_code(&mut n, &rom, &BASES[0], &z).unwrap();
        let code = offset(0x0F, 0x50D7);
        // Mario starts 48 pixels below the entry point, which is $23
        assert_eq!([n[code + 1], n[code + 6], n[code + 11]], [48, 158, 0x23]);
        let o = offset(0x0F, PASS_FROM);
        assert_eq!(n[o + 4..o + 8], [0x25, 0xCC, 0x9A, 0x42]);
        assert_eq!(n[o + 8 + 4], 0x14, "another zone's entry stays");
        assert_eq!(n[o + 16 + 4], NONE, "a pass slot the new zone does not use matches no point");
        assert_eq!(n[o + 4 + TWIN], 0x25);
        z.points[0].pass = Some(0x42A0);
        z.points.push(ZonePoint { pass: Some(0x42A6), number: 0x26, ..z.points[1].clone() });
        assert_eq!(write_code(&mut n, &rom, &BASES[0], &z), Err(CartError::TooManyPassPoints { max: 2 }));
        z.points.pop();
        z.entry = 0x24;
        assert_eq!(write_code(&mut n, &rom, &BASES[0], &z), Err(CartError::NoEntryPoint));
    }

    #[test]
    fn writes_and_their_undo_give_the_copy_back() {
        let a: Vec<u8> = (0..4096).map(|i| (i * 7 % 256) as u8).collect();
        let mut b = a.clone();
        for i in [3, 4, 6, 600, 4095] {
            b[i] ^= 0x5A;
        }
        let w = runs(&a, &b);
        assert_eq!(w.len(), 3);
        let mut r = a.clone();
        let undo = apply(&mut r, &w);
        assert!(r == b);
        apply(&mut r, &undo);
        assert!(r == a);
    }

    #[test]
    fn points_outside_the_bases_block_are_refused() {
        let rom = fake();
        let mut z = zone(3);
        z.points[1].number = 0x28;
        assert_eq!(writes_for(&rom, &[z], 0), Err(CartError::PointOutsideZone { point: 0x28 }));
    }
}
