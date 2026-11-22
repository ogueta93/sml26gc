//! What the editor needs from the computer. `main.rs` plugs in the implementations.
use cart_model::start_point::Start;
use crate::domain::prefs::Prefs;
use std::path::{Path, PathBuf};

/// Where the edited copy is kept. The original cartridge is only ever read.
pub trait EditedCopyStore {
    fn path(&self) -> &Path;
    fn write(&self, rom: &[u8]) -> std::io::Result<()>;
    /// The project file, written before the cartridge (which is made from it).
    fn write_project(&self, text: &str) -> std::io::Result<()>;
    /// Where the editor was when the project was last saved (`place` text), if anywhere.
    fn read_place(&self) -> Option<String>;
    fn write_place(&self, text: &str) -> std::io::Result<()>;
    /// What imported levels and zones restore to (`files::restore_points_text`), if kept.
    fn read_restore_points(&self) -> Option<String>;
    /// None: there are none (the file is removed).
    fn write_restore_points(&self, text: Option<&str>) -> std::io::Result<()>;
}

/// The editor's own settings.
pub trait PrefsStore {
    fn load(&self) -> Prefs;
    fn save(&self, prefs: &Prefs);
}

/// What a test game gives Mario: lives, coins and power (0 small, 1 big, 2 carrot, 3 fire).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TestGame {
    pub lives: u32,
    pub coins: u32,
    pub power: u32,
    /// The editor's language: the game speaks it in this test game only.
    pub lang: i18n::Lang,
}

/// Starting the game on the edited copy.
pub trait GameLauncher {
    /// A test game (no saves) with Mario as `game` says. `level`: go straight into that level;
    /// `start`: with Mario there (his left x and feet y) this time only. Without a level the
    /// game goes straight to the map, Mario on `point` this time only (None: his start). The
    /// error names the program that failed.
    fn launch(&self, rom: &Path, level: Option<usize>, start: Option<(u16, u16)>, point: Option<Start>, game: TestGame) -> Result<(), String>;
    /// The last game started is still open (one test game at a time).
    fn running(&self) -> bool;
}

/// Where the captured object pictures are kept between starts (`look_cache` text).
pub trait LookStore {
    fn load(&self) -> Option<String>;
    fn save(&self, text: &str);
}

/// Background capture workers (the browser's), each running one job line at a time
/// (`look_capture`).
pub trait CapturePool: Sync {
    /// Start workers for these cartridges, stopping earlier ones: their generation, or None
    /// when none can start.
    fn start(&self, rom: &[u8], original: &[u8]) -> Option<u32>;
    /// How many more jobs the workers take now (a few ahead each); None when every worker
    /// failed to start.
    fn idle(&self) -> Option<usize>;
    fn send(&self, job: &str);
    /// The next result line.
    fn take(&self) -> Option<String>;
    /// Stop the workers of `generation` (not newer ones).
    fn stop(&self, generation: u32);
}

/// The speakers, for listening to a level's music.
pub trait SoundOut {
    fn sample_rate(&self) -> u32;
    /// Interleaved stereo samples, at `volume` (0-1).
    fn push(&self, samples: &[f32], volume: f32);
    fn queued_seconds(&self) -> f32;
}

/// The kinds of file the editor exports and imports.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FileKind {
    Level,
    Zone,
    World,
    Project,
    /// The project's edited cartridge (its custom ROM).
    Cartridge,
}

impl FileKind {
    pub fn extension(self) -> &'static str {
        match self {
            FileKind::Level => "nmblevel",
            FileKind::Zone => "nmbzone",
            FileKind::World => "nmbworld",
            FileKind::Project => "nmbproject",
            FileKind::Cartridge => "gb",
        }
    }
}

/// Asking the person for a file. None if they cancel.
pub trait FileDialogs {
    /// Where to write a new file, `name` suggested.
    fn save_as(&self, kind: FileKind, name: &str) -> Option<PathBuf>;
    fn open(&self, kind: FileKind) -> Option<PathBuf>;
    /// A Game Boy cartridge file, for the base cartridge.
    fn pick_cartridge(&self) -> Option<PathBuf>;
    /// The language of the dialogs' titles from now on.
    fn set_lang(&mut self, lang: i18n::Lang);
    /// Can a dialog open on this computer? (A cancelled one and a missing one both give None.)
    fn available(&self) -> bool;
    /// A chooser that answers later (the browser's): the last `open` or `pick_cartridge` that
    /// gave None opened one. True, once, when its file has come: the same call made again gets
    /// it.
    fn arrived(&self) -> bool {
        false
    }
    /// A file dropped on the window, unasked: its kind (a world map file is `World`). The next
    /// `open` of that kind (a zone's takes a world map's too) or `pick_cartridge` gets it.
    fn dropped(&self) -> Option<FileKind> {
        None
    }
    /// How a file written to a path `save_as` gave is named to the user.
    fn shown(&self, path: &Path) -> String {
        path.display().to_string()
    }
}

pub trait TextFiles {
    fn read(&self, path: &Path) -> std::io::Result<String>;
    fn write(&self, path: &Path, text: &str) -> std::io::Result<()>;
    fn write_bytes(&self, path: &Path, data: &[u8]) -> std::io::Result<()>;
}

pub enum MenuEntry<'a> {
    /// `keys`: its shortcut, shown at the right.
    Item { id: u16, label: &'a str, keys: &'a str },
    Separator,
}

/// What a menu bar drawn in the window took from the views this frame.
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Taken {
    pub mouse: bool,
    pub keys: bool,
}

/// The window's menu bar: the system's (nothing to draw), or one drawn along the top of the
/// window, whose `height` the views leave free.
pub trait MenuBar {
    /// Add a menu at the right of the bar. Ids are not 0.
    fn add_menu(&mut self, title: &str, entries: &[MenuEntry]);
    /// The ids of the entries chosen since the last call.
    fn chosen(&mut self) -> Vec<u16>;
    fn enable(&mut self, id: u16, on: bool);
    /// Tick entry `id` or take its tick away.
    fn check(&mut self, id: u16, on: bool);
    /// Take every menu off the bar (to build it again).
    fn clear(&mut self);
    /// The pixels it takes at the top of the window.
    fn height(&self) -> f32 {
        0.0
    }
    /// Read the mouse and keys, before the views do.
    fn input(&mut self) -> Taken {
        Taken::default()
    }
    /// Draw it, over the views.
    fn draw(&mut self) {}
}
