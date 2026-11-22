//! A project's cartridge, made from its project file over the base cartridge when it is missing
//! or older than the file (copied in, or written by another editor), for the editor and the game.
use crate::paths::{project_file, project_rom, project_rom_in};
use crate::texts::tr;
use i18n::Lang;
use project_model::build::{self, BuildError};
use project_model::files::FileError;
use std::path::{Path, PathBuf};

/// Why a project's cartridge could not be had.
#[derive(Debug)]
pub enum ProjectRomError {
    /// Neither a project file nor a cartridge.
    Missing,
    Unreadable(String),
    /// Made over another cartridge than the base one.
    OtherCartridge,
    /// Not a usable project file.
    Damaged,
    /// The project does not fit the cartridge.
    TooBig(String),
}

impl ProjectRomError {
    pub fn text(&self, lang: Lang) -> String {
        match self {
            ProjectRomError::Missing => tr(lang, "The project has no project file.").to_string(),
            ProjectRomError::Unreadable(e) => format!("{} {e}", tr(lang, "The project's files could not be read or written:")),
            ProjectRomError::OtherCartridge => tr(lang, "The project was made over another cartridge.").to_string(),
            ProjectRomError::Damaged => tr(lang, "The project file is damaged or not a project file.").to_string(),
            ProjectRomError::TooBig(e) => format!("{} {e}", tr(lang, "The project does not fit the cartridge:")),
        }
    }
}

fn modified(p: &Path) -> Option<std::time::SystemTime> {
    crate::storage::modified(p)
}

/// Does project `name` have what makes it (its project file, or at least a cartridge)?
pub fn playable(name: &str) -> bool {
    crate::storage::is_file(project_file(name)) || project_rom_in(name).is_some()
}

/// Project `name`'s cartridge over the base cartridge `base`: as it is, or made again from the
/// project file when it is missing or older than the file.
pub fn ensure_rom(base: &Path, name: &str) -> Result<PathBuf, ProjectRomError> {
    let file = project_file(name);
    let rom = project_rom_in(name);
    let fresh = match (&rom, modified(&file)) {
        (Some(r), Some(f)) => modified(r).is_some_and(|r| r >= f),
        (Some(_), None) => true,
        (None, Some(_)) => false,
        (None, None) => return Err(ProjectRomError::Missing),
    };
    if fresh {
        return Ok(rom.unwrap_or_else(|| project_rom(name, base)));
    }
    let text = crate::storage::read_to_string(&file).map_err(|e| ProjectRomError::Unreadable(e.to_string()))?;
    let original = crate::storage::read(base).map_err(|e| ProjectRomError::Unreadable(e.to_string()))?;
    let made = build::project_rom(&original, &text).map_err(|e| match e {
        BuildError::File(FileError::OtherCartridge) => ProjectRomError::OtherCartridge,
        BuildError::File(_) => ProjectRomError::Damaged,
        BuildError::Cartridge(e) => ProjectRomError::TooBig(e.to_string()),
    })?;
    let out = rom.unwrap_or_else(|| project_rom(name, base));
    crate::storage::write(&out, made).map_err(|e| ProjectRomError::Unreadable(e.to_string()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use project_model::document::EditorDocument;
    use project_model::files;

    #[test]
    fn a_project_file_alone_gets_its_cartridge() {
        let Some(base) = crate::find_rom(None) else { return };
        let name = format!("ensure test {}", std::process::id());
        let dir = crate::paths::project_dir(&name);
        std::fs::create_dir_all(&dir).unwrap();
        let mut doc = EditorDocument::open(std::fs::read(&base).unwrap(), None);
        std::fs::write(project_file(&name), files::project_text(&doc.project_file())).unwrap();
        assert!(playable(&name) && project_rom_in(&name).is_none());
        let original = std::fs::read(&base).unwrap();
        let holds = |rom: &Path| files::project_text(&EditorDocument::open(original.clone(), std::fs::read(rom).ok()).project_file());
        let rom = ensure_rom(&base, &name).unwrap();
        assert_eq!(holds(&rom), files::project_text(&doc.project_file()), "the cartridge holds the project");
        // a newer project file makes it again
        std::thread::sleep(std::time::Duration::from_millis(20));
        doc.levels[2].content.map[5] ^= 1;
        doc.levels[2].blocks_dirty = true;
        let text = files::project_text(&doc.project_file());
        std::fs::write(project_file(&name), &text).unwrap();
        assert_eq!(holds(&ensure_rom(&base, &name).unwrap()), text);
        std::fs::write(project_file(&name), "NMB project 7\nend\n").unwrap();
        assert!(matches!(ensure_rom(&base, &name), Err(ProjectRomError::Damaged)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
