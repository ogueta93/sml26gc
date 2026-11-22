//! Golden regression fingerprint: the workspace crates must reproduce, line for line, the
//! recorded fingerprint (`golden.txt`).
//!
//! Runs the emulator and the cartridge-knowledge code over the person's own ROM and writes
//! a list of `key = value` lines (hashes and small values) to `regression/golden.txt` at the
//! repository root. The refactored crates must reproduce the very same lines.
//!
//!   cargo test --release --test golden                  compare against the stored file
//!   $env:GOLDEN_UPDATE=1; cargo test --release --test golden   (re)write the stored file
//!
//! The ROM is taken from the `SML2_ROM` environment variable, else found the way the game
//! finds it. Without a ROM the test is skipped (it prints why). Nothing from the ROM is
//! written to the golden file except hashes, sizes and offsets.
use cart_model::{levels, objects, world, GameDriver};
use gb_core::Buttons;
use gb_core::Gb;
use std::path::PathBuf;

const SAMPLE_RATE: u32 = 44_100;
/// Point levels the fingerprint covers: the game's table ($00-$81).
const GAME_POINTS: usize = 0x82;

/// FNV-1a, 64 bit: tiny, dependency-free and stable across versions.
fn fnv(data: impl AsRef<[u8]>) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in data.as_ref() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

fn fnv_f32(data: &[f32]) -> String {
    let bytes: Vec<u8> = data.iter().flat_map(|s| s.to_bits().to_le_bytes()).collect();
    fnv(&bytes)
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("golden.txt")
}

fn rom_path() -> Option<PathBuf> {
    host::test_cartridge()
}

/// A save result as the fingerprint writes it (`Ok("...")` / `Err("...")`).
fn shown<T: std::fmt::Display, E: std::fmt::Display>(r: &Result<T, E>) -> String {
    match r {
        Ok(v) => format!("Ok({:?})", v.to_string()),
        Err(e) => format!("Err({:?})", e.to_string()),
    }
}

struct Out(Vec<String>);
impl Out {
    fn put(&mut self, key: impl AsRef<str>, value: impl std::fmt::Display) {
        self.0.push(format!("{} = {}", key.as_ref(), value));
    }
}

/// Buttons for frame `f` of the scripted run: Start pulses through the title and file
/// select, then a fixed cycle of walking, running and jumping (whatever the game does with
/// it is deterministic, which is all a fingerprint needs).
fn script(f: u32) -> Buttons {
    if f < 200 {
        Buttons::NONE
    } else if f < 1000 {
        if f % 50 < 5 { Buttons::START } else { Buttons::NONE }
    } else {
        const CYCLE: [Buttons; 8] = [
            Buttons::RIGHT,
            Buttons::from_bits(0x21), // Right + B
            Buttons::from_bits(0x11), // Right + A
            Buttons::from_bits(0x31), // Right + B + A
            Buttons::NONE,
            Buttons::LEFT,
            Buttons::RIGHT,
            Buttons::A,
        ];
        let b = CYCLE[((f / 30) % 8) as usize];
        // an occasional Start, to cover the map -> level transition too
        if f % 600 < 4 { b | Buttons::START } else { b }
    }
}

/// 1. The raw emulator: power-on, scripted buttons, picture and sound fingerprints.
fn emulation(rom: &[u8], out: &mut Out) {
    let mut gb = Gb::new(rom.to_vec(), SAMPLE_RATE).expect("cartridge");
    let mut audio: Vec<f32> = Vec::new();
    for f in 0..3600u32 {
        gb.set_buttons(script(f));
        gb.run_frame();
        audio.extend_from_slice(gb.audio());
        gb.clear_audio();
        if f % 60 == 59 {
            out.put(format!("emu.f{f:04}.screen"), fnv(&gb.screen()));
            out.put(format!("emu.f{f:04}.sound"), format!("{} {}", audio.len(), fnv_f32(&audio)));
            audio.clear();
        }
    }
    out.put("emu.end.mode", format!("{:02x}", gb.mode().byte()));
    out.put("emu.end.wram", fnv(&gb.wram()));
    out.put("emu.end.cart_ram", fnv(&gb.cart().ram()));
    out.put("emu.end.vram", fnv(&gb.video().vram()));
    out.put("emu.end.oam", fnv(&gb.video().oam()));
}

/// 2. The game-specific use cases: boot to the map and enter each level.
fn use_cases(rom: &[u8], out: &mut Out) {
    let mut map = Gb::new(rom.to_vec(), SAMPLE_RATE).expect("cartridge");
    out.put("uc.boot_to_map", map.boot_to_map());
    out.put("uc.map.screen", fnv(&map.screen()));
    let art = cart_model::art::WorldArt::capture(&map);
    out.put("uc.map.world_art", fnv(&[art.vram.as_slice(), &[art.lcdc, art.bgp]].concat()));

    for level in 0..levels::LEVELS {
        let mut g = map.clone();
        let ok = g.enter_level_anywhere(level as u8);
        let la = cart_model::art::LevelArt::capture(&g);
        out.put(format!("uc.level{level:02}.entered"), format!("{ok} mode={:02x}", g.mode().byte()));
        out.put(format!("uc.level{level:02}.screen"), fnv(&g.screen()));
        out.put(format!("uc.level{level:02}.map_in_memory"), fnv(&cart_model::live::map_in_memory(&g)));
        out.put(
            format!("uc.level{level:02}.art"),
            fnv(&[la.metatiles.as_slice(), &la.vram, &[la.lcdc, la.bgp]].concat()),
        );
        for _ in 0..120 {
            g.run_frame_with(Buttons::RIGHT | Buttons::B);
        }
        out.put(format!("uc.level{level:02}.after_run.screen"), fnv(&g.screen()));
        let mut back = g.clone();
        back.back_to_map();
        for _ in 0..240 {
            back.run_frame_with(Buttons::NONE);
        }
        out.put(format!("uc.level{level:02}.back_to_map.mode"), format!("{:02x}", back.mode().byte()));
    }
}

/// 3. The cartridge knowledge: levels, objects, world map, ROM expansion and re-packing.
fn rom_data(rom: &[u8], out: &mut Out) {
    out.put("rom.size", rom.len());
    out.put("rom.can_expand", cart_model::patch::can_expand(rom));

    let mut lists = Vec::new();
    for level in 0..levels::LEVELS {
        let h = levels::header(rom, level);
        let (entry, start) = levels::map_location(rom, h);
        let (map, used) = levels::decode(rom, start);
        let enc = levels::encode(&map);
        let (again, _) = levels::decode(&enc, 0);
        out.put(
            format!("rom.level{level:02}.header"),
            format!("at={:05x} bank={:02x} map={} entry={:05x} start={:05x}", h.at, h.bank, h.map, entry, start),
        );
        out.put(format!("rom.level{level:02}.map"), format!("{} used={used}", fnv(&map)));
        out.put(
            format!("rom.level{level:02}.encode"),
            format!("len={} roundtrip={} same_as_rom={}", enc.len(), again == map, enc[..] == rom[start..start + used]),
        );
        out.put(format!("rom.level{level:02}.sharing"), format!("{:?}", levels::sharing(rom, level)));

        let (objs, bytes) = objects::read(rom, level);
        let packed: Vec<u8> = objs.iter().flat_map(|o| o.to_bytes()).collect();
        let desc: Vec<String> = objs.iter().map(|o| format!("{},{},{:02x},{}", o.x, o.y, o.kind, o.flag as u8)).collect();
        out.put(format!("rom.level{level:02}.objects"), format!("n={} bytes={bytes} {}", objs.len(), fnv(desc.join(";").as_bytes())));
        out.put(format!("rom.level{level:02}.objects_packed"), fnv(&packed));
        lists.push(objs);
    }
    out.put("rom.objects.region", format!("{:?}", objects::region(rom)));

    out.put("rom.world.picture", fnv(&world::read(rom)));
    out.put("rom.world.point_levels", fnv(&world::point_levels(rom)[..GAME_POINTS]));

    let mut banks: Vec<u8> = (0..levels::LEVELS).map(|l| levels::header(rom, l).bank).collect();
    banks.sort();
    banks.dedup();
    for &b in &banks {
        out.put(format!("rom.bank{b:02x}.maps"), format!("{:?}", cart_model::layout::bank_maps(rom, b)));
        out.put(format!("rom.bank{b:02x}.capacity"), format!("{:?}", cart_model::layout::bank_capacity(rom, rom, b)));
    }

    match cart_model::patch::expand(rom) {
        Ok(mut big) => {
            out.put("exp.rom", fnv(&big));
            out.put("exp.flags", format!("expanded={} objects_patched={}", cart_model::patch::is_expanded(&big), cart_model::patch::objects_patched(&big)));
            out.put("exp.objects.region", format!("{:?}", objects::region_for(&big, rom)));
            out.put("exp.objects.space", format!("{:?}", objects::space(&big, rom, &lists)));
            // Re-pack every bank and the objects unchanged, then edit one level and re-pack.
            for &b in &banks {
                let r = cart_model::layout::store_bank(&mut big, rom, b, &[]);
                out.put(format!("exp.bank{b:02x}.store"), shown(&r));
            }
            out.put("exp.objects.store", shown(&objects::store(&mut big, rom, &lists)));
            out.put("exp.after_store.rom", fnv(&big));

            let h = levels::header(&big, 0);
            let (_, start) = levels::map_location(&big, h);
            let (mut map, _) = levels::decode(&big, start);
            for x in 0..levels::COLS {
                map[x] = (x % 7) as u8; // a recognisable edit in the top row
            }
            out.put("exp.edit.space", format!("{:?}", cart_model::layout::bank_space(&big, rom, h.bank, &[(h.map, &map[..])])));
            out.put("exp.edit.store", shown(&cart_model::layout::store_bank(&mut big, rom, h.bank, &[(h.map, &map[..])])));
            let (_, start) = levels::map_location(&big, levels::header(&big, 0));
            out.put("exp.edit.reread_ok", levels::decode(&big, start).0 == map);

            world::write(&mut big, &world::read(rom));
            let mut pts = world::point_levels(rom)[..GAME_POINTS].to_vec();
            pts.reverse();
            world::write_point_levels(&mut big, &pts);
            out.put("exp.after_edit.rom", fnv(&big));

            // The edited copy must still boot and play the edited level.
            let mut g = Gb::new(big, SAMPLE_RATE).expect("expanded cartridge");
            let booted = g.boot_to_map();
            let entered = g.enter_level_anywhere(0);
            out.put("exp.play.level00", format!("boot={booted} entered={entered} {}", fnv(&cart_model::live::map_in_memory(&g))));
        }
        Err(e) => out.put("exp.error", e),
    }
}

#[test]
fn golden() {
    // Without the cartridge nothing is checked, so that is a failure, not a pass.
    let Some(path) = rom_path().filter(|p| p.is_file()) else {
        panic!("golden: no cartridge found, nothing was checked. Set SML2_ROM to your .gb file or put it where the game finds it.");
    };
    let rom = std::fs::read(&path).expect("read ROM");
    let mut out = Out(Vec::new());
    out.put("rom.fingerprint", fnv(&rom));
    rom_data(&rom, &mut out);
    emulation(&rom, &mut out);
    use_cases(&rom, &mut out);
    let text = out.0.join("\n") + "\n";

    let file = golden_path();
    if std::env::var_os("GOLDEN_UPDATE").is_some() || !file.exists() {
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, &text).unwrap();
        eprintln!("golden: wrote {} lines to {}", out.0.len(), file.display());
        return;
    }
    let want = std::fs::read_to_string(&file).unwrap().replace("\r\n", "\n");
    let mut diffs = Vec::new();
    let (got, exp): (Vec<&str>, Vec<&str>) = (text.lines().collect(), want.lines().collect());
    for i in 0..got.len().max(exp.len()) {
        let (g, e) = (got.get(i).copied().unwrap_or("<missing>"), exp.get(i).copied().unwrap_or("<missing>"));
        if g != e {
            diffs.push(format!("  expected: {e}\n       got: {g}"));
        }
    }
    assert!(diffs.is_empty(), "{} golden lines differ:\n{}", diffs.len(), diffs.iter().take(20).cloned().collect::<Vec<_>>().join("\n"));
}
