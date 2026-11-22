//! The level editor's side panel. A fixed head: the level picker (its grid folds away), a
//! toolbar (undo, redo, restore; save, play, play here), the Blocks / Objects switch with its tools,
//! and two tabs. Below them the chosen tab fills the rest: the palette, or the level's
//! inspector (level, goals, settings, warnings), which scrolls with the wheel when it does not
//! fit. Buttons carry short names; their keys show in a tooltip.
use super::names_view::Named;
use super::shortcuts::{keys, Action};
use super::start_view::Placing;
use super::testing_view::warn_col;
use super::widgets::*;
use super::{Editor, Mode};
use crate::domain::document::CheckpointRefused;
use crate::domain::prefs::MUSIC_VOLUMES;
use crate::domain::tools::ToolKind;
use cart_model::checkpoint;
use cart_model::level_goals::Goal;
use cart_model::level_settings::{MUSICS, PALETTES, TIME_MAX, TIME_MIN};
use cart_model::levels::LEVELS;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_pressed, mouse_wheel};

/// What the side panel shows below its head.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum SideTab {
    Palette,
    Level,
}

pub(super) const PAD: f32 = 10.0;
pub(super) const ROW: f32 = 26.0;
pub(super) const GAP: f32 = 4.0;
/// The inspector's label column.
pub(super) const LABEL_W: f32 = 96.0;
/// The button beside Play that opens the test game's settings.
const SPLIT_W: f32 = 22.0;
const LEVEL_BUTTON: (f32, f32) = (33.0, 23.0);
/// Seconds a warning's place stays marked after it is clicked.
const MARK_SECONDS: f64 = 1.5;
pub(super) const SCROLL_STEP: f32 = 40.0;

/// Rows laid out down the inspector, drawn only while inside it.
struct Rows {
    y: f32,
    top: f32,
    bottom: f32,
}

impl Rows {
    fn next(&mut self, h: f32) -> Option<f32> {
        let y = self.y;
        self.y += h + GAP;
        (y >= self.top - 0.5 && y + h <= self.bottom).then_some(y)
    }
}

pub(super) use super::widgets::fit;

impl Editor {
    pub(super) fn inner_w(&self) -> f32 {
        self.panel - 2.0 * PAD - 3.0
    }

    pub(super) fn tip_for(&mut self, r: Rect, name: &str, a: Action) {
        if r.contains(mouse_position().into()) {
            let k = keys(a);
            self.tip = Some(if k.is_empty() { name.to_string() } else { format!("{name}  ({k})") });
        }
    }

    /// A button that starts the test game: faded while one is open (one at a time), its
    /// tooltip saying so.
    pub(super) fn play_button(&mut self, r: Rect, name: &str, a: Action, on: bool) -> bool {
        if !self.launcher.running() {
            return self.tool_button(r, name, a, on);
        }
        if r.contains(mouse_position().into()) {
            self.tip = Some(tr(self.lang, "The game is already open: close it to play again.").into());
        }
        unavailable_button(r, name, 15.0);
        false
    }

    /// A button whose name and keys show in a tooltip while the mouse is on it.
    pub(super) fn tool_button(&mut self, r: Rect, name: &str, a: Action, on: bool) -> bool {
        self.tip_button(r, name, name, a, on)
    }

    /// A button labelled `label` whose tooltip says `tip` and its keys.
    pub(super) fn tip_button(&mut self, r: Rect, label: &str, tip: &str, a: Action, on: bool) -> bool {
        self.tip_for(r, tip, a);
        button(r, label, on, 15.0)
    }

    /// A section's title with a rule after it, and `right` at the rule's end.
    pub(super) fn section_title(&self, y: f32, text: &str, right: &str) {
        let w = self.inner_w();
        put_text(text, PAD, y + 13.0, 14.0, col(2));
        let d = text_size(text, 14);
        let rw = if right.is_empty() { 0.0 } else { text_size(right, 14).width.ceil() + 8.0 };
        draw_rectangle(PAD + d.width.ceil() + 8.0, y + 8.0, w - d.width.ceil() - 8.0 - rw, 1.0, col(1));
        if !right.is_empty() {
            put_text(right, PAD + w - rw + 8.0, y + 13.0, 14.0, col(2));
        }
    }

    /// The three actions under a picker (copy, blank, delete), always shown in their own
    /// style, the unavailable ones faded; set apart by a rule from the picker's grid when it is
    /// open. Returns where the panel continues and the action clicked.
    pub(super) fn picker_actions(&mut self, y: f32, open: bool, items: [(&str, Action, bool); 3]) -> (f32, Option<Action>) {
        let w = self.inner_w();
        let third = (w - 2.0 * GAP) / 3.0;
        let mut y = y;
        if open {
            draw_rectangle(PAD, y + 4.0, w, 1.0, col(2));
            y += 10.0;
        }
        let mut act = None;
        for (i, (name, a, available)) in items.into_iter().enumerate() {
            let r = Rect::new(PAD + i as f32 * (third + GAP), y, third, ROW);
            self.tip_for(r, name, a);
            if action_button(r, name, 15.0, available) {
                act = Some(a);
            }
        }
        (y + ROW + GAP + 4.0, act)
    }

    /// The toolbar: history and Restore, then file and game (`second`). Returns where the
    /// panel continues and the action clicked.
    pub(super) fn toolbar(&mut self, y: f32, second: &[(&str, Action, bool)]) -> (f32, Option<Action>) {
        let lang = self.lang;
        let w = self.inner_w();
        let mut act = None;
        let history = [(tr(lang, "Undo"), Action::Undo, false), (tr(lang, "Redo"), Action::Redo, false), (tr(lang, "Restore"), Action::Restore, false)];
        let mut y = y;
        for row in [&history[..], second] {
            let n = row.len() as f32;
            let bw = (w - (n - 1.0) * GAP) / n;
            for (i, &(name, a, on)) in row.iter().enumerate() {
                let mut r = Rect::new(PAD + i as f32 * (bw + GAP), y, bw, ROW);
                if a == Action::Play {
                    r.w -= SPLIT_W - 2.0;
                    self.test_game_button(Rect::new(r.x + r.w - 2.0, y, SPLIT_W, ROW));
                }
                let clicked = if matches!(a, Action::Play | Action::PlayFromHere) { self.play_button(r, name, a, on) } else { self.tool_button(r, name, a, on) };
                if clicked {
                    act = Some(a);
                }
            }
            y += ROW + GAP;
        }
        (y + 6.0, act)
    }

    /// The tooltip of the button under the mouse, or the whole of a text cut short under it,
    /// over everything.
    pub(super) fn draw_tip(&mut self) {
        let Some(t) = self.tip.take().or_else(take_cut_tip) else { return };
        let m: Vec2 = mouse_position().into();
        let d = text_size(&t, 15);
        let (w, h) = (d.width + 12.0, 22.0);
        let x = (m.x + 14.0).min(screen_width() - w - 4.0);
        let y = (m.y + 18.0).min(screen_height() - h - 4.0);
        draw_rectangle(x, y, w, h, col(3));
        draw_rectangle_lines(x, y, w, h, 1.0, col(1));
        put_text(&t, x + 6.0, y + 15.0, 15.0, col(0));
    }

    /// Level buttons, toolbar, mode, tools, tabs and the chosen tab. Returns the action
    /// chosen (done after drawing) and the level picked.
    pub(super) fn level_panel(&mut self) -> (Option<Action>, Option<usize>) {
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let lang = self.lang;
        let w = self.inner_w();
        let mut act = None;
        let mut pick = None;
        put_text(tr(lang, "LEVEL EDITOR"), PAD, 25.0, 20.0, col(3));
        if self.tool_button(Rect::new(PAD + w - 104.0, 8.0, 104.0, 24.0), tr(lang, "Zones"), Action::WorldEditor, false) {
            act = Some(Action::WorldEditor);
        }
        let mut y = 40.0;
        // the level picker: < Level 07 · name v >
        if self.tool_button(Rect::new(PAD, y, 30.0, ROW), "<", Action::PrevLevel, false) {
            act = Some(Action::PrevLevel);
        }
        if self.tool_button(Rect::new(PAD + w - 30.0, y, 30.0, ROW), ">", Action::NextLevel, false) {
            act = Some(Action::NextLevel);
        }
        let lv = &self.doc.levels[self.cur];
        let mut label = format!("{} {:02}", tr(lang, "Level"), self.cur);
        if !lv.content.name.is_empty() {
            label += &format!("  {}", lv.content.name);
        }
        if lv.dirty() {
            label += " *";
        }
        if self.doc.mods.level(self.cur) {
            label += "  mod";
        }
        let mid = Rect::new(PAD + 34.0, y, w - 68.0, ROW);
        let shown = format!("{}  {}", fit(&label, mid.w - 30.0, 15.0), if self.levels_open { "^" } else { "v" });
        if button(mid, &shown, self.levels_open, 15.0) {
            self.levels_open = !self.levels_open;
        }
        y += ROW + GAP;
        if self.levels_open {
            let per_row = ((w + 2.0) / (LEVEL_BUTTON.0 + 2.0)).floor().max(1.0) as usize;
            let count = self.doc.levels.len();
            for l in 0..count {
                let (cx, cy) = (PAD + (l % per_row) as f32 * (LEVEL_BUTTON.0 + 2.0), y + (l / per_row) as f32 * (LEVEL_BUTTON.1 + 2.0));
                let label = format!("{:02}{}", l, if self.doc.levels[l].dirty() { "*" } else { "" });
                let r = Rect::new(cx, cy, LEVEL_BUTTON.0, LEVEL_BUTTON.1);
                if button(r, &label, l == self.cur, 15.0) {
                    pick = Some(l);
                }
                if l >= LEVELS {
                    user_tag(r, l == self.cur);
                    if r.contains(mouse_position().into()) {
                        let name = self.doc.level_name(l);
                        self.tip = Some(format!("{} {l:02}{}{name}  ({})", tr(lang, "Level"), if name.is_empty() { "" } else { "  " }, tr(lang, "made in the editor")));
                    }
                }
                if self.doc.mods.level(l) {
                    draw_triangle(vec2(cx + LEVEL_BUTTON.0, cy), vec2(cx + LEVEL_BUTTON.0 - 8.0, cy), vec2(cx + LEVEL_BUTTON.0, cy + 8.0), col(2));
                }
                let n = self.doc.warnings(l).len();
                if n > 0 {
                    draw_circle(cx + LEVEL_BUTTON.0 - 5.0, cy + LEVEL_BUTTON.1 - 5.0, 5.0, warn_col());
                    put_text(&n.to_string(), cx + LEVEL_BUTTON.0 - 8.0, cy + LEVEL_BUTTON.1 - 1.0, 12.0, BLACK);
                }
            }
            y += count.div_ceil(per_row) as f32 * (LEVEL_BUTTON.1 + 2.0) + 2.0;
            if count > LEVELS {
                y += user_tag_legend(PAD, y + 2.0, tr(lang, "made in the editor"));
            }
        }
        let items = [(tr(lang, "Copy level"), Action::NewLevel, true), (tr(lang, "Blank level"), Action::NewBlankLevel, true), (tr(lang, "Delete level"), Action::DeleteLevel, self.cur >= LEVELS)];
        let (next, a) = self.picker_actions(y, self.levels_open, items);
        y = next;
        act = a.or(act);
        let here = self.placing == Some(Placing::PlayHere);
        let (next, a) = self.toolbar(y, &[(tr(lang, "Save"), Action::Save, false), (tr(lang, "Play"), Action::Play, false), (tr(lang, "Play here"), Action::PlayFromHere, here)]);
        y = next;
        act = a.or(act);
        // Blocks / Objects, then the mode's tools
        let half = (w - GAP) / 2.0;
        let blocks = self.mode == Mode::Blocks;
        if self.tool_button(Rect::new(PAD, y, half, ROW), tr(lang, "Blocks"), Action::BlocksObjects, blocks) {
            self.set_mode(Mode::Blocks);
        }
        if self.tool_button(Rect::new(PAD + half + GAP, y, half, ROW), tr(lang, "Objects"), Action::BlocksObjects, !blocks) {
            self.set_mode(Mode::Objects);
        }
        y += ROW + GAP;
        let qw = (w - 3.0 * GAP) / 4.0;
        if blocks {
            let tool = self.tool.kind();
            for (i, (name, a, k)) in [(tr(lang, "Pencil"), Action::Pencil, ToolKind::Pencil), (tr(lang, "Box"), Action::BoxFill, ToolKind::Box), (tr(lang, "Select"), Action::Select, ToolKind::Select)].into_iter().enumerate() {
                if self.tool_button(Rect::new(PAD + i as f32 * (qw + GAP), y, qw, ROW), name, a, tool == k) {
                    act = Some(a);
                }
            }
        } else {
            let n = self.doc.levels[self.cur].content.objs.len();
            put_text(&format!("{n} {}", tr(lang, "objects")), PAD + 2.0, y + 18.0, 16.0, col(3));
        }
        if self.tool_button(Rect::new(PAD + 3.0 * (qw + GAP), y, qw, ROW), tr(lang, "Grid"), Action::Grid, self.grid) {
            act = Some(Action::Grid);
        }
        y += ROW + GAP + 6.0;
        // tabs
        let warnings = self.doc.warnings(self.cur).len();
        let level_tab = if warnings > 0 { format!("{}  ! {warnings}", tr(lang, "Level")) } else { tr(lang, "Level").to_string() };
        for (i, (name, tab)) in [(tr(lang, "Palette").to_string(), SideTab::Palette), (level_tab, SideTab::Level)].into_iter().enumerate() {
            let r = Rect::new(PAD + i as f32 * (half + GAP), y, half, ROW);
            if button(r, &name, self.side_tab == tab, 15.0) {
                self.side_tab = tab;
            }
            if tab == SideTab::Level && warnings > 0 && self.side_tab != tab {
                draw_circle(r.x + r.w - 8.0, r.y + 8.0, 4.0, warn_col());
            }
        }
        y += ROW;
        draw_rectangle(PAD, y, w, 2.0, col(3));
        y += 8.0;
        match self.side_tab {
            SideTab::Palette if self.mode == Mode::Objects => self.object_palette(y),
            SideTab::Palette => self.block_palette(y),
            SideTab::Level => {
                if let Some(a) = self.inspector(y) {
                    act = Some(a);
                }
            }
        }
        (act, pick)
    }

    fn heading(&self, rows: &mut Rows, text: &str) {
        let Some(y) = rows.next(18.0) else { return };
        self.section_title(y, text, "");
    }

    fn label(y: f32, text: &str) {
        put_text(text, PAD, y + 18.0, 15.0, col(3));
    }

    /// A note over as many rows as it needs.
    fn explain(rows: &mut Rows, w: f32, text: &str) {
        let per_line = ((w / 7.5) as usize).max(10);
        for line in wrap(text, per_line) {
            if let Some(y) = rows.next(16.0) {
                put_text(&line, PAD, y + 12.0, 14.0, INFO);
            }
        }
    }

    fn note(x: f32, y: f32, w: f32, text: &str) {
        put_text(&fit(text, w, 14.0), x, y + 18.0, 14.0, INFO);
    }

    /// The shown level's inspector: level, goals, settings and warnings.
    fn inspector(&mut self, top: f32) -> Option<Action> {
        let lang = self.lang;
        let w = self.inner_w();
        let bottom = screen_height() - 6.0;
        let m: Vec2 = mouse_position().into();
        if m.x < self.panel && m.y > top {
            let (_, wheel) = mouse_wheel();
            let most = (self.side_content - (bottom - top)).max(0.0);
            self.side_scroll = (self.side_scroll - wheel.signum() * SCROLL_STEP * (wheel != 0.0) as u8 as f32).clamp(0.0, most);
        }
        let mut rows = Rows { y: top - self.side_scroll, top, bottom };
        let mut act = None;
        let (cx, cw) = (PAD + LABEL_W, w - LABEL_W);
        let l = self.cur;

        if l >= LEVELS || self.doc.mods.level(l) {
            self.heading(&mut rows, tr(lang, "LEVEL"));
        }
        if l >= LEVELS {
            if let Some(y) = rows.next(ROW) {
                self.name_field(Rect::new(PAD, y, w, ROW), Named::Level(l));
            }
            if let Some(y) = rows.next(ROW) {
                Self::label(y, tr(lang, "Plays as"));
                Self::note(cx, y, cw, &format!("{} {:02}", tr(lang, "level"), self.doc.base_of(l)));
            }
        }
        if self.doc.mods.level(l) {
            if let Some(y) = rows.next(ROW) {
                Self::label(y, tr(lang, "Original"));
                Self::note(cx, y, cw, tr(lang, "mod (saved; Restore puts it back)"));
            }
        }

        self.heading(&mut rows, tr(lang, "GOALS"));
        let place = 70.0;
        let start = self.doc.level_start(l);
        let goals = [
            (tr(lang, "Start"), Action::PlaceStart, Placing::Start, format!("x {:04X} y {:04X}", start.mario_x, start.mario_y), true),
            (tr(lang, "Finish"), Action::PlaceFinish, Placing::Finish, self.goal_text(Goal::Finish), !self.doc.ends_with_boss(l)),
            (tr(lang, "Secret exit"), Action::PlaceSecret, Placing::Secret, self.goal_text(Goal::Secret), true),
        ];
        for (name, a, p, text, can) in goals {
            if let Some(y) = rows.next(ROW) {
                Self::label(y, name);
                if can && self.tool_button(Rect::new(cx, y, place, ROW), tr(lang, "Place"), a, self.placing == Some(p)) {
                    act = Some(a);
                }
                Self::note(cx + place + 6.0, y, cw - place - 6.0, &text);
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Leads to"));
            if self.tool_button(Rect::new(cx, y, place, ROW), tr(lang, "Choose"), Action::SecretLeads, false) {
                act = Some(Action::SecretLeads);
            }
            Self::note(cx + place + 6.0, y, cw - place - 6.0, &self.secret_leads());
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Checkpoints"));
            let refused = self.doc.checkpoint_refused(l);
            if refused != Some(CheckpointRefused::NoRoom) && self.tool_button(Rect::new(cx, y, place, ROW), tr(lang, "Add"), Action::PlaceCheckpoint, self.placing == Some(Placing::Checkpoint)) {
                act = Some(Action::PlaceCheckpoint);
            }
            let n = checkpoint::bells(&self.doc.levels[l].content.objs).len();
            let t = match refused {
                Some(CheckpointRefused::NoRoom) => tr(lang, "none in this level").to_string(),
                _ => format!("{n} / {}", checkpoint::MAX),
            };
            Self::note(cx + place + 6.0, y, cw - place - 6.0, &t);
        }

        self.heading(&mut rows, tr(lang, "SETTINGS"));
        let mut s = self.doc.level_settings(l);
        let stepper = |rows_y: f32, minus: &str, plus: &str, value: &str| stepper(Rect::new(cx, rows_y, cw, ROW), minus, plus, value);
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Time"));
            let d = stepper(y, "-", "+", &format!("{}", s.time as u32 * 100));
            s.time = (s.time as i32 + d).clamp(TIME_MIN as i32, TIME_MAX as i32) as u8;
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Background"));
            let names = [tr(lang, "normal"), tr(lang, "dark"), tr(lang, "darker")];
            let i = PALETTES.iter().position(|&p| p == s.palette);
            let d = stepper(y, "<", ">", i.map_or("?", |i| names[i]));
            if d != 0 {
                s.palette = PALETTES[(i.unwrap_or(0) as i32 + d).rem_euclid(PALETTES.len() as i32) as usize];
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Music"));
            let i = MUSICS.iter().position(|&(m, _)| m == s.music);
            let name = i.map_or(format!("${:02X}", s.music), |i| format!("{} {:02}", tr(lang, "as level"), MUSICS[i].1));
            let d = stepper(y, "<", ">", &name);
            if d != 0 {
                s.music = MUSICS[(i.unwrap_or(0) as i32 + d).rem_euclid(MUSICS.len() as i32) as usize].0;
            }
        }
        let playing = self.music_playing();
        if let Some(y) = rows.next(ROW) {
            if button(Rect::new(PAD, y, LABEL_W - GAP, ROW), if playing { tr(lang, "Stop") } else { tr(lang, "Listen") }, playing, 15.0) {
                self.toggle_music(s.music);
            }
            // how loud Listen plays: the editor's own setting, not the game's volume
            let v = self.prefs.music_volume as i32;
            let d = stepper(y, "-", "+", &format!("{} {v}", tr(lang, "editor vol")));
            let nv = (v + d).clamp(*MUSIC_VOLUMES.start() as i32, *MUSIC_VOLUMES.end() as i32) as u32;
            if nv != self.prefs.music_volume {
                self.set_music_volume(nv);
            }
        }
        if s != self.doc.level_settings(l) {
            self.doc.set_level_settings(l, s);
            if playing {
                self.toggle_music(s.music);
                self.toggle_music(s.music);
            }
        }
        // the game's per-level tables: two choices each
        let game = self.doc.game_tables(l);
        let mut t = self.doc.level_tables(l);
        let choice = |y: f32, names: [&str; 2], on: bool| -> Option<bool> {
            let hw = (cw - GAP) / 2.0;
            let mut pick = None;
            for (i, name) in names.iter().enumerate() {
                if button(Rect::new(cx + i as f32 * (hw + GAP), y, hw, ROW), name, on == (i == 1), 15.0) {
                    pick = Some(i == 1);
                }
            }
            pick
        };
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Gravity"));
            if let Some(on) = choice(y, [tr(lang, "normal"), tr(lang, "space")], t.space_gravity()) {
                t.set_space_gravity(on, &game);
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Auto-scroll"));
            if let Some(on) = choice(y, [tr(lang, "off"), tr(lang, "on")], t.scrolls()) {
                t.set_scrolls(on);
            }
        }
        if let Some(y) = rows.next(ROW) {
            Self::label(y, tr(lang, "Camera"));
            if let Some(on) = choice(y, [tr(lang, "normal"), tr(lang, "wide")], t.wide_camera()) {
                t.set_wide_camera(on);
            }
        }
        Self::explain(&mut rows, w, tr(lang, "Wide: when Mario climbs or jumps high the camera goes up sooner, so more of what is above shows."));
        if game.animated() {
            if let Some(y) = rows.next(ROW) {
                Self::label(y, tr(lang, "Animation"));
                if let Some(on) = choice(y, [tr(lang, "none"), tr(lang, "its own")], t.animated()) {
                    t.set_animated(on, &game);
                }
            }
        }
        if t.scrolls() {
            Self::explain(&mut rows, w, tr(lang, "Auto-scroll: the screen moves right by itself; caught against a wall Mario loses a life."));
        }
        if t != self.doc.level_tables(l) {
            self.doc.set_level_tables(l, t);
        }

        let warnings = self.doc.warnings(l);
        self.heading(&mut rows, &format!("{} ({})", tr(lang, "WARNINGS"), warnings.len()));
        if warnings.is_empty() {
            if let Some(y) = rows.next(20.0) {
                put_text(tr(lang, "None: the level looks fine."), PAD, y + 14.0, 15.0, INFO);
            }
        }
        for wa in &warnings {
            let Some(y) = rows.next(20.0) else { continue };
            let r = Rect::new(PAD, y, w, 20.0);
            let place = wa.place();
            if r.contains(m) && place.is_some() {
                draw_rectangle(r.x, r.y, r.w, r.h, col(1));
                self.tip = Some(tr(lang, "Click: show it on the level").into());
                if is_mouse_button_pressed(MouseButton::Left) {
                    let (x, py) = place.unwrap_or_default();
                    let p = vec2(x as f32, py as f32);
                    self.look = Some((p, true));
                    self.mark = Some((p, get_time() + MARK_SECONDS));
                }
            }
            draw_circle(PAD + 6.0, y + 10.0, 4.0, warn_col());
            put_text(&fit(&self.warning_text(wa), w - 18.0, 15.0), PAD + 16.0, y + 15.0, 15.0, col(3));
        }
        self.side_content = rows.y + self.side_scroll - top;
        if self.side_content > bottom - top {
            // where the view is in the whole inspector
            let frac = (bottom - top) / self.side_content;
            let at = self.side_scroll / self.side_content;
            draw_rectangle(self.panel - 8.0, top + at * (bottom - top), 3.0, frac * (bottom - top), col(2));
        }
        act
    }

    /// Where a goal is, or that the level has none.
    fn goal_text(&self, g: Goal) -> String {
        match self.doc.goal_cells(self.cur, g).first() {
            Some(&(c, r)) => format!("{c}, {r}"),
            None if g == Goal::Finish && self.doc.ends_with_boss(self.cur) => tr(self.lang, "ends with its boss").to_string(),
            None => tr(self.lang, "none").to_string(),
        }
    }
}

/// A value between - and + (or < and >) buttons in `r`; returns -1, 0 or 1.
pub(super) fn stepper(r: Rect, minus: &str, plus: &str, value: &str) -> i32 {
    let small = 26.0;
    let mut d = 0;
    if button(Rect::new(r.x, r.y, small, r.h), minus, false, 16.0) {
        d = -1;
    }
    let vw = r.w - 2.0 * small - 2.0 * GAP;
    let t = fit(value, vw - 4.0, 15.0);
    let dim = text_size(&t, 15);
    put_text(&t, r.x + small + GAP + (vw - dim.width) / 2.0, r.y + 18.0, 15.0, col(3));
    if button(Rect::new(r.x + r.w - small, r.y, small, r.h), plus, false, 16.0) {
        d = 1;
    }
    d
}
