//! The window's File and Tools menus: their entries come from the shortcut table and run as
//! actions; entries that can't run where the editor is are greyed out.
use super::shortcuts::{keys, Action};
use super::language_menu;
use super::widgets::{set_text_language, tr, Lang, Severity};
use super::Editor;
use crate::ports::MenuEntry;
use cart_model::levels::LEVELS;
use cart_model::original_points::OnMap;

/// The File menu, top to bottom (None: a separator). An entry's id is its place here + 1.
const FILE_MENU: [Option<Action>; 14] = [
    Some(Action::Save),
    None,
    Some(Action::Projects),
    Some(Action::ExportProject),
    Some(Action::ExportCartridge),
    None,
    Some(Action::NewBlankLevel),
    Some(Action::ExportLevel),
    Some(Action::ImportLevel),
    Some(Action::ReplaceLevel),
    None,
    Some(Action::ExportZone),
    Some(Action::ImportZone),
    Some(Action::ReplaceZone),
];

/// The Tools menu; its entries' ids start at `TOOLS_ID`.
const TOOLS_MENU: [Option<Action>; 3] = [Some(Action::ObjectCatalogue), None, Some(Action::RecaptureObjects)];
/// How many entries the Tools menu has, separators included.
pub(super) const TOOLS_MENU_LEN: usize = TOOLS_MENU.len();
const TOOLS_ID: u16 = 101;

fn label(lang: Lang, a: Action) -> &'static str {
    match a {
        Action::ObjectCatalogue => tr(lang, "Object catalogue"),
        Action::Projects => tr(lang, "Projects..."),
        Action::RecaptureObjects => tr(lang, "Capture the object pictures again"),
        Action::Save => tr(lang, "Save"),
        Action::NewBlankLevel => tr(lang, "New blank level..."),
        Action::ExportLevel => tr(lang, "Export level..."),
        Action::ImportLevel => tr(lang, "Import level as a new level..."),
        Action::ReplaceLevel => tr(lang, "Replace this level from a file..."),
        Action::ExportZone => tr(lang, "Export zone..."),
        Action::ImportZone => tr(lang, "Import zone as a new zone..."),
        Action::ReplaceZone => tr(lang, "Replace this zone from a file..."),
        Action::ExportProject => tr(lang, "Export project..."),
        Action::ExportCartridge => tr(lang, "Export cartridge..."),
        _ => "",
    }
}

impl Editor {
    pub(super) fn build_menu(&mut self) {
        let entries: Vec<MenuEntry> = FILE_MENU
            .iter()
            .enumerate()
            .map(|(i, a)| match a {
                Some(a) => MenuEntry::Item { id: i as u16 + 1, label: label(self.lang, *a), keys: keys(*a) },
                None => MenuEntry::Separator,
            })
            .collect();
        self.menu.add_menu(tr(self.lang, "&File"), &entries);
        let tools: Vec<MenuEntry> = TOOLS_MENU
            .iter()
            .enumerate()
            .map(|(i, a)| match a {
                Some(a) => MenuEntry::Item { id: TOOLS_ID + i as u16, label: label(self.lang, *a), keys: keys(*a) },
                None => MenuEntry::Separator,
            })
            .collect();
        self.menu.add_menu(tr(self.lang, "&Tools"), &tools);
        language_menu::add(&mut *self.menu, self.lang, self.prefs.language);
    }

    /// Change the editor's language (None: the system's), kept in `editor.cfg`: every text,
    /// the menu bar and the file dialogs at once.
    fn set_language(&mut self, choice: Option<Lang>) {
        self.prefs.language = choice;
        self.prefs_store.save(&self.prefs);
        self.lang = language_menu::resolve(choice);
        set_text_language(self.lang);
        self.dialogs.set_lang(self.lang);
        self.menu.clear();
        self.menu_enabled = [None; FILE_MENU_LEN];
        self.tools_enabled = [None; TOOLS_MENU_LEN];
        self.build_menu();
        self.status = format!("{}: {}", tr(self.lang, "Language"), self.lang.name());
        self.mark(Severity::Done);
    }

    /// Run the entries chosen since the last frame and grey out those that can't run now.
    pub(super) fn update_menu(&mut self) {
        for id in self.menu.chosen() {
            if let Some(choice) = language_menu::chosen(id) {
                self.set_language(choice);
                continue;
            }
            let a = if id >= TOOLS_ID { TOOLS_MENU.get((id - TOOLS_ID) as usize) } else { FILE_MENU.get((id as usize).wrapping_sub(1)) };
            if let Some(Some(a)) = a {
                self.run(*a);
            }
        }
        for (i, a) in FILE_MENU.iter().enumerate() {
            let Some(a) = *a else { continue };
            let on = match a {
                Action::ExportLevel | Action::ImportLevel | Action::NewBlankLevel => !self.in_world,
                Action::ReplaceLevel => !self.in_world && self.cur >= LEVELS,
                Action::ExportZone => self.in_world && self.zone_exportable(),
                Action::ImportZone => self.in_world,
                Action::ReplaceZone => self.in_world && matches!(self.world_map, OnMap::NewZone(_) | OnMap::World),
                _ => true,
            };
            if self.menu_enabled[i] != Some(on) {
                self.menu_enabled[i] = Some(on);
                self.menu.enable(i as u16 + 1, on);
            }
        }
        for (i, a) in TOOLS_MENU.iter().enumerate() {
            let Some(a) = *a else { continue };
            let on = a != Action::ObjectCatalogue || !self.in_world;
            if self.tools_enabled[i] != Some(on) {
                self.tools_enabled[i] = Some(on);
                self.menu.enable(TOOLS_ID + i as u16, on);
            }
        }
    }
}

/// How many entries the File menu has, separators included.
pub(super) const FILE_MENU_LEN: usize = FILE_MENU.len();
