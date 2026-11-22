//! Starts the game (`sml26gc.exe`, next to the editor) on the project's cartridge, as a test game, and
//! keeps its process to tell whether it is still open.
use super::test_game_args::{test_game_args, Cartridge};
use crate::ports::{GameLauncher, TestGame};
use cart_model::start_point::Start;
use std::cell::RefCell;
use std::path::Path;
use std::process::Child;

#[derive(Default)]
pub struct GameProcess {
    last: RefCell<Option<Child>>,
}

impl GameLauncher for GameProcess {
    fn launch(&self, rom: &Path, level: Option<usize>, start: Option<(u16, u16)>, point: Option<Start>, game: TestGame) -> Result<(), String> {
        let exe = host::game_exe();
        let mut cmd = std::process::Command::new(&exe);
        cmd.args(test_game_args(Cartridge::File(rom), level, start, point, game));
        let child = cmd.spawn().map_err(|e| format!("{}: {e}", exe.display()))?;
        *self.last.borrow_mut() = Some(child);
        Ok(())
    }

    fn running(&self) -> bool {
        let mut last = self.last.borrow_mut();
        let open = last.as_mut().is_some_and(|c| matches!(c.try_wait(), Ok(None)));
        if !open {
            *last = None;
        }
        open
    }
}
