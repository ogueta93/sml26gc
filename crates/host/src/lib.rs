//! The computer side shared by the game, the editor and the tools: the base cartridge, where
//! every file lives and how it is read and written, battery saves, settings, PNG pictures, the
//! file dialog off Windows, and the page's functions in the browser. Nothing here knows about
//! windows, sound or keyboards.
pub mod base_cartridge;
pub mod battery;
pub mod clock;
pub mod console;
pub mod file_dialog;
pub mod font_files;
pub mod paths;
pub mod png_out;
pub mod projects;
pub mod rom_locator;
pub mod settings_file;
pub mod shortcut_key;
pub mod storage;
pub mod system_language;
pub mod texts;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use base_cartridge::{base_cartridge, BaseProblem};
pub use battery::{load_gb, write_battery_save};
pub use paths::{battery_file, project_file, project_place, project_restore_points, editor_exe, editor_settings_path, exe_dir, exports_dir, game_exe, project_dir, project_rom, project_rom_in, projects, projects_dir, saves_dir, settings_path, valid_project_name, zones_file, VANILLA};
pub use rom_locator::{find_rom, require_rom, NO_CARTRIDGE};
