//! Where the game keeps things in memory while it runs (found by reverse engineering; see
//! `docs/MECHANICS.md` for the routines each one comes from).

/// The game mode byte: which screen the program is on.
pub const GAME_MODE: u16 = 0xFF9B;
/// The level being entered or played (0-31). 01:558C copies the header of this level.
pub const LEVEL_NUMBER: u16 = 0xA269;
/// The world map point Mario stands on (0-129).
pub const MAP_POINT: u16 = 0xA840;
/// Player position in the level, 16 bits little-endian each (Y grows downwards).
pub const PLAYER_Y: u16 = 0xFFC0;
pub const PLAYER_X: u16 = 0xFFC2;
/// Lives left (BCD).
pub const LIVES: u16 = 0xA22C;
/// Mario's power-up: 0 small, 1 big, 2 carrot, 3 fire (kept from level to level).
pub const POWER: u16 = 0xA216;

/// Cartridge RAM starts here on the bus.
pub const CART_RAM: u16 = 0xA000;
/// The level's 128 block definitions (4 tile numbers each), copied from bank 8 by 0:03FF.
pub const BLOCK_DEFS: u16 = 0xA600;
pub const BLOCK_DEFS_LEN: usize = 128 * 4;
/// The decoded level map: 48 rows x 256 blocks from here to $DFFF (cartridge RAM up to
/// $BFFF, then work RAM).
pub const LEVEL_MAP: u16 = 0xB000;
/// The music request: the sound code starts the music written here (0:0595 for a level, each
/// map's set-up code for a map).
pub const MUSIC_REQUEST: u16 = 0xA468;
/// The music playing, copied from the request by the sound code (04:4078).
pub const MUSIC_PLAYING: u16 = 0xA469;
/// The objects of the level being played, 6 bytes each (X page, X, flag, Y page, Y, type).
pub const OBJECT_RECORDS: u16 = 0xAB06;

/// What the game's own pause menu writes to leave a cleared level for the map with
/// Select (0:2774-0:278B), before setting the mode to `GameMode::ToMap`.
pub const LEAVE_LEVEL_WRITES: [(u16, u8); 6] = [(0xA45E, 0x02), (0xA224, 0x02), (0xA2C8, 0), (0xA299, 0), (0xA2B4, 0), (0xA50D, 0)];

/// Map points that Start does not enter like the others (0F:418F-0F:41A1 checks them).
pub const SPECIAL_MAP_POINTS: [u8; 6] = [0x05, 0x09, 0x0E, 0x17, 0x1D, 0x11];

/// Offset into cartridge RAM of a bus address in $A000-$BFFF.
pub const fn cart_ram_offset(addr: u16) -> usize {
    (addr - CART_RAM) as usize
}

/// The values of the game mode byte (`GAME_MODE`), from a write-watch on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameMode {
    /// $1A: the A/B/C file select.
    FileSelect,
    /// $0C: the world map.
    Map,
    /// $0B: copying the header of the level at `LEVEL_NUMBER` (the moment to change it).
    LevelHeader,
    /// $16, $02, $03: the other steps of loading a level.
    LoadingLevel(u8),
    /// $04: playing a level.
    Playing,
    /// $08: the game's own pause.
    Paused,
    /// $09: losing a life.
    LosingLife,
    /// $14: going back to the map.
    ToMap,
    /// $05: game over.
    GameOver,
    /// Any other value (title screen, intro, ending...).
    Other(u8),
}

impl GameMode {
    pub const fn from_byte(b: u8) -> GameMode {
        match b {
            0x1A => GameMode::FileSelect,
            0x0C => GameMode::Map,
            0x0B => GameMode::LevelHeader,
            0x16 | 0x02 | 0x03 => GameMode::LoadingLevel(b),
            0x04 => GameMode::Playing,
            0x08 => GameMode::Paused,
            0x09 => GameMode::LosingLife,
            0x14 => GameMode::ToMap,
            0x05 => GameMode::GameOver,
            _ => GameMode::Other(b),
        }
    }

    pub const fn byte(self) -> u8 {
        match self {
            GameMode::FileSelect => 0x1A,
            GameMode::Map => 0x0C,
            GameMode::LevelHeader => 0x0B,
            GameMode::LoadingLevel(b) | GameMode::Other(b) => b,
            GameMode::Playing => 0x04,
            GameMode::Paused => 0x08,
            GameMode::LosingLife => 0x09,
            GameMode::ToMap => 0x14,
            GameMode::GameOver => 0x05,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mode_byte_round_trips() {
        for b in 0..=255u8 {
            assert_eq!(GameMode::from_byte(b).byte(), b);
        }
        assert_eq!(GameMode::from_byte(0x0C), GameMode::Map);
        assert_eq!(GameMode::from_byte(0x77), GameMode::Other(0x77));
    }

    #[test]
    fn cart_ram_offsets() {
        assert_eq!(cart_ram_offset(BLOCK_DEFS), 0x600);
        assert_eq!(cart_ram_offset(LEVEL_MAP), 0x1000);
        assert_eq!(cart_ram_offset(OBJECT_RECORDS), 0xB06);
    }
}
