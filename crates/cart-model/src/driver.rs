//! Driving this cartridge's program from outside: reading which screen it is on, and the
//! button sequences and memory writes that take it to the map, into a level or back.
use crate::ram_map::{GameMode, GAME_MODE, LEAVE_LEVEL_WRITES, LEVEL_NUMBER, MAP_POINT, SPECIAL_MAP_POINTS};
use crate::world::point_levels;
use gb_core::{BusObserver, Buttons, GameBoy};

/// The game-specific actions on a running Game Boy with this cartridge.
pub trait GameDriver {
    /// Which screen the program is on (game mode byte, $FF9B).
    fn mode(&mut self) -> GameMode;
    /// Leave the level for the map exactly as the game's own pause menu does when a cleared
    /// level is exited with Select (0:2774-0:278B): no life is lost.
    fn back_to_map(&mut self);
    /// Start the cartridge again (like switching the console off and on: battery saves stay)
    /// and press Start on the title screen until the A/B/C file select is up.
    fn back_to_file_select(&mut self);
    /// From power-on, press Start through the title and file A until the world map is up.
    fn boot_to_map(&mut self) -> bool;
    /// From the world map, enter level `level` (0..=31): press Start, and when the game
    /// copies the level's header (mode $0B, routine 01:5579 reads the level number at
    /// $A269) put the wanted number there first. Returns once the level is playing.
    fn enter_level(&mut self, level: u8) -> bool;
    /// The first half of `enter_level`: up to the level header, with `level`'s number put there.
    fn enter_level_header(&mut self, level: u8) -> bool;
    /// The rest of `enter_level` after `enter_level_header`: the level loads and plays.
    fn finish_entering(&mut self) -> bool;
    /// Enter `level` from the world map whatever point the game's own Mario stands on:
    /// Start only works on a point that opens a level (0F:41B1), so first put him on one
    /// ($A840 = point number; the special points 0F:418F-0F:41A1 checks are avoided, and
    /// the start point $00 is only the fallback).
    fn enter_level_anywhere(&mut self, level: u8) -> bool;
    /// From the world map, show the map that point `point` is on (the world map or a zone
    /// map) the way the game does after a level: enter one, put Mario on the point
    /// ($A840) and leave. Returns when the map has been up for a while.
    fn show_map_at(&mut self, point: u8) -> bool;
}

fn start_if(pressed: bool) -> Buttons {
    if pressed {
        Buttons::START
    } else {
        Buttons::NONE
    }
}

impl<O: BusObserver> GameDriver for GameBoy<O> {
    fn mode(&mut self) -> GameMode {
        GameMode::from_byte(self.read(GAME_MODE))
    }

    fn back_to_map(&mut self) {
        for (addr, v) in LEAVE_LEVEL_WRITES {
            self.write(addr, v);
        }
        self.write(GAME_MODE, GameMode::ToMap.byte());
    }

    fn back_to_file_select(&mut self) {
        let ram = self.cart_mut().take_ram();
        let rom = self.cart().rom().to_vec();
        let rate = self.sample_rate();
        if let Ok(mut g) = GameBoy::with_observer(rom, rate, self.observer().clone()) {
            g.cart_mut().set_ram(ram);
            for f in 0..1200u32 {
                g.run_frame_with(start_if(f % 40 < 4 && f > 60));
                if g.mode() == GameMode::FileSelect {
                    break;
                }
            }
            g.set_buttons(Buttons::NONE);
            *self = g;
        } else {
            self.cart_mut().set_ram(ram);
        }
    }

    fn boot_to_map(&mut self) -> bool {
        for f in 0..3000u32 {
            if self.mode() == GameMode::Map {
                // let the map finish appearing
                for _ in 0..90 {
                    self.run_frame_with(Buttons::NONE);
                }
                return true;
            }
            self.run_frame_with(start_if(f % 50 < 5 && f > 120));
        }
        false
    }

    fn enter_level(&mut self, level: u8) -> bool {
        self.enter_level_header(level) && self.finish_entering()
    }

    fn enter_level_header(&mut self, level: u8) -> bool {
        for f in 0..900u32 {
            let m = self.mode();
            if m == GameMode::LevelHeader {
                self.write(LEVEL_NUMBER, level);
                return true;
            }
            self.run_frame_with(start_if(m == GameMode::Map && f % 30 < 4));
        }
        false
    }

    fn finish_entering(&mut self) -> bool {
        for _ in 0..900u32 {
            if self.mode() == GameMode::Playing {
                for _ in 0..3 {
                    self.run_frame_with(Buttons::NONE);
                }
                return true;
            }
            self.run_frame_with(Buttons::NONE);
        }
        false
    }

    fn enter_level_anywhere(&mut self, level: u8) -> bool {
        let table = point_levels(self.cart().rom());
        // Point $00 (where a new game starts) stops taking Start once its level is cleared,
        // so another level point is tried first.
        let mut points: Vec<u8> = (0..crate::map_routes::POINTS as u8).filter(|&p| table[p as usize] < 32 && !SPECIAL_MAP_POINTS.contains(&p)).collect();
        points.sort_by_key(|&p| p == 0);
        for p in points.into_iter().take(2) {
            let mut g = self.clone();
            g.write(MAP_POINT, p);
            if g.enter_level(level) {
                *self = g;
                return true;
            }
        }
        false
    }

    fn show_map_at(&mut self, point: u8) -> bool {
        if !self.enter_level_anywhere(1) {
            return false;
        }
        for _ in 0..60 {
            self.run_frame_with(Buttons::NONE);
        }
        self.write(MAP_POINT, point);
        self.back_to_map();
        let mut up = 0;
        for _ in 0..900u32 {
            self.run_frame_with(Buttons::NONE);
            up = if self.mode() == GameMode::Map { up + 1 } else { 0 };
            if up >= 120 {
                return true;
            }
        }
        false
    }
}
