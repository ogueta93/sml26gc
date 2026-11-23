//! The cartridge a project file makes: the base cartridge with the project put in and saved,
//! as the editor saves it.
use crate::document::EditorDocument;
use crate::files::{self, FileError};
use cart_model::error::CartError;

#[derive(Debug)]
pub enum BuildError {
    /// Not a usable project file, or one made over another cartridge.
    File(FileError),
    /// The project does not fit the cartridge.
    Cartridge(CartError),
}

/// The cartridge of the project file `text` over the base cartridge `base`.
pub fn project_rom(base: &[u8], text: &str) -> Result<Vec<u8>, BuildError> {
    let project = files::read_project(text).map_err(BuildError::File)?;
    let mut doc = EditorDocument::open(base.to_vec(), None);
    doc.replace_project(project).map_err(BuildError::File)?;
    doc.commit().map_err(BuildError::Cartridge)?;
    Ok(doc.rom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_file_makes_a_cartridge_holding_it() {
        let Some(path) = host::test_cartridge() else { return };
        let base = std::fs::read(path).unwrap();
        let mut doc = EditorDocument::open(base.clone(), None);
        doc.levels[3].content.map[0] ^= 1;
        doc.levels[3].blocks_dirty = true;
        let text = files::project_text(&doc.project_file());
        let rom = project_rom(&base, &text).unwrap();
        assert_eq!(files::project_text(&EditorDocument::open(base.clone(), Some(rom)).project_file()), text);
        assert!(matches!(project_rom(&base, "NMB level 5\nend\n"), Err(BuildError::File(_))));
    }
}
