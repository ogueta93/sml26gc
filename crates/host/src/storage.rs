//! Every file the programs read or write goes through here. On the desktop these are the
//! system's files (`std::fs`). In the browser they are kept in memory, loaded from the page's
//! storage (IndexedDB) before the program starts, and each change is sent back to it; another
//! page of the site that changes a file is seen at the next call.
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    pub fn read(p: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(p)
    }
    pub fn write(p: &Path, data: &[u8]) -> io::Result<()> {
        std::fs::write(p, data)
    }
    pub fn create_dir_all(p: &Path) -> io::Result<()> {
        std::fs::create_dir_all(p)
    }
    pub fn remove_file(p: &Path) -> io::Result<()> {
        std::fs::remove_file(p)
    }
    pub fn remove_dir_all(p: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(p)
    }
    pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }
    pub fn read_dir(p: &Path) -> io::Result<Vec<PathBuf>> {
        Ok(std::fs::read_dir(p)?.flatten().map(|e| e.path()).collect())
    }
    pub fn is_file(p: &Path) -> bool {
        p.is_file()
    }
    pub fn is_dir(p: &Path) -> bool {
        p.is_dir()
    }
    pub fn modified(p: &Path) -> Option<SystemTime> {
        std::fs::metadata(p).ok()?.modified().ok()
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use crate::web;
    use std::cell::RefCell;

    thread_local! {
        static FILES: RefCell<MemFiles> = RefCell::new(MemFiles::default());
    }

    /// The map, with what other pages (or the page's first load) changed taken in first.
    fn with<T>(f: impl FnOnce(&mut MemFiles) -> T) -> T {
        FILES.with(|m| {
            let mut m = m.borrow_mut();
            while let Some((path, data, time)) = web::storage_next() {
                match data {
                    Some(d) => m.put(&path, d, time),
                    None => m.forget(&path),
                }
            }
            f(&mut m)
        })
    }

    fn put(p: &Path, data: &[u8]) {
        let time = web::now_ms();
        with(|m| m.put(&key(p), data.to_vec(), time));
        web::storage_put(&key(p), data, time);
    }

    fn forget(paths: Vec<String>) {
        with(|m| paths.iter().for_each(|p| m.forget(p)));
        paths.iter().for_each(|p| web::storage_delete(p));
    }

    pub fn read(p: &Path) -> io::Result<Vec<u8>> {
        with(|m| m.get(&key(p)).map(|f| f.data.clone())).ok_or_else(|| not_found(p))
    }
    pub fn write(p: &Path, data: &[u8]) -> io::Result<()> {
        put(p, data);
        Ok(())
    }
    pub fn create_dir_all(p: &Path) -> io::Result<()> {
        with(|m| m.make_dir(&key(p)));
        Ok(())
    }
    pub fn remove_file(p: &Path) -> io::Result<()> {
        if !is_file(p) {
            return Err(not_found(p));
        }
        forget(vec![key(p)]);
        Ok(())
    }
    pub fn remove_dir_all(p: &Path) -> io::Result<()> {
        let k = key(p);
        let gone = with(|m| {
            m.drop_dir(&k);
            m.under(&k)
        });
        forget(gone);
        Ok(())
    }
    pub fn rename(from: &Path, to: &Path) -> io::Result<()> {
        let (from, to) = (key(from), key(to));
        let moved = with(|m| m.moves(&from, &to));
        if moved.is_empty() && !with(|m| m.is_dir(&from)) {
            return Err(io::Error::new(io::ErrorKind::NotFound, from));
        }
        with(|m| {
            m.drop_dir(&from);
            m.make_dir(&to)
        });
        for (old, new) in moved {
            let f = with(|m| m.get(&old).cloned()).expect("listed");
            forget(vec![old]);
            with(|m| m.put(&new, f.data.clone(), f.time));
            web::storage_put(&new, &f.data, f.time);
        }
        Ok(())
    }
    pub fn read_dir(p: &Path) -> io::Result<Vec<PathBuf>> {
        let k = key(p);
        with(|m| m.is_dir(&k).then(|| m.children(&k).into_iter().map(PathBuf::from).collect())).ok_or_else(|| not_found(p))
    }
    pub fn is_file(p: &Path) -> bool {
        with(|m| m.get(&key(p)).is_some())
    }
    pub fn is_dir(p: &Path) -> bool {
        with(|m| m.is_dir(&key(p)))
    }
    pub fn modified(p: &Path) -> Option<SystemTime> {
        with(|m| m.get(&key(p)).map(|f| from_ms(f.time)))
    }

    fn not_found(p: &Path) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, p.display().to_string())
    }
}

/// A folder tree kept in memory: files by their full path, and the folders made empty.
#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Default)]
pub(crate) struct MemFiles {
    files: std::collections::BTreeMap<String, MemFile>,
    dirs: std::collections::BTreeSet<String>,
}

#[cfg(any(test, target_arch = "wasm32"))]
#[derive(Clone)]
pub(crate) struct MemFile {
    pub data: Vec<u8>,
    /// Milliseconds since 1970.
    pub time: f64,
}

#[cfg(any(test, target_arch = "wasm32"))]
impl MemFiles {
    pub fn put(&mut self, path: &str, data: Vec<u8>, time: f64) {
        self.files.insert(path.to_string(), MemFile { data, time });
    }
    pub fn forget(&mut self, path: &str) {
        self.files.remove(path);
    }
    pub fn get(&self, path: &str) -> Option<&MemFile> {
        self.files.get(path)
    }
    pub fn make_dir(&mut self, path: &str) {
        self.dirs.insert(path.trim_end_matches('/').to_string());
    }
    pub fn drop_dir(&mut self, path: &str) {
        let inside = format!("{path}/");
        self.dirs.retain(|d| d != path && !d.starts_with(&inside));
    }
    /// A folder: made, or holding a file somewhere below it.
    pub fn is_dir(&self, path: &str) -> bool {
        let inside = format!("{}/", path.trim_end_matches('/'));
        path == "/" || self.dirs.iter().any(|d| d == path || d.starts_with(&inside)) || self.files.keys().any(|f| f.starts_with(&inside))
    }
    /// Every file below `path`.
    pub fn under(&self, path: &str) -> Vec<String> {
        let inside = format!("{path}/");
        self.files.keys().filter(|f| f.starts_with(&inside)).cloned().collect()
    }
    /// The files and folders right inside `path`, by full path.
    pub fn children(&self, path: &str) -> Vec<String> {
        let inside = format!("{}/", path.trim_end_matches('/'));
        let mut out: Vec<String> = self.files.keys().chain(self.dirs.iter()).filter_map(|p| p.strip_prefix(&inside)).filter(|rest| !rest.is_empty()).map(|rest| format!("{inside}{}", rest.split('/').next().unwrap_or(rest))).collect();
        out.sort();
        out.dedup();
        out
    }
    /// The old and new paths of the file `from`, or of every file below the folder `from`, moved to `to`.
    pub fn moves(&self, from: &str, to: &str) -> Vec<(String, String)> {
        if self.files.contains_key(from) {
            return vec![(from.to_string(), to.to_string())];
        }
        self.under(from).into_iter().map(|f| (f.clone(), format!("{to}{}", &f[from.len()..]))).collect()
    }
}

#[cfg(target_arch = "wasm32")]
fn key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let trimmed = s.trim_end_matches('/');
    if trimmed.is_empty() { "/".into() } else { trimmed.to_string() }
}

/// The time `ms` milliseconds after 1970.
pub fn from_ms(ms: f64) -> SystemTime {
    SystemTime::UNIX_EPOCH + std::time::Duration::from_millis(ms.max(0.0) as u64)
}

pub fn read(p: impl AsRef<Path>) -> io::Result<Vec<u8>> {
    imp::read(p.as_ref())
}
pub fn read_to_string(p: impl AsRef<Path>) -> io::Result<String> {
    String::from_utf8(read(p)?).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
pub fn write(p: impl AsRef<Path>, data: impl AsRef<[u8]>) -> io::Result<()> {
    imp::write(p.as_ref(), data.as_ref())
}
pub fn create_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
    imp::create_dir_all(p.as_ref())
}
pub fn remove_file(p: impl AsRef<Path>) -> io::Result<()> {
    imp::remove_file(p.as_ref())
}
pub fn remove_dir_all(p: impl AsRef<Path>) -> io::Result<()> {
    imp::remove_dir_all(p.as_ref())
}
pub fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    imp::rename(from.as_ref(), to.as_ref())
}
/// The paths of what is right inside folder `p`.
pub fn read_dir(p: impl AsRef<Path>) -> io::Result<Vec<PathBuf>> {
    imp::read_dir(p.as_ref())
}
/// A copy of file `from` at `to`.
pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    write(to, read(from)?)
}
pub fn is_file(p: impl AsRef<Path>) -> bool {
    imp::is_file(p.as_ref())
}
pub fn is_dir(p: impl AsRef<Path>) -> bool {
    imp::is_dir(p.as_ref())
}
pub fn modified(p: impl AsRef<Path>) -> Option<SystemTime> {
    imp::modified(p.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folders_come_from_their_files() {
        let mut m = MemFiles::default();
        m.put("/d/editor/projects/Sky/Sky.nmbproject", b"p".to_vec(), 5.0);
        m.put("/d/settings.cfg", b"s".to_vec(), 6.0);
        m.make_dir("/d/saves");
        assert!(m.is_dir("/d") && m.is_dir("/d/editor/projects/Sky") && m.is_dir("/d/saves"));
        assert!(!m.is_dir("/d/settings.cfg") && !m.is_dir("/d/edit"));
        assert_eq!(m.children("/d"), ["/d/editor", "/d/saves", "/d/settings.cfg"]);
        assert_eq!(m.children("/d/editor/projects"), ["/d/editor/projects/Sky"]);
        assert_eq!(m.get("/d/settings.cfg").map(|f| (f.data.as_slice(), f.time)), Some((&b"s"[..], 6.0)));
        m.forget("/d/settings.cfg");
        m.drop_dir("/d/saves");
        assert_eq!(m.children("/d"), ["/d/editor"]);
    }

    #[test]
    fn a_folder_moves_with_its_files() {
        let mut m = MemFiles::default();
        m.put("/p/Sky/Sky.nmbproject", b"p".to_vec(), 1.0);
        m.put("/p/Sky/base - Sky.gb", b"g".to_vec(), 1.0);
        m.put("/p/Skyline/x", b"x".to_vec(), 1.0);
        let moves = m.moves("/p/Sky", "/p/Sea");
        assert_eq!(moves, [("/p/Sky/Sky.nmbproject".to_string(), "/p/Sea/Sky.nmbproject".to_string()), ("/p/Sky/base - Sky.gb".to_string(), "/p/Sea/base - Sky.gb".to_string())]);
        assert_eq!(m.moves("/p/Skyline/x", "/p/y"), [("/p/Skyline/x".to_string(), "/p/y".to_string())]);
        assert_eq!(m.under("/p/Sky").len(), 2, "Skyline is not inside Sky");
    }

    #[test]
    fn times_since_1970() {
        assert_eq!(from_ms(1500.0).duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis(), 1500);
    }
}
