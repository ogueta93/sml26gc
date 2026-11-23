//! Pictures captured from the running game: a level's blocks, the world map's tiles, and
//! what each object type looks like. The editor draws with these.
use crate::driver::GameDriver;
use crate::level_start::Start;
use crate::patch::is_expanded;
use crate::{blank_level, extra_levels};
use crate::levels::LEVELS;
use crate::live::player_position;
use crate::objects::{read, store, Obj};
use crate::ram_map::{cart_ram_offset, BLOCK_DEFS, BLOCK_DEFS_LEN, OBJECT_RECORDS};
use gb_core::{Buttons, Gb, State};

/// Shade (0-3) of pixel (x, y) of an 8x8 background tile, with the game's tile addressing.
fn tile_pixel(vram: &[u8], lcdc: u8, bgp: u8, tile: u8, x: usize, y: usize) -> u8 {
    let base = if lcdc & 0x10 != 0 { tile as usize * 16 } else { (0x1000 + (tile as i8 as i32) * 16) as usize };
    let a = base + y * 2;
    let bit = 7 - x;
    let c = ((vram[a + 1] >> bit) & 1) << 1 | ((vram[a] >> bit) & 1);
    (bgp >> (c * 2)) & 3
}

/// Everything needed to draw a level: its blocks, the block definitions and the tile
/// graphics the game loads for it, captured by letting the game itself load the level.
#[derive(Clone)]
pub struct LevelArt {
    pub metatiles: Vec<u8>, // 128 * 4 tile numbers
    pub vram: Vec<u8>,
    pub lcdc: u8,
    pub bgp: u8,
}

impl LevelArt {
    pub fn capture(gb: &Gb) -> LevelArt {
        let from = cart_ram_offset(BLOCK_DEFS);
        LevelArt {
            metatiles: gb.cart().ram()[from..from + BLOCK_DEFS_LEN].to_vec(),
            vram: gb.video().vram().to_vec(),
            lcdc: gb.video().lcdc(),
            bgp: gb.video().bgp(),
        }
    }

    /// Shade (0-3) of pixel (x, y) of block `id`.
    pub fn block_pixel(&self, id: u8, x: usize, y: usize) -> u8 {
        let q = (y / 8) * 2 + x / 8;
        let tile = self.metatiles[(id as usize & 0x7F) * 4 + q];
        tile_pixel(&self.vram, self.lcdc, self.bgp, tile, x % 8, y % 8)
    }
}

/// The tile graphics the game shows the world map with (captured from the running game).
#[derive(Clone)]
pub struct WorldArt {
    pub vram: Vec<u8>,
    pub lcdc: u8,
    pub bgp: u8,
}

impl WorldArt {
    pub fn capture(gb: &Gb) -> WorldArt {
        WorldArt { vram: gb.video().vram().to_vec(), lcdc: gb.video().lcdc(), bgp: gb.video().bgp() }
    }

    pub fn pixel(&self, tile: u8, x: usize, y: usize) -> u8 {
        tile_pixel(&self.vram, self.lcdc, self.bgp, tile, x, y)
    }
}

/// What an object type looks like in a level: shades 0-3, None = transparent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Look {
    pub w: usize,
    pub h: usize,
    pub px: Vec<Option<u8>>,
    /// Where the picture's top-left sits relative to the object's position.
    pub dx: i32,
    pub dy: i32,
    /// The sprite tile sets it uses (`level_graphics::SETS` bits): it looks the same in
    /// levels that load the same graphics into those.
    pub sets: u8,
}

fn sprites(g: &Gb) -> Vec<[u8; 4]> {
    let oam = g.video().oam();
    (0..40).map(|i| [oam[i * 4], oam[i * 4 + 1], oam[i * 4 + 2], oam[i * 4 + 3]]).filter(|s| s[0] != 0 && s[0] < 160).collect()
}

/// The start of `level` with no objects, Mario having walked right a little, and the sprites
/// over the next frames: what every capture in that level compares with, the same for every
/// object type, so it is played once. In an expanded copy the level is played as a blank level
/// made from it (`blank_level`: its graphics, catalogue and tables, a flat floor), as Mario
/// cannot walk on at the start of every level (a wall in level 18). `map_state` is a machine
/// standing on the world map; `rom`/`original` the cartridge images.
pub struct Baseline {
    /// The sprites at the end of the walk and on each frame after it.
    sprites: Vec<Vec<[u8; 4]>>,
    /// Where an object goes: just past the right edge of the screen (the game brings objects
    /// in as the view scrolls), at Mario's height.
    at: (u16, u16),
    /// Where a picture capture can skip ahead (none when the level never shows an object).
    shortcut: Option<Shortcut>,
}

/// The game reads a level's object list (`OBJECT_RECORDS`) when it loads the level, and then
/// only when the view reaches an object. Most types do nothing at load, so a capture loads
/// the level with its object from `header`, and if the memory is then as with no objects,
/// it goes on from `walked` instead (the walk with no objects `frames` frames in, before the
/// view reaches the object) with the record written there.
struct Shortcut {
    header: State,
    checked: Vec<u8>,
    /// The divider at `checked`: loading an object takes a few more cycles, and the game's
    /// random numbers come from the divider, so that difference is carried over to `walked`.
    checked_divider: u16,
    walked: State,
    frames: u32,
}

/// The one object's record and the end of the list after it.
const ONE_RECORD: std::ops::Range<usize> = cart_ram_offset(OBJECT_RECORDS)..cart_ram_offset(OBJECT_RECORDS) + 8;
const WALK_FRAMES: u32 = 150;
/// An object type that does nothing until it comes into view (a Goomba), to find when that is.
const PROBE_KIND: u8 = 0x1F;
/// Frames walked after the level loads before the memory is compared: some types (the
/// platforms of levels 13 and 22) leave it as with no objects at load but act at once.
const CHECK_FRAMES: u32 = 2;

fn walk_frame(g: &mut Gb, f: u32) {
    g.run_frame_with(if f < 130 { Buttons::RIGHT } else { Buttons::NONE });
    g.clear_audio();
}

/// The memory the game plays from (cart RAM but the object list, work RAM, high RAM, video,
/// the timer but its divider), to tell whether two consoles stand in the same place.
fn memory(g: &mut Gb) -> Vec<u8> {
    let ram = g.cart().ram();
    let mut m: Vec<u8> = ram[..ONE_RECORD.start].iter().chain(&ram[ONE_RECORD.end..]).copied().collect();
    m.extend((0xC000..0xE000u16).chain(0xFF05..0xFF08).chain(0xFF80..0xFFFF).map(|a| g.read(a)));
    m.extend_from_slice(g.video().vram());
    m.extend_from_slice(g.video().oam());
    m
}

/// The level's map and start as captures play it (see `Baseline`), with these objects.
pub fn capture_rom(rom: &[u8], original: &[u8], level: usize, objs: Vec<Obj>) -> Option<Vec<u8>> {
    if is_expanded(rom) {
        let mut e = extra_levels::based_on(rom, level as u8);
        let (map, start) = blank_level::blank(&e.map, &Start::read(&e.header));
        e.map = map;
        start.write(&mut e.header);
        e.objs = objs;
        let mut r = rom.to_vec();
        extra_levels::apply(&mut r, &extra_levels::writes_for(rom, &e));
        Some(r)
    } else {
        let mut lists: Vec<Vec<Obj>> = (0..LEVELS).map(|l| read(rom, l).0).collect();
        lists[level] = objs;
        let mut r = rom.to_vec();
        store(&mut r, original, &lists).ok()?;
        Some(r)
    }
}

fn walk(map_state: &Gb, r: &[u8], level: usize) -> Option<Gb> {
    let mut g = map_state.clone();
    g.cart_mut().set_rom(r);
    if !g.enter_level(level as u8) {
        return None;
    }
    for f in 0..WALK_FRAMES {
        walk_frame(&mut g, f);
    }
    Some(g)
}

/// `walk`, through `shortcut` when the object in `r` lets it.
fn walk_through(map_state: &Gb, r: &[u8], shortcut: &Shortcut) -> Option<Gb> {
    let mut g = map_state.clone();
    g.cart_mut().set_rom(r);
    g.set_state(&shortcut.header);
    if !g.finish_entering() {
        return None;
    }
    for f in 0..CHECK_FRAMES {
        walk_frame(&mut g, f);
    }
    let mut from = CHECK_FRAMES;
    if memory(&mut g) == shortcut.checked {
        let record = g.cart().ram()[ONE_RECORD].to_vec();
        let later = g.divider().wrapping_sub(shortcut.checked_divider);
        g.set_state(&shortcut.walked);
        g.cart_mut().ram_mut()[ONE_RECORD].copy_from_slice(&record);
        g.set_divider(g.divider().wrapping_add(later));
        from = shortcut.frames;
    }
    for f in from..WALK_FRAMES {
        walk_frame(&mut g, f);
    }
    Some(g)
}

/// The `Shortcut` for `r0` (`level` with no objects) and `r1` (with a `PROBE_KIND` at `at`),
/// and the walk with no objects.
fn shortcut(map_state: &Gb, r0: &[u8], r1: &[u8], level: usize) -> Option<(Option<Shortcut>, Gb)> {
    let mut g = map_state.clone();
    g.cart_mut().set_rom(r0);
    if !g.enter_level_header(level as u8) {
        return None;
    }
    let header = g.state();
    if !g.finish_entering() {
        return None;
    }
    let mut probe = g.clone();
    probe.cart_mut().set_rom(r1);
    probe.set_state(&header);
    let mut same = probe.finish_entering();
    let mut checked = Vec::new();
    let mut checked_divider = 0;
    let mut later = 0;
    let mut walked = None;
    let mut frames = 0;
    for f in 0..WALK_FRAMES {
        walk_frame(&mut g, f);
        if same {
            walk_frame(&mut probe, f);
            let now = memory(&mut g);
            same = memory(&mut probe) == now && (f + 1 <= CHECK_FRAMES || probe.divider().wrapping_sub(g.divider()) == later);
            if f + 1 == CHECK_FRAMES {
                checked = now;
                checked_divider = g.divider();
                later = probe.divider().wrapping_sub(g.divider());
            } else if same && f + 1 > CHECK_FRAMES {
                walked = Some(g.state());
                frames = f + 1;
            }
        }
    }
    let shortcut = walked.map(|walked| Shortcut { header, checked, checked_divider, walked, frames });
    Some((shortcut, g))
}

/// How `object_tiles` plays an object besides watching it with Mario still: Mario invincible,
/// walking right and jumping every `.0` frames for `.1` (never when `.0` is 0), so that the
/// object shows what it does when Mario moves, touches and stomps it (research `fulltiles`:
/// watching alone misses tiles of a third of the enemies, which then look cut in a set).
const PLAYS: [(u32, u32); 4] = [(0, 0), (45, 12), (30, 8), (60, 20)];
const PLAY_FRAMES: u32 = 900;

/// Frames with no object in the game's slots after which a capture stops: with one object in
/// the level and Mario only moving right or standing, once it has gone (and what it threw) it
/// does not come back.
const GONE_FRAMES: u32 = 30;

/// Counts the frames since the level last had an object out; `true` when a capture can stop.
struct Gone {
    seen: bool,
    empty: u32,
}

impl Gone {
    fn new() -> Gone {
        Gone { seen: false, empty: 0 }
    }

    fn after(&mut self, g: &Gb, stop: bool) -> bool {
        if crate::cheats::slots_in_use(g).is_empty() {
            self.empty += 1;
        } else {
            (self.seen, self.empty) = (true, 0);
        }
        stop && self.seen && self.empty >= GONE_FRAMES
    }
}

/// The tiles shown while playing `r` on `level` as `PLAYS` says (`r` made invincible); with
/// `stop`, only until its object is gone.
fn played_tiles(map_state: &Gb, r: &[u8], level: usize, (every, hold): (u32, u32), stop: bool) -> Vec<u8> {
    let mut g = map_state.clone();
    g.cart_mut().set_rom(r);
    if !g.enter_level(level as u8) {
        return Vec::new();
    }
    let mut tiles = Vec::new();
    let mut gone = Gone::new();
    for f in 0..PLAY_FRAMES {
        g.run_frame_with(if every > 0 && f % every < hold { Buttons::RIGHT | Buttons::A } else { Buttons::RIGHT });
        g.clear_audio();
        let tall = g.video().lcdc() & 0x04 != 0;
        for s in sprites(&g) {
            let t = if tall { s[2] & 0xFE } else { s[2] };
            for t in if tall { [t, t | 1].to_vec() } else { vec![t] } {
                if !tiles.contains(&t) {
                    tiles.push(t);
                }
            }
        }
        if gone.after(&g, stop) {
            break;
        }
    }
    tiles
}

/// Play `level`'s capture start with no objects and keep its sprites for `frames` frames more
/// (for `object_tiles_with`; 0 for pictures).
pub fn baseline(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, frames: u32) -> Option<Baseline> {
    let r = capture_rom(rom, original, level, Vec::new())?;
    let mut g = map_state.clone();
    g.cart_mut().set_rom(&r);
    if !g.enter_level(level as u8) {
        return None;
    }
    let (mx, my) = player_position(&mut g);
    let at = (((mx + 216) / 8) * 8, (my / 8) * 8);
    let probe = capture_rom(rom, original, level, vec![Obj { x: at.0, y: at.1, kind: PROBE_KIND, flag: false }])?;
    let (shortcut, mut g) = shortcut(map_state, &r, &probe, level)?;
    let mut sprites = vec![self::sprites(&g)];
    for _ in 0..frames {
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
        sprites.push(self::sprites(&g));
    }
    Some(Baseline { sprites, at, shortcut })
}

/// The tiles Mario shows in the `PLAYS` of `level` with no object, left out of an object's
/// (`object_tiles_with`); the same for every object of the level, so played once.
pub fn mario_tiles(map_state: &Gb, rom: &[u8], original: &[u8], level: usize) -> Vec<u8> {
    let Some(mut safe) = capture_rom(rom, original, level, Vec::new()) else { return Vec::new() };
    crate::cheats::set_invincible(&mut safe, true);
    let mut mario: Vec<u8> = PLAYS.into_iter().flat_map(|play| played_tiles(map_state, &safe, level, play, false)).collect();
    mario.sort_unstable();
    mario.dedup();
    mario
}

/// The same start as `base` with one object of type `kind` (through its shortcut when `rom` is
/// the one `base` was played with).
fn with_object(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline, same_rom: bool) -> Option<(Gb, Obj)> {
    let obj = Obj { x: base.at.0, y: base.at.1, kind, flag: false };
    let r = capture_rom(rom, original, level, vec![obj])?;
    let g = match base.shortcut.as_ref().filter(|_| same_rom) {
        Some(s) => walk_through(map_state, &r, s)?,
        None => walk(map_state, &r, level)?,
    };
    Some((g, obj))
}

/// Every sprite tile an object of type `kind` shows in `level`: over `frames` frames after it
/// came in with Mario still (see `Baseline`), and in the `PLAYS` (Mario's own tiles left out),
/// sorted. Mario is invincible, so an enemy reaching him does not change his sprites.
pub fn object_tiles(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, frames: u32) -> Vec<u8> {
    match baseline(map_state, rom, original, level, frames) {
        Some(base) => object_tiles_with(map_state, rom, original, level, kind, &base, &mario_tiles(map_state, rom, original, level)),
        None => Vec::new(),
    }
}

/// `object_tiles` against a baseline already played (as many frames as it has) and the
/// level's `mario_tiles`.
pub fn object_tiles_with(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline, mario: &[u8]) -> Vec<u8> {
    tiles_until_gone(map_state, rom, original, level, kind, base, mario, true)
}

/// `object_tiles_with`, with `stop` each play and the watch ending once the object is gone.
fn tiles_until_gone(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline, mario: &[u8], stop: bool) -> Vec<u8> {
    let mut safe = rom.to_vec();
    crate::cheats::set_invincible(&mut safe, true);
    let Some((mut g, obj)) = with_object(map_state, &safe, original, level, kind, base, false) else { return Vec::new() };
    let mut tiles = Vec::new();
    if let Some(r) = capture_rom(&safe, original, level, vec![obj]) {
        for play in PLAYS {
            tiles.extend(played_tiles(map_state, &r, level, play, stop).into_iter().filter(|t| !mario.contains(t)));
        }
    }
    let mut gone = Gone::new();
    for still in &base.sprites {
        let tall = g.video().lcdc() & 0x04 != 0;
        for s in sprites(&g).into_iter().filter(|s| !still.contains(s)) {
            let t = if tall { s[2] & 0xFE } else { s[2] };
            for t in if tall { vec![t, t | 1] } else { vec![t] } {
                if !tiles.contains(&t) {
                    tiles.push(t);
                }
            }
        }
        g.run_frame_with(Buttons::NONE);
        g.clear_audio();
        if gone.after(&g, stop) {
            break;
        }
    }
    tiles.sort_unstable();
    tiles.dedup();
    tiles
}

/// Let the game show one object of type `kind` alone beside Mario at the start of `level`,
/// and cut out the sprites it draws (compared with the same level with no objects).
pub fn capture_look(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8) -> Option<Look> {
    capture_look_with(map_state, rom, original, level, kind, &baseline(map_state, rom, original, level, 0)?)
}

/// `capture_look` against a baseline already played.
pub fn capture_look_with(map_state: &Gb, rom: &[u8], original: &[u8], level: usize, kind: u8, base: &Baseline) -> Option<Look> {
    let (mut g1, obj) = with_object(map_state, rom, original, level, kind, base, true)?;
    let still = &base.sprites[0];
    let new: Vec<[u8; 4]> = sprites(&g1).into_iter().filter(|s| !still.contains(s)).collect();
    if std::env::var("LOOK_DEBUG").is_ok() {
        eprintln!("obj {:?} rec {:02x?} base {:?} one {:?}", obj, &g1.cart().ram()[0xB00..0xB12], still, sprites(&g1));
    }
    if new.is_empty() {
        return None;
    }
    let (w, h, px, x0, y0) = cut(&g1, &new)?;
    // screen position of the object = its world position minus the camera; the camera is
    // where Mario's world X/Y minus his sprite's screen place puts it (OAM is screen + 8/16)
    let (cx, cy) = player_position(&mut g1);
    let oam = g1.video().oam();
    let cam_x = cx as i32 - (oam[1] as i32 - 8);
    let cam_y = cy as i32 - (oam[0] as i32 - 16);
    let dx = (x0 - 8 + cam_x) - obj.x as i32;
    let dy = (y0 - 16 + cam_y) - obj.y as i32;
    let tall = g1.video().lcdc() & 0x04 != 0;
    let sets = new.iter().fold(0, |m, s| m | crate::level_graphics::set_of(s[2]) | if tall { crate::level_graphics::set_of(s[2] | 1) } else { 0 });
    Some(Look { w, h, px, dx: dx.clamp(-32, 32), dy: dy.clamp(-32, 32), sets })
}

/// Mario as he stands at the start of level 0 (all the sprites then are his); `dx`/`dy`
/// from his position ($FFC2, $FFC0) to the picture's top-left.
pub fn capture_mario(map_state: &Gb) -> Option<Look> {
    let mut g = map_state.clone();
    if !g.enter_level(0) {
        return None;
    }
    let all = sprites(&g);
    let (w, h, px, x0, y0) = cut(&g, &all)?;
    let at = |g: &mut Gb, a: u16| g.read(a) as i32 | (g.read(a + 1) as i32) << 8;
    // the screen's top-left in level pixels: the camera ($FFCA, $FFC8) is its middle
    let (sx, sy) = (at(&mut g, 0xFFCA) - 0x50, at(&mut g, 0xFFC8) - 0x48);
    let dx = x0 - 8 + sx - at(&mut g, 0xFFC2);
    let dy = y0 - 16 + sy - at(&mut g, 0xFFC0);
    Some(Look { w, h, px, dx, dy, sets: 0 })
}

/// The picture the sprites `new` draw: width, height, shades, and its top-left in OAM terms.
fn cut(g: &Gb, new: &[[u8; 4]]) -> Option<(usize, usize, Vec<Option<u8>>, i32, i32)> {
    let tall = g.video().lcdc() & 0x04 != 0;
    let sh = if tall { 16 } else { 8 };
    let x0 = new.iter().map(|s| s[1] as i32).min()?;
    let y0 = new.iter().map(|s| s[0] as i32).min()?;
    let x1 = new.iter().map(|s| s[1] as i32 + 8).max()?;
    let y1 = new.iter().map(|s| s[0] as i32 + sh).max()?;
    let (w, h) = ((x1 - x0) as usize, (y1 - y0) as usize);
    if w > 64 || h > 64 {
        return None;
    }
    let vram = g.video().vram();
    let mut px = vec![None; w * h];
    for s in new {
        let mut tile = s[2] as usize;
        if tall {
            tile &= 0xFE;
        }
        let pal = g.video().obp((s[3] >> 4) & 1);
        for row in 0..sh as usize {
            let r = if s[3] & 0x40 != 0 { sh as usize - 1 - row } else { row };
            let a = tile * 16 + r * 2;
            for col in 0..8usize {
                let bit = if s[3] & 0x20 != 0 { col } else { 7 - col };
                let c = ((vram[a + 1] >> bit) & 1) << 1 | ((vram[a] >> bit) & 1);
                if c == 0 {
                    continue;
                }
                let x = (s[1] as i32 - x0) as usize + col;
                let y = (s[0] as i32 - y0) as usize + row;
                px[y * w + x] = Some((pal >> (c * 2)) & 3);
            }
        }
    }
    Some((w, h, px, x0, y0))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Levels 13 and 22 have the object types that use random numbers at once.
    #[test]
    fn shortcuts_capture_the_same_pictures() {
        let Some(rom) = std::env::var("SML2_ROM").ok().and_then(|p| std::fs::read(p).ok()) else {
            eprintln!("SML2_ROM not set: test skipped");
            return;
        };
        let mut map = Gb::new(rom.clone(), 1000).unwrap();
        map.set_unseen(true);
        assert!(map.boot_to_map());
        std::thread::scope(|scope| {
            for level in [4, 13, 22] {
                let (map, rom) = (&map, &rom);
                scope.spawn(move || {
                    let quick = baseline(map, rom, rom, level, 0).unwrap();
                    assert!(quick.shortcut.is_some(), "level {level} has no shortcut");
                    let slow = Baseline { shortcut: None, ..baseline(map, rom, rom, level, 0).unwrap() };
                    for k in 0..128u8 {
                        let look = |base: &Baseline| capture_look_with(map, rom, rom, level, k, base).map(|l| (l.w, l.h, l.px, l.dx, l.dy, l.sets));
                        assert_eq!(look(&quick), look(&slow), "level {level} type {k:#04x}");
                    }
                });
            }
        });
    }

    /// Every object of levels 7, 15 and 31 (where it matters; all levels pass). Stopping can miss tiles Mario shows after the
    /// object changed his path, never one of the enemy area (all the editor uses).
    #[test]
    fn stopping_once_the_object_is_gone_keeps_its_enemy_tiles() {
        let Some(rom) = std::env::var("SML2_ROM").ok().and_then(|p| std::fs::read(p).ok()) else {
            eprintln!("SML2_ROM not set: test skipped");
            return;
        };
        let mut map = Gb::new(rom.clone(), 1000).unwrap();
        map.set_unseen(true);
        assert!(map.boot_to_map());
        std::thread::scope(|scope| {
            for level in [7, 15, 31] {
                let (map, rom) = (&map, &rom);
                scope.spawn(move || {
                    let base = baseline(map, rom, rom, level, 600).unwrap();
                    let mario = mario_tiles(map, rom, rom, level);
                    let mut kinds: Vec<u8> = read(rom, level).0.iter().map(|o| o.kind).collect();
                    kinds.sort_unstable();
                    kinds.dedup();
                    for k in kinds {
                        let area = |stop| crate::enemy_sets::area_tiles(&tiles_until_gone(map, rom, rom, level, k, &base, &mario, stop));
                        assert_eq!(area(true), area(false), "level {level} type {k:#04x}");
                    }
                });
            }
        });
    }
}
