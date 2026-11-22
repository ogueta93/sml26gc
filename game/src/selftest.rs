//! Scripted play without a window: frame dumps for development and the --self-test report.
use gb_core::{ppu, Gb};
use host::load_gb;
pub use gb_core::script::{buttons_at, parse_script};
pub use host::png_out::save_png;
use std::path::Path;

pub fn dump(rom: Option<&Path>, dir: &Path, script: &str, frames: u32, every: u32) {
    let Some(rom) = rom else {
        eprintln!("no cartridge found");
        return;
    };
    let mut gb = match load_gb(rom, 44100, None) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return;
        }
    };
    let _ = host::storage::create_dir_all(dir);
    let sc = parse_script(script);
    for f in 0..frames {
        gb.set_buttons(buttons_at(&sc, f));
        gb.run_frame();
        gb.clear_audio();
        if every > 0 && f % every == 0 {
            save_png(&gb, &dir.join(format!("f{f:05}.png")));
            let oam: Vec<String> = (0..40)
                .filter(|i| gb.video().oam()[i * 4] != 0)
                .map(|i| format!("{}:{},{},{:02x}", i, gb.video().oam()[i * 4 + 1], gb.video().oam()[i * 4], gb.video().oam()[i * 4 + 2]))
                .collect();
            println!("f{f} pc={:04x} oam=[{}]", gb.pc(), oam.join(" "));
        }
    }
}

/// Movement rules read from the cartridge's own code (bank 0, see `docs/MECHANICS.md`).
pub struct RomRules {
    pub accel: i32,
    pub friction: i32,
    pub walk_cap: i32,
    pub run_cap: i32,
    pub jump_full: i32,
    pub jump_short: i32,
    pub jump_cut: i32,
    pub apex_skip: i32,
    pub apex: i32,
    pub speed_table: Vec<i32>,
    pub jump_table: Vec<i32>,
}

impl RomRules {
    pub fn read(gb: &Gb) -> RomRules {
        let b = |o: usize| gb.cart().rom_byte(o) as i32;
        RomRules {
            accel: b(0x172A),     // 0:1729 add a,$08  (speed index per frame while the d-pad is held)
            friction: b(0x16ED),  // 0:16EC sub $04    (speed index back toward $80 each frame on the ground)
            walk_cap: b(0x175D),  // 0:175C ld a,$bf   (walk cap before friction)
            run_cap: b(0x176B),   // 0:176A ld a,$f3   (run cap with B held, before friction)
            jump_full: b(0x1682), // 0:1681 ld a,$4b   (jump table start: running, or Up held)
            jump_short: b(0x169F),// 0:169E ld a,$30   (jump table start at walking speed or standing)
            jump_cut: b(0x152F),  // 0:152E ld a,$21   (index forced when A is released while rising)
            apex_skip: b(0x153E), // 0:153D cp $30     (long jump skips from here...)
            apex: b(0x1548),      // 0:1547 ld a,$18   (...to the apex index)
            speed_table: (0..256).map(|i| gb.cart().rom_byte(0x19FF + i) as i8 as i32).collect(),
            jump_table: (0..0x50).map(|i| gb.cart().rom_byte(0x18AF + i) as i8 as i32).collect(),
        }
    }
    /// Average pixels per frame at a speed index (the game adds frame&7 to the index).
    pub fn px_per_frame(&self, index: i32) -> f32 {
        (0..8).map(|k| self.speed_table[((index + k) & 0xFF) as usize]).sum::<i32>() as f32 / 8.0
    }
    /// Pixels risen by a jump that starts at table index `start` (index drops 1 per frame
    /// and is applied after the drop; rising stops at index $18).
    pub fn rise(&self, start: i32) -> i32 {
        // 0:153D: when the index drops to $30 (normal physics) it is set to the apex $18,
        // so a jump that starts above $30 only uses the entries above it.
        let low = if start > self.apex_skip { self.apex_skip + 1 } else { self.apex + 1 };
        -(low..start).map(|i| self.jump_table[i as usize]).sum::<i32>()
    }
}

fn world_x(gb: &mut Gb) -> i32 {
    ((gb.read(0xFFC3) as i32) << 8) | gb.read(0xFFC2) as i32
}
fn world_y(gb: &mut Gb) -> i32 {
    ((gb.read(0xFFC1) as i32) << 8) | gb.read(0xFFC0) as i32
}

pub fn run(rom: Option<&Path>) -> String {
    let Some(rom) = rom else {
        return r#"{"version":1,"steps":[{"step":"move","passed":false,"detail":"no cartridge file found"},{"step":"act","passed":false,"detail":"no cartridge file found"},{"step":"hud","passed":false,"detail":"no cartridge file found"}]}"#.into();
    };
    let mut gb = match load_gb(rom, 44100, None) {
        Ok(g) => g,
        Err(e) => {
            let e = e.replace('"', "'").replace('\\', "/");
            return format!(r#"{{"version":1,"steps":[{{"step":"move","passed":false,"detail":"{e}"}},{{"step":"act","passed":false,"detail":"{e}"}},{{"step":"hud","passed":false,"detail":"{e}"}}]}}"#);
        }
    };
    // What a player does: Start on the title, Start on the file screen, Start on the
    // map to enter the first level, then walk right and jump.
    // Then: walk right, run right (B held), stop, jump from standing.
    let rules = RomRules::read(&gb);
    let script = parse_script("S@400-405;S@600-605;S@800-805;R@960-1060;BR@1080-1200;A@1260-1280;BR@1360-1500;A@1430-1500");
    let mut accel_idx = 0;
    let mut run_ground_y = 0;
    let mut run_top_y = i32::MAX;
    let mut run_jump_idx = 0;
    let hud_rows = |g: &Gb| g.screen()[136 * ppu::W..].to_vec();

    let mut hud_a = Vec::new();
    let mut hud_b = Vec::new();
    let mut field_a = Vec::new();
    let mut field_b = Vec::new();
    let (mut walk_x0, mut walk_x1, mut run_x0, mut run_x1) = (0, 0, 0, 0);
    let (mut walk_idx, mut run_idx) = (0, 0);
    let mut ground_y = 0;
    let mut top_y = i32::MAX;
    let mut start_x = 0;
    for f in 0..1560u32 {
        gb.set_buttons(buttons_at(&script, f));
        gb.run_frame();
        gb.clear_audio();
        let x = world_x(&mut gb);
        let y = world_y(&mut gb);
        if (1430..1560).contains(&f) {
            run_top_y = run_top_y.min(y);
        }
        match f {
            963 => accel_idx = gb.read(0xA200) as i32,
            1429 => run_ground_y = y,
            1430 => run_jump_idx = gb.read(0xA201) as i32 + 1,
            959 => start_x = x,
            992 => walk_x0 = x,
            1056 => {
                walk_x1 = x;
                walk_idx = gb.read(0xA200) as i32;
            }
            1130 => run_x0 = x,
            1194 => {
                run_x1 = x;
                run_idx = gb.read(0xA200) as i32;
            }
            1259 => ground_y = y,
            _ => {}
        }
        if (1260..1340).contains(&f) {
            top_y = top_y.min(y);
        }
        if f == 950 {
            hud_a = hud_rows(&gb);
            field_a = gb.screen()[..136 * ppu::W].to_vec();
        }
        if f == 1100 {
            hud_b = hud_rows(&gb);
            field_b = gb.screen()[..136 * ppu::W].to_vec();
        }
    }
    let world_dx = run_x1 - start_x;
    let walk = (walk_x1 - walk_x0) as f32 / 64.0;
    let run = (run_x1 - run_x0) as f32 / 64.0;
    let walk_rom_idx = rules.walk_cap - rules.friction;
    let run_rom_idx = rules.run_cap - rules.friction;
    let walk_rom = rules.px_per_frame(walk_rom_idx);
    let run_rom = rules.px_per_frame(run_rom_idx);
    let moved = world_dx >= 32 && walk_idx == walk_rom_idx && run_idx == run_rom_idx && (walk - walk_rom).abs() < 0.05 && (run - run_rom).abs() < 0.05;
    let rise = ground_y - top_y;
    let rise_rom = rules.rise(rules.jump_short);
    let run_rise = run_ground_y - run_top_y;
    let run_rise_rom = rules.rise(rules.jump_full);
    let accel_rom = 0x80 + 4 * (rules.accel - rules.friction);
    let jumped = rise == rise_rom && rise > 0 && run_rise == run_rise_rom && run_jump_idx == rules.jump_full;
    let moved = moved && accel_idx == accel_rom;
    let _ = rules.jump_cut;
    let dark = hud_a.iter().filter(|&&s| s >= 2).count();
    let light = hud_a.iter().filter(|&&s| s < 2).count();
    let same = |a: &[u8], b: &[u8]| a.iter().zip(b).filter(|(x, y)| x == y).count() as f32 / a.len().max(1) as f32;
    let hud_stable = same(&hud_a, &hud_b);
    let field_stable = same(&field_a, &field_b);
    let hud_ok = dark > 100 && light > 100 && hud_stable > 0.85 && field_stable < hud_stable;
    format!(
        r#"{{"version":1,"steps":[{{"step":"move","passed":{moved},"detail":"moved {world_dx} px; walking settles at speed index {walk_idx:#04x} (ROM 0:175C cap {:#04x} minus friction {} = {walk_rom_idx:#04x}) covering {walk:.3} px/frame (ROM table $19FF: {walk_rom:.3}); running with B settles at {run_idx:#04x} (ROM {run_rom_idx:#04x}) covering {run:.3} px/frame (ROM: {run_rom:.3})"}},{{"step":"act","passed":{jumped},"detail":"a standing jump (A) rose {rise} px; the ROM jump table $18AF from start index {:#04x} (0:169E) gives {rise_rom} px; a running jump started at index {run_jump_idx:#04x} (ROM 0:1681 {:#04x}) and rose {run_rise} px (ROM table: {run_rise_rom} px); after 4 frames holding Right the speed index was {accel_idx:#04x} (ROM: +{} -{} per frame from $80 = {accel_rom:#04x})"}},{{"step":"hud","passed":{hud_ok},"detail":"status bar on the bottom 8 rows (lives, coins, enemies, timer): {dark} dark / {light} light pixels, {:.0}% unchanged while the level scrolled ({:.0}% of the playfield unchanged)"}}]}}"#,
        rules.walk_cap,
        rules.friction,
        rules.jump_short,
        rules.jump_full,
        rules.accel,
        rules.friction,
        hud_stable * 100.0,
        field_stable * 100.0
    )
}
