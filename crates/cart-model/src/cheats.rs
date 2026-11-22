//! Cheats on a running game: what each one writes, found by reverse engineering
//! (`tools/research`, `cheats.rs`). Nothing here touches the cartridge file; `set_invincible`
//! swaps bytes of the program in the console's memory only.
//!
//! - Mario: power-up $A216 (0 small, 1 big, 2 carrot, 3 fire), facing $A22B (1 right, $FF
//!   left), hurt flag $A224 ($0F hurt, $80 deadly).
//! - Enemy contact hurts him at 0:2E95 (`ld a,$0F; ld ($A224),a`) inside a routine whose
//!   other way out is a `ret` (0:2F71). Spikes (blocks $50/$51) hurt at 0:1C58 and kill at
//!   0:1C75 when he is low on the screen, then both go on at 0:1D71. Pits are apart (0:3174).
//! - Active objects: 16 records of $20 bytes from $AD00, free when byte +$08 is 0 (02:4D41
//!   looks for one that way). An item out of its block is type $11 (+$05) with +$12 =
//!   $80 | kind and look +$0B = $18 + kind; touching it puts the kind in $A26C, applied by
//!   0:3451. +$06/+$07 point at the state byte of the level object record it came from,
//!   where the game writes $02 once taken and $10 once off the screen.
//! - Level object records: 6 bytes from $AB06 (X page, X, state, Y page, Y, type), ended by
//!   a record whose X page is $FF.
//! - The goal bell (kind 6) sets $A24F = $0C: going back to the map then plays the bonus
//!   game. The goal block (0:1CC3-0:1CF6) does the rest of `END_WRITES` and mode $18; the
//!   secret exit block $4B also sets $A2B4 = $FF, and the map (0F:6F56) then takes Mario
//!   where the secret exit leads.
//! - Timer $A254 (BCD, two low digits) and $A255 (hundreds); coins $A262 (BCD) and $A263
//!   (hundreds); lives $A22C (BCD).
//! - The golden coins are the cleared bit (7) of the six boss levels' flags at $A848 + flag
//!   number: Wario's castle door (0C:45D7) checks those six.
//! - A point's flag number is 0F:650E[point] (below $30; $FE, $FF: none). Finishing its level
//!   sets bit 7, its secret exit bit 0 (`flagsprobe`); the paths open from them. The added
//!   points of the map shown keep theirs at `added_points::FLAG_RAM`.
//! - Active objects: type +$05, x +$00/+$01, y +$03/+$04 (big-endian); the camera ($FFC8 y,
//!   $FFCA x, little-endian) is the middle of the screen (SCX = x - $50, SCY = y - $48).
use crate::level_goals::{self, Goal};
use crate::live::map_in_memory;
use crate::ram_map::{GameMode, GAME_MODE, LEVEL_NUMBER, LIVES, MAP_POINT, PLAYER_X, PLAYER_Y, SPECIAL_MAP_POINTS};
use crate::{added_points, levels, rom, world, GameDriver};
use gb_core::{BusObserver, GameBoy};

/// Program bytes in bank 0 swapped while Mario is invincible: (address, game's, cheat's).
pub const INVINCIBLE_PATCHES: [(usize, &[u8], &[u8]); 2] = [
    // enemy contact returns before hurting
    (0x2E95, &[0x3E], &[0xC9]),
    // spikes: jp $1D71, as ground
    (0x1C58, &[0x3E, 0x0F, 0xEA], &[0xC3, 0x71, 0x1D]),
];

const TIMER: u16 = 0xA254;
const COINS: u16 = 0xA262;
const FACING: u16 = 0xA22B;
const CLEARED_FLAGS: u16 = 0xA848;
const CLEARED: u8 = 0x80;
const SLOTS: u16 = 0xAD00;
const CAMERA_Y: u16 = 0xFFC8;
const CAMERA_X: u16 = 0xFFCA;
const POINT_FLAG_TABLE: u16 = 0x650E;
const POINT_FLAGS_USED: usize = 0x80;
const SLOT_LEN: u16 = 0x20;
const SLOT_COUNT: u16 = 16;
const SLOT_USED: u16 = 0x08;
const OBJECT_RECORDS: u16 = 0xAB06;
const RECORD_LEN: u16 = 6;
const RECORDS_END: u16 = SLOTS;
/// An item out of its block (taken from a mushroom that came out of a block in level 13);
/// position, kind, look and record pointer are filled in by `summon`.
const ITEM: [u8; SLOT_LEN as usize] = [0x01, 0x48, 0x10, 0x01, 0x97, 0x11, 0xAB, 0x08, 0x02, 0x35, 0x46, 0x19, 0x80, 0, 0, 0x01, 0, 0, 0x81, 0xF3, 0xFD, 0xFA, 0x06, 0, 0x01, 0, 0, 0, 0, 0, 0, 0];
/// How far ahead of Mario a summoned item stands, in pixels.
const AHEAD: u16 = 0x20;
/// What a goal block writes before mode $18, besides the bell and the secret exit flags.
const END_WRITES: [(u16, u8); 9] = [(0xA53F, 0), (0xA468, 0x03), (0xA460, 0), (0xA478, 0), (0xA470, 0), (0xA266, 0xC0), (0xA267, 0), (0xA2A0, 0), (0xA299, 0)];
const BELL_RUNG: u16 = 0xA24F;
const SECRET_EXIT: u16 = 0xA2B4;
const LEVEL_END: u8 = 0x18;

/// The items that can be summoned, as the game numbers their kinds ($A26C).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Mushroom = 1,
    Flower = 2,
    Carrot = 3,
    Star = 4,
    Heart = 5,
}

impl Item {
    pub const ALL: [Item; 5] = [Item::Star, Item::Mushroom, Item::Flower, Item::Carrot, Item::Heart];

    pub const fn kind(self) -> u8 {
        self as u8
    }

    /// Its 8x8 sprite tiles at $8000 (the same in every level): top left, top right, bottom
    /// left, bottom right, with whether each is drawn mirrored.
    pub const fn tiles(self) -> [(u8, bool); 4] {
        match self {
            Item::Mushroom => [(0x90, false), (0x91, false), (0x92, false), (0x92, true)],
            Item::Flower => [(0x97, false), (0x97, true), (0x98, false), (0x98, true)],
            Item::Carrot => [(0xA0, false), (0xA1, false), (0xA2, false), (0xA2, true)],
            Item::Star => [(0x93, false), (0x93, true), (0x94, false), (0x94, true)],
            Item::Heart => [(0xA3, false), (0xA3, true), (0xA4, false), (0xA4, true)],
        }
    }
}

/// What the info panel shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Info {
    pub level: u8,
    pub x: u16,
    pub y: u16,
    pub point: u8,
    pub mode: u8,
    /// Active object slots in use.
    pub objects: u8,
}

/// An active object: its type and where it is on the screen (pixels, may be off it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Active {
    pub kind: u8,
    pub x: i32,
    pub y: i32,
}

fn word<O: BusObserver>(g: &mut GameBoy<O>, a: u16) -> u16 {
    g.read(a) as u16 | (g.read(a + 1) as u16) << 8
}

/// Cartridge RAM written as kept, whether the game has it switched on or not.
fn set_ram<O: BusObserver>(g: &mut GameBoy<O>, a: u16, v: u8) {
    if let Some(b) = g.cart_mut().ram_mut().get_mut((a - 0xA000) as usize) {
        *b = v;
    }
}

fn ram<O: BusObserver>(g: &GameBoy<O>, a: u16) -> u8 {
    g.cart().ram().get((a - 0xA000) as usize).copied().unwrap_or(0)
}

/// Mario is in a level, playing.
pub fn in_level<O: BusObserver>(g: &mut GameBoy<O>) -> bool {
    g.read(GAME_MODE) == GameMode::Playing.byte()
}

/// Swap the invincible patches in or out of the program. False (and nothing changed) when
/// the program does not have the game's bytes or the cheat's there.
pub fn set_invincible(rom: &mut [u8], on: bool) -> bool {
    let fits = INVINCIBLE_PATCHES.iter().all(|&(at, game, cheat)| rom.get(at..at + game.len()).is_some_and(|b| b == game || b == cheat));
    if fits {
        for (at, game, cheat) in INVINCIBLE_PATCHES {
            rom[at..at + game.len()].copy_from_slice(if on { cheat } else { game });
        }
    }
    fits
}

/// End the level as the goal with its bell rung does: the bonus game, then the map. Only
/// while playing a level.
pub fn finish_level<O: BusObserver>(g: &mut GameBoy<O>) -> bool {
    in_level(g) && end_level(g, Goal::Finish)
}

/// The level being played has a secret exit (block $4B in its map).
pub fn has_secret_exit<O: BusObserver>(g: &mut GameBoy<O>) -> bool {
    in_level(g) && !level_goals::cells(&map_in_memory(g), Goal::Secret).is_empty()
}

/// End the level through its secret exit, as touching block $4B does. Only in a level that
/// has one.
pub fn take_secret_exit<O: BusObserver>(g: &mut GameBoy<O>) -> bool {
    has_secret_exit(g) && end_level(g, Goal::Secret)
}

fn end_level<O: BusObserver>(g: &mut GameBoy<O>, goal: Goal) -> bool {
    for (a, v) in END_WRITES {
        set_ram(g, a, v);
    }
    set_ram(g, BELL_RUNG, if goal == Goal::Finish { 0x0C } else { 0 });
    set_ram(g, SECRET_EXIT, if goal == Goal::Secret { 0xFF } else { 0 });
    g.write(GAME_MODE, LEVEL_END);
    true
}

/// Put `item` in a free active object slot two blocks ahead of Mario, at his height. Its
/// record pointer is the end record's state byte, which nothing reads, so taking it or
/// leaving it behind marks no object of the level. False when not playing or no slot is free.
pub fn summon<O: BusObserver>(g: &mut GameBoy<O>, item: Item) -> bool {
    if !in_level(g) {
        return false;
    }
    let Some(slot) = (0..SLOT_COUNT).map(|s| SLOTS + s * SLOT_LEN).find(|&s| ram(g, s + SLOT_USED) == 0) else { return false };
    let Some(end) = (0..).map(|n| OBJECT_RECORDS + n * RECORD_LEN).take_while(|&r| r + RECORD_LEN <= RECORDS_END).find(|&r| ram(g, r) == 0xFF) else { return false };
    let (x, y) = (word(g, PLAYER_X), word(g, PLAYER_Y));
    let x = if g.read(FACING) == 0xFF { x.saturating_sub(AHEAD) } else { x + AHEAD };
    let mut b = ITEM;
    [b[0], b[1]] = x.to_be_bytes();
    [b[3], b[4]] = y.to_be_bytes();
    [b[6], b[7]] = (end + 2).to_be_bytes();
    b[0x0B] = 0x18 + item.kind();
    b[0x12] = 0x80 | item.kind();
    for (k, v) in (0..).zip(b) {
        set_ram(g, slot + k, v);
    }
    true
}

pub fn max_lives<O: BusObserver>(g: &mut GameBoy<O>) {
    set_lives(g, 99);
}

/// 999 coins.
pub fn max_coins<O: BusObserver>(g: &mut GameBoy<O>) {
    set_coins(g, 999);
}

/// `n` lives (at most 99; BCD, as the game counts them).
pub fn set_lives<O: BusObserver>(g: &mut GameBoy<O>, n: u8) {
    let n = n.min(99);
    set_ram(g, LIVES, (n / 10) << 4 | n % 10);
}

/// `n` coins (at most 999; BCD, the two low digits first).
pub fn set_coins<O: BusObserver>(g: &mut GameBoy<O>, n: u16) {
    let n = n.min(999);
    set_ram(g, COINS, ((n / 10 % 10) << 4 | n % 10) as u8);
    set_ram(g, COINS + 1, (n / 100) as u8);
}

/// The six golden coins: the six bosses marked beaten, as the game marks them.
pub fn golden_coins<O: BusObserver>(g: &mut GameBoy<O>) {
    for flag in SPECIAL_MAP_POINTS {
        let a = CLEARED_FLAGS + flag as u16;
        let v = ram(g, a) | CLEARED;
        set_ram(g, a, v);
    }
}

/// The six golden coins are all won.
pub fn has_golden_coins<O: BusObserver>(g: &GameBoy<O>) -> bool {
    SPECIAL_MAP_POINTS.iter().all(|&flag| ram(g, CLEARED_FLAGS + flag as u16) & CLEARED != 0)
}

/// The level timer as the game keeps it (two low BCD digits, hundreds).
pub fn timer<O: BusObserver>(g: &GameBoy<O>) -> [u8; 2] {
    [ram(g, TIMER), ram(g, TIMER + 1)]
}

pub fn set_timer<O: BusObserver>(g: &mut GameBoy<O>, t: [u8; 2]) {
    set_ram(g, TIMER, t[0]);
    set_ram(g, TIMER + 1, t[1]);
}

/// The coins as a number (0-999).
pub fn coins<O: BusObserver>(g: &GameBoy<O>) -> u16 {
    bcd(ram(g, COINS + 1)) as u16 * 100 + bcd(ram(g, COINS)) as u16
}

/// The lives as a number.
pub fn lives<O: BusObserver>(g: &GameBoy<O>) -> u8 {
    bcd(ram(g, LIVES))
}

pub const fn bcd(b: u8) -> u8 {
    (b >> 4) * 10 + (b & 0x0F)
}

pub fn info<O: BusObserver>(g: &mut GameBoy<O>) -> Info {
    let objects = slots_in_use(g).len() as u8;
    Info { level: g.read(LEVEL_NUMBER), x: word(g, PLAYER_X), y: word(g, PLAYER_Y), point: g.read(MAP_POINT), mode: g.read(GAME_MODE), objects }
}

pub(crate) fn slots_in_use<O: BusObserver>(g: &GameBoy<O>) -> Vec<u16> {
    (0..SLOT_COUNT).map(|s| SLOTS + s * SLOT_LEN).filter(|&s| ram(g, s + SLOT_USED) != 0).collect()
}

/// The active objects of the level being played, on the screen's coordinates.
pub fn active_objects<O: BusObserver>(g: &mut GameBoy<O>) -> Vec<Active> {
    if !in_level(g) {
        return Vec::new();
    }
    let (cam_x, cam_y) = (word(g, CAMERA_X) as i32 - 0x50, word(g, CAMERA_Y) as i32 - 0x48);
    slots_in_use(g)
        .into_iter()
        .map(|s| {
            let x = (ram(g, s) as i32) << 8 | ram(g, s + 1) as i32;
            let y = (ram(g, s + 3) as i32) << 8 | ram(g, s + 4) as i32;
            Active { kind: ram(g, s + 5), x: x - cam_x, y: y - cam_y }
        })
        .collect()
}

/// Every level of the copy's points marked as the game marks a finished level (and its
/// secret exit taken where it has one), and the added points of the map shown: all paths
/// open.
pub fn open_map<O: BusObserver>(g: &mut GameBoy<O>) {
    let image = g.cart().rom();
    let point_levels = world::point_levels(image);
    let flags = rom::offset(0x0F, POINT_FLAG_TABLE);
    let secret: Vec<bool> = (0..levels::LEVELS)
        .map(|l| {
            let map = levels::decode(image, levels::map_location(image, levels::header(image, l)).1).0;
            !level_goals::cells(&map, Goal::Secret).is_empty()
        })
        .collect();
    let marks: Vec<(u16, u8)> = (0..POINT_FLAGS_USED)
        .filter_map(|p| {
            let f = image[flags + p];
            (f < 0x30).then(|| {
                let has_secret = point_levels.get(p).is_some_and(|&l| secret.get(l as usize).copied().unwrap_or(false));
                (CLEARED_FLAGS + f as u16, if has_secret { CLEARED | 1 } else { CLEARED })
            })
        })
        .collect();
    for (a, bits) in marks {
        let v = ram(g, a) | bits;
        set_ram(g, a, v);
    }
    for i in 0..added_points::MAX as u16 {
        let v = ram(g, added_points::FLAG_RAM + i) | CLEARED;
        set_ram(g, added_points::FLAG_RAM + i, v);
    }
}

/// Leave the level being played as the game's pause menu does (no life lost) and wait for
/// the map. False when the map did not come up.
pub fn to_map<O: BusObserver>(g: &mut GameBoy<O>) -> bool {
    if in_level(g) {
        g.back_to_map();
        for _ in 0..900 {
            g.run_frame_with(gb_core::Buttons::NONE);
            if g.mode() == GameMode::Map {
                break;
            }
        }
    }
    g.mode() == GameMode::Map
}

/// Leave the level being played (as the game's pause menu does) and enter `level` (0-31)
/// from its own map point when the maps have one. False when it could not be entered.
pub fn go_to_level<O: BusObserver>(g: &mut GameBoy<O>, level: u8) -> bool {
    if !to_map(g) {
        return false;
    }
    if let Some(p) = world::point_levels(g.cart().rom()).iter().position(|&l| l == level) {
        g.show_map_at(p as u8);
    }
    g.enter_level(level)
}

/// `item`'s 16x16 picture from video memory in the sprite palette's shades (0-3), `None`
/// where it is see-through.
pub fn item_picture<O: BusObserver>(g: &mut GameBoy<O>, item: Item) -> [Option<u8>; 256] {
    let palette = g.read(0xFF48);
    let mut out = [None; 256];
    for (i, (tile, mirrored)) in item.tiles().into_iter().enumerate() {
        let base = 0x8000 + tile as u16 * 16;
        for row in 0..8u16 {
            let (lo, hi) = (g.read(base + row * 2), g.read(base + row * 2 + 1));
            for col in 0..8u16 {
                let bit = if mirrored { col } else { 7 - col };
                let colour = (lo >> bit) & 1 | ((hi >> bit) & 1) << 1;
                if colour != 0 {
                    let (x, y) = ((i as u16 % 2) * 8 + col, (i as u16 / 2) * 8 + row);
                    out[(y * 16 + x) as usize] = Some((palette >> (colour * 2)) & 3);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bcd_reads_the_game_counters() {
        assert_eq!(bcd(0x99), 99);
        assert_eq!(bcd(0x05), 5);
        assert_eq!(bcd(0x42), 42);
    }

    #[test]
    fn invincible_swaps_only_the_game_bytes_and_back() {
        let mut rom = vec![0u8; 0x8000];
        assert!(!set_invincible(&mut rom, true));
        assert_eq!(rom, vec![0u8; 0x8000]);
        for (at, game, _) in INVINCIBLE_PATCHES {
            rom[at..at + game.len()].copy_from_slice(game);
        }
        let game = rom.clone();
        assert!(set_invincible(&mut rom, true));
        assert_eq!(&rom[0x2E95..0x2E96], &[0xC9]);
        assert_eq!(&rom[0x1C58..0x1C5B], &[0xC3, 0x71, 0x1D]);
        assert!(set_invincible(&mut rom, true));
        assert!(set_invincible(&mut rom, false));
        assert_eq!(rom, game);
    }

    #[test]
    fn items_are_the_game_kinds() {
        assert_eq!(Item::ALL.map(Item::kind), [4, 1, 2, 3, 5]);
    }
}
