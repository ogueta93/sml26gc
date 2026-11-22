//! Objects mode: each level's objects drawn with the game's own pictures, placed, dragged,
//! retyped and deleted with the mouse.
use super::widgets::*;
use super::{Editor, Mode};
use crate::application::look_loader::LookLoader;
use crate::domain::look_cache;
use crate::domain::object_catalogue::{self, CatalogueObject, Seen};
use crate::domain::object_names::{self, Category};
use cart_model::art::Look;
use cart_model::objects::Obj;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_pressed, is_mouse_button_down};

/// New pictures between two writes of the cache.
const LOOKS_SAVE_EVERY: usize = 16;

/// The first level of the second object catalogue.
const SECOND: usize = 10;

/// A block content's mark over the picture of what it gives: a small ? in the corner `r` ends at.
pub(super) fn hidden_mark(r: Rect) {
    let s = (r.w.min(r.h) * 0.4).clamp(9.0, 14.0);
    let (x, y) = (r.x + r.w - s, r.y + r.h - s);
    draw_rectangle(x, y, s, s, col(3));
    let d = text_size("?", s as u16);
    put_text("?", x + (s - d.width) / 2.0, y + (s + d.offset_y) / 2.0, s, col(0));
}

impl Editor {
    /// Take the object pictures the background capture has made (asking for the shown
    /// level's first) and keep them in the cache now and then.
    pub(super) fn pump_looks(&mut self) {
        let base = self.doc.base_of(self.cur);
        if self.mode == Mode::Objects && !self.in_world {
            let mut want: Vec<(usize, u8)> = self.doc.levels[self.cur].content.objs.iter().map(|o| (base, o.kind)).collect();
            want.extend(self.kind.map(|k| (base, k)));
            want.extend(self.offered.get(&base).into_iter().flatten().map(|(k, _)| (base, *k)));
            want.retain(|key| !self.looks.contains_key(key));
            self.look_loader.prefer(&want);
            if self.look_loader.tiles_left() > 0 {
                let mut tiles: Vec<(usize, u8)> = cart_model::objects::read(&self.doc.original, base).0.iter().map(|o| (base, o.kind)).collect();
                tiles.extend(self.doc.level_enemies(self.cur).into_iter().flat_map(|set| set.enemies.iter().map(|e| (e.level as usize, e.kind))));
                self.look_loader.prefer_tiles(&tiles);
            }
        }
        self.look_loader.pump();
        for (l, k, look) in self.look_loader.take() {
            self.add_look(l, k, look);
        }
        for (l, k, t) in self.look_loader.take_tiles() {
            self.add_tiles(l, k, t);
        }
        let all = self.look_data.len() + self.tile_data.len();
        if all >= self.looks_saved + LOOKS_SAVE_EVERY || (all > self.looks_saved && self.look_loader.done()) {
            self.look_store.save(&look_cache::to_text(&self.look_data, &self.tile_data));
            self.looks_saved = all;
        }
    }

    /// Forget every captured picture and tile (the cache too) and capture them all again.
    pub(super) fn recapture_objects(&mut self) {
        self.looks.clear();
        self.captured.clear();
        self.look_data.clear();
        self.tile_data.clear();
        self.object_tiles.clear();
        self.offered.clear();
        self.catalogue_index = (usize::MAX, Vec::new());
        self.looks_saved = 0;
        self.look_store.save(&look_cache::to_text(&[], &[]));
        // the old loader's threads stop when their next result has nowhere to go
        self.look_loader = LookLoader::start(self.doc.rom.clone(), self.doc.original.clone(), &Default::default(), &Default::default());
        self.status = tr(self.lang, "Capturing every object picture again (see the box at the bottom right).").into();
    }

    /// Wait until every picture and tile is captured, and keep them (for the command line).
    pub fn capture_all(&mut self) {
        while !self.look_loader.done() {
            self.look_loader.pump();
            for (l, k, look) in self.look_loader.take() {
                self.add_look(l, k, look);
            }
            for (l, k, t) in self.look_loader.take_tiles() {
                self.add_tiles(l, k, t);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
        for (l, k, look) in self.look_loader.take() {
            self.add_look(l, k, look);
        }
        for (l, k, t) in self.look_loader.take_tiles() {
            self.add_tiles(l, k, t);
        }
        self.look_store.save(&look_cache::to_text(&self.look_data, &self.tile_data));
    }

    pub(super) fn add_tiles(&mut self, l: usize, k: u8, tiles: Vec<u8>) {
        self.object_tiles.insert((l, k), tiles.clone());
        self.tile_data.push((l, k, tiles));
    }

    pub(super) fn add_look(&mut self, l: usize, k: u8, look: Option<Look>) {
        let v = look.as_ref().map(|lk| (look_texture(lk), lk.dx as f32, lk.dy as f32));
        if let Some(lk) = &look {
            self.captured.insert((l, k), lk.clone());
        }
        self.looks.insert((l, k), v);
        self.look_data.push((l, k, look));
    }

    /// Where type `k`'s picture in the shown level comes from: the level whose sprites draw it
    /// (an enemy of the level's own set comes from its own level), and the type.
    pub(super) fn look_key(&self, k: u8) -> (usize, u8) {
        let from = self.doc.level_enemies(self.cur).and_then(|s| s.enemies.iter().find(|e| e.kind == k)).map(|e| e.level as usize);
        (from.unwrap_or_else(|| self.doc.base_of(self.cur)), k)
    }

    pub(super) fn look(&self, k: u8) -> Option<&(Texture2D, f32, f32)> {
        self.looks.get(&self.look_key(k)).and_then(|v| v.as_ref())
    }

    /// The block content at `key` (level, type): what it gives (`object_names::hidden`), if it
    /// is one.
    fn hidden_at(key: (usize, u8)) -> Option<object_names::Hidden> {
        let (_, Category::BlockContent) = object_names::name(key.0, key.1)? else { return None };
        object_names::hidden(key.1)
    }

    /// The picture of what the block content at `key` gives (its own draws nothing): a
    /// captured power-up of that type, from its object catalogue first.
    pub(super) fn hidden_look(&self, key: (usize, u8)) -> Option<&(Texture2D, f32, f32)> {
        let object_names::Gives::Object(gives) = Self::hidden_at(key)?.gives else { return None };
        let second = key.0 >= SECOND;
        let power_up = |&(l, k): &(usize, u8)| k == gives && object_names::name(l, k).is_some_and(|n| n.1 == Category::PowerUp);
        self.looks.iter().filter(|(k, v)| power_up(k) && v.is_some()).min_by_key(|(k, _)| ((k.0 >= SECOND) != second, k.0)).and_then(|(_, v)| v.as_ref())
    }

    /// What the block content at `key` gives and where it works, as the panels say it.
    pub(super) fn hidden_note(&self, key: (usize, u8)) -> Option<String> {
        let h = Self::hidden_at(key)?;
        let lang = self.lang;
        let other = if key.0 >= SECOND { 0 } else { SECOND };
        let name = |k: u8| object_names::name(key.0, k).or_else(|| object_names::name(other, k)).map_or(format!("#{k:02X}"), |n| tr(lang, n.0).to_string());
        let big = h.big_gives.map_or(String::new(), |b| format!(" ({} {})", name(b), tr(lang, "when Mario is big")));
        let gives = match h.gives {
            object_names::Gives::Object(k) => name(k),
            object_names::Gives::Coins(n) => format!("{} {n} {}", tr(lang, "an item marked M:"), tr(lang, "coins")),
        };
        let blocks: Vec<String> = object_names::HIDDEN_IN.iter().map(|b| format!("${b:02X}")).collect();
        Some(format!(
            "{} {}{big}. {} {} {}",
            tr(lang, "Hidden in a block: hitting the block under it from below gives"),
            gives,
            tr(lang, "It only works on blocks"),
            blocks.join(", "),
            tr(lang, "(the plain block, the ? block and the invisible block among them); anywhere else it does nothing.")
        ))
    }

    /// The game's objects once each (`object_catalogue`), grouped again when more pictures
    /// have come in.
    pub(super) fn catalogue_objects(&mut self) -> &[CatalogueObject] {
        let known = self.looks.len();
        if self.catalogue_index.0 != known {
            let (looks, captured) = (&self.looks, &self.captured);
            let objects = object_catalogue::objects(&self.doc.original, |key| match (looks.get(&key), captured.get(&key)) {
                (None, _) => Seen::NotYet,
                (Some(_), Some(l)) => Seen::Picture(l),
                (Some(_), None) => Seen::Nothing,
            });
            self.catalogue_index = (known, objects);
        }
        &self.catalogue_index.1
    }

    /// The catalogue's name and category for the picture at `key` (level, type): the object
    /// that level uses it as, or the one drawn with the same pixels, or the names table's.
    pub(super) fn object_label(&mut self, key: (usize, u8)) -> Option<(&'static str, Category)> {
        self.catalogue_objects();
        let here = self.captured.get(&key);
        let same = |o: &CatalogueObject| here.is_some_and(|h| self.captured.get(&o.first()).is_some_and(|l| (l.w, l.h, &l.px) == (h.w, h.h, &h.px)));
        let first = self.catalogue_index.1.iter().find(|o| o.uses.contains(&key)).or_else(|| self.catalogue_index.1.iter().find(|o| same(o))).map(|o| o.first());
        let (l, k) = first.unwrap_or(key);
        object_names::name(l, k).map(|(n, c)| (tr(self.lang, n), c))
    }

    /// For the check pictures: wait for the shown level's object pictures.
    pub(super) fn wait_for_level_looks(&mut self) {
        let base = self.doc.base_of(self.cur);
        let want: Vec<(usize, u8)> = self.doc.levels[self.cur].content.objs.iter().map(|o| (base, o.kind)).collect();
        self.look_loader.prefer(&want);
        while want.iter().any(|key| !self.looks.contains_key(key)) {
            let Some((l, k, look)) = self.look_loader.wait() else { break };
            self.add_look(l, k, look);
        }
    }

    /// The object under a point of the level (level pixels), topmost first.
    fn object_at(&self, w: Vec2) -> Option<usize> {
        self.doc.levels[self.cur].content.objs.iter().rposition(|o| Rect::new(o.x as f32, o.y as f32, 16.0, 16.0).contains(w))
    }

    pub(super) fn update_objects(&mut self, m: Vec2) {
        let in_view = self.cell_at(m).is_some();
        let w = self.world_pos(m);
        let snap = |v: f32, max: f32| ((v / 8.0).floor() * 8.0).clamp(0.0, max) as u16;
        let l = self.cur;
        if is_mouse_button_pressed(MouseButton::Left) && in_view {
            self.doc.snapshot(l);
            if let Some(i) = self.object_at(w) {
                let o = self.doc.levels[l].content.objs[i];
                self.sel_obj = Some(i);
                self.kind = Some(o.kind);
                self.drag_obj = Some((i, w - vec2(o.x as f32, o.y as f32)));
            } else if let Some(kind) = self.kind {
                let i = self.doc.add_object(l, Obj { x: snap(w.x, 4088.0), y: snap(w.y, 760.0), kind, flag: false });
                self.sel_obj = Some(i);
                self.drag_obj = Some((i, vec2(4.0, 4.0)));
            } else {
                self.sel_obj = None;
            }
        }
        if let Some((i, off)) = self.drag_obj {
            if is_mouse_button_down(MouseButton::Left) {
                let p = w - off;
                self.doc.move_object(l, i, snap(p.x, 4088.0), snap(p.y, 760.0));
            } else {
                self.drag_obj = None;
            }
        }
        // right click: on an object, delete it; elsewhere, put the palette's type down
        if is_mouse_button_pressed(MouseButton::Right) && in_view {
            match self.object_at(w) {
                Some(i) => self.delete_object(i),
                None => self.drop_kind(),
            }
        }
    }

    fn delete_object(&mut self, i: usize) {
        self.doc.snapshot(self.cur);
        self.doc.remove_object(self.cur, i);
        self.sel_obj = None;
        self.drag_obj = None;
    }

    pub(super) fn delete_selected_object(&mut self) {
        if let Some(i) = self.sel_obj {
            self.delete_object(i);
        }
    }

    /// Pick an object type in the palette: the selected object takes it too. The type already
    /// chosen is put down instead.
    /// Type `k` chosen to place: the selected object is let go, or with `change` (Shift+click)
    /// it becomes that type and stays selected.
    pub(super) fn choose_kind(&mut self, k: u8, change: bool) {
        self.remember(self.recent_key(true), k);
        if let (true, Some(i)) = (change, self.sel_obj) {
            self.kind = Some(k);
            self.doc.set_object_kind(self.cur, i, k);
            return;
        }
        if self.kind == Some(k) {
            self.drop_kind();
            return;
        }
        self.kind = Some(k);
        self.sel_obj = None;
    }

    /// Choose type `k` in the palette (for the command line).
    pub fn pick_kind(&mut self, k: u8) {
        self.kind = Some(k);
    }

    /// No type chosen in the palette and no object selected: clicks only pick objects.
    fn drop_kind(&mut self) {
        self.kind = None;
        self.sel_obj = None;
    }

    pub(super) fn draw_objects(&self) {
        let objs_mode = self.mode == Mode::Objects;
        for (i, o) in self.doc.levels[self.cur].content.objs.iter().enumerate() {
            let x = self.panel + (o.x as f32 - self.cam.x) * self.scale();
            let y = (o.y as f32 - self.cam.y) * self.scale();
            let s = 16.0 * self.scale();
            if x + s < self.panel || x > screen_width() || y + s < 0.0 || y > screen_height() {
                continue;
            }
            let a = if objs_mode { 0.75 } else { 0.4 };
            let sel = objs_mode && self.sel_obj == Some(i);
            if let Some((tex, dx, dy)) = self.look(o.kind) {
                // the object's own picture, as the game draws it
                let tint = if objs_mode { WHITE } else { Color::new(1.0, 1.0, 1.0, 0.7) };
                draw_texture_ex(tex, x + dx * self.scale(), y + dy * self.scale(), tint, DrawTextureParams {
                    dest_size: Some(vec2(tex.width() * self.scale(), tex.height() * self.scale())),
                    ..Default::default()
                });
                if objs_mode {
                    marker(Rect::new(x, y, s, s), if sel { 3.0 } else { 1.5 }, if sel { MARK_SELECTED } else { MARK });
                    marked_text(&format!("{:02X}", o.kind), x + 2.0, y + s - 2.0, 14.0, if sel { MARK_SELECTED } else { MARK });
                }
                continue;
            }
            draw_rectangle(x, y, s, s, Color::new(0.12, 0.12, 0.12, a));
            let c = if sel { MARK_SELECTED } else { Color { a: a + 0.25, ..MARK } };
            marker(Rect::new(x, y, s, s), if sel { 3.0 } else { 1.5 }, c);
            let t = format!("{:02X}", o.kind);
            let fs = (s * 0.55).clamp(10.0, 26.0);
            let dim = text_size(&t, fs as u16);
            marked_text(&t, x + (s - dim.width) / 2.0, y + (s + dim.offset_y) / 2.0, fs, c);
        }
    }

    /// What a click would do, at the bottom of the view.
    pub(super) fn object_tip(&self, ch: f32) {
        let m: Vec2 = mouse_position().into();
        if self.cell_at(m).is_none() {
            return;
        }
        let tip = match self.object_at(self.world_pos(m)) {
            Some(i) => {
                let o = self.doc.levels[self.cur].content.objs[i];
                format!("#{:02X}  x {} y {}  {}", o.kind, o.x, o.y, tr(self.lang, "drag: move   right click: delete"))
            }
            None => match self.kind {
                Some(k) => format!("{} #{k:02X}   {}", tr(self.lang, "click: place object"), tr(self.lang, "right click: none")),
                None => tr(self.lang, "choose an object in the palette to place it").to_string(),
            },
        };
        put_text(&tip, self.panel + 8.0, ch - 8.0, 20.0, WHITE);
    }
}
