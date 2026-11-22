//! Test games in the browser: the game's page in a tab of its own, with the test game's
//! arguments in its address. The tab has a fixed name, so a new test game replaces the one open.
use super::test_game_args::{test_game_args, Cartridge};
use crate::ports::{GameLauncher, TestGame};
use cart_model::start_point::Start;
use std::path::Path;

const TAB: &str = "sml26gc-test";

#[derive(Default)]
pub struct WebGameTab;

impl GameLauncher for WebGameTab {
    /// `rom`: the project's cartridge, in its folder (`editor/projects/<name>/`): the game is
    /// given the project's name.
    fn launch(&self, rom: &Path, level: Option<usize>, start: Option<(u16, u16)>, point: Option<Start>, game: TestGame) -> Result<(), String> {
        let project = rom.parent().and_then(|d| d.file_name()).map(|n| n.to_string_lossy().into_owned()).ok_or_else(|| rom.display().to_string())?;
        let args = test_game_args(Cartridge::Project(&project), level, start, point, game);
        host::web::open_page(TAB, &format!("index.html?{}", host::web::query(&args)));
        Ok(())
    }

    /// Never in the way: the next test game takes the tab.
    fn running(&self) -> bool {
        false
    }
}
