//! Where every file lives. The game and the editor both use these, so they always agree
//! (this is part of the contract between them, see docs/ARCHITECTURE.md, "Contract between game and editor").
//!
//! In the data folder (`data_dir`: the executables' own folder, as the programs are portable, or
//! the user's data folder for the installed form and the macOS app bundles):
//!   base_cartridge.txt                the original cartridge file both programs use
//!   settings.cfg                      the game's settings
//!   editor.cfg                        the editor's own settings
//!   editor\projects\<name>\           a project: its file `<name>.nmbproject` and the cartridge
//!                                     made from it over the base cartridge, `<base> - <name>.gb`,
//!                                     editor_place.txt, where the editor was when it saved, and
//!                                     restore_points.txt, what imported levels and zones restore to
//!   saves\vanilla\, saves\<project>\  battery save (game.sav) and new zone progress (zones.txt)
//!   exports\                          level, zone, world map and project files the editor writes
use std::path::{Path, PathBuf};

/// The folder the running program sits in (where the other program is found).
pub fn exe_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| PathBuf::from("."))
}

/// The file the installed form (the Linux `.deb`) puts next to the executables: its data lives in
/// the user's folder, since the executables' folder cannot be written.
pub const INSTALLED_MARKER: &str = "installed";

/// The folder every data file lives in, created when missing: the executables' folder, so the
/// unpacked package is self-contained; for the installed form, `$XDG_DATA_HOME/<id>`; for a macOS
/// app bundle, whose folder must not be written, `~/Library/Application Support/<id>`.
pub fn data_dir() -> PathBuf {
    static DIR: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    DIR.get_or_init(|| {
        let exe = exe_dir();
        let dir = if cfg!(target_os = "macos") && bundle_of(&exe).is_some() {
            app_support_dir(&exe, std::env::var_os("HOME"))
        } else {
            data_dir_for(&exe, std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
        };
        let _ = std::fs::create_dir_all(&dir);
        dir
    })
    .clone()
}

/// The app bundle (`.../Name.app`) holding the executables' folder `exe_dir`, which is its
/// `Contents/MacOS`.
fn bundle_of(exe_dir: &Path) -> Option<&Path> {
    let contents = exe_dir.parent().filter(|_| exe_dir.file_name().is_some_and(|n| n == "MacOS"))?;
    let bundle = contents.parent().filter(|_| contents.file_name().is_some_and(|n| n == "Contents"))?;
    bundle.extension().is_some_and(|e| e.eq_ignore_ascii_case("app")).then_some(bundle)
}

/// The macOS apps' data folder, shared by both; the executables' folder without a home.
fn app_support_dir(exe_dir: &Path, home: Option<std::ffi::OsString>) -> PathBuf {
    home.map(PathBuf::from).filter(|h| h.is_absolute()).map_or_else(|| exe_dir.to_path_buf(), |h| h.join("Library/Application Support").join(env!("PRODUCT_ID")))
}

/// A relative `XDG_DATA_HOME` is invalid and ignored (XDG Base Directory specification).
fn data_dir_for(exe_dir: &Path, xdg_data_home: Option<std::ffi::OsString>, home: Option<std::ffi::OsString>) -> PathBuf {
    if !exe_dir.join(INSTALLED_MARKER).is_file() {
        return exe_dir.to_path_buf();
    }
    let base = xdg_data_home.map(PathBuf::from).filter(|d| d.is_absolute()).or_else(|| home.map(|h| PathBuf::from(h).join(".local/share")));
    base.map_or_else(|| exe_dir.to_path_buf(), |b| b.join(env!("PRODUCT_ID")))
}

/// Why the data folder is of no use, in `lang`: None when it can be written. Tried by making and
/// removing a file.
pub fn data_dir_problem(lang: i18n::Lang) -> Option<&'static str> {
    (!data_dir_writable()).then(|| crate::texts::tr(lang, "This folder cannot be written: settings, projects and saves will not be kept. Move the program's folder somewhere you can write, such as Documents."))
}

fn data_dir_writable() -> bool {
    let probe = data_dir().join(format!(".write-test-{}", std::process::id()));
    let ok = std::fs::write(&probe, b"").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

fn stem(rom: &Path) -> String {
    rom.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "game".into())
}

/// The file naming the base cartridge (`base_cartridge`).
pub fn base_cartridge_path() -> PathBuf {
    data_dir().join("base_cartridge.txt")
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.cfg")
}

pub fn editor_settings_path() -> PathBuf {
    data_dir().join("editor.cfg")
}

/// The longest project name.
pub const PROJECT_NAME_MAX: usize = 40;
/// The game's start screen name for the original cartridge, which no project may take.
pub const VANILLA: &str = "VANILLA";

/// Can `name` name a project (and its folder)? Printable ASCII (the editor's font has no
/// accents), no path characters, not VANILLA.
pub fn valid_project_name(name: &str) -> bool {
    let t = name.trim();
    !t.is_empty()
        && t == name
        && t.chars().count() <= PROJECT_NAME_MAX
        && t.chars().all(|c| c.is_ascii() && !c.is_ascii_control() && !r#"\/:*?"<>|"#.contains(c))
        && !t.ends_with('.')
        && !t.eq_ignore_ascii_case(VANILLA)
}

pub fn projects_dir() -> PathBuf {
    data_dir().join("editor").join("projects")
}

pub fn project_dir(name: &str) -> PathBuf {
    projects_dir().join(name)
}

/// The edited cartridge of project `name`, made from the base cartridge `base`.
pub fn project_rom(name: &str, base: &Path) -> PathBuf {
    project_dir(name).join(format!("{} - {name}.gb", stem(base)))
}

/// Project `name`'s file, what the project is (its cartridge is made from it).
pub fn project_file(name: &str) -> PathBuf {
    project_dir(name).join(format!("{name}.nmbproject"))
}

/// Where the editor was in project `name` when it was last saved.
pub fn project_place(name: &str) -> PathBuf {
    project_dir(name).join("editor_place.txt")
}

/// What project `name`'s imported levels and zones restore to (the editor's only).
pub fn project_restore_points(name: &str) -> PathBuf {
    project_dir(name).join("restore_points.txt")
}

/// The cartridge file in project `name`'s folder, whatever it is called.
pub fn project_rom_in(name: &str) -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(project_dir(name)).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gb"))).collect();
    v.sort();
    v.into_iter().next()
}

/// The projects (folders of `projects_dir`), sorted by name.
pub fn projects() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(projects_dir()).into_iter().flatten().flatten().filter(|e| e.path().is_dir()).filter_map(|e| e.file_name().to_str().map(String::from)).collect();
    v.sort_by_key(|n| n.to_lowercase());
    v
}

/// Where a game's saves go: `saves/vanilla` for the original cartridge, else the project's.
pub fn saves_dir(project: Option<&str>) -> PathBuf {
    data_dir().join("saves").join(project.unwrap_or("vanilla"))
}

/// The battery save in a saves folder.
pub fn battery_file(saves: &Path) -> PathBuf {
    saves.join("game.sav")
}

/// The progress in new zones and added points in a saves folder.
pub fn zones_file(saves: &Path) -> PathBuf {
    saves.join("zones.txt")
}

/// Where the editor's Save and Open dialogs start, by kind of file.
pub fn exports_dir() -> PathBuf {
    data_dir().join("exports")
}

pub fn project_exports(name: &str) -> PathBuf {
    exports_dir().join("projects").join(name)
}

pub fn level_exports() -> PathBuf {
    exports_dir().join("levels")
}

pub fn zone_exports() -> PathBuf {
    exports_dir().join("zones")
}

/// The game program, next to the editor (or in its app bundle beside the editor's).
pub fn game_exe() -> PathBuf {
    other_program(&exe_dir(), GAME_PROGRAM, GAME_BUNDLE)
}

/// The editor program, next to the game (or in its app bundle beside the game's).
pub fn editor_exe() -> PathBuf {
    other_program(&exe_dir(), EDITOR_PROGRAM, EDITOR_BUNDLE)
}

/// `program` next to the running one in `exe_dir`; when it is not there and the running one is in
/// a macOS app bundle, in `bundle` beside it. The first place when neither has it.
fn other_program(exe_dir: &Path, program: &str, bundle: &str) -> PathBuf {
    let beside = exe_dir.join(format!("{program}{}", std::env::consts::EXE_SUFFIX));
    if beside.is_file() || !cfg!(target_os = "macos") {
        return beside;
    }
    other_bundle_program(exe_dir, program, bundle).filter(|p| p.is_file()).unwrap_or(beside)
}

/// Where `program` is in `bundle` when the running program's folder `exe_dir` is in an app bundle.
fn other_bundle_program(exe_dir: &Path, program: &str, bundle: &str) -> Option<PathBuf> {
    Some(bundle_of(exe_dir)?.parent()?.join(bundle).join("Contents/MacOS").join(program))
}

/// The programs' names (the `[[bin]]` names of `game` and `editor`), without the system's suffix.
pub const GAME_PROGRAM: &str = "sml26gc";
pub const EDITOR_PROGRAM: &str = "sml26gc_editor";

/// The macOS app bundles of the game and the editor (made by `xtask/src/macos.rs`), kept side by side.
pub const GAME_BUNDLE: &str = "SML2 6GC.app";
pub const EDITOR_BUNDLE: &str = "SML2 6GC Editor.app";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_projects_files() {
        let rom = Path::new("C:/Games/Some Game (USA).gb");
        assert!(project_rom("Islands", rom).ends_with("editor/projects/Islands/Some Game (USA) - Islands.gb"));
        assert!(project_file("Islands").ends_with("editor/projects/Islands/Islands.nmbproject"));
        assert!(battery_file(&saves_dir(Some("Islands"))).ends_with("saves/Islands/game.sav"));
        assert!(zones_file(&saves_dir(None)).ends_with("saves/vanilla/zones.txt"));
        assert!(project_exports("Islands").ends_with("exports/projects/Islands"));
    }

    #[test]
    fn the_programs_find_each_other_by_their_bin_names() {
        let manifest = |crate_dir: &str| std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(crate_dir).join("Cargo.toml")).unwrap().replace('\r', "");
        assert!(manifest("game").contains(&format!("[[bin]]\nname = \"{GAME_PROGRAM}\"")));
        assert!(manifest("editor").contains(&format!("[[bin]]\nname = \"{EDITOR_PROGRAM}\"")));
        assert!(game_exe().file_stem().is_some_and(|s| s == GAME_PROGRAM));
    }

    #[cfg(unix)]
    #[test]
    fn the_data_folder_of_each_form() {
        let exe = std::env::temp_dir().join(format!("sml26gc-data-dir-{}", std::process::id()));
        std::fs::create_dir_all(&exe).unwrap();
        let (xdg, home) = (Some("/home/u/.data".into()), Some("/home/u".into()));
        assert_eq!(data_dir_for(&exe, xdg.clone(), home.clone()), exe);
        std::fs::write(exe.join(INSTALLED_MARKER), "").unwrap();
        assert_eq!(data_dir_for(&exe, xdg, home.clone()), Path::new("/home/u/.data/sml26gc"));
        assert_eq!(data_dir_for(&exe, None, home.clone()), Path::new("/home/u/.local/share/sml26gc"));
        assert_eq!(data_dir_for(&exe, Some("relative".into()), home), Path::new("/home/u/.local/share/sml26gc"));
        assert_eq!(data_dir_for(&exe, None, None), exe);
        std::fs::remove_dir_all(&exe).unwrap();
    }

    #[test]
    fn the_macos_app_bundles() {
        let editor = Path::new("/Applications/SML2 6GC Editor.app/Contents/MacOS");
        assert_eq!(bundle_of(editor), Some(Path::new("/Applications/SML2 6GC Editor.app")));
        assert_eq!(bundle_of(Path::new("/Users/u/sml26gc/target/release")), None);
        assert_eq!(bundle_of(Path::new("/x/Contents/MacOS")), None);
        assert_eq!(other_bundle_program(editor, GAME_PROGRAM, GAME_BUNDLE).unwrap(), Path::new("/Applications/SML2 6GC.app/Contents/MacOS/sml26gc"));
        assert_eq!(other_bundle_program(Path::new("/Users/u/target/release"), GAME_PROGRAM, GAME_BUNDLE), None);
    }

    /// `/Users/u` is an absolute home only on Unix (on Windows it lacks a drive).
    #[cfg(unix)]
    #[test]
    fn the_macos_apps_data_folder() {
        let editor = Path::new("/Applications/SML2 6GC Editor.app/Contents/MacOS");
        assert_eq!(app_support_dir(editor, Some("/Users/u".into())), Path::new("/Users/u/Library/Application Support/sml26gc"));
        assert_eq!(app_support_dir(editor, None), editor);
    }

    #[test]
    fn the_xtask_makes_the_bundles_the_programs_look_for() {
        let macos = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../xtask/src/macos.rs")).unwrap();
        for bundle in [GAME_BUNDLE, EDITOR_BUNDLE] {
            let name = bundle.strip_suffix(".app").unwrap();
            assert!(macos.contains(&format!("\"{name}\"")), "xtask/src/macos.rs does not name {name}");
        }
    }

    #[test]
    fn project_names() {
        assert!(valid_project_name("Sky Islands 2"));
        assert!(valid_project_name("Mundo (v2) - copia_1"));
        for bad in ["", " lead", "trail ", "a/b", "a:b", "dot.", "vanilla", "VANILLA", "Mundo ñ", &"x".repeat(41)] {
            assert!(!valid_project_name(bad), "{bad:?}");
        }
    }
}
