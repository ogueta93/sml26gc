//! Finding the person's own cartridge file.
use std::path::{Path, PathBuf};

/// What to tell the person when no cartridge was found.
pub const NO_CARTRIDGE: &str = "No Game Boy cartridge file (.gb) was found.\nPut it in a 'rom' folder next to this program\nor pass its path on the command line.";

fn is_rom(p: &Path) -> bool {
    p.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("gb") || e.eq_ignore_ascii_case("gbc"))
        .unwrap_or(false)
}

fn first_rom_in(dir: &Path) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = crate::storage::read_dir(dir).ok()?.into_iter().filter(|p| is_rom(p)).collect();
    v.sort();
    v.into_iter().next()
}

/// Where to find the cartridge (for the tools and tests; the programs use the base cartridge):
/// the command line, the base cartridge, a `rom` folder beside the program (or its parent
/// folders), then the person's Games folder.
pub fn find_rom(arg: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = arg {
        if crate::storage::is_dir(&p) {
            return first_rom_in(&p);
        }
        return Some(p);
    }
    if let Ok(p) = crate::base_cartridge::base_cartridge() {
        return Some(p);
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut d = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..5 {
            let Some(dir) = d else { break };
            for sub in ["rom", "roms", "."] {
                if let Some(r) = first_rom_in(&dir.join(sub)) {
                    return Some(r);
                }
            }
            d = dir.parent().map(|p| p.to_path_buf());
        }
    }
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).ok()?;
    let games = Path::new(&home).join("Games");
    if let Some(r) = first_rom_in(&games.join("Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1)")) {
        return Some(r);
    }
    for p in crate::storage::read_dir(&games).ok()? {
        if crate::storage::is_dir(&p) {
            if let Some(r) = first_rom_in(&p) {
                return Some(r);
            }
        } else if is_rom(&p) {
            return Some(p);
        }
    }
    None
}

/// Like `find_rom`, but an error message the person can act on when there is none, or when
/// the path given doesn't exist.
pub fn require_rom(arg: Option<PathBuf>) -> Result<PathBuf, String> {
    match find_rom(arg) {
        Some(p) if crate::storage::is_file(&p) => Ok(p),
        Some(p) => Err(format!("The cartridge file was not found:\n{}", p.display())),
        None => Err(NO_CARTRIDGE.to_string()),
    }
}
