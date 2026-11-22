//! The system's own file dialog off Windows (the programs' Windows adapters call Win32 directly).
//! On macOS the Open and Save panels. On Linux the XDG desktop portal, with zenity when there is no
//! portal; neither says whether the person cancelled or no dialog could open, so `available` is
//! asked first.
use i18n::Lang;
use std::path::{Path, PathBuf};

/// One request: `exts` without the dot; `save`: the suggested name, for a Save dialog (None for
/// an Open dialog).
pub struct Request<'a> {
    pub title: &'a str,
    pub what: &'a str,
    pub exts: &'a [&'a str],
    pub dir: &'a Path,
    pub save: Option<&'a str>,
}

/// The file chosen; None if cancelled or no dialog could open (`available`).
pub fn ask(request: &Request) -> Option<PathBuf> {
    imp::ask(request)
}

/// Can a dialog open on this computer?
pub fn available() -> bool {
    imp::available()
}

/// What to tell the person when no dialog can open, in `lang`.
pub fn unavailable_text(lang: Lang) -> &'static str {
    crate::texts::tr(lang, "No file dialog is available on this computer: install xdg-desktop-portal (with the one of your desktop) or zenity. The game also takes the cartridge file as its argument.")
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod imp {
    use super::Request;
    use std::path::PathBuf;

    pub fn ask(r: &Request) -> Option<PathBuf> {
        let _ = std::fs::create_dir_all(r.dir);
        let dialog = rfd::FileDialog::new().set_title(r.title).add_filter(r.what, r.exts).set_directory(r.dir);
        match r.save {
            Some(name) => dialog.set_file_name(name).save_file().map(|p| with_extension(p, r.exts[0])),
            None => dialog.pick_file(),
        }
    }

    /// Some dialogs keep the name as typed: the kind's extension is added when it is missing.
    fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
        if path.extension().is_some_and(|e| e.eq_ignore_ascii_case(ext)) { path } else { PathBuf::from(format!("{}.{ext}", path.display())) }
    }

    #[cfg(target_os = "macos")]
    pub fn available() -> bool {
        true
    }

    /// zenity on the PATH, or a session bus with the portal's service installed.
    #[cfg(target_os = "linux")]
    pub fn available() -> bool {
        use std::path::Path;
        let on_path = |exe: &str| std::env::var_os("PATH").is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(exe).is_file()));
        let bus = std::env::var_os("DBUS_SESSION_BUS_ADDRESS").is_some() || std::env::var_os("XDG_RUNTIME_DIR").is_some_and(|d| Path::new(&d).join("bus").exists());
        let data_dirs = std::env::var("XDG_DATA_DIRS").ok().filter(|d| !d.is_empty()).unwrap_or_else(|| "/usr/local/share:/usr/share".into());
        let portal = data_dirs.split(':').any(|d| Path::new(d).join("dbus-1/services/org.freedesktop.portal.Desktop.service").is_file());
        on_path("zenity") || (bus && portal)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::path::Path;

        #[test]
        fn saved_files_get_their_extension() {
            assert_eq!(with_extension(PathBuf::from("/x/Islands 07"), "nmblevel"), Path::new("/x/Islands 07.nmblevel"));
            assert_eq!(with_extension(PathBuf::from("/x/a.nmblevel"), "nmblevel"), Path::new("/x/a.nmblevel"));
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod imp {
    use super::Request;
    use std::path::PathBuf;

    pub fn ask(_: &Request) -> Option<PathBuf> {
        None
    }

    pub fn available() -> bool {
        false
    }
}
