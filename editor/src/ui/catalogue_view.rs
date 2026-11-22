//! The object catalogue (F3): the game's objects once each (`object_catalogue`), with the
//! editor's names (`object_names`), filtered by category and by "usable here". Choosing one
//! the shown level draws right picks its type; an enemy, trap or platform of another level of
//! the same object catalogue can be added to the level's own enemy set (`enemy_sets`) when its
//! tiles fit.
use super::level_panel::fit;
use super::shortcuts::{keys, Action};
use super::widgets::*;
use super::Editor;
use crate::domain::document::EnemySet;
use crate::domain::object_catalogue::CatalogueObject;
use crate::domain::object_names::{self, Category, CATEGORIES};
use cart_model::enemy_sets::{self, Enemy};
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_pressed, mouse_wheel, is_key_pressed};

const PAD: f32 = 10.0;
const ROW: f32 = 26.0;
const CELL: (f32, f32) = (150.0, 74.0);
/// The first level of the second object catalogue.
const SECOND: usize = 10;

/// The catalogue as shown: its objects, the filters, the one chosen, the view's scroll.
pub struct CatalogueView {
    objects: Vec<CatalogueObject>,
    filter: Option<Category>,
    show: Show,
    chosen: Option<usize>,
    scroll: f32,
}

/// Which objects the catalogue shows for the shown level. One choice: each is part of
/// Compatible, and Usable and Addable never overlap, so two at once would add nothing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Show {
    All,
    /// Usable here, or that can join the level's set (room or not).
    Compatible,
    /// The level draws it right (its own, or added to its set).
    Usable,
    /// Can be added to the level's set now.
    Addable,
}

/// What an object is to the shown level.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Standing {
    Usable,
    /// Brought into the level's set from another level.
    Added,
    /// Could join the level's set, as `Fit` says.
    Joins(Fit),
    /// Of the other object catalogue.
    OtherCatalogue,
    /// Of the level's catalogue, but the level does not draw it right and it cannot join.
    NotDrawn,
}

impl Standing {
    fn shown_by(self, show: Show) -> bool {
        match show {
            Show::All => true,
            Show::Compatible => matches!(self, Standing::Usable | Standing::Added) || matches!(self, Standing::Joins(f) if f != Fit::Graphics),
            Show::Usable => matches!(self, Standing::Usable | Standing::Added),
            Show::Addable => self == Standing::Joins(Fit::Yes),
        }
    }
}

/// A tag in a cell's bottom right corner; its width.
fn tag(r: Rect, text: &str, bg: Color, fg: Color) -> f32 {
    let w = text_size(text, 12).width + 10.0;
    draw_rectangle(r.x + r.w - w, r.y + r.h - 16.0, w, 16.0, bg);
    put_text(text, r.x + r.w - w + 5.0, r.y + r.h - 4.0, 12.0, fg);
    w
}

/// Whether an enemy of another level can join the shown level's enemy set.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fit {
    Yes,
    /// Its tiles are those of this enemy of the set.
    Clash(Enemy),
    /// The set has as many enemies as it can.
    Full,
    /// Its tiles, or the set's, are still being captured.
    Pending,
    /// It also draws with tiles outside the enemy area that this level loads with other
    /// graphics.
    Graphics,
}

/// The categories that can join a level's set.
const JOINS: [Category; 3] = [Category::Enemy, Category::Trap, Category::Platform];

/// A category as the palette's filter says it.
pub fn category_plural(lang: Lang, c: Option<Category>) -> &'static str {
    match c {
        None => tr(lang, "All objects"),
        Some(Category::PowerUp) => tr(lang, "Power-ups"),
        Some(Category::Enemy) => tr(lang, "Enemies"),
        Some(Category::Trap) => tr(lang, "Traps"),
        Some(Category::Platform) => tr(lang, "Platforms"),
        Some(Category::Goal) => tr(lang, "Goals"),
        Some(Category::Marker) => tr(lang, "Invisible markers"),
        Some(Category::Scripted) => tr(lang, "Boss and scripted"),
        Some(Category::BlockContent) => tr(lang, "Block contents"),
    }
}

pub fn category_name(lang: Lang, c: Category) -> &'static str {
    match c {
        Category::PowerUp => tr(lang, "power-up"),
        Category::Enemy => tr(lang, "enemy"),
        Category::Trap => tr(lang, "trap"),
        Category::Platform => tr(lang, "platform"),
        Category::Goal => tr(lang, "goal"),
        Category::Marker => tr(lang, "invisible marker"),
        Category::Scripted => tr(lang, "boss or scripted"),
        Category::BlockContent => tr(lang, "block content"),
    }
}

impl Editor {
    pub(super) fn open_catalogue(&mut self) {
        self.catalogue = Some(CatalogueView { objects: Vec::new(), filter: None, show: Show::All, chosen: None, scroll: 0.0 });
        self.status = tr(self.lang, "Object catalogue: the game's objects. Esc: close.").into();
    }

    /// The catalogue open (only power-ups with `power_ups`) once every object of the game's
    /// levels has its picture (for the check pictures).
    pub fn show_catalogue(&mut self, power_ups: bool) {
        let want: Vec<(usize, u8)> = (0..cart_model::levels::LEVELS).flat_map(|l| cart_model::objects::read(&self.doc.original, l).0.into_iter().map(move |o| (l, o.kind))).collect();
        self.look_loader.prefer(&want);
        while want.iter().any(|key| !self.looks.contains_key(key)) {
            let Some((l, k, look)) = self.look_loader.wait() else { break };
            self.add_look(l, k, look);
        }
        self.open_catalogue();
        if let Some(v) = &mut self.catalogue {
            v.filter = power_ups.then_some(Category::PowerUp);
        }
    }

    /// Choose in the open catalogue the object level `level` has as type `kind`.
    pub fn choose_in_catalogue(&mut self, level: usize, kind: u8) {
        let objects = self.catalogue_objects().to_vec();
        if let Some(v) = &mut self.catalogue {
            v.objects = objects;
            v.chosen = v.objects.iter().position(|o| o.uses.contains(&(level, kind)));
        }
    }

    /// The level shown's game level and whether it is in the second object catalogue.
    fn catalogue_of_shown(&self) -> (usize, bool) {
        let base = self.doc.base_of(self.cur);
        (base, base >= SECOND)
    }

    /// The catalogue's frame, in place of the level editor's.
    pub(super) fn catalogue_frame(&mut self) {
        let objects = self.catalogue_objects().to_vec();
        if let Some(v) = &mut self.catalogue {
            v.objects = objects;
        }
        self.pump_looks();
        let (base, second) = self.catalogue_of_shown();
        let offered = self.offered_here();
        let set: Vec<(usize, u8)> = self.doc.level_enemies(self.cur).map(|s| s.enemies.iter().map(|e| (e.level as usize, e.kind)).collect()).unwrap_or_default();
        let lang = self.lang;
        let members = self.set_with_tiles();
        let Some(v) = self.catalogue.take() else { return };
        let mut v = v;
        clear_background(col(3));
        // which the level draws right (the type it has here), what each object is to the level,
        // and which show
        let here = |o: &CatalogueObject| o.uses.iter().find(|u| set.contains(u)).or_else(|| o.uses.iter().find(|u| (u.0 >= SECOND) == second && offered.contains(&u.1))).map(|u| u.1);
        let standings: Vec<Standing> = v
            .objects
            .iter()
            .map(|o| {
                if o.uses.iter().any(|u| set.contains(u) && u.0 != base) {
                    Standing::Added
                } else if here(o).is_some() {
                    Standing::Usable
                } else if let Some(e) = self.joinable(o, second, false) {
                    Standing::Joins(self.fit_with(e, members.as_deref()))
                } else if !o.uses.iter().any(|u| (u.0 >= SECOND) == second) {
                    Standing::OtherCatalogue
                } else {
                    Standing::NotDrawn
                }
            })
            .collect();
        let shown: Vec<usize> = (0..v.objects.len())
            .filter(|&i| {
                let o = &v.objects[i];
                let cat = object_names::name(o.first().0, o.first().1).map(|n| n.1);
                v.filter.is_none_or(|f| cat == Some(f)) && standings[i].shown_by(v.show)
            })
            .collect();
        // the side panel
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let w = self.panel - 2.0 * PAD - 8.0;
        put_text(tr(lang, "OBJECT CATALOGUE"), PAD, 25.0, 20.0, col(3));
        let half = (w - 4.0) / 2.0;
        // two filters, combined: what the objects are to this level (one choice), and category
        let mut y = 46.0;
        put_text(tr(lang, "SHOW"), PAD, y, 13.0, col(2));
        y += 8.0;
        let shows = [
            (Show::All, tr(lang, "All")),
            (Show::Compatible, tr(lang, "Compatible")),
            (Show::Usable, tr(lang, "Usable here")),
            (Show::Addable, tr(lang, "Can be added")),
        ];
        for (i, (s, label)) in shows.iter().enumerate() {
            let r = Rect::new(PAD + (i % 2) as f32 * (half + 4.0), y + (i / 2) as f32 * (ROW + 4.0), half, ROW);
            if button(r, label, v.show == *s, 14.0) {
                v.show = *s;
            }
        }
        y += 2.0 * (ROW + 4.0) + 18.0;
        put_text(tr(lang, "CATEGORY"), PAD, y, 13.0, col(2));
        y += 8.0;
        let categories: Vec<(Option<Category>, &str)> = std::iter::once((None, tr(lang, "All kinds"))).chain(CATEGORIES.iter().map(|c| (Some(*c), category_name(lang, *c)))).collect();
        for (i, (c, label)) in categories.iter().enumerate() {
            let r = Rect::new(PAD + (i % 2) as f32 * (half + 4.0), y + (i / 2) as f32 * (ROW + 4.0), half, ROW);
            if button(r, label, v.filter == *c, 14.0) {
                v.filter = *c;
            }
        }
        y += categories.len().div_ceil(2) as f32 * (ROW + 4.0) + 6.0;
        draw_rectangle(PAD, y, w, 1.0, col(2));
        y += 8.0;
        put_text(&format!("{} {}", shown.len(), tr(lang, "objects")), PAD, y + 12.0, 15.0, INFO);
        y += 24.0;
        let mut pick = None;
        let mut add = None;
        let mut remove = None;
        let mut blank_from = None;
        if let Some(o) = v.chosen.and_then(|i| v.objects.get(i)) {
            let (l, k) = o.first();
            let (name, cat) = object_names::name(l, k).unwrap_or(("?", Category::Marker));
            let name = tr(lang, name);
            put_text(&fit(name, w, 18.0), PAD, y + 14.0, 18.0, col(3));
            y += 22.0;
            for line in wrap(&self.object_details(o), 34) {
                put_text(&line, PAD, y + 12.0, 15.0, col(3));
                y += 17.0;
            }
            y += 6.0;
            if let Some(note) = self.hidden_note((l, k)) {
                y += message_lines(&note, PAD, y + 12.0, w, 14.0, Severity::Info) + 4.0;
                // the blocks it works on, as the shown level draws them
                if let Some(atlas) = self.atlases[self.doc.base_of(self.cur)].clone() {
                    for (i, &b) in object_names::HIDDEN_IN.iter().enumerate() {
                        let x = PAD + i as f32 * 44.0;
                        draw_texture_ex(&atlas, x, y, WHITE, DrawTextureParams { source: Some(super::palette::source(b as usize, 16.0)), dest_size: Some(vec2(28.0, 28.0)), ..Default::default() });
                        draw_rectangle_lines(x, y, 28.0, 28.0, 1.0, col(2));
                        put_text(&format!("{b:02X}"), x, y + 40.0, 12.0, col(2));
                    }
                    y += 48.0;
                }
            }
            // in this level's set (its own enemies, or brought from another level): it can be
            // taken out, to make room or to undo an Add
            let member = o.uses.iter().find_map(|&(l, k)| members.as_ref()?.iter().find(|m| m.0.level as usize == l && m.0.kind == k).map(|m| m.0));
            if let Some(e) = member {
                let n = self.doc.levels[self.cur].content.objs.iter().filter(|ob| ob.kind == e.kind).count();
                if button(Rect::new(PAD, y, w, ROW), &format!("{} ({n} {})", tr(lang, "Remove from this level"), tr(lang, "placed")), false, 15.0) {
                    remove = Some(e);
                }
                y += ROW + 4.0;
            }
            match here(o) {
                Some(k) => {
                    if button(Rect::new(PAD, y, w, ROW), tr(lang, "Use in this level"), false, 15.0) {
                        pick = Some(k);
                    }
                }
                None if JOINS.contains(&cat) => {
                    if let Some(e) = self.joinable(o, second, false) {
                        let label = tr(lang, "Add to this level's enemies");
                        let r = Rect::new(PAD, y, w, ROW);
                        let fit = self.fit_with(e, members.as_deref());
                        if fit == Fit::Yes {
                            if button(r, label, false, 15.0) {
                                add = Some(e);
                            }
                        } else {
                            draw_rectangle(r.x, r.y, r.w, r.h, col(1));
                            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, col(2));
                            let d = text_size(label, 15);
                            put_text(label, r.x + (r.w - d.width) / 2.0, r.y + 17.0, 15.0, col(2));
                        }
                        y += ROW + 4.0;
                        let (why, kind) = match fit {
                            Fit::Yes => (tr(lang, "Fits: its tiles are free in this level.").to_string(), Severity::Done),
                            Fit::Graphics => (tr(lang, "Not usable here: it also draws with common sprite tiles, which this level loads with other graphics (it would look broken).").to_string(), Severity::Warning),
                            Fit::Clash(other) => (format!("{} {} {}", tr(lang, "Does not fit: it needs the tiles of"), Self::enemy_name(lang, other), tr(lang, "(remove it first).")), Severity::Warning),
                            Fit::Full => (format!("{} {} {}.", tr(lang, "Does not fit: the set has"), enemy_sets::MOST, tr(lang, "enemies")), Severity::Warning),
                            Fit::Pending => (format!("{} ({} {}).", tr(lang, "Known once the enemies' tiles are captured"), self.look_loader.tiles_left(), tr(lang, "left")), Severity::Info),
                        };
                        y += message_lines(&why, PAD, y + 12.0, w, 14.0, kind);
                        y -= ROW + 4.0;
                    } else if !o.uses.iter().any(|u| (u.0 >= SECOND) == second) {
                        // of the other object catalogue and not joinable: say why, and where it can be used
                        let theirs = if second { "0-9" } else { "10-31" };
                        let mine = if second { "10-31" } else { "0-9" };
                        let why = format!(
                            "{} {theirs} {} {mine} {} {theirs}.",
                            tr(lang, "Not usable here: it belongs to the object catalogue of levels"),
                            tr(lang, "and this level plays as one of levels"),
                            tr(lang, "(a level uses one catalogue, and a type number is another object in the other). It can be used in a level, or a blank level, made from levels")
                        );
                        y += message_lines(&why, PAD, y + 12.0, w, 14.0, Severity::Info) + 4.0;
                        if button(Rect::new(PAD, y, w, ROW), &format!("{} {theirs}", tr(lang, "New blank level from levels")), false, 14.0) {
                            blank_from = o.levels().first().copied();
                        }
                    } else {
                        y += message_lines(tr(lang, "This level does not draw it right (its graphics are not loaded here)."), PAD, y + 12.0, w, 14.0, Severity::Info) - ROW + 4.0;
                    }
                }
                None => {
                    y += message_lines(tr(lang, "This level does not draw it right (its graphics are not loaded here)."), PAD, y + 12.0, w, 14.0, Severity::Info) - ROW + 4.0;
                }
            }
            y += ROW + 8.0;
        }
        // what the last action did
        y += message_lines(&self.status.clone(), PAD, y + 12.0, w, 15.0, self.severity());
        y += 6.0;
        if button(Rect::new(PAD, y, w, ROW), &format!("{} ({})", tr(lang, "Close"), keys(Action::ObjectCatalogue)), false, 15.0) || is_key_pressed(KeyCode::Escape) {
            self.catalogue = None;
            return;
        }
        // the view: the objects in a grid
        let x0 = self.panel + 12.0;
        let per_row = (((screen_width() - x0 - 8.0) / CELL.0).floor() as usize).max(1);
        let rows = shown.len().div_ceil(per_row);
        let view_h = screen_height() - 30.0;
        let m: Vec2 = mouse_position().into();
        if m.x > self.panel {
            let wheel = mouse_wheel().1;
            v.scroll = (v.scroll - wheel.signum() * CELL.1 * (wheel != 0.0) as u8 as f32).clamp(0.0, (rows as f32 * CELL.1 - view_h + 12.0).max(0.0));
        }
        for (n, &i) in shown.iter().enumerate() {
            let o = &v.objects[i];
            let r = Rect::new(x0 + (n % per_row) as f32 * CELL.0, 8.0 + (n / per_row) as f32 * CELL.1 - v.scroll, CELL.0 - 6.0, CELL.1 - 6.0);
            if r.y + r.h < 0.0 || r.y > view_h {
                continue;
            }
            let usable = here(o).is_some();
            draw_rectangle(r.x, r.y, r.w, r.h, if v.chosen == Some(i) { col(1) } else { col(0) });
            if !usable {
                draw_rectangle_lines(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, Color::new(0.5, 0.5, 0.5, 1.0));
            }
            // what it is to this level, as a tag (none: the level draws it right)
            let muted = (col(2), col(0));
            let badge_w = match standings[i] {
                Standing::Usable => 0.0,
                Standing::Added => {
                    // framed in the palette's mark colour too
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, MARK);
                    tag(r, tr(lang, "ADDED"), MARK, WHITE)
                }
                Standing::Joins(Fit::Yes) => tag(r, tr(lang, "FITS"), Severity::Done.colour(), WHITE),
                Standing::Joins(Fit::Clash(_) | Fit::Full) => tag(r, tr(lang, "NO ROOM"), Severity::Warning.colour(), WHITE),
                Standing::Joins(Fit::Pending) => tag(r, "...", muted.0, muted.1),
                Standing::Joins(Fit::Graphics) | Standing::NotDrawn => tag(r, tr(lang, "NOT HERE"), muted.0, muted.1),
                Standing::OtherCatalogue => tag(r, tr(lang, "OTHER CATALOGUE"), col(3), col(0)),
            };
            let (l, k) = o.first();
            let hidden = self.hidden_look((l, k));
            if let Some((tex, _, _)) = self.looks.get(&(l, k)).and_then(|t| t.as_ref()).or(hidden) {
                let f = (40.0 / tex.width()).min(40.0 / tex.height()).min(2.0);
                draw_texture_ex(tex, r.x + 4.0, r.y + 4.0, WHITE, DrawTextureParams { dest_size: Some(vec2(tex.width() * f, tex.height() * f)), ..Default::default() });
                if hidden.is_some() {
                    super::objects_view::hidden_mark(Rect::new(r.x + 4.0, r.y + 4.0, tex.width() * f, tex.height() * f));
                }
            }
            let (name, cat) = object_names::name(l, k).map_or((format!("#{k:02X}"), "".to_string()), |(n, c)| (tr(lang, n).to_string(), category_name(lang, c).to_string()));
            // the name on up to two lines, then the category
            let per_line = ((r.w - 52.0) / 7.5).max(6.0) as usize;
            let lines = wrap(&name, per_line);
            for (i, line) in lines.iter().take(2).enumerate() {
                put_text(&fit(line, r.w - 50.0, 15.0), r.x + 48.0, r.y + 16.0 + i as f32 * 15.0, 15.0, col(3));
            }
            put_text(&fit(&cat, r.w - 50.0, 13.0), r.x + 48.0, r.y + 18.0 + lines.len().min(2) as f32 * 15.0, 13.0, col(2));
            let levels: Vec<String> = o.levels().iter().map(|l| format!("{l:02}")).collect();
            put_text(&fit(&levels.join(" "), r.w - 8.0 - badge_w, 13.0), r.x + 4.0, r.y + r.h - 6.0, 13.0, col(2));
            if r.contains(m) && is_mouse_button_pressed(MouseButton::Left) {
                v.chosen = Some(i);
            }
        }
        self.catalogue = Some(v);
        if self.own_pictures_pending() {
            let t = tr(lang, "Still capturing the objects' pictures: until it ends some objects are missing or show twice, and Usable here may leave some out.");
            let lines = wrap(t, ((screen_width() - x0 - 24.0) / 8.0) as usize);
            let h = lines.len() as f32 * 17.0 + 10.0;
            draw_rectangle(x0, view_h - h - 34.0, screen_width() - x0 - 10.0, h, col(0));
            draw_rectangle_lines(x0, view_h - h - 34.0, screen_width() - x0 - 10.0, h, 2.0, col(3));
            for (i, line) in lines.iter().enumerate() {
                put_text(line, x0 + 8.0, view_h - h - 34.0 + 18.0 + i as f32 * 17.0, 15.0, col(3));
            }
        }
        draw_rectangle(self.panel, screen_height() - 22.0, screen_width() - self.panel, 22.0, col(1));
        status_message(&self.status, self.panel, screen_width(), screen_height() - 22.0, 22.0, 16.0, self.severity());
        self.capture_badge(screen_height() - 22.0);
        if let Some(k) = pick {
            self.catalogue = None;
            self.kind = None;
            self.choose_kind(k, false);
        }
        if let Some(e) = add {
            self.add_enemy(e);
        }
        if let Some(e) = remove {
            self.remove_enemy(e);
        }
        if let Some(l) = blank_from {
            self.catalogue = None;
            self.show_blank_chooser_at(l);
        }
    }

    /// An object's type numbers, levels and enemy tiles, for the catalogue's side panel.
    fn object_details(&self, o: &CatalogueObject) -> String {
        let lang = self.lang;
        let mut types: Vec<String> = o.uses.iter().map(|u| format!("${:02X} ({})", u.1, if u.0 >= SECOND { "10-31" } else { "0-9" })).collect();
        types.dedup();
        let levels: Vec<String> = o.levels().iter().map(|l| format!("{l:02}")).collect();
        let tiles = o.uses.iter().find_map(|u| self.object_tiles.get(u)).map(|t| enemy_sets::area_tiles(t).len());
        let mut s = format!("{}: {}. {}: {}.", tr(lang, "Type"), types.join(", "), tr(lang, "Levels"), levels.join(" "));
        if let Some(n) = tiles {
            s += &format!(" {} {n} {}.", tr(lang, "Uses"), tr(lang, "of the 56 enemy tiles"));
        }
        s
    }

    /// The enemies of game level `base` (types whose tiles there are in the enemy area), with
    /// their tiles; None while some are not captured yet.
    fn own_enemies(&self, base: usize) -> Option<Vec<(Enemy, Vec<u8>)>> {
        let mut kinds: Vec<u8> = cart_model::objects::read(&self.doc.original, base).0.iter().map(|o| o.kind).collect();
        kinds.sort_unstable();
        kinds.dedup();
        let mut v = Vec::new();
        for k in kinds {
            let t = self.object_tiles.get(&(base, k))?;
            if !enemy_sets::area_tiles(t).is_empty() {
                v.push((Enemy { level: base as u8, kind: k }, t.clone()));
            }
        }
        Some(v)
    }

    /// The shown level's enemy set with its members' tiles (its own enemies when it has none).
    pub(super) fn set_with_tiles(&self) -> Option<Vec<(Enemy, Vec<u8>)>> {
        let base = self.doc.base_of(self.cur);
        match self.doc.level_enemies(self.cur) {
            Some(set) => set.enemies.iter().map(|e| Some((*e, self.object_tiles.get(&(e.level as usize, e.kind))?.clone()))).collect(),
            None => self.own_enemies(base),
        }
    }

    fn enemy_name(lang: Lang, e: Enemy) -> String {
        object_names::name(e.level as usize, e.kind).map_or(format!("#{:02X}", e.kind), |n| tr(lang, n.0).to_string())
    }

    /// Whether `e` can join the shown level's set, given the set with its tiles.
    fn fit_with(&self, e: Enemy, members: Option<&[(Enemy, Vec<u8>)]>) -> Fit {
        let (Some(members), Some(tiles)) = (members, self.object_tiles.get(&(e.level as usize, e.kind))) else { return Fit::Pending };
        let outside = tiles.iter().filter(|t| !enemy_sets::AREA.contains(t)).fold(0, |m, &t| m | cart_model::level_graphics::set_of(t));
        if !cart_model::level_graphics::same_sets(&self.doc.original, self.doc.base_of(self.cur), e.level as usize, outside) {
            return Fit::Graphics;
        }
        if members.len() >= enemy_sets::MOST {
            return Fit::Full;
        }
        members.iter().find(|m| enemy_sets::clash(tiles, &m.1)).map_or(Fit::Yes, |m| Fit::Clash(m.0))
    }

    /// The member an object would be in the shown level's set (from a level of its catalogue),
    /// if it is an enemy, trap or platform that level does not draw right yet and that draws
    /// with enemy tiles (or is not captured yet).
    fn joinable(&self, o: &CatalogueObject, second: bool, usable: bool) -> Option<Enemy> {
        let (l, k) = o.first();
        let joins = object_names::name(l, k).is_some_and(|n| JOINS.contains(&n.1));
        if !joins || usable {
            return None;
        }
        let &(l, k) = o.uses.iter().find(|u| (u.0 >= SECOND) == second)?;
        let no_enemy_tiles = self.object_tiles.get(&(l, k)).is_some_and(|t| enemy_sets::area_tiles(t).is_empty());
        (!no_enemy_tiles).then_some(Enemy { level: l as u8, kind: k })
    }

    /// Put enemy `e` in the shown level's set, if its tiles are known and fit.
    pub(super) fn add_enemy(&mut self, e: Enemy) {
        let lang = self.lang;
        let (Some(members), Some(tiles)) = (self.set_with_tiles(), self.object_tiles.get(&(e.level as usize, e.kind)).cloned()) else {
            self.status = format!("{} ({} {}).", tr(lang, "Not added: the enemies' tiles are still being captured; try again in a moment"), self.look_loader.tiles_left(), tr(lang, "left"));
            self.mark(Severity::Warning);
            return;
        };
        if members.iter().any(|m| m.0 == e) {
            return;
        }
        if self.fit_with(e, Some(&members)) == Fit::Graphics {
            self.status = format!("{} {}", Self::enemy_name(lang, e), tr(lang, "not added: it also draws with common sprite tiles this level loads with other graphics."));
            self.mark(Severity::Warning);
            return;
        }
        if members.len() >= enemy_sets::MOST {
            self.status = format!("{} {} {}.", tr(lang, "Not added: a level's set has at most"), enemy_sets::MOST, tr(lang, "enemies"));
            self.mark(Severity::Warning);
            return;
        }
        if let Some((other, _)) = members.iter().find(|m| enemy_sets::clash(&tiles, &m.1)) {
            self.status = format!("{} {} {} {}.", Self::enemy_name(lang, e), tr(lang, "does not fit: it needs the tiles of"), Self::enemy_name(lang, *other), tr(lang, "(remove it first: choose it in the catalogue)"));
            self.mark(Severity::Warning);
            return;
        }
        let mut all = members;
        all.push((e, tiles));
        self.set_level_enemies_from(all);
        self.status = format!("{} {}.", Self::enemy_name(lang, e), tr(lang, "added to this level's enemies"));
        self.mark(Severity::Done);
    }

    /// Take enemy `e` out of the shown level's set, and its objects out of the level (they
    /// would look broken), in one undo step.
    pub(super) fn remove_enemy(&mut self, e: Enemy) {
        let lang = self.lang;
        let Some(mut members) = self.set_with_tiles() else {
            self.status = tr(lang, "Not removed: the enemies' tiles are still being captured.").into();
            self.mark(Severity::Warning);
            return;
        };
        members.retain(|m| m.0 != e);
        let base = self.doc.base_of(self.cur);
        let block = enemy_sets::build(&self.doc.original, base, &members);
        let set = EnemySet { enemies: members.iter().map(|m| m.0).collect(), block };
        let l = self.cur;
        let gone = self.doc.drop_set_enemy(l, set, e.kind);
        self.offered.clear();
        self.sel_obj = None;
        if self.kind == Some(e.kind) {
            self.kind = None;
        }
        self.status = format!("{} {}; {gone} {} {}", Self::enemy_name(lang, e), tr(lang, "removed from this level's enemies"), tr(lang, "of its objects removed from the level"), super::shortcuts::on_this_system(tr(lang, "(Ctrl+Z brings them back).")));
        self.mark(Severity::Done);
    }

    /// The enemy of the shown level's set that came from another level and is type `k`.
    pub(super) fn imported_enemy(&self, k: u8) -> Option<Enemy> {
        let base = self.doc.base_of(self.cur) as u8;
        self.doc.level_enemies(self.cur)?.enemies.iter().find(|e| e.kind == k && e.level != base).copied()
    }

    fn set_level_enemies_from(&mut self, members: Vec<(Enemy, Vec<u8>)>) {
        let base = self.doc.base_of(self.cur);
        let block = enemy_sets::build(&self.doc.original, base, &members);
        let set = EnemySet { enemies: members.iter().map(|m| m.0).collect(), block };
        let l = self.cur;
        self.doc.set_level_enemies(l, Some(set));
        self.offered.clear();
    }
}
