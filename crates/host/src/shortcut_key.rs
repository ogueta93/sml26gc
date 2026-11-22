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

/// This system's shortcut key.
pub const SHORTCUT_KEY: ShortcutKey = if cfg!(target_os = "macos") { ShortcutKey::Command } else { ShortcutKey::Control };
