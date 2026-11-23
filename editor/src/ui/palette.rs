//! The side panel's palettes: the level's 128 blocks, the 128 object types and the shown map's
//! 256 tiles (as the game draws them: a zone's picture or the world map's pieces, then the
//! tiles left out). Each is zoomed
//! with its own setting and keeps its layout; when it is bigger than the panel it scrolls
//! (wheel, right-drag) and shows scroll bars. Dragging over the blocks or tiles selects a
//! rectangle of them.
use super::grid_edit::Brush;
use super::shortcuts::{self, Action};
use super::widgets::*;
use super::testing_view::warn_col;
use super::Editor;
use crate::domain::object_names::{Category, CATEGORIES};
use cart_model::original_points::OnMap;
use crate::domain::tile_palette::{self, TileCells};
use cart_model::{checkpoint, level_graphics, world, zone_maps};
use crate::domain::stamp::{Area, Stamp};
use crate::domain::tools::ToolKind;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_pressed, is_mouse_button_down, mouse_wheel};

/// Draw only inside `r` (or everywhere with None); the scissor is in window pixels, below the
/// menu bar's inset.
pub(super) fn clip(r: Option<Rect>) {
    let d = screen_dpi_scale();
    let gl = unsafe { get_internal_gl() };
    let r = r.map(|r| Rect { y: r.y + super::inset::top(), ..r });
    gl.quad_gl.scissor(r.map(|r| ((r.x * d) as i32, (r.y * d) as i32, (r.w * d) as i32, (r.h * d) as i32)));
}

/// Panel pixels beside a palette: its left margin, and the scroll bar and border on the right.
const MARGINS: f32 = 24.0;

/// A palette's layout: `count` cells, `cols` per row, of `px` source pixels, `gap` pixels apart.
#[derive(Clone, Copy)]
struct Layout {
    count: usize,
    cols: usize,
    px: f32,
    gap: f32,
}

impl Layout {
    const BLOCKS: Layout = Layout { count: 128, cols: 8, px: 16.0, gap: 2.0 };
    /// A map's tiles as the game draws them (`tile_palette`).
    const MAP_TILES: Layout = Layout { count: 256, cols: tile_palette::COLS, px: 8.0, gap: 1.0 };

    fn cell(&self, zoom: u32) -> f32 {
        self.px * zoom as f32 + self.gap
    }

    fn width(&self, zoom: u32) -> f32 {
        self.cols as f32 * self.cell(zoom)
    }
}

/// How a palette is scrolled: its offset and a right-button drag (mouse and offset when it
/// started).
#[derive(Clone, Copy, Default)]
pub struct PaletteView {
    scroll: Vec2,
    drag: Option<(Vec2, Vec2)>,
}

/// The chosen cells' outline: the bright mark colour between two black lines, plain over
/// light and dark cells alike.
fn chosen_frame(r: Rect) {
    draw_rectangle_lines(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0, 2.0, BLACK);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, MARK_SELECTED);
    draw_rectangle_lines(r.x + 3.0, r.y + 3.0, r.w - 6.0, r.h - 6.0, 1.0, BLACK);
}

/// The cells drawn by `draw` inside `area`, scrolled by the wheel (Shift: sideways) or a
/// right-drag, with scroll bars when cut; `outline` frames a rectangle of cells. Returns the
/// cell under the mouse (column, row), none while dragging.
fn cell_grid(area: Rect, l: Layout, cell: f32, view: &mut PaletteView, outline: Option<Area>, mut draw: impl FnMut(usize, Rect)) -> Option<(i32, i32)> {
    let size = vec2(l.cols as f32 * cell, l.count.div_ceil(l.cols) as f32 * cell);
    let m: Vec2 = mouse_position().into();
    let over = area.contains(m);
    let wheel = mouse_wheel().1;
    if over && wheel != 0.0 {
        let step = -wheel.signum() * cell * 2.0;
        if is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift) {
            view.scroll.x += step;
        } else {
            view.scroll.y += step;
        }
    }
    if over && is_mouse_button_pressed(MouseButton::Right) {
        view.drag = Some((m, view.scroll));
    }
    if let Some((m0, s0)) = view.drag {
        if is_mouse_button_down(MouseButton::Right) {
            view.scroll = s0 - (m - m0);
        } else {
            view.drag = None;
        }
    }
    let max = (size - area.size()).max(Vec2::ZERO);
    view.scroll = view.scroll.clamp(Vec2::ZERO, max);
    let scroll = view.scroll;
    let at = |c: i32, r: i32| vec2(area.x + c as f32 * cell - scroll.x, area.y + r as f32 * cell - scroll.y);
    clip(Some(area));
    for i in 0..l.count {
        let p = at((i % l.cols) as i32, (i / l.cols) as i32);
        let r = Rect::new(p.x, p.y, cell - l.gap, cell - l.gap);
        if r.overlaps(&area) {
            draw(i, r);
        }
    }
    if let Some(a) = outline {
        let p = at(a.x0, a.y0);
        chosen_frame(Rect::new(p.x - 1.0, p.y - 1.0, a.w() as f32 * cell, a.h() as f32 * cell));
    }
    clip(None);
    let bar = Color::new(0.12, 0.12, 0.12, 0.8);
    if max.x > 0.0 {
        let w = area.w * area.w / size.x;
        draw_rectangle(area.x + (area.w - w) * scroll.x / max.x, area.y + area.h + 1.0, w, 4.0, bar);
    }
    if max.y > 0.0 {
        let h = area.h * area.h / size.y;
        draw_rectangle(area.x + area.w + 2.0, area.y + (area.h - h) * scroll.y / max.y, 4.0, h, bar);
    }
    if !over || view.drag.is_some() {
        return None;
    }
    let c = ((m.x - area.x + scroll.x) / cell).floor() as i32;
    let r = ((m.y - area.y + scroll.y) / cell).floor() as i32;
    (c >= 0 && (c as usize) < l.cols && r >= 0 && (r as usize * l.cols + c as usize) < l.count).then_some((c, r))
}

/// Cell `i` of a 16-per-row source picture of `px`-pixel cells.
pub(super) fn source(i: usize, px: f32) -> Rect {
    Rect::new((i % 16) as f32 * px, (i / 16) as f32 * px, px, px)
}

impl Editor {
    /// The side panel's width for the palette shown: all its columns, but never narrower
    /// than the buttons nor wider than half the window.
    pub(super) fn fitted_panel(&self) -> f32 {
        let w = if self.in_world { Layout::MAP_TILES.width(self.prefs.map_palette_zoom) } else { Layout::BLOCKS.width(self.prefs.level_palette_zoom) };
        (w + MARGINS).clamp(PANEL_MIN, (screen_width() / 2.0).max(PANEL_MIN))
    }

    /// The palette's title, then its zoom row; returns where the palette starts.
    fn palette_title(&mut self, y: f32, title: &str) -> f32 {
        put_text(title, 10.0, y + 14.0, 18.0, col(3));
        self.zoom_row(y + 20.0)
    }

    /// The palette's zoom keys and - + buttons; returns where the palette starts.
    fn zoom_row(&mut self, y: f32) -> f32 {
        let key = host::shortcut_key::shortcut_key().label();
        let keys = format!("Zoom  {key} {}/{}", shortcuts::keys(Action::ZoomOut).trim_start_matches(key).trim_start_matches('+'), shortcuts::keys(Action::ZoomIn).trim_start_matches(key).trim_start_matches('+'));
        put_text(&keys, 10.0, y + 15.0, 15.0, INFO);
        let right = self.panel - 10.0;
        if button(Rect::new(right - 54.0, y, 26.0, 20.0), "-", false, 18.0) {
            self.run(Action::ZoomOut);
        }
        if button(Rect::new(right - 26.0, y, 26.0, 20.0), "+", false, 18.0) {
            self.run(Action::ZoomIn);
        }
        y + 26.0
    }

    /// Where a palette goes: from `y` down to above its tip line, leaving room for the bars.
    fn palette_area(&self, y: f32) -> Rect {
        Rect::new(10.0, y, self.panel - MARGINS, (screen_height() - y - 30.0).max(0.0))
    }

    fn tip(text: &str, area: Rect) {
        put_text(&fit(text, area.w, 15.0), 10.0, area.y + area.h + 22.0, 15.0, INFO);
    }

    /// The palette's title: what is chosen (a piece by its name), and its size.
    fn brush_title(&self, what: &str) -> String {
        let Some(b) = self.brush() else { return format!("{what}  ({})", tr(self.lang, "none chosen")) };
        let size = if b.stamp.is_single() { String::new() } else { format!("  {}x{}", b.stamp.w, b.stamp.h) };
        match b.piece {
            Some(name) => format!("{what}  {}{size}", tr(self.lang, name)),
            None => format!("{what} #{:02X}{size}", b.first()),
        }
    }

    /// The map tile palette of the map shown (`tile_palette`): a zone's picture (a new zone's
    /// base zone's), or the world map's pieces, both as the game has them, then the tiles left
    /// out.
    pub(super) fn tile_cells(&self) -> TileCells {
        match self.world_map {
            OnMap::World => TileCells::world(&world::read(&self.doc.original)),
            OnMap::Zone(z) => TileCells::zone(&zone_maps::read(&self.doc.original, z), zone_maps::W),
            OnMap::NewZone(k) => TileCells::zone(&zone_maps::read(&self.doc.original, self.doc.new_zones[k].base), zone_maps::W),
        }
    }

    /// Object types 00-7F; the ones this level uses are dark.
    /// The object palette: its header (title and catalogue, the chosen object, the category
    /// filter and Show all, the zoom), then the types.
    pub(super) fn object_palette(&mut self, y: f32) {
        let lang = self.lang;
        let right = self.panel - 10.0;
        let width = right - 10.0;
        // title and the catalogue
        put_text(tr(lang, "OBJECTS"), 10.0, y + 15.0, 18.0, col(3));
        let cat_label = format!("{} ({})", tr(lang, "Catalogue"), shortcuts::keys(Action::ObjectCatalogue));
        let cw = text_size(&cat_label, 14).width + 16.0;
        if button(Rect::new(right - cw, y, cw, 20.0), &cat_label, false, 14.0) {
            self.run(Action::ObjectCatalogue);
        }
        let y = y + 26.0;
        // the object chosen
        let chosen = match self.kind {
            Some(k) => match self.object_label(self.look_key(k)) {
                Some((n, c)) => format!("#{k:02X}  {n}  ({})", super::catalogue_view::category_name(lang, c)),
                None => format!("#{k:02X}"),
            },
            None => tr(lang, "No object chosen: pick one below").to_string(),
        };
        // an enemy brought from the catalogue can be taken out from here
        let imported = self.kind.and_then(|k| self.imported_enemy(k));
        let remove_label = tr(lang, "Remove");
        let rw = if imported.is_some() { text_size(remove_label, 14).width + 16.0 } else { 0.0 };
        let bw = width - if rw > 0.0 { rw + 6.0 } else { 0.0 };
        draw_rectangle(10.0, y, bw, 22.0, col(0));
        draw_rectangle_lines(10.0, y, bw, 22.0, 1.0, col(2));
        put_text(&super::level_panel::fit(&chosen, bw - 12.0, 15.0), 16.0, y + 16.0, 15.0, if self.kind.is_some() { col(3) } else { col(2) });
        if let Some(e) = imported {
            if button(Rect::new(right - rw, y, rw, 22.0), remove_label, false, 14.0) {
                self.remove_enemy(e);
            }
        }
        let y = y + 28.0;
        // the category filter and Show all
        let all_label = tr(lang, "Show all");
        let aw = text_size(all_label, 14).width + 16.0;
        let fw = width - aw - 6.0;
        let filters: Vec<Option<Category>> = std::iter::once(None).chain(CATEGORIES.iter().copied().map(Some)).collect();
        let at = filters.iter().position(|f| *f == self.palette_filter).unwrap_or(0);
        let mut step = 0i32;
        if button(Rect::new(10.0, y, 22.0, 22.0), "<", false, 15.0) {
            step = -1;
        }
        if button(Rect::new(10.0 + fw - 22.0, y, 22.0, 22.0), ">", false, 15.0) {
            step = 1;
        }
        let label = super::catalogue_view::category_plural(lang, self.palette_filter);
        let lw = text_size(label, 15).width;
        put_text(label, 10.0 + (fw - lw) / 2.0, y + 16.0, 15.0, col(3));
        if step != 0 {
            self.palette_filter = filters[(at as i32 + step).rem_euclid(filters.len() as i32) as usize];
        }
        if button(Rect::new(right - aw, y, aw, 22.0), all_label, self.show_all_objects, 14.0) {
            self.show_all_objects = !self.show_all_objects;
        }
        let y = self.search_field(y + 28.0, width);
        let y = self.zoom_row(y);
        let y = self.recent_row(y, true);
        let area = self.palette_area(y);
        let zoom = self.prefs.level_palette_zoom;
        let cell = Layout::BLOCKS.cell(zoom);
        let used: Vec<u8> = self.doc.levels[self.cur].content.objs.iter().map(|o| o.kind).collect();
        let offered = self.offered_here();
        let mut shown: Vec<u8> = if self.show_all_objects { (0..128).collect() } else { offered.clone() };
        if let Some(f) = self.palette_filter {
            shown.retain(|&k| self.object_label(self.look_key(k)).is_some_and(|l| l.1 == f));
        }
        shown.retain(|&k| self.search_matches(k));
        let imported: Vec<u8> = shown.iter().copied().filter(|&k| self.imported_enemy(k).is_some()).collect();
        let layout = Layout { count: shown.len(), ..Layout::BLOCKS };
        let mut view = self.level_palette;
        let chosen = self.kind.and_then(|k| shown.iter().position(|&s| s == k)).map(|i| ((i % 8) as i32, (i / 8) as i32)).map(|c| Area::between(c, c));
        let hover = cell_grid(area, layout, cell, &mut view, chosen, |i, r| {
            let k = shown[i];
            let (bg, fg) = if Some(k) == self.kind { (col(3), col(0)) } else if used.contains(&k) { (col(2), col(0)) } else { (col(1), col(3)) };
            draw_rectangle(r.x, r.y, r.w, r.h, bg);
            if !offered.contains(&k) {
                draw_rectangle_lines(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0, 2.0, warn_col());
            }
            if imported.contains(&k) {
                // from another level through the catalogue: a corner in the marks' colour
                draw_triangle(vec2(r.x + r.w, r.y), vec2(r.x + r.w - 10.0, r.y), vec2(r.x + r.w, r.y + 10.0), MARK);
            }
            let fs = (6.0 + 5.0 * zoom as f32).min(18.0);
            let hidden = self.look(k).is_none().then(|| self.hidden_look(self.look_key(k))).flatten();
            if let Some((tex, _, _)) = self.look(k).or(hidden) {
                let f = ((r.w - 4.0) / tex.width()).min((r.h - 4.0) / tex.height()).min(zoom as f32);
                let (tw, th) = (tex.width() * f, tex.height() * f);
                draw_texture_ex(tex, r.x + (r.w - tw) / 2.0, r.y + (r.h - th) / 2.0, WHITE, DrawTextureParams { dest_size: Some(vec2(tw, th)), ..Default::default() });
                put_text(&format!("{k:02X}"), r.x + 1.0, r.y + fs * 0.7, fs * 0.7, fg);
                if hidden.is_some() {
                    super::objects_view::hidden_mark(r);
                }
            } else {
                let t = format!("{k:02X}");
                let dim = text_size(&t, fs as u16);
                let known = self.looks.contains_key(&self.look_key(k));
                put_text(&t, r.x + (r.w - dim.width) / 2.0, r.y + (r.h + dim.offset_y) / 2.0, fs, if known { fg } else { Color::new(fg.r, fg.g, fg.b, 0.5) });
            }
        });
        self.level_palette = view;
        if let (true, Some(&k)) = (is_mouse_button_pressed(MouseButton::Left), hover.and_then(|(c, r)| shown.get((r * 8 + c) as usize))) {
            self.choose_kind(k, is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift));
        }
        // a right click on an enemy brought from the catalogue takes it, and its objects, out
        if let (true, Some(e)) = (is_mouse_button_pressed(MouseButton::Right), hover.and_then(|(c, r)| shown.get((r * 8 + c) as usize)).and_then(|&k| self.imported_enemy(k))) {
            self.remove_enemy(e);
        }
        let base = self.doc.base_of(self.cur);
        let named = hover.and_then(|(c, r)| shown.get((r * 8 + c) as usize)).copied().map(|k| {
            let name = match self.object_label(self.look_key(k)) {
                Some((n, cat)) => format!("#{k:02X} {n} ({})", super::catalogue_view::category_name(self.lang, cat)),
                None => format!("#{k:02X}"),
            };
            let name = match self.hidden_note(self.look_key(k)) {
                Some(note) => format!("{name}. {note}"),
                None => name,
            };
            match self.imported_enemy(k) {
                Some(e) => {
                    let n = self.doc.levels[self.cur].content.objs.iter().filter(|o| o.kind == k).count();
                    format!("{name}, {} {:02}. {} {n} {}", tr(self.lang, "from level"), e.level, tr(self.lang, "Right click: remove it and its"), tr(self.lang, "objects"))
                }
                None if self.sel_obj.is_some() => format!("{name}. {}", tr(self.lang, "Shift+click: the selected object becomes it")),
                None => name,
            }
        });
        let chosen_note = self.kind.and_then(|k| self.hidden_note(self.look_key(k)));
        let tip = if let Some(t) = named {
            t
        } else if let Some(note) = chosen_note {
            note
        } else if self.pictures_pending(base) {
            tr(self.lang, "Still capturing pictures: more types will be offered.").to_string()
        } else if self.show_all_objects {
            tr(self.lang, "Framed: may look broken in this level.").to_string()
        } else {
            format!("{} {}", tr(self.lang, "Dark: used here. Delete chosen:"), shortcuts::keys(Action::DeleteObject))
        };
        Self::tip(&tip, area);
    }

    /// The object types game level `base` draws right (`level_graphics`): the bells, those the
    /// game's levels with its sprites and catalogue use, and, once its picture there is
    /// captured, a type a game level of its catalogue uses that loads the same graphics into
    /// the sprite sets the picture uses or draws it with the same pixels.
    pub(super) fn offered_types(&mut self, base: usize) -> Vec<u8> {
        if !self.offered.contains_key(&base) {
            let users = (0..128u8).map(|k| (k, level_graphics::levels_using(&self.doc.original, base, k))).filter(|(_, l)| !l.is_empty()).collect();
            self.offered.insert(base, users);
        }
        let group = level_graphics::same_objects(&self.doc.original, base);
        let users = &self.offered[&base];
        let original = &self.doc.original;
        (0..128u8)
            .filter(|&k| {
                if k == cart_model::level_goals::BELL || k == checkpoint::BELL {
                    return true;
                }
                let Some((_, levels)) = users.iter().find(|(u, _)| *u == k) else { return false };
                let Some(here) = self.captured.get(&(base, k)) else { return levels.iter().any(|l| group.contains(l)) };
                levels.iter().any(|&l| group.contains(&l) || level_graphics::same_sets(original, base, l, here.sets) || self.captured.get(&(l, k)).is_some_and(|there| (there.w, there.h, &there.px) == (here.w, here.h, &here.px)))
            })
            .collect()
    }

    /// The object types the shown level draws right: its game level's (`offered_types`), and
    /// with its own enemy set, the set's enemies instead of the game's.
    pub(super) fn offered_here(&mut self) -> Vec<u8> {
        let base = self.doc.base_of(self.cur);
        let mut v = self.offered_types(base);
        if let Some(set) = self.doc.level_enemies(self.cur) {
            let members: Vec<u8> = set.enemies.iter().map(|e| e.kind).collect();
            v.retain(|k| members.contains(k) || self.object_tiles.get(&(base, *k)).is_none_or(|t| cart_model::enemy_sets::area_tiles(t).is_empty()));
            for k in members {
                if !v.contains(&k) {
                    v.push(k);
                }
            }
            v.sort_unstable();
        }
        v
    }

    /// The level's 128 blocks as the game draws them, 8 per row.
    pub(super) fn block_palette(&mut self, y: f32) {
        let title = self.brush_title(tr(self.lang, "BLOCKS"));
        let y = self.palette_title(y, &title);
        let y = self.recent_row(y, false);
        let area = self.palette_area(y);
        let Some(atlas) = self.atlases[self.doc.base_of(self.cur)].clone() else { return };
        let cells = TileCells::blocks();
        let cell = Layout::BLOCKS.cell(self.prefs.level_palette_zoom);
        let mut view = self.level_palette;
        let hover = self.grouped_grid(area, Layout::BLOCKS, cell, &mut view, &cells, &atlas, 16.0, self.blocks.as_ref().and_then(|b| b.in_palette));
        self.level_palette = view;
        self.tile_select(hover, &cells);
        Self::tip(tr(self.lang, "Drag: several. Right drag: move it"), area);
    }

    /// A grouped palette's cells drawn from `atlas` (`px` source pixels each), a rule over the
    /// rest; the cell under the mouse.
    #[allow(clippy::too_many_arguments)]
    fn grouped_grid(&self, area: Rect, layout: Layout, cell: f32, view: &mut PaletteView, cells: &TileCells, atlas: &Texture2D, px: f32, outline: Option<Area>) -> Option<(i32, i32)> {
        let rule = cells.rest_row * cells.cols;
        cell_grid(area, layout, cell, view, outline, |i, r| {
            if let Some(t) = cells.cells[i] {
                // map tiles a pixel wider, so no seam shows between them
                let seam = if px < 16.0 { 1.0 } else { 0.0 };
                draw_texture_ex(atlas, r.x, r.y, WHITE, DrawTextureParams { source: Some(source(t as usize, px)), dest_size: Some(vec2(r.w + seam, r.h + seam)), ..Default::default() });
            }
            if rule > 0 && (rule..rule + cells.cols).contains(&i) {
                draw_rectangle(r.x, r.y - 1.0 - cell / 2.0, r.w + layout.gap, 2.0, col(3));
            }
        })
    }

    /// The shown map's tiles as the game draws them (`tile_cells`): a zone's picture or the
    /// world map's pieces, a rule, then the tiles they leave out. A drag stays in the piece it
    /// starts in, so dragging past one takes all of it.
    pub(super) fn tile_palette(&mut self, y: f32) {
        let title = self.brush_title("TILES");
        let y = self.palette_title(y, &title);
        let y = self.recent_row(y, false);
        let area = self.palette_area(y);
        let Some(atlas) = self.map_atlas().cloned() else { return };
        let cells = self.tile_cells();
        let layout = Layout { count: cells.cells.len(), ..Layout::MAP_TILES };
        let cell = layout.cell(self.prefs.map_palette_zoom);
        let mut view = self.map_palette;
        let hover = self.grouped_grid(area, layout, cell, &mut view, &cells, &atlas, 8.0, self.tiles.as_ref().and_then(|b| b.in_palette));
        self.map_palette = view;
        if let Some(name) = hover.and_then(|c| cells.group(c)).map(|g| cells.names[g]).filter(|n| !n.is_empty()) {
            self.tip = Some(tr(self.lang, name).to_string());
        }
        self.tile_select(hover, &cells);
        let tip = match self.world_map {
            OnMap::World => tr(self.lang, "Pieces of the world map, then unused tiles. Drag: several"),
            _ => tr(self.lang, "Zone picture, then unused tiles. Drag: several"),
        };
        Self::tip(tip, area);
    }

    /// Mouse on the map tile palette: a click or a drag chooses tiles inside the piece (or
    /// picture, or tiles left out) it starts in.
    fn tile_select(&mut self, hover: Option<(i32, i32)>, cells: &TileCells) {
        if let (true, Some(c)) = (is_mouse_button_pressed(MouseButton::Left), hover) {
            if cells.group(c).is_none() {
                return;
            }
            self.palette_drag = Some(c);
            if matches!(self.tool.kind(), ToolKind::Select | ToolKind::Paste) {
                self.leave_grid();
                self.set_tool(ToolKind::Pencil);
            }
        }
        let Some(from) = self.palette_drag else { return };
        if let Some((a, b, tiles)) = hover.and_then(|to| cells.select(from, to)) {
            let area = Area::between(a, b);
            let piece = cells.group(from).map(|g| cells.names[g]).filter(|n| !n.is_empty());
            *self.brush_mut() = Some(Brush { stamp: Stamp { w: area.w(), h: area.h(), cells: tiles }, in_palette: Some(area), piece });
        }
        if !is_mouse_button_down(MouseButton::Left) {
            self.palette_drag = None;
            self.remember_brush();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_at_the_default_zoom_fit_the_narrowest_panel() {
        assert!(Layout::BLOCKS.width(2) + MARGINS <= PANEL_MIN);
        assert!(Layout::MAP_TILES.width(1) + MARGINS <= PANEL_MIN);
        assert_eq!(Layout::MAP_TILES.width(4), 20.0 * 33.0);
    }
}
