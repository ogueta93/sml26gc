//! The ports' implementations: files (through `host`), the system's file dialogs, the window's
//! menu bar and starting the game (`sml26gc.exe`); in the browser, the page's (`web_*`).
//! `platform` says which the composition root uses.
#[cfg(not(target_arch = "wasm32"))]
pub mod cpal_sound;
#[cfg(not(target_arch = "wasm32"))]
pub mod file_dialogs;
pub mod fs_repository;
pub mod menu_bar;
pub mod platform;
pub mod test_game_args;
#[cfg(not(target_arch = "wasm32"))]
pub mod process_launcher;
#[cfg(target_arch = "wasm32")]
pub mod web_capture;
#[cfg(target_arch = "wasm32")]
pub mod web_files;
#[cfg(target_arch = "wasm32")]
pub mod web_game_tab;
#[cfg(target_arch = "wasm32")]
pub mod web_sound;
