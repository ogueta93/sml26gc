//! Settings, battery saves and new zone progress kept in files (where: `host::paths`), plus
//! stores that keep nothing, for unattended checks.
use crate::domain::settings::Settings;
use crate::domain::zone_progress::ZoneProgress;
use crate::ports::{SaveStore, SettingsStore, ZoneProgressStore};
use gb_core::Gb;
use std::path::PathBuf;

/// `settings.cfg` next to the programs.
pub struct SettingsFile;

impl SettingsStore for SettingsFile {
    /// An automatic language is the system's (English when it is none of the game's).
    fn load(&self) -> Settings {
        let mut s = host::settings_file::read().map(|t| Settings::from_text(&t)).unwrap_or_default();
        if s.lang_auto {
            s.lang = host::system_language::system_language() as usize;
        }
        s
    }
    fn save(&self, settings: &Settings) {
        let _ = host::settings_file::write(&settings.to_text());
    }
}

/// Default settings, never written (checks must not touch the player's file).
pub struct DefaultSettings;

impl SettingsStore for DefaultSettings {
    fn load(&self) -> Settings {
        Settings::default()
    }
    fn save(&self, _: &Settings) {}
}

/// The battery save in a saves folder (`host::battery_file`).
pub struct BatteryFile {
    pub saves: PathBuf,
}

impl SaveStore for BatteryFile {
    fn save(&self, gb: &mut Gb) {
        if let Err(e) = host::write_battery_save(gb, &host::battery_file(&self.saves)) {
            eprintln!("could not write the save file: {e}");
        }
    }
}

/// No battery save kept (checks, and the editor's test games).
pub struct NoBattery;

impl SaveStore for NoBattery {
    fn save(&self, _: &mut Gb) {}
}

/// The new zone progress in a saves folder (`host::zones_file`).
pub struct ZoneFile {
    pub saves: PathBuf,
}

impl ZoneProgressStore for ZoneFile {
    fn load(&self) -> Option<ZoneProgress> {
        ZoneProgress::from_text(&host::storage::read_to_string(host::zones_file(&self.saves)).ok()?)
    }
    fn save(&self, progress: &ZoneProgress) {
        let p = host::zones_file(&self.saves);
        let written = p.parent().map_or(Ok(()), host::storage::create_dir_all).and_then(|_| host::storage::write(&p, progress.to_text()));
        if let Err(e) = written {
            eprintln!("could not write the new zones' progress: {e}");
        }
    }
}

/// No progress kept (checks must not touch the player's files).
pub struct NoZoneProgress;

impl ZoneProgressStore for NoZoneProgress {
    fn load(&self) -> Option<ZoneProgress> {
        None
    }
    fn save(&self, _: &ZoneProgress) {}
}
