//! The system around the window: keeping the check window out of the way (it goes behind the
//! others and focus returns to whatever had it; Windows only), and the system's dialog to open a
//! cartridge file (Win32, `host::file_dialog` elsewhere, or the browser's file chooser, whose
//! file arrives later: `arrived_cartridge`).

#[cfg(windows)]
mod imp {
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    static PREV: AtomicIsize = AtomicIsize::new(0);

    pub fn remember_foreground() {
        unsafe { PREV.store(GetForegroundWindow() as isize, Ordering::SeqCst) }
    }

    /// The system's "Open" dialog for a cartridge file (.gb, .gbc), over the window titled
    /// `owner`, starting in `dir`. None if cancelled.
    pub fn pick_cartridge(owner: &str, dir: Option<&std::path::Path>, title: &str, kind: &str) -> Option<std::path::PathBuf> {
        use windows_sys::Win32::UI::Controls::Dialogs::*;
        let wide = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };
        let owner = wide(owner);
        // filter: pairs of (description, pattern), each ended by 0, the list by another 0
        let filter: Vec<u16> = format!("{kind} (*.gb, *.gbc)\0*.gb;*.gbc\0\0").encode_utf16().collect();
        let dir = dir.map(|d| wide(&d.to_string_lossy()));
        let title = wide(title);
        let mut file = vec![0u16; 4096];
        unsafe {
            let mut ofn: OPENFILENAMEW = std::mem::zeroed();
            ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
            ofn.hwndOwner = FindWindowW(std::ptr::null(), owner.as_ptr());
            ofn.lpstrFilter = filter.as_ptr();
            ofn.nFilterIndex = 1;
            ofn.lpstrFile = file.as_mut_ptr();
            ofn.nMaxFile = file.len() as u32;
            ofn.lpstrInitialDir = dir.as_ref().map(|d| d.as_ptr()).unwrap_or(std::ptr::null());
            ofn.lpstrTitle = title.as_ptr();
            ofn.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
            if GetOpenFileNameW(&mut ofn) == 0 {
                return None;
            }
        }
        let n = file.iter().position(|&c| c == 0).unwrap_or(file.len());
        Some(std::path::PathBuf::from(String::from_utf16_lossy(&file[..n])))
    }

    pub fn dialog_available() -> bool {
        true
    }

    pub fn arrived_cartridge() -> Option<std::path::PathBuf> {
        None
    }

    /// Is the window titled `title` the one in front (taking the keyboard)?
    pub fn in_front(title: &str) -> bool {
        let w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), w.as_ptr());
            !hwnd.is_null() && hwnd == GetForegroundWindow()
        }
    }

    pub fn send_to_back(title: &str) {
        let w: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let hwnd = FindWindowW(std::ptr::null(), w.as_ptr());
            if !hwnd.is_null() {
                SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            let prev = PREV.load(Ordering::SeqCst);
            if prev != 0 {
                SetForegroundWindow(prev as _);
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    pub fn remember_foreground() {}
    pub fn send_to_back(_: &str) {}
    pub fn in_front(_: &str) -> bool {
        true
    }
    /// Opens the browser's file chooser; the file comes through `arrived_cartridge`.
    pub fn pick_cartridge(_: &str, _: Option<&std::path::Path>, _: &str, _: &str) -> Option<std::path::PathBuf> {
        host::web::call_text(host::web::TextCall::ChooseFile, ".gb,.gbc");
        None
    }

    pub fn dialog_available() -> bool {
        true
    }

    /// A file chosen in the browser's file chooser or dropped on the page since the last frame.
    pub fn arrived_cartridge() -> Option<std::path::PathBuf> {
        host::web::text(host::web::Text::Arrived).map(|name| host::web::incoming_dir().join(name))
    }
}

#[cfg(all(not(windows), not(target_arch = "wasm32")))]
mod imp {
    pub fn remember_foreground() {}
    pub fn send_to_back(_: &str) {}
    pub fn in_front(_: &str) -> bool {
        true
    }
    pub fn pick_cartridge(_: &str, dir: Option<&std::path::Path>, title: &str, kind: &str) -> Option<std::path::PathBuf> {
        let dir = dir.map_or_else(host::paths::data_dir, |d| d.to_path_buf());
        host::file_dialog::ask(&host::file_dialog::Request { title, what: kind, exts: &["gb", "gbc"], dir: &dir, save: None })
    }

    pub fn dialog_available() -> bool {
        host::file_dialog::available()
    }

    pub fn arrived_cartridge() -> Option<std::path::PathBuf> {
        None
    }
}

pub use imp::{arrived_cartridge, dialog_available, in_front, pick_cartridge, remember_foreground, send_to_back};
