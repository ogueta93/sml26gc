//! The screens before the editor: the base cartridge (`host::base_cartridge`), asked for until
//! a valid one is chosen, and the projects (`editor\projects\<name>\`): open, new, rename,
//! duplicate, delete, import. The editor comes back here from File, Projects.
use super::level_panel::fit;
use super::widgets::*;
use crate::adapters::fs_repository::{self, ProjectError};
use crate::adapters::platform::CAN_QUIT;
use crate::application::save;
use crate::domain::files;
use crate::ports::{FileDialogs, FileKind};
use host::BaseProblem;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, is_key_pressed, get_char_pressed};
use std::path::PathBuf;

const W: f32 = 640.0;
const ROW: f32 = 34.0;

/// The panel the screens draw in: centred, `h` high; its left, top and width.
fn panel(h: f32) -> Rect {
    clear_background(col(3));
    let r = Rect::new(((screen_width() - W) / 2.0).max(16.0), ((screen_height() - h) / 2.0).max(16.0), W.min(screen_width() - 32.0), h);
    draw_rectangle(r.x, r.y, r.w, r.h, col(0));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, col(3));
    r
}

fn lines(text: &str, x: f32, mut y: f32, w: f32, size: f32, c: Color) -> f32 {
    for l in text.lines().flat_map(|l| wrap_px(l, w, size)) {
        put_text(&l, x, y, size, c);
        y += size + 4.0;
    }
    y
}

/// Asking for the base cartridge.
pub struct BaseScreen {
    lang: Lang,
    problem: BaseProblem,
    /// The data folder cannot be written (the choice would not be kept).
    unwritable: bool,
    /// Choosing the file found no dialog to open.
    no_dialog: bool,
}

impl BaseScreen {
    pub fn new(lang: Lang, problem: BaseProblem) -> BaseScreen {
        BaseScreen { lang, problem, unwritable: host::paths::data_dir_problem(lang).is_some(), no_dialog: false }
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    /// One frame; the cartridge once a valid one is chosen.
    pub fn frame(&mut self, dialogs: &dyn FileDialogs) -> Option<PathBuf> {
        let lang = self.lang;
        let r = panel(240.0);
        let x = r.x + 24.0;
        put_text(tr(lang, "BASE CARTRIDGE"), x, r.y + 40.0, 24.0, col(3));
        let c = if self.problem == BaseProblem::NotChosen { col(3) } else { Severity::Error.colour() };
        let after = lines(&self.problem.text(lang), x, r.y + 76.0, r.w - 48.0, 17.0, c);
        let problems = [self.unwritable.then(|| host::paths::data_dir_problem(lang)).flatten(), self.no_dialog.then(|| host::file_dialog::unavailable_text(lang))];
        problems.into_iter().flatten().fold(after, |y, problem| lines(problem, x, y + 8.0, r.w - 48.0, 15.0, Severity::Error.colour()));
        let note = tr(lang, "The editor and the game share it; the file may have any name.");
        lines(note, x, r.y + r.h - 76.0, r.w - 48.0, 15.0, INFO);
        let chosen = button(Rect::new(x, r.y + r.h - 50.0, r.w - 48.0, ROW), tr(lang, "Choose the cartridge file..."), true, 17.0);
        if chosen || dialogs.arrived() || dialogs.dropped() == Some(FileKind::Cartridge) {
            let Some(p) = dialogs.pick_cartridge() else {
                self.no_dialog = !dialogs.available();
                return None;
            };
            match host::base_cartridge::choose(&p) {
                Ok(()) => return Some(p),
                Err(e) => self.problem = e,
            }
        }
        None
    }
}

/// What the project screen ends with.
pub enum ProjectsChoice {
    Open(String),
    Quit,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Typing {
    New,
    Rename,
    Duplicate,
}

/// The project list and its actions.
pub struct ProjectsScreen {
    lang: Lang,
    base: PathBuf,
    names: Vec<String>,
    selected: usize,
    typing: Option<(Typing, String)>,
    deleting: bool,
    message: Option<(String, Severity)>,
    scroll: usize,
}

impl ProjectsScreen {
    pub fn new(lang: Lang, base: PathBuf, chosen: Option<&str>) -> ProjectsScreen {
        let message = host::paths::data_dir_problem(lang).map(|p| (p.to_string(), Severity::Error));
        let mut s = ProjectsScreen { lang, base, names: Vec::new(), selected: 0, typing: None, deleting: false, message, scroll: 0 };
        s.reload(chosen);
        s
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    /// Start typing a new project's name (the game's "new project in the editor").
    pub fn start_new(&mut self) {
        self.typing = Some((Typing::New, String::new()));
    }

    fn reload(&mut self, chosen: Option<&str>) {
        self.names = host::projects();
        self.selected = chosen.and_then(|c| self.names.iter().position(|n| n == c)).unwrap_or(0);
    }

    fn chosen(&self) -> Option<&String> {
        self.names.get(self.selected)
    }

    fn error_text(&self, e: &ProjectError) -> String {
        let lang = self.lang;
        match e {
            ProjectError::BadName => format!("{} {} {}", tr(lang, "A project name has up to"), host::paths::PROJECT_NAME_MAX, tr(lang, "characters (no accents, nor \\ / : * ? \" < > |), does not end in a dot and is not VANILLA.")),
            ProjectError::Taken => tr(lang, "Another project has that name.").into(),
            ProjectError::Missing => tr(lang, "That project is not there any more.").into(),
            ProjectError::Io(m) => format!("{} {m}", tr(lang, "The files could not be written:")),
        }
    }

    fn done(&mut self, result: Result<(), ProjectError>, name: &str, ok: String) {
        match result {
            Ok(()) => {
                self.reload(Some(name));
                self.message = Some((ok, Severity::Done));
            }
            Err(e) => self.message = Some((self.error_text(&e), Severity::Error)),
        }
    }

    /// A project file from any folder as a new project (named after the file).
    fn import(&mut self, dialogs: &dyn FileDialogs) {
        let lang = self.lang;
        let Some(path) = dialogs.open(FileKind::Project) else {
            if !dialogs.available() {
                self.message = Some((host::file_dialog::unavailable_text(lang).into(), Severity::Error));
            }
            return;
        };
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let file = host::storage::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| files::read_project(&t).map_err(|e| format!("{e:?}")));
        let p = match file {
            Ok(p) => p,
            Err(e) => {
                self.message = Some((format!("{} {e}", tr(lang, "Not a usable project file:")), Severity::Error));
                return;
            }
        };
        let name = fs_repository::free_name(&stem);
        let made = fs_repository::create_project(&self.base, &name).and_then(|_| fs_repository::open_project(&self.base, &name)).and_then(|mut o| {
            o.doc.replace_project(p).map_err(|e| ProjectError::Io(format!("{e:?}")))?;
            save::save(&mut o.doc, &o.store).map(|_| ()).map_err(|e| match e {
                save::SaveError::Layout(e) => ProjectError::Io(e.to_string()),
                save::SaveError::Write(e) => ProjectError::Io(e.to_string()),
            })
        });
        if made.is_err() {
            let _ = fs_repository::delete_project(&name);
        }
        let ok = format!("{} {name}", tr(lang, "Imported as the project"));
        self.done(made, &name, ok);
    }

    /// One frame; what to do once the person has chosen.
    pub fn frame(&mut self, dialogs: &dyn FileDialogs) -> Option<ProjectsChoice> {
        new_frame_texts();
        let choice = self.list_frame(dialogs);
        if let Some(t) = take_cut_tip() {
            draw_tooltip(&t);
        }
        choice
    }

    fn list_frame(&mut self, dialogs: &dyn FileDialogs) -> Option<ProjectsChoice> {
        let lang = self.lang;
        if dialogs.arrived() || dialogs.dropped() == Some(FileKind::Project) {
            self.import(dialogs);
        }
        let visible = 9;
        let r = panel(130.0 + visible as f32 * (ROW + 4.0) + 180.0);
        let (x, w) = (r.x + 24.0, r.w - 48.0);
        put_text(tr(lang, "PROJECTS"), x, r.y + 40.0, 24.0, col(3));
        let base = self.base.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        put_text(&fit(&format!("{} {base}", tr(lang, "Base cartridge:")), w, 15.0), x, r.y + 64.0, 15.0, INFO);
        let mut y = r.y + 82.0;
        // the list
        if self.names.is_empty() {
            lines(tr(lang, "No projects yet: make a new one, or import a project file."), x, y + 20.0, w, 17.0, col(3));
        }
        if self.typing.is_none() && !self.deleting && !self.names.is_empty() {
            if is_key_pressed(KeyCode::Down) {
                self.selected = (self.selected + 1).min(self.names.len() - 1);
            }
            if is_key_pressed(KeyCode::Up) {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        self.scroll = self.scroll.min(self.selected).max((self.selected + 1).saturating_sub(visible));
        let mut open = None;
        for (i, name) in self.names.iter().enumerate().skip(self.scroll).take(visible) {
            let row = Rect::new(x, y, w, ROW);
            let rom = host::project_rom_in(name).and_then(host::storage::modified);
            if button(row, "", i == self.selected, 16.0) {
                if i == self.selected && self.typing.is_none() {
                    open = Some(name.clone());
                }
                self.selected = i;
            }
            let (fg, dim) = if i == self.selected { (col(0), col(1)) } else { (col(3), col(2)) };
            put_text(&fit(name, w * 0.6, 18.0), row.x + 12.0, row.y + 23.0, 18.0, fg);
            if let Some(t) = rom {
                let when = saved_when(lang, t);
                let tw = text_size(&when, 14).width;
                put_text(&when, row.x + row.w - tw - 12.0, row.y + 22.0, 14.0, dim);
            }
            y += ROW + 4.0;
        }
        let y0 = r.y + 82.0 + visible as f32 * (ROW + 4.0) + 8.0;
        // a name being typed, or the delete question
        if let Some((what, text)) = &mut self.typing {
            let label = match what {
                Typing::New => tr(lang, "New project's name:"),
                Typing::Rename => tr(lang, "New name:"),
                Typing::Duplicate => tr(lang, "Name of the copy:"),
            };
            put_text(label, x, y0 + 16.0, 17.0, col(3));
            while let Some(c) = get_char_pressed() {
                if c.is_ascii() && !c.is_ascii_control() && text.chars().count() < host::paths::PROJECT_NAME_MAX {
                    text.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                text.pop();
            }
            let field = Rect::new(x, y0 + 26.0, w, ROW);
            draw_rectangle(field.x, field.y, field.w, field.h, col(3));
            let caret = if (get_time() * 2.0) as i64 % 2 == 0 { "_" } else { "" };
            put_text(&format!("{text}{caret}"), field.x + 8.0, field.y + 23.0, 18.0, col(0));
            let (what, text) = (*what, text.trim().to_string());
            let bw = (w - 12.0) / 2.0;
            if button(Rect::new(x, y0 + 70.0, bw, ROW), tr(lang, "OK (Enter)"), true, 16.0) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
                self.typing = None;
                let old = self.chosen().cloned().unwrap_or_default();
                let (result, ok) = match what {
                    Typing::New => (fs_repository::create_project(&self.base, &text), format!("{} {text}", tr(lang, "New project"))),
                    Typing::Rename => (fs_repository::rename_project(&self.base, &old, &text), format!("{old} {} {text}", tr(lang, "is now"))),
                    Typing::Duplicate => (fs_repository::duplicate_project(&self.base, &old, &text), format!("{old} {} {text}", tr(lang, "copied as"))),
                };
                self.done(result, &text, ok);
            } else if button(Rect::new(x + bw + 12.0, y0 + 70.0, bw, ROW), tr(lang, "Cancel (Esc)"), false, 16.0) || is_key_pressed(KeyCode::Escape) {
                self.typing = None;
            }
        } else if self.deleting {
            let name = self.chosen().cloned().unwrap_or_default();
            lines(&format!("{} {name}? {}", tr(lang, "Delete"), tr(lang, "Its cartridge and its saves are deleted for good.")), x, y0 + 16.0, w, 17.0, Severity::Warning.colour());
            let bw = (w - 12.0) / 2.0;
            if button(Rect::new(x, y0 + 70.0, bw, ROW), tr(lang, "Delete"), true, 16.0) {
                self.deleting = false;
                let ok = format!("{} {name}", tr(lang, "Deleted"));
                let result = fs_repository::delete_project(&name);
                self.done(result, "", ok);
            } else if button(Rect::new(x + bw + 12.0, y0 + 70.0, bw, ROW), tr(lang, "Keep it (Esc)"), false, 16.0) || is_key_pressed(KeyCode::Escape) {
                self.deleting = false;
            }
        } else {
            // the actions: open first, then what makes, changes or removes a project
            let bw = (w - 3.0 * 8.0) / 4.0;
            let at = |i: usize, row: usize| Rect::new(x + i as f32 * (bw + 8.0), y0 + 8.0 + row as f32 * (ROW + 8.0), bw, ROW);
            let some = self.chosen().is_some();
            if (button(at(0, 0), tr(lang, "Open (Enter)"), some, 16.0) || is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter)) && some {
                open = self.chosen().cloned();
            }
            if button(at(1, 0), tr(lang, "New"), false, 16.0) {
                while get_char_pressed().is_some() {}
                self.typing = Some((Typing::New, String::new()));
            }
            if button(at(2, 0), tr(lang, "Import..."), false, 16.0) {
                self.import(dialogs);
            }
            if CAN_QUIT && button(at(3, 0), tr(lang, "Quit"), false, 16.0) {
                return Some(ProjectsChoice::Quit);
            }
            if some {
                let name = self.chosen().cloned().unwrap_or_default();
                if button(at(0, 1), tr(lang, "Rename"), false, 16.0) {
                    while get_char_pressed().is_some() {}
                    self.typing = Some((Typing::Rename, name.clone()));
                }
                if button(at(1, 1), tr(lang, "Duplicate"), false, 16.0) {
                    while get_char_pressed().is_some() {}
                    self.typing = Some((Typing::Duplicate, fs_repository::free_name(&format!("{name} copy"))));
                }
                if button(at(2, 1), tr(lang, "Delete"), false, 16.0) {
                    self.deleting = true;
                }
            }
        }
        if let Some((m, kind)) = &self.message {
            message_lines(m, x, r.y + r.h - 20.0, w, 15.0, *kind);
        }
        open.map(ProjectsChoice::Open)
    }
}

/// When a project was last saved, as the screen says it.
fn saved_when(lang: Lang, t: std::time::SystemTime) -> String {
    let ago = host::clock::now().duration_since(t).map(|d| d.as_secs()).unwrap_or(0);
    let (n, unit) = match ago {
        0..=59 => return tr(lang, "saved just now").into(),
        60..=3599 => (ago / 60, 0),
        3600..=86399 => (ago / 3600, 1),
        _ => (ago / 86400, 2),
    };
    let pattern = match unit {
        0 => tr(lang, "saved %n min ago"),
        1 => tr(lang, "saved %n h ago"),
        _ => tr(lang, "saved %n days ago"),
    };
    pattern.replace("%n", &n.to_string())
}
