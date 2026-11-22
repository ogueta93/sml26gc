//! What the editor does differently on the desktop and in the browser, for the composition root:
//! the file dialogs and the files written, the test games, the speakers, how the object pictures
//! are captured, where the arguments come from, the window and the check pictures.
use crate::application::look_loader::Capture;

#[cfg(not(target_arch = "wasm32"))]
pub use super::cpal_sound::Speakers as Sound;
#[cfg(not(target_arch = "wasm32"))]
pub use super::file_dialogs::SystemDialogs as Dialogs;
#[cfg(not(target_arch = "wasm32"))]
pub use super::fs_repository::TextFile as Files;
#[cfg(not(target_arch = "wasm32"))]
pub use super::process_launcher::GameProcess as Launcher;
#[cfg(target_arch = "wasm32")]
pub use super::web_files::{WebDialogs as Dialogs, WebFiles as Files};
#[cfg(target_arch = "wasm32")]
pub use super::web_game_tab::WebGameTab as Launcher;
#[cfg(target_arch = "wasm32")]
pub use super::web_sound::WebSound as Sound;

/// Can the editor be quit from its own screens? (A page is closed instead.)
pub const CAN_QUIT: bool = !cfg!(target_arch = "wasm32");

/// The browser has no threads: the pictures are captured by its capture workers there.
#[cfg(target_arch = "wasm32")]
pub const CAPTURE: Capture = Capture::Workers(&super::web_capture::WebCapture);
#[cfg(not(target_arch = "wasm32"))]
pub const CAPTURE: Capture = Capture::Threads;

/// The program's arguments: the command line's, or in the browser the page's address's.
pub fn arguments() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    return host::web::text(host::web::Text::Arguments).unwrap_or_default().lines().map(String::from).collect();
    #[cfg(not(target_arch = "wasm32"))]
    std::env::args().skip(1).collect()
}

pub fn window_conf(title: &str) -> macroquad::window::Conf {
    macroquad::window::Conf {
        window_title: title.into(),
        window_width: 1280,
        window_height: 820,
        window_resizable: true,
        high_dpi: false,
        icon: include!(concat!(env!("OUT_DIR"), "/window_icon.rs")),
        ..Default::default()
    }
}

/// The window as a picture at `path` (`--screenshot`; nothing in the browser).
pub fn save_screenshot(path: &std::path::Path) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(d) = path.parent() {
            let _ = host::storage::create_dir_all(d);
        }
        macroquad::texture::get_screen_data().export_png(&path.to_string_lossy());
    }
    #[cfg(target_arch = "wasm32")]
    let _ = path;
}

/// In the browser: the keys the editor uses with Ctrl or Cmd, which the browser then leaves to
/// it (Ctrl+S saves the project, not the page).
pub fn claim_keys(keys: &[macroquad::input::KeyCode]) {
    #[cfg(target_arch = "wasm32")]
    {
        use macroquad::input::KeyCode as K;
        let code = |k: &K| match k {
            K::Minus => "Minus".to_string(),
            K::Slash => "Slash".to_string(),
            K::Equal => "Equal".to_string(),
            K::RightBracket => "BracketRight".to_string(),
            K::KpSubtract => "NumpadSubtract".to_string(),
            K::KpAdd => "NumpadAdd".to_string(),
            K::Delete => "Delete".to_string(),
            K::Left | K::Right | K::Up | K::Down => format!("Arrow{k:?}"),
            K::F5 => "F5".to_string(),
            k => format!("Key{k:?}"),
        };
        host::web::call_text(host::web::TextCall::OwnKeys, &keys.iter().map(code).collect::<Vec<_>>().join(" "));
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = keys;
}

/// In the browser: closing the page asks first while there are unsaved changes.
pub fn set_unsaved(unsaved: bool) {
    #[cfg(target_arch = "wasm32")]
    {
        use std::sync::atomic::{AtomicBool, Ordering};
        static TOLD: AtomicBool = AtomicBool::new(false);
        if TOLD.swap(unsaved, Ordering::Relaxed) != unsaved {
            host::web::call(host::web::Call::Unsaved, if unsaved { 1.0 } else { 0.0 });
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = unsaved;
}
