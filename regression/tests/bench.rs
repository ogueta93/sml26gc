//! Emulation speed, to check refactors don't slow the core down.
//!   cargo test --release -p regression --test bench -- --ignored --nocapture
use gb_core::Buttons;
use gb_core::Gb;
use std::time::Instant;

#[test]
#[ignore]
fn frames_per_second() {
    let Some(path) = host::test_cartridge() else {
        eprintln!("bench: no ROM; skipped");
        return;
    };
    let rom = std::fs::read(path).unwrap();
    let mut best = 0.0f64;
    for _ in 0..3 {
        let mut gb = Gb::new(rom.clone(), 44_100).unwrap();
        let t = Instant::now();
        for f in 0..6000u32 {
            gb.set_buttons(if f % 50 < 5 && f > 120 && f < 1200 { Buttons::START } else if f >= 1200 { Buttons::RIGHT } else { Buttons::NONE });
            gb.run_frame();
            gb.clear_audio();
        }
        best = best.max(6000.0 / t.elapsed().as_secs_f64());
    }
    println!("emulation: {best:.0} frames/s ({:.1}x real time)", best / 59.73);
}
