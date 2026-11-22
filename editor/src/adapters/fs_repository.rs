//! The cartridge files: the base cartridge (read only) and the projects in
//! `editor\projects\<name>\`, each with its edited cartridge (paths: `host::paths`).
use crate::domain::document::EditorDocument;
use crate::domain::prefs::Prefs;
use crate::ports::{EditedCopyStore, LookStore, PrefsStore, TextFiles};
use std::path::{Path, PathBuf};

pub struct EditedCopyFile {
    path: PathBuf,
    project: PathBuf,
    place: PathBuf,
    restore_points: PathBuf,
}

impl EditedCopyFile {
    fn make_dir(&self) {
        if let Some(d) = self.path.parent() {
            let _ = host::storage::create_dir_all(d);
        }
    }
}

impl EditedCopyStore for EditedCopyFile {
    fn path(&self) -> &Path {
        &self.path
    }
    fn write(&self, rom: &[u8]) -> std::io::Result<()> {
        self.make_dir();
        host::storage::write(&self.path, rom)
    }
    fn write_project(&self, text: &str) -> std::io::Result<()> {
        self.make_dir();
        host::storage::write(&self.project, text)
    }
    fn read_place(&self) -> Option<String> {
        host::storage::read_to_string(&self.place).ok()
    }
    fn write_place(&self, text: &str) -> std::io::Result<()> {
        self.make_dir();
        host::storage::write(&self.place, text)
    }
    fn read_restore_points(&self) -> Option<String> {
        host::storage::read_to_string(&self.restore_points).ok()
    }
    fn write_restore_points(&self, text: Option<&str>) -> std::io::Result<()> {
        match text {
            Some(t) => {
                self.make_dir();
                host::storage::write(&self.restore_points, t)
            }
            None if host::storage::is_file(&self.restore_points) => host::storage::remove_file(&self.restore_points),
            None => Ok(()),
        }
    }
}

/// What opening a project gives: the document and where it saves.
pub struct Opened {
    pub doc: EditorDocument,
    pub store: EditedCopyFile,
}

/// Why a project could not be made, renamed or opened.
#[derive(Debug, PartialEq, Eq)]
pub enum ProjectError {
    BadName,
    Taken,
    Missing,
    Io(String),
}

fn io(e: std::io::Error) -> ProjectError {
    ProjectError::Io(e.to_string())
}

/// Is `name` another project's (names differ only in case on Windows)?
fn taken(name: &str) -> bool {
    host::projects().iter().any(|p| p.eq_ignore_ascii_case(name))
}

/// Open project `name` over the base cartridge `base` (its cartridge made from its project file
/// when missing or older, `host::projects::ensure_rom`).
pub fn open_project(base: &Path, name: &str) -> Result<Opened, ProjectError> {
    if !host::storage::is_dir(host::project_dir(name)) {
        return Err(ProjectError::Missing);
    }
    let original = host::storage::read(base).map_err(io)?;
    let path = host::projects::ensure_rom(base, name).map_err(|e| ProjectError::Io(e.text(i18n::Lang::En)))?;
    let edited = host::storage::read(&path).ok();
    Ok(Opened { doc: EditorDocument::open(original, edited), store: EditedCopyFile { path, project: host::project_file(name), place: host::project_place(name), restore_points: host::project_restore_points(name) } })
}

/// A new project `name`: its folder, its project file and cartridge as the game is.
pub fn create_project(base: &Path, name: &str) -> Result<(), ProjectError> {
    if !host::valid_project_name(name) {
        return Err(ProjectError::BadName);
    }
    if taken(name) {
        return Err(ProjectError::Taken);
    }
    let original = host::storage::read(base).map_err(io)?;
    let doc = EditorDocument::open(original, None);
    host::storage::create_dir_all(host::project_dir(name)).map_err(io)?;
    host::storage::write(host::project_file(name), crate::domain::files::project_text(&doc.project_file())).map_err(io)?;
    host::storage::write(host::project_rom(name, base), &doc.rom).map_err(io)
}

/// Rename project `old` to `new`: its folder, its cartridge and its saves folder.
pub fn rename_project(base: &Path, old: &str, new: &str) -> Result<(), ProjectError> {
    if !host::valid_project_name(new) {
        return Err(ProjectError::BadName);
    }
    if !old.eq_ignore_ascii_case(new) && taken(new) {
        return Err(ProjectError::Taken);
    }
    let rom = host::project_rom_in(old);
    let file = host::project_file(old).file_name().map(|n| n.to_owned());
    host::storage::rename(host::project_dir(old), host::project_dir(new)).map_err(io)?;
    if let Some(rom) = rom.and_then(|r| r.file_name().map(|n| host::project_dir(new).join(n))) {
        host::storage::rename(rom, host::project_rom(new, base)).map_err(io)?;
    }
    if let Some(f) = file.map(|n| host::project_dir(new).join(n)).filter(|f| host::storage::is_file(f)) {
        host::storage::rename(f, host::project_file(new)).map_err(io)?;
    }
    let saves = host::saves_dir(Some(old));
    if host::storage::is_dir(&saves) {
        host::storage::rename(saves, host::saves_dir(Some(new))).map_err(io)?;
    }
    Ok(())
}

/// A copy of project `name` as project `new` (with no saves).
pub fn duplicate_project(base: &Path, name: &str, new: &str) -> Result<(), ProjectError> {
    if !host::valid_project_name(new) {
        return Err(ProjectError::BadName);
    }
    if taken(new) {
        return Err(ProjectError::Taken);
    }
    host::storage::create_dir_all(host::project_dir(new)).map_err(io)?;
    if host::storage::is_file(host::project_file(name)) {
        host::storage::copy(host::project_file(name), host::project_file(new)).map_err(io)?;
    }
    if let Some(rom) = host::project_rom_in(name) {
        host::storage::copy(rom, host::project_rom(new, base)).map_err(io)?;
    }
    if host::storage::is_file(host::project_restore_points(name)) {
        host::storage::copy(host::project_restore_points(name), host::project_restore_points(new)).map_err(io)?;
    }
    Ok(())
}

/// Delete project `name`, its folder and its saves.
pub fn delete_project(name: &str) -> Result<(), ProjectError> {
    host::storage::remove_dir_all(host::project_dir(name)).map_err(io)?;
    let saves = host::saves_dir(Some(name));
    if host::storage::is_dir(&saves) {
        host::storage::remove_dir_all(saves).map_err(io)?;
    }
    Ok(())
}

/// A free project name from `wanted`: itself, or with " 2", " 3"... after it.
pub fn free_name(wanted: &str) -> String {
    let base: String = wanted.chars().filter(|c| c.is_ascii() && !c.is_control() && !r#"\/:*?"<>|"#.contains(*c)).collect::<String>().trim().trim_end_matches('.').chars().take(host::paths::PROJECT_NAME_MAX - 4).collect();
    let base = if host::valid_project_name(&base) { base } else { "Project".to_string() };
    (1..).map(|n| if n == 1 { base.clone() } else { format!("{base} {n}") }).find(|n| !taken(n)).unwrap_or(base)
}

/// `editor.cfg` next to the editor; a missing or unreadable file gives the defaults.
pub struct PrefsFile;

impl PrefsStore for PrefsFile {
    fn load(&self) -> Prefs {
        host::storage::read_to_string(host::editor_settings_path()).map(|t| Prefs::from_text(&t)).unwrap_or_default()
    }
    fn save(&self, prefs: &Prefs) {
        let _ = host::storage::write(host::editor_settings_path(), prefs.to_text());
    }
}

/// The object pictures captured for one cartridge: `cache\looks-<hash>.txt` next to the
/// editor (`hash` of the original file, so another cartridge has its own).
pub struct LookFile {
    path: PathBuf,
}

impl LookFile {
    pub fn for_cartridge(original: &[u8]) -> LookFile {
        // FNV-1a
        let hash = original.iter().fold(0xcbf2_9ce4_8422_2325u64, |h, &b| (h ^ b as u64).wrapping_mul(0x0100_0000_01b3));
        LookFile { path: host::paths::data_dir().join("cache").join(format!("looks-{hash:016x}.txt")) }
    }
}

impl LookStore for LookFile {
    fn load(&self) -> Option<String> {
        host::storage::read_to_string(&self.path).ok()
    }
    fn save(&self, text: &str) {
        if let Some(d) = self.path.parent() {
            let _ = host::storage::create_dir_all(d);
        }
        let _ = host::storage::write(&self.path, text);
    }
}

/// Language from the game's settings file (English unless Spanish was chosen).
/// The editor's language: the one chosen (`editor.cfg`), else the system's.
pub fn language() -> i18n::Lang {
    PrefsFile.load().language.unwrap_or_else(host::system_language::system_language)
}

/// Level and project files, wherever the person chose.
pub struct TextFile;

impl TextFiles for TextFile {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        host::storage::read_to_string(path)
    }
    fn write(&self, path: &Path, text: &str) -> std::io::Result<()> {
        host::storage::write(path, text)
    }
    fn write_bytes(&self, path: &Path, data: &[u8]) -> std::io::Result<()> {
        host::storage::write(path, data)
    }
}
