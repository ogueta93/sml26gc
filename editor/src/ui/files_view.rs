//! Level, zone and project files from the panels: the file actions, run once the keys are let go,
//! and what the status bar says about each.
use super::shortcuts::Action;
use super::widgets::tr;
use super::widgets::Severity;
use super::Editor;
use crate::application::exchange::{self, ExchangeError};
use crate::domain::document::ZoneRefusal;
use crate::domain::files::{FileError, Origin, OriginOf};
use cart_model::levels::LEVELS;
use cart_model::original_points::OnMap;
use macroquad::prelude::*;
use super::inset::is_mouse_button_down;

impl Editor {
    /// A dialog takes the keyboard while it is open, so the editor would never see Ctrl (Cmd) or
    /// Shift go up: the action waits until nothing is held.
    pub(super) fn queue_file(&mut self, a: Action) {
        if a == Action::ReplaceLevel && self.cur < LEVELS {
            self.status = tr(self.lang, "Only new levels (32 and up) can be replaced from a file; the game's 32 are only edited.").into();
            self.mark(Severity::Warning);
            return;
        }
        if a == Action::ReplaceZone && !matches!(self.world_map, OnMap::NewZone(_) | OnMap::World) {
            self.status = tr(self.lang, "Only new zones and the world map can be replaced from a file; the game's zones are only edited.").into();
            self.mark(Severity::Warning);
            return;
        }
        if a == Action::ExportZone && !self.zone_exportable() {
            self.status = tr(self.lang, "Show the world map, a new zone, or the Tree, Pumpkin, Macro, Space or Mario zone, to export it.").into();
            self.mark(Severity::Done);
            self.mark(Severity::Warning);
            return;
        }
        self.pending_file = Some(a);
    }

    /// Can the map shown be exported: the world map (a world map file), a new zone or a zone a
    /// door enters (a zone file)?
    pub(super) fn zone_exportable(&self) -> bool {
        match self.world_map {
            OnMap::Zone(z) => cart_model::new_zones::base_of(z).is_some(),
            OnMap::NewZone(_) | OnMap::World => true,
        }
    }

    pub(super) fn run_pending_file(&mut self) {
        let held = super::shortcuts::MODIFIERS.iter().any(|&k| is_key_down(k))
            || [MouseButton::Left, MouseButton::Right, MouseButton::Middle].iter().any(|&b| is_mouse_button_down(b));
        if held {
            return;
        }
        if let Some(a) = self.pending_file.take() {
            self.run_file(a);
        }
    }

    /// Where a file was exported from, as the status bar says it (`Origin::text`; "a file"
    /// when unknown).
    pub(super) fn origin_text(&self, origin: &str) -> String {
        let lang = self.lang;
        let Some(o) = Origin::parse(origin) else { return tr(lang, "a file").to_string() };
        let named = |n: &str| if n.is_empty() { String::new() } else { format!(" {n}") };
        let what = match &o.of {
            OriginOf::Level(l, n) => format!("{} {l:02}{}", tr(lang, "level"), named(n)),
            OriginOf::Zone(z) if *z < cart_model::zone_maps::ZONES.len() => crate::texts::zone_name(lang, *z).to_string(),
            OriginOf::Zone(_) => tr(lang, "a file").to_string(),
            OriginOf::NewZone(k, n) => format!("{} {}{}", tr(lang, "New zone"), k + 1, named(n)),
        };
        format!("{what} {} {}", tr(lang, "of project"), o.project)
    }

    /// The project's name, to suggest file names.
    fn cartridge_name(&self) -> String {
        self.project.clone()
    }

    /// The file name offered for an export: the name the level or zone was given, else
    /// `fallback` (characters a file name cannot have left out).
    fn export_name(named: &str, fallback: String) -> String {
        let clean: String = named.chars().filter(|c| !r#"<>:"/\|?*"#.contains(*c) && !c.is_control()).collect();
        let clean = clean.trim().trim_end_matches('.').trim();
        if clean.is_empty() {
            fallback
        } else {
            clean.to_string()
        }
    }

    fn run_file(&mut self, a: Action) {
        let lang = self.lang;
        let (dialogs, files) = (&*self.dialogs, &*self.text_files);
        let l = self.cur;
        let status = match a {
            Action::ExportLevel => exchange::export_level(&self.doc, l, &self.project, &Self::export_name(self.doc.level_name(l), format!("{} {} {l:02}", self.cartridge_name(), tr(lang, "level"))), dialogs, files)
                .map(|p| p.map(|p| format!("{} {l:02} {} {}", tr(lang, "Level"), tr(lang, "exported to"), p.display()))),
            Action::ImportLevel => match exchange::import_level(&mut self.doc, dialogs, files) {
                Ok(Some(n)) => {
                    self.select_level(n);
                    Ok(Some(format!("{} {n:02} {} {:02}. {}", tr(lang, "New level"), tr(lang, "from the file; plays as level"), self.doc.base_of(n), tr(lang, "Save to keep it."))))
                }
                other => other.map(|_| None),
            },
            Action::ReplaceLevel => {
                let base = self.doc.base_of(l);
                match exchange::replace_level(&mut self.doc, l, dialogs, files) {
                    Ok(Some(())) => {
                        self.level_changed(base);
                        Ok(Some(format!("{} {l:02} {} {:02}. {}", tr(lang, "Level"), tr(lang, "replaced from the file; plays as level"), self.doc.base_of(l), tr(lang, "Undo brings it back."))))
                    }
                    other => other.map(|_| None),
                }
            }
            Action::ExportZone if self.world_map == OnMap::World => exchange::export_world(&self.doc, &format!("{} {}", self.project, tr(lang, "world map")), dialogs, files).map(|p| p.map(|p| format!("{} {}", tr(lang, "World map exported to"), p.display()))),
            Action::ExportZone => {
                let named = match self.world_map {
                    OnMap::NewZone(k) => self.doc.new_zones.get(k).map_or("", |z| z.name.as_str()),
                    _ => "",
                };
                let name = Self::export_name(named, format!("{} {}", self.cartridge_name(), self.map_name(self.world_map)));
                exchange::export_zone(&self.doc, self.world_map, &self.project, &name, dialogs, files).map(|p| p.map(|p| format!("{} {}", tr(lang, "Zone exported to"), p.display())))
            }
            Action::ImportZone => match exchange::import_zone(&mut self.doc, dialogs, files) {
                Ok(Some((k, start))) => {
                    self.clipboard = None;
                    self.show_map(OnMap::NewZone(k));
                    self.point = self.doc.new_zones[k].entry as usize;
                    self.offer_start(OnMap::NewZone(k), start);
                    Ok(Some(format!("{} {}. {}", self.map_name(OnMap::NewZone(k)), tr(lang, "from the file; it has no door yet"), tr(lang, "Save to keep the zone."))))
                }
                other => other.map(|_| None),
            },
            Action::ReplaceZone if self.world_map != OnMap::World => match exchange::replace_zone(&mut self.doc, self.world_map, dialogs, files) {
                Ok(Some(start)) => {
                    self.clipboard = None;
                    if let OnMap::NewZone(k) = self.world_map {
                        self.point = self.doc.new_zones[k].entry as usize;
                    }
                    self.offer_start(self.world_map, start);
                    Ok(Some(format!("{} {}", self.map_name(self.world_map), tr(lang, "replaced from the file; it keeps its door. Save to keep it."))))
                }
                other => other.map(|_| None),
            },
            Action::ExportProject => exchange::export_project(&self.doc, &self.project, dialogs, files).map(|p| p.map(|p| format!("{} {}", tr(lang, "Project exported to"), p.display()))),
            Action::ReplaceZone if self.world_map == OnMap::World => match exchange::import_world(&mut self.doc, dialogs, files) {
                Ok(Some((points, exits, start))) => {
                    self.clipboard = None;
                    self.overview = None;
                    self.show_map(OnMap::World);
                    self.offer_start(OnMap::World, start);
                    let lost = if points + exits == 0 {
                        String::new()
                    } else {
                        format!(" {points} {} {exits} {}", tr(lang, "points opened new levels this project does not have, and"), tr(lang, "exits led to missing new zones: they lead nowhere now."))
                    };
                    Ok(Some(format!("{}{lost}", tr(lang, "World map imported. Save to keep it."))))
                }
                other => other.map(|_| None),
            },
            _ => Ok(None),
        };
        match status {
            Ok(Some(s)) => self.status = s,
            Ok(None) => {}
            Err(e) => {
                self.status = self.file_error(&e);
                self.mark(Severity::Error);
            }
        }
    }

    fn file_error(&self, e: &ExchangeError) -> String {
        let lang = self.lang;
        match e {
            ExchangeError::Io(e) => format!("{}: {e}", tr(lang, "The file could not be read or written")),
            ExchangeError::NoRoom => tr(lang, "There can't be more levels.").into(),
            ExchangeError::NoDialog => host::file_dialog::unavailable_text(lang).into(),
            ExchangeError::Zone(ZoneRefusal::TooManyZones(max)) => format!("{} {max}.", tr(lang, "No more new zones: there can be")),
            ExchangeError::Zone(_) => tr(lang, "New zones are made from the Tree, Pumpkin, Macro, Space or Mario zone.").into(),
            ExchangeError::WorldAsZone => tr(lang, "This is a world map file: the game has one world map, so it replaces it (Replace zone on the world map) and is not a new zone.").into(),
            ExchangeError::ProtectedZone => tr(lang, "Only new zones can be replaced from a file.").into(),
            ExchangeError::Protected => tr(lang, "Only new levels (32 and up) can be replaced from a file.").into(),
            ExchangeError::File(f) => match f {
                FileError::NotLevel => tr(lang, "This is not a level file.").into(),
                FileError::NotZone => tr(lang, "This is not a zone file.").into(),
                FileError::NotWorld => tr(lang, "This is not a world map file.").into(),
                FileError::NotABase(z) => format!("{} {z}.", tr(lang, "The file's zone is made from a zone no door enters:")),
                FileError::NotProject => tr(lang, "This is not a project file.").into(),
                FileError::Newer => tr(lang, "This file was made by a newer editor.").into(),
                FileError::Damaged(line) => format!("{} {line}).", tr(lang, "The file is damaged or incomplete (line")),
                FileError::BaseOutside(b) => format!("{} {b}.", tr(lang, "The file's level plays as a level that isn't one of the 32:")),
                FileError::OtherCartridge => tr(lang, "This project belongs to another cartridge; nothing was changed.").into(),
            },
        }
    }
}
