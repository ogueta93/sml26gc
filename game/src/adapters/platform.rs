//! What the game does differently on the desktop and in the browser, for the composition root:
//! the sound and controller adapters, where the arguments come from, the window, and what the
//! platform can do (quit, open the editor).
#[cfg(not(target_arch = "wasm32"))]
pub use super::cpal_audio::Audio;
#[cfg(not(target_arch = "wasm32"))]
pub use super::gilrs_pad::GilrsPad as Pad;
#[cfg(target_arch = "wasm32")]
pub use super::web_audio::Audio;
#[cfg(target_arch = "wasm32")]
pub use super::web_pad::WebPad as Pad;

/// The program's arguments: the command line's, or in the browser the page's address's (the
/// page keeps only the ones that make sense there).
pub fn arguments() -> Vec<String> {
    #[cfg(target_arch = "wasm32")]
    return host::web::text(host::web::Text::Arguments).unwrap_or_default().lines().map(String::from).collect();
    #[cfg(not(target_arch = "wasm32"))]
    std::env::args().skip(1).collect()
}

/// Can the program be quit from its own menus? (A page is closed instead.)
pub const CAN_QUIT: bool = !cfg!(target_arch = "wasm32");

/// Can the start screen open the editor? (In the browser only on a computer: the editor needs a
/// mouse or trackpad and a keyboard.)
pub fn can_edit() -> bool {
    #[cfg(target_arch = "wasm32")]
    return has_keyboard();
    #[cfg(not(target_arch = "wasm32"))]
    true
}

/// Open the editor on `project`, or on its project screen typing a new one. The error names
/// what failed.
pub fn open_editor(project: Option<&str>) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        let args: Vec<String> = match project {
            Some(p) => vec!["--project".into(), p.into()],
            None => vec!["--new-project".into()],
        };
        host::web::open_page("_blank", &format!("editor.html?{}", host::web::query(&args)));
        Ok(())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut cmd = std::process::Command::new(host::editor_exe());
        match project {
            Some(p) => cmd.arg("--project").arg(p),
            None => cmd.arg("--new-project"),
        };
        cmd.spawn().map(|_| ()).map_err(|e| format!("{}: {e}", host::editor_exe().display()))
    }
}

/// The window: the desktop's at its usual size; in the browser the page's canvas, drawn at the
/// screen's own pixel density.
pub fn window_conf(title: String) -> macroquad::window::Conf {
    macroquad::window::Conf {
        window_title: title,
        window_width: 640,
        window_height: 576,
        high_dpi: cfg!(target_arch = "wasm32"),
        window_resizable: true,
        icon: include!(concat!(env!("OUT_DIR"), "/window_icon.rs")),
        ..Default::default()
    }
}

/// A tag telling this run's window apart, for checks (`--screenshot`).
pub fn run_tag() -> u32 {
    #[cfg(target_arch = "wasm32")]
    return 0;
    #[cfg(not(target_arch = "wasm32"))]
    std::process::id()
}

/// The font file of `face` (`host::font_files`).
pub fn font_bytes(face: fonts::Face) -> Option<std::borrow::Cow<'static, [u8]>> {
    host::font_files::bytes(face)
}

/// Can `face` be drawn with now, without downloading anything?
pub fn font_ready(face: fonts::Face) -> bool {
    host::font_files::ready(face)
}

/// Can the page go fullscreen from a button of its own? (The desktop has F11 and the Display
/// setting instead.)
pub fn can_fullscreen() -> bool {
    #[cfg(target_arch = "wasm32")]
    return host::web::call(host::web::Call::CanFullscreen, 0.0) != 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Is the page fullscreen now?
pub fn is_fullscreen() -> bool {
    #[cfg(target_arch = "wasm32")]
    return host::web::call(host::web::Call::IsFullscreen, 0.0) != 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Vibrate for `ms` milliseconds, where the device can.
pub fn vibrate(ms: u32) {
    #[cfg(target_arch = "wasm32")]
    host::web::call(host::web::Call::Vibrate, ms as f64);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = ms;
}

/// The buttons whose taps the page handles itself, inside the browser's own event: the only
/// place it may enter fullscreen or open the file chooser.
#[derive(Clone, Copy)]
pub enum PageButton {
    Fullscreen,
    Browse,
}

/// Tell the page where `button` is this frame (None: it is not shown).
pub fn place_page_button(button: PageButton, at: Option<crate::domain::touch::Rect>) {
    #[cfg(target_arch = "wasm32")]
    {
        let r = at.unwrap_or(crate::domain::touch::Rect::new(0.0, 0.0, 0.0, 0.0));
        let what = match button {
            PageButton::Fullscreen => host::web::UiRect::Fullscreen,
            PageButton::Browse => host::web::UiRect::Browse,
        };
        host::web::ui_rect(what, r.x, r.y, r.w, r.h);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (button, at);
}

/// Is there a keyboard to show the buttons' keys for? (In the browser: a mouse or trackpad as the
/// main pointer, as on a computer; on a phone or tablet a key pressed tells, `macroquad_touch`.)
pub fn has_keyboard() -> bool {
    #[cfg(target_arch = "wasm32")]
    return host::web::call(host::web::Call::FinePointer, 0.0) != 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    true
}

/// Is a cartridge file on its way (the browser's chooser open, the file being read)? The start
/// screen waits for it.
pub fn cartridge_coming() -> bool {
    #[cfg(target_arch = "wasm32")]
    return host::web::call(host::web::Call::CartridgeComing, 0.0) != 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Has the device a touch screen? (On the desktop, only known once one is touched.)
pub fn has_touch() -> bool {
    #[cfg(target_arch = "wasm32")]
    return host::web::call(host::web::Call::HasTouch, 0.0) != 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    false
}

/// Are the cartridge and saves kept in the browser's storage (lost when its data is cleared)?
pub const fn keeps_files_in_browser() -> bool {
    cfg!(target_arch = "wasm32")
}

/// Show how to add the page to the home screen to play without the browser's bars (Safari on an
/// iPhone, which has no fullscreen, not opened from the home screen)?
pub fn home_screen_hint() -> bool {
    #[cfg(target_arch = "wasm32")]
    return host::web::call(host::web::Call::HomeScreenHint, 0.0) != 0.0;
    #[cfg(not(target_arch = "wasm32"))]
    false
}
