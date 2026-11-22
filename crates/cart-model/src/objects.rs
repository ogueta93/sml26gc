//! Enemies and other objects placed in levels, as the cartridge stores them (read from
//! routine 03:6BC7, which the game calls when a level starts):
//!
//! * Bank 3, word table at $6037, one entry per level number ($A269) → a list of 3-byte
//!   objects ended by $FF.
//! * Object bytes b, c, d:
//!   - X = (b & $0F) · 256 + (c & $1F) · 8 pixels
//!   - Y = ((d >> 5) & 3) · 256 + (d & $1F) · 8 pixels
//!   - d bit 7: a flag the game keeps with the object
//!   - type (0-127) = b.4→bit6, b.7→bit5, b.6→bit4, b.5→bit3, c bits 5-7 → bits 0-2
//!   The game unpacks each one into 6 bytes at $AB06 (0, X page, X, flag, Y page, Y, type).
//! * Block contents (research `blockprobe`): types $11-$15 draw nothing and sit half a block
//!   above a block; hitting that block from below brings out an item (active object type
//!   $10 + kind, +$12 = $80 | kind, its 32 bytes copied by 0:3CAA) instead of a coin: $11 a
//!   mushroom (a fire flower when Mario is big), $12 a fire flower, $13 a carrot, $14 a star,
//!   $15 a heart. Only blocks $00 (the plain block; a big Mario breaks it), $01 (the ? block),
//!   $02, $03 and $7E (the invisible block) release it; on any other block it does nothing.
//!   $19 is one only in level 02: kind 9, an item marked M that gives 50 coins when taken,
//!   from the same blocks. Over the ? blocks of levels 04, 17 and 26 nothing comes out of it
//!   (the block gives its coin): the type means something else there.
use crate::error::{CartError, ObjectsUsage};
use crate::levels::LEVELS;
use crate::patch::objects_patched;
use crate::rom::{set_word, BANK_SIZE};

const TABLE: u16 = 0x6037;

/// The block contents of every level (see above; $19 is one in level 02 only) and the
/// blocks that release them.
pub const BLOCK_CONTENTS: [u8; 5] = [0x11, 0x12, 0x13, 0x14, 0x15];
pub const RELEASED_BY: [u8; 5] = [0x00, 0x01, 0x02, 0x03, 0x7E];

/// The block (column, row) a block content at `o` sits on: half a block below its place.
pub fn block_of(o: &Obj) -> (usize, usize) {
    (o.x as usize / 16, (o.y as usize + 8) / 16)
}

/// The bank the object loader reads: 3, or its twin $23 in an expanded copy
/// (see `patch::expand`).
pub fn bank(rom: &[u8]) -> usize {
    if objects_patched(rom) {
        0x23
    } else {
        3
    }
}

fn table(rom: &[u8]) -> usize {
    bank(rom) * BANK_SIZE + (TABLE as usize - 0x4000)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obj {
    pub x: u16,
    pub y: u16,
    pub kind: u8,
    pub flag: bool,
}

impl Obj {
    pub fn from_bytes(b: u8, c: u8, d: u8) -> Obj {
        let kind = ((b >> 4) & 1) << 6 | ((b >> 7) & 1) << 5 | ((b >> 6) & 1) << 4 | ((b >> 5) & 1) << 3 | (c >> 5);
        Obj {
            x: (b as u16 & 0x0F) * 256 + (c as u16 & 0x1F) * 8,
            y: ((d as u16 >> 5) & 3) * 256 + (d as u16 & 0x1F) * 8,
            kind,
            flag: d & 0x80 != 0,
        }
    }

    pub fn to_bytes(&self) -> [u8; 3] {
        let x = self.x.min(4095);
        let y = self.y.min(1023);
        let k = self.kind & 0x7F;
        let b = (x / 256) as u8 | ((k >> 6) & 1) << 4 | ((k >> 3) & 1) << 5 | ((k >> 4) & 1) << 6 | ((k >> 5) & 1) << 7;
        let c = ((x % 256) / 8) as u8 | (k & 7) << 5;
        let d = ((y % 256) / 8) as u8 | (((y / 256) as u8) & 3) << 5 | (self.flag as u8) << 7;
        [b, c, d]
    }
}

fn ptr(rom: &[u8], level: usize) -> usize {
    let t = table(rom) + level * 2;
    bank(rom) * BANK_SIZE + ((rom[t] as usize | (rom[t + 1] as usize) << 8) & 0x3FFF)
}

/// A level's objects and the bytes its list takes (with the $FF end mark).
pub fn read(rom: &[u8], level: usize) -> (Vec<Obj>, usize) {
    let mut i = ptr(rom, level);
    let start = i;
    let mut v = Vec::new();
    while i + 2 < rom.len() && rom[i] != 0xFF && v.len() < 400 {
        v.push(Obj::from_bytes(rom[i], rom[i + 1], rom[i + 2]));
        i += 3;
    }
    (v, i + 1 - start)
}

/// Where the lists may go: in an expanded copy, $4000-$6036 of bank $23 (which the loader
/// doesn't use); otherwise the original lists' own space (they follow each other).
pub fn region_for(rom: &[u8], original: &[u8]) -> (usize, usize) {
    if objects_patched(rom) {
        return (0x23 * BANK_SIZE, TABLE as usize - 0x4000);
    }
    region(original)
}

/// Where the original lists sit (they follow each other): first start, total bytes.
pub fn region(original: &[u8]) -> (usize, usize) {
    let mut starts: Vec<usize> = (0..LEVELS).map(|l| ptr(original, l)).collect();
    starts.sort();
    starts.dedup();
    let start = starts[0];
    let end = starts
        .iter()
        .map(|&s| {
            let mut i = s;
            while original[i] != 0xFF {
                i += 3;
            }
            i + 1
        })
        .max()
        .unwrap_or(start);
    (start, end - start)
}

/// Bytes the lists need (levels that shared one list keep sharing it).
pub fn space(rom: &[u8], original: &[u8], lists: &[Vec<Obj>]) -> (usize, usize) {
    let mut seen: Vec<usize> = Vec::new();
    let mut need = 0;
    for l in 0..LEVELS {
        let p = ptr(rom, l);
        if !seen.contains(&p) {
            seen.push(p);
            need += lists[l].len() * 3 + 1;
        }
    }
    (need, region_for(rom, original).1)
}

/// Write every level's list back, one after another in the original lists' space, sorted
/// left to right by x (the order the game walks them in; its own lists are), and point the
/// table at them.
pub fn store(rom: &mut [u8], original: &[u8], lists: &[Vec<Obj>]) -> Result<ObjectsUsage, CartError> {
    let (start, capacity) = region_for(rom, original);
    let (need, _) = space(rom, original, lists);
    let b = bank(rom);
    let tbl = table(rom);
    if need > capacity {
        return Err(CartError::ObjectsFull { need, capacity });
    }
    let old: Vec<usize> = (0..LEVELS).map(|l| ptr(rom, l)).collect();
    let mut placed: Vec<(usize, u16)> = Vec::new(); // (old pointer, new address)
    let mut at = start;
    for l in 0..LEVELS {
        let addr = if let Some(&(_, a)) = placed.iter().find(|p| p.0 == old[l]) {
            a
        } else {
            let a = 0x4000 + (at - b * BANK_SIZE) as u16;
            let mut objs = lists[l].clone();
            objs.sort_by_key(|o| o.x);
            for o in &objs {
                rom[at..at + 3].copy_from_slice(&o.to_bytes());
                at += 3;
            }
            rom[at] = 0xFF;
            at += 1;
            placed.push((old[l], a));
            a
        };
        set_word(rom, tbl + l * 2, addr);
    }
    Ok(ObjectsUsage { used: need, capacity })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_storable_object_round_trips() {
        for kind in 0..128u8 {
            for &(x, y) in &[(0u16, 0u16), (8, 8), (4088, 1016), (2048, 512), (1200, 248)] {
                for flag in [false, true] {
                    let o = Obj { x, y, kind, flag };
                    let [b, c, d] = o.to_bytes();
                    assert_eq!(Obj::from_bytes(b, c, d), o);
                }
            }
        }
    }

    #[test]
    fn positions_are_clamped_and_snapped() {
        let o = Obj { x: 5000, y: 2000, kind: 0xFF, flag: false };
        let [b, c, d] = o.to_bytes();
        let back = Obj::from_bytes(b, c, d);
        assert_eq!((back.x, back.y, back.kind), (4088, 1016, 0x7F));
    }

    #[test]
    fn lists_are_stored_left_to_right() {
        let mut original = vec![0u8; 0x80000];
        let at = 3 * BANK_SIZE + 0x1000;
        let tbl = table(&original);
        for l in 0..LEVELS {
            set_word(&mut original, tbl + l * 2, 0x5000);
        }
        original[at + 9] = 0xFF;
        let o = |x| Obj { x, y: 448, kind: 0x1B, flag: false };
        let mut lists = vec![vec![o(224), o(472), o(272)]; LEVELS];
        let mut rom = original.clone();
        store(&mut rom, &original, &lists).unwrap();
        lists[0].sort_by_key(|o| o.x);
        assert_eq!(read(&rom, 0).0, lists[0]);
    }
}
