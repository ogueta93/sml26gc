//! The music each map plays. Each map's set-up code in bank $0F asks for its music with
//! `ld a,nn / ld ($A468),a` (`3E nn EA 68 A4`, $A468 the music request) and nothing else picks
//! it, when the map is entered, when Mario comes back from a level and when a game starts on
//! it; `nn` is the map's music. The world map's code asks at two places (0F:6A9B, 0F:71FD).
use crate::level_settings::MUSICS;
use crate::patch::TWIN;
use crate::rom::offset;
use crate::zone_maps::ZONES;

const BANK: usize = 0x0F;
/// The music byte of each zone map, in `zone_maps::ZONES` order.
const ZONE_SITES: [u16; 8] = [0x5186, 0x5297, 0x5401, 0x552B, 0x5694, 0x5840, 0x5758, 0x58D6];
const WORLD_SITES: [u16; 2] = [0x6A9B, 0x71FD];

/// A map: the world map or a zone map (index into `zone_maps::ZONES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Map {
    World,
    Zone(usize),
}

fn sites(map: Map) -> Vec<usize> {
    match map {
        Map::World => WORLD_SITES.iter().map(|&a| offset(BANK, a)).collect(),
        Map::Zone(z) => ZONE_SITES.get(z).map(|&a| vec![offset(BANK, a)]).unwrap_or_default(),
    }
}

/// The file offset of zone `z`'s music byte.
pub fn zone_site(z: usize) -> Option<usize> {
    ZONE_SITES.get(z).map(|&a| offset(BANK, a))
}

pub fn read(rom: &[u8], map: Map) -> u8 {
    sites(map).first().and_then(|&o| rom.get(o).copied()).unwrap_or(0)
}

/// Write `map`'s music (the twin bank of an expanded copy too).
pub fn write(rom: &mut [u8], map: Map, music: u8) {
    let twin = rom.len() >= 2 * TWIN;
    for o in sites(map) {
        rom[o] = music;
        if twin {
            rom[o + TWIN] = music;
        }
    }
}

/// What names a music in the editor: a map that plays it or a level that plays it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Named {
    Map(Map),
    Level(usize),
}

/// The game's own music of each map: the eight zone maps, then the world map.
const GAME_MUSIC: [u8; 9] = [0x10, 0x1E, 0x05, 0x1C, 0x12, 0x1B, 0x0E, 0x0D, 0x06];

/// The musics a map can play: the game's maps' (named by the map), then its levels'
/// (`level_settings::MUSICS`, named by a level).
pub fn choices() -> Vec<(u8, Named)> {
    let maps = (0..ZONES.len()).map(|z| (GAME_MUSIC[z], Named::Map(Map::Zone(z)))).chain([(GAME_MUSIC[8], Named::Map(Map::World))]);
    maps.chain(MUSICS.iter().map(|&(m, l)| (m, Named::Level(l)))).collect()
}

/// The music the game itself gives `map`.
pub fn game_music(map: Map) -> u8 {
    match map {
        Map::World => GAME_MUSIC[8],
        Map::Zone(z) => GAME_MUSIC.get(z).copied().unwrap_or(GAME_MUSIC[0]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_maps_have_their_own_sites() {
        let mut rom = vec![0u8; 2 * TWIN];
        write(&mut rom, Map::World, 0x14);
        write(&mut rom, Map::Zone(1), 0x28);
        assert_eq!(read(&rom, Map::World), 0x14);
        assert_eq!(rom[offset(BANK, 0x71FD)], 0x14, "both world sites");
        assert_eq!(rom[offset(BANK, 0x6A9B) + TWIN], 0x14, "and the twin bank");
        assert_eq!((read(&rom, Map::Zone(1)), read(&rom, Map::Zone(0))), (0x28, 0));
    }

    #[test]
    fn eighteen_different_musics() {
        let mut m: Vec<u8> = choices().iter().map(|c| c.0).collect();
        assert_eq!(m.len(), 18);
        m.sort_unstable();
        m.dedup();
        assert_eq!(m.len(), 18);
    }

    #[test]
    fn the_cartridge_has_the_game_music() {
        let Some(rom) = std::env::var("SML2_ROM").ok().and_then(|p| std::fs::read(p).ok()) else {
            eprintln!("SML2_ROM not set: test skipped");
            return;
        };
        for map in (0..ZONES.len()).map(Map::Zone).chain([Map::World]) {
            assert_eq!(read(&rom, map), game_music(map), "{map:?}");
            assert!(sites(map).iter().all(|&o| rom[o - 1] == 0x3E && rom[o + 1..o + 4] == [0xEA, 0x68, 0xA4]), "{map:?} asks for its music");
        }
    }

    #[derive(Clone, Default)]
    struct Requests(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl gb_core::BusObserver for Requests {
        const ENABLED: bool = true;
        fn on_write(&mut self, _pc: u16, _bank: usize, addr: u16, value: u8) {
            if addr == crate::ram_map::MUSIC_REQUEST && value != 0 && value != 0xFF {
                self.0.lock().unwrap().push(value);
            }
        }
    }

    /// The game plays a changed map's music on starting a game there and on coming back to it
    /// from a level, and the other maps keep theirs.
    #[test]
    fn the_game_plays_each_maps_music() {
        use crate::driver::GameDriver;
        let Some(mut rom) = std::env::var("SML2_ROM").ok().and_then(|p| std::fs::read(p).ok()) else {
            eprintln!("SML2_ROM not set: test skipped");
            return;
        };
        write(&mut rom, Map::World, 0x14);
        write(&mut rom, Map::Zone(0), 0x28);
        let heard = Requests::default();
        let mut g = gb_core::GameBoy::with_observer(rom.clone(), 1000, heard.clone()).unwrap();
        assert!(g.boot_to_map());
        let last = || heard.0.lock().unwrap().drain(..).last();
        assert_eq!(last(), Some(0x14), "a game starts on the world map");
        for (z, zone) in ZONES.iter().enumerate() {
            let mut h = g.clone();
            assert!(h.show_map_at(zone.point));
            assert_eq!(last(), Some(read(&rom, Map::Zone(z))), "{}", zone.name);
        }
        let mut h = g.clone();
        assert!(h.show_map_at(0x01));
        assert_eq!(last(), Some(0x14), "back on the world map from a level");
    }
}
