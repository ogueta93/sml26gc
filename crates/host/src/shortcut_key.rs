//! The key that makes the programs' shortcuts with a letter (Save, Undo, Copy...): Ctrl, or Cmd
//! on macOS, as each system's own programs do.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShortcutKey {
    Control,
    Command,
}

impl ShortcutKey {
    /// As the programs write it in front of a key (`Ctrl+S`, `Cmd+S`).
    pub fn label(self) -> &'static str {
        match self {
            ShortcutKey::Control => "Ctrl",
            ShortcutKey::Command => "Cmd",
        }
    }
}

/// This system's shortcut key (in the browser, the computer's).
pub fn shortcut_key() -> ShortcutKey {
    #[cfg(target_arch = "wasm32")]
    return if crate::web::call(crate::web::Call::IsMac, 0.0) != 0.0 { ShortcutKey::Command } else { ShortcutKey::Control };
    #[cfg(not(target_arch = "wasm32"))]
    if cfg!(target_os = "macos") { ShortcutKey::Command } else { ShortcutKey::Control }
}

/// A browser keeps the shortcut key with N (and Shift+N) for its own windows: the programs use
/// Alt for those there.
pub const BROWSER_KEEPS_N: bool = cfg!(target_arch = "wasm32");
