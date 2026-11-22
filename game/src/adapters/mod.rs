//! The ports' implementations: macroquad (window, picture, keyboard, touches), cpal or the
//! page's Web Audio (speakers), gilrs or the page's Gamepad API (controller), files (through
//! `host`) and the system around the window.
#[cfg(not(target_arch = "wasm32"))]
pub mod cpal_audio;
pub mod fs_stores;
#[cfg(not(target_arch = "wasm32"))]
pub mod gilrs_pad;
pub mod macroquad_display;
pub mod macroquad_keyboard;
pub mod macroquad_picker;
pub mod macroquad_touch;
pub mod platform;
pub mod system_window;
pub mod title_preview;
#[cfg(target_arch = "wasm32")]
pub mod web_audio;
#[cfg(target_arch = "wasm32")]
pub mod web_pad;
