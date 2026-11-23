//! Files in the browser: the page's file chooser answers later (the file arrives in
//! `host::web::incoming_dir`, and the same call made again gets it), a file dropped on the page
//! arrives there too, and files written to `host::web::downloads_dir` are downloaded.
use super::fs_repository::TextFile;
use crate::ports::{FileDialogs, FileKind, TextFiles};
use host::web::{self, Text, TextCall};
use i18n::Lang;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Default)]
struct Waiting {
    /// The kind the open chooser was asked for.
    asked: Option<FileKind>,
    /// A file that came (asked for or dropped), for the next call of its kind.
    ready: Option<(FileKind, PathBuf)>,
    arrived: bool,
    dropped: Option<FileKind>,
}

pub struct WebDialogs {
    waiting: RefCell<Waiting>,
}

impl WebDialogs {
    pub fn new(_: Lang, _: &'static str) -> WebDialogs {
        WebDialogs { waiting: RefCell::new(Waiting::default()) }
    }

    /// Take in the files that came since the last look.
    fn look(&self) {
        let mut w = self.waiting.borrow_mut();
        while let Some(name) = web::text(Text::Arrived) {
            let Some(kind) = kind_of(&name) else { continue };
            if w.asked.is_some_and(|a| fits(a, kind)) {
                w.asked = None;
                w.arrived = true;
            } else {
                w.dropped = Some(kind);
            }
            w.ready = Some((kind, web::incoming_dir().join(name)));
        }
    }

    /// The file come for `kind`, or else the chooser opened for it.
    fn take_or_choose(&self, kind: FileKind) -> Option<PathBuf> {
        self.look();
        let mut w = self.waiting.borrow_mut();
        if let Some((_, path)) = w.ready.take_if(|(k, _)| fits(kind, *k)) {
            return Some(path);
        }
        let accept = match kind {
            FileKind::Zone => ".nmbzone,.nmbworld".to_string(),
            FileKind::Cartridge => ".gb,.gbc".to_string(),
            k => format!(".{}", k.extension()),
        };
        web::call_text(TextCall::ChooseFile, &accept);
        w.asked = Some(kind);
        None
    }
}

/// A file's kind from its name.
fn kind_of(name: &str) -> Option<FileKind> {
    let ext = Path::new(name).extension()?.to_str()?.to_ascii_lowercase();
    [FileKind::Level, FileKind::Zone, FileKind::World, FileKind::Project, FileKind::Cartridge].into_iter().find(|k| k.extension() == ext || (ext == "gbc" && *k == FileKind::Cartridge))
}

/// Does a file of kind `got` answer a call for `asked`? (Zones and world maps come through the
/// same entries.)
fn fits(asked: FileKind, got: FileKind) -> bool {
    asked == got || (asked == FileKind::Zone && got == FileKind::World)
}

impl FileDialogs for WebDialogs {
    fn save_as(&self, kind: FileKind, name: &str) -> Option<PathBuf> {
        Some(web::downloads_dir().join(format!("{name}.{}", kind.extension())))
    }
    fn open(&self, kind: FileKind) -> Option<PathBuf> {
        self.take_or_choose(kind)
    }
    /// Downloads keep only their names.
    fn shown(&self, path: &Path) -> String {
        path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
    }
    fn pick_cartridge(&self) -> Option<PathBuf> {
        self.take_or_choose(FileKind::Cartridge)
    }
    fn set_lang(&mut self, _: Lang) {}
    fn available(&self) -> bool {
        true
    }
    fn arrived(&self) -> bool {
        self.look();
        std::mem::take(&mut self.waiting.borrow_mut().arrived)
    }
    fn dropped(&self) -> Option<FileKind> {
        self.look();
        self.waiting.borrow_mut().dropped.take()
    }
}

/// The browser's storage, and downloads for what is written to `host::web::downloads_dir`.
pub struct WebFiles;

impl TextFiles for WebFiles {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        TextFile.read(path)
    }
    fn write(&self, path: &Path, text: &str) -> std::io::Result<()> {
        self.write_bytes(path, text.as_bytes())
    }
    fn write_bytes(&self, path: &Path, data: &[u8]) -> std::io::Result<()> {
        match path.strip_prefix(web::downloads_dir()) {
            Ok(name) => {
                web::download(&name.to_string_lossy(), data);
                Ok(())
            }
            Err(_) => TextFile.write_bytes(path, data),
        }
    }
}
