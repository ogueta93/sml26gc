//! The system's Save and Open dialogs for level, zone, world map and project files, starting
//! in the kind's folder of `exports\` (`host::paths`; a project's in its own,
//! `exports\projects\<name>`). Win32 on Windows, the desktop's dialog elsewhere
//! (`host::file_dialog`).
use crate::ports::{FileDialogs, FileKind};
use crate::texts::tr;
use i18n::Lang;
use std::path::PathBuf;

pub struct SystemDialogs {
    pub lang: Lang,
    /// The editor window's title: the dialogs open over it.
    pub owner: &'static str,
}

impl SystemDialogs {
    pub fn new(lang: Lang, owner: &'static str) -> SystemDialogs {
        SystemDialogs { lang, owner }
    }

    fn names(&self, kind: FileKind) -> (&'static str, &'static str, &'static str) {
        let lang = self.lang;
        match kind {
            FileKind::Level => (tr(lang, "Level file"), tr(lang, "Export level"), tr(lang, "Import level")),
            FileKind::Zone => (tr(lang, "Zone or world map file"), tr(lang, "Export zone"), tr(lang, "Import zone")),
            FileKind::World => (tr(lang, "World map file"), tr(lang, "Export world map"), tr(lang, "Import world map")),
            FileKind::Project => (tr(lang, "Project file"), tr(lang, "Export project"), tr(lang, "Import project")),
            FileKind::Cartridge => (tr(lang, "Game Boy cartridges"), tr(lang, "Export cartridge"), tr(lang, "Choose the Super Mario Land 2 cartridge")),
        }
    }
}

/// Where a kind's dialogs start (`name`: the project, for its export).
fn folder(kind: FileKind, name: Option<&str>) -> PathBuf {
    match kind {
        FileKind::Level => host::paths::level_exports(),
        FileKind::Zone => host::paths::zone_exports(),
        FileKind::World => host::paths::zone_exports(),
        FileKind::Project => name.map_or_else(|| host::exports_dir().join("projects"), host::paths::project_exports),
        FileKind::Cartridge => host::exports_dir().join("cartridges"),
    }
}

impl FileDialogs for SystemDialogs {
    fn save_as(&self, kind: FileKind, name: &str) -> Option<PathBuf> {
        let (what, title, _) = self.names(kind);
        imp::dialog(self.owner, what, title, kind.extension(), Some(name), &folder(kind, Some(name)))
    }

    fn open(&self, kind: FileKind) -> Option<PathBuf> {
        let (what, _, title) = self.names(kind);
        // zones and world maps come through the same entries
        let ext = if kind == FileKind::Zone { "nmbzone;*.nmbworld" } else { kind.extension() };
        imp::dialog(self.owner, what, title, ext, None, &folder(kind, None))
    }

    fn pick_cartridge(&self) -> Option<PathBuf> {
        let dir = host::base_cartridge::chosen_before().and_then(|p| p.parent().map(|d| d.to_path_buf())).unwrap_or_else(host::paths::data_dir);
        let (what, title) = (tr(self.lang, "Game Boy cartridges"), tr(self.lang, "Choose the Super Mario Land 2 cartridge"));
        imp::dialog(self.owner, what, title, "gb", None, &dir)
    }

    fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    fn available(&self) -> bool {
        imp::available()
    }
}

#[cfg(windows)]
mod imp {
    use std::path::PathBuf;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    use windows_sys::Win32::UI::WindowsAndMessaging::FindWindowW;

    pub fn available() -> bool {
        true
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// `save`: the suggested name, for a Save dialog; None for an Open dialog.
    pub fn dialog(owner: &str, what: &str, title: &str, ext: &str, save: Option<&str>, dir: &std::path::Path) -> Option<PathBuf> {
        let _ = std::fs::create_dir_all(dir);
        let owner = wide(owner);
        // pairs of (description, pattern), each ended by 0, the list by another 0
        let filter: Vec<u16> = format!("{what} (*.{ext})\0*.{ext}\0\0").encode_utf16().collect();
        let dir = wide(&dir.to_string_lossy());
        let title = wide(title);
        let ext_w = wide(ext);
        let mut file = vec![0u16; 4096];
        if let Some(name) = save {
            let name: Vec<u16> = name.encode_utf16().take(file.len() - 1).collect();
            file[..name.len()].copy_from_slice(&name);
        }
        let ok = unsafe {
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = FindWindowW(std::ptr::null(), owner.as_ptr());
            ofn.lpstrFilter = filter.as_ptr();
            ofn.nFilterIndex = 1;
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.lpstrInitialDir = dir.as_ptr();
            ofn.lpstrTitle = title.as_ptr();
            ofn.lpstrDefExt = ext_w.as_ptr();
            if save.is_some() {
                ofn.Flags = OFN_OVERWRITEPROMPT | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
                GetSaveFileNameW(&mut ofn)
            } else {
                ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
                GetOpenFileNameW(&mut ofn)
            }
        };
        if ok == 0 {
            return None;
        }
        let n = file.iter().position(|&c| c == 0).unwrap_or(file.len());
        Some(PathBuf::from(String::from_utf16_lossy(&file[..n])))
    }
}

#[cfg(not(windows))]
mod imp {
    pub use host::file_dialog::available;

    /// `ext`: one extension, or several in the Win32 filter's form (`nmbzone;*.nmbworld`).
    pub fn dialog(_: &str, what: &str, title: &str, ext: &str, save: Option<&str>, dir: &std::path::Path) -> Option<std::path::PathBuf> {
        let exts: Vec<&str> = ext.split(";*.").collect();
        host::file_dialog::ask(&host::file_dialog::Request { title, what, exts: &exts, dir, save })
    }
}
