//! Listening to a level music in the editor: a console of the copy standing in a level
//! (Mario invincible, so nothing interrupts it) is asked for the music the way a level start
//! asks for it ($A468, 0:0595), and its sound goes to the speakers.
use crate::ports::SoundOut;
use cart_model::ram_map::MUSIC_REQUEST;
use cart_model::{cheats, GameDriver};
use gb_core::{Buttons, Gb};

/// Sound kept queued ahead, in seconds.
const AHEAD: f32 = 0.08;
/// Console frames run at most per editor frame.
const MOST_FRAMES: u32 = 8;

#[derive(Default)]
pub struct MusicPlayer {
    /// The console in a level, booted once.
    ready: Option<Gb>,
    playing: Option<Gb>,
}

impl MusicPlayer {
    pub fn playing(&self) -> bool {
        self.playing.is_some()
    }

    /// Start `music` (booting the copy `rom` the first time, at `rate` Hz).
    pub fn play(&mut self, rom: &[u8], music: u8, rate: u32) -> Result<(), String> {
        if self.ready.is_none() {
            let mut g = Gb::new(rom.to_vec(), rate)?;
            cheats::set_invincible(g.cart_mut().rom_mut(), true);
            if !g.boot_to_map() || !g.enter_level_anywhere(0) {
                return Err("the game did not reach a level".into());
            }
            self.ready = Some(g);
        }
        let mut g = self.ready.clone().ok_or("no console")?;
        g.write(MUSIC_REQUEST, music);
        self.playing = Some(g);
        Ok(())
    }

    pub fn stop(&mut self) {
        self.playing = None;
    }

    /// Run the console enough to keep the sound going, at `volume` (0-1).
    pub fn pump(&mut self, out: &dyn SoundOut, volume: f32) {
        let Some(g) = self.playing.as_mut() else { return };
        for _ in 0..MOST_FRAMES {
            if out.queued_seconds() >= AHEAD {
                break;
            }
            g.set_buttons(Buttons::NONE);
            g.run_frame();
            out.push(g.audio(), volume);
            g.clear_audio();
        }
    }
}
