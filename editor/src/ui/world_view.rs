//! The world map editor: the world map, the zone maps and the new zones, each picture painted
//! tile by tile (Picture tab), and the points drawn where the game shows them with the level
//! each one opens and the paths Mario walks between them (Points tab): levels, paths, doors,
//! exits and new points can be changed there. New zones are made from a zone and deleted here.
use super::shortcuts::Action;
use super::level_panel::{fit, stepper, GAP, LABEL_W, PAD, ROW, SCROLL_STEP};
use super::palette::clip;
use super::widgets::*;
use super::names_view::Named;
use crate::domain::prefs::MUSIC_VOLUMES;
use cart_model::map_music::{self, Map, Named as MusicNamed};
use super::{Editor, TAB_PICTURE, TAB_POINTS};
use crate::domain::document::{EditorDocument, ZoneRefusal};
use crate::domain::tools::ToolKind;
use cart_model::added_points;
use cart_model::doors::{self, Leads};
use cart_model::map_routes::{self, Way, DIRS};
use cart_model::original_notes::{self, Note};
use cart_model::original_points::{self, OnMap};
use cart_model::start_point::Start;
use cart_model::world;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_pressed, is_mouse_button_down, mouse_wheel, is_key_pressed};

/// Mario's place on a zone map is this far below where the game draws its point.
const FEET: i32 = 4;
/// A map's button in the map picker.
const MAP_BUTTON_H: f32 = 23.0;
/// A point's button in the list of the map's points (narrowest, height).
const POINT_BUTTON: (f32, f32) = (64.0, 22.0);

impl Editor {
    /// The maps in the order the map selector steps through them.
    pub(super) fn all_maps(&self) -> Vec<OnMap> {
        std::iter::once(OnMap::World).chain((0..ZONES.len()).map(OnMap::Zone)).chain((0..self.doc.new_zones.len()).map(OnMap::NewZone)).collect()
    }

    /// The entry point of the game zone shown, if a door enters it.
    fn game_entry_shown(&self) -> Option<u8> {
        match self.world_map {
            OnMap::Zone(z) => self.doc.game_entry(z),
            _ => None,
        }
    }

    /// Can the chosen point be the entry of the game zone shown (one of its own points)?
    fn can_be_game_entry(&self) -> bool {
        match self.world_map {
            OnMap::Zone(z) => self.game_entry_shown().is_some() && cart_model::new_zones::base_of(z).is_some_and(|b| b.has(self.point as u8)) && self.chosen_at().is_some(),
            _ => false,
        }
    }

    /// The new zone shown, if one is.
    fn new_zone_shown(&self) -> Option<usize> {
        match self.world_map {
            OnMap::NewZone(k) if k < self.doc.new_zones.len() => Some(k),
            _ => None,
        }
    }

    /// Size of the shown map picture in pixels.
    fn map_px(&self) -> Vec2 {
        let (w, h) = EditorDocument::map_size(self.world_map);
        vec2(w as f32 * 8.0, h as f32 * 8.0)
    }

    pub(super) fn map_name(&self, map: OnMap) -> String {
        let zone = |z: usize| zone_name(self.lang, z);
        match map {
            OnMap::World => tr(self.lang, "World map").to_string(),
            OnMap::Zone(z) => zone(z).to_string(),
            OnMap::NewZone(k) => match self.doc.new_zones.get(k) {
                Some(n) if !n.name.is_empty() => format!("{} ({})", n.name, zone(n.base)),
                Some(n) => format!("{} {} ({})", tr(self.lang, "New zone"), k + 1, zone(n.base)),
                None => tr(self.lang, "New zone").to_string(),
            },
        }
    }

    /// Screen position of a pixel of the shown map picture.
    pub(super) fn on_screen(&self, x: i32, y: i32) -> Vec2 {
        vec2(self.panel + (x as f32 - self.cam.x) * self.scale(), (y as f32 - self.cam.y) * self.scale())
    }

    /// The points drawn on the shown map: the game's own (where it draws them), the new ones
    /// of the world map or the game zone, on a new zone its own. (point, x, y, new)
    pub(super) fn map_points(&self) -> Vec<(u8, i32, i32, bool)> {
        if let Some(k) = self.new_zone_shown() {
            return self.doc.new_zones[k].points.iter().map(|p| (p.number, p.place.x as i32, p.place.y as i32 - FEET, true)).collect();
        }
        let mut v: Vec<(u8, i32, i32, bool)> = original_points::on(self.world_map).map(|s| self.spot_at(s)).collect();
        let added_here = self.doc.added_on() == self.world_map;
        for &p in &self.doc.routes.added {
            let pl = self.doc.routes.places[p as usize];
            match self.world_map {
                OnMap::World if added_here || added_points::index(p).is_none() => v.push((p, pl.on_world().0, pl.on_world().1, true)),
                OnMap::Zone(_) if added_here && added_points::index(p).is_some() => v.push((p, pl.x as i32, pl.y as i32 - FEET, true)),
                _ => {}
            }
        }
        v
    }

    /// Can new points be put on the shown map?
    fn takes_new_points(&self) -> bool {
        match self.world_map {
            OnMap::World | OnMap::NewZone(_) => true,
            OnMap::Zone(z) => z < added_points::ZONES,
        }
    }

    /// Where a game's point is drawn: where the game draws it, moved as far as its place was.
    fn spot_at(&self, s: &original_points::Spot) -> (u8, i32, i32, bool) {
        let (dx, dy) = self.doc.point_moved(self.world_map, s.point);
        (s.point, s.x + dx, s.y + dy, false)
    }

    /// How far a dragged point of the world map or a game zone has moved, as it will be placed.
    fn drag_offset(&self) -> Option<(i32, i32)> {
        let (_, from) = self.drag_from?;
        let d = self.world_pos(mouse_position().into()) - from;
        let snap = |v: f32| (v / 4.0).round() as i32 * 4;
        Some((snap(d.x), snap(d.y)))
    }

    /// The level point `p` of the shown map opens (`world::NO_LEVEL`: none).
    pub(super) fn level_of(&self, p: usize) -> u8 {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].point(p as u8).map_or(world::NO_LEVEL, |q| q.level),
            None => self.doc.points.get(p).copied().unwrap_or(world::NO_LEVEL),
        }
    }

    /// The ways out of point `p` of the shown map.
    fn ways_of(&self, p: usize) -> Option<&[Option<Way>; 4]> {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].point(p as u8).map(|q| &q.ways),
            None => self.doc.routes.ways.get(p),
        }
    }

    /// The point drawn under the mouse on the shown map.
    pub(super) fn point_at(&self, m: Vec2) -> Option<(u8, i32, i32, bool)> {
        let w = self.world_pos(m);
        self.map_points().into_iter().find(|s| (vec2(s.1 as f32, s.2 as f32) - w).length() <= 7.0)
    }

    /// Where the chosen point is drawn on the shown map, if it is.
    fn chosen_at(&self) -> Option<(i32, i32)> {
        self.map_points().into_iter().find(|s| s.0 as usize == self.point).map(|s| (s.1, s.2))
    }

    /// Is `p` a point of the world map (drawn there, the twin of a zone's entrance, or new)?
    pub(super) fn is_world_point(&self, p: u8) -> bool {
        (self.doc.routes.added.contains(&p) && (added_points::index(p).is_none() || self.doc.added_on() == OnMap::World)) || map_routes::ZONE_EXITS.iter().any(|z| z.1 == p) || original_points::SPOTS.iter().any(|s| s.point == p && s.map == OnMap::World)
    }

    pub(super) fn leads_name(&self, l: Leads) -> String {
        match l {
            Leads::World(p) => format!("{p:02X} ({})", tr(self.lang, "world")),
            Leads::Zone(i) => self.map_name(OnMap::Zone(doors::INTO[i].zone)),
            Leads::NewZone(k) => self.map_name(OnMap::NewZone(k)),
        }
    }

    /// The exit (index, where it leads) on point `p` of the shown map.
    pub(super) fn exit_on(&self, p: u8) -> Option<(usize, Leads)> {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].exits.iter().position(|e| e.0 == p).map(|i| (i, self.doc.new_zones[k].exits[i].1)),
            None => self.doc.doors.exit_on(p).map(|i| (i, self.doc.doors.exits[i].1)),
        }
    }

    /// Where world point `p`'s door leads, if anywhere.
    pub(super) fn door_of(&self, p: u8) -> Option<Leads> {
        self.doc.doors.door_of(p).map(Leads::Zone).or(self.doc.zone_door_of(p).map(Leads::NewZone))
    }

    /// The label of a point on the map: its number and, for a door or an exit, where it goes.
    fn point_tag(&self, p: u8) -> Option<String> {
        if self.new_zone_shown().is_none() {
            if self.world_map != OnMap::World && self.is_world_point(p) {
                // a world point drawn where Mario walks out of a zone
                return Some(format!("{p:02X} ({})", tr(self.lang, "world")));
            }
            if let Some(to) = self.door_of(p) {
                return Some(format!("{p:02X} > {}", self.leads_name(to)));
            }
        }
        let (_, leads) = self.exit_on(p)?;
        Some(format!("{p:02X} > {}", self.leads_name(leads)))
    }

    /// The exits of the shown zone that can be moved to the chosen point.
    fn exit_sites(&self) -> Vec<usize> {
        let p = self.point as u8;
        if self.doc.doors.into.is_empty() || self.exit_on(p).is_some() || self.chosen_at().is_none() {
            return Vec::new();
        }
        if let Some(k) = self.new_zone_shown() {
            return (0..self.doc.new_zones[k].exits.len()).collect();
        }
        let OnMap::Zone(z) = self.world_map else { return Vec::new() };
        if self.is_world_point(p) {
            return Vec::new();
        }
        (0..doors::EXITS.len()).filter(|&i| original_points::SPOTS.iter().any(|s| s.point == doors::EXITS[i].point && s.map == OnMap::Zone(z))).collect()
    }

    /// Where exit `i` of the shown zone is and leads.
    fn exit_at(&self, i: usize) -> (u8, Leads) {
        match self.new_zone_shown() {
            Some(k) => self.doc.new_zones[k].exits[i],
            None => self.doc.doors.exits[i],
        }
    }

    pub(super) fn set_exit(&mut self, i: usize, at: u8, leads: Leads) {
        match self.new_zone_shown() {
            Some(k) => self.doc.zone_set_exit(k, i, at, leads),
            None => self.doc.set_exit(i, at, leads),
        }
    }

    /// Move the `k`th exit of the shown zone to the chosen point.
    fn make_exit(&mut self, k: usize) {
        if let Some(&i) = self.exit_sites().get(k) {
            let leads = self.exit_at(i).1;
            self.set_exit(i, self.point as u8, leads);
        }
    }

    /// A label, then `-` value `+` over the rest of the panel (the keys in the buttons' tooltips).
    /// Returns the action clicked.
    fn stepper(&mut self, y: f32, name: &str, value: &str, (minus, minus_tip): (Action, &str), (plus, plus_tip): (Action, &str)) -> Option<Action> {
        let (cx, cw) = (PAD + LABEL_W, self.inner_w() - LABEL_W);
        put_text(name, PAD, y + 18.0, 15.0, col(3));
        let mut act = None;
        if self.tip_button(Rect::new(cx, y, ROW, ROW), "-", minus_tip, minus, false) {
            act = Some(minus);
        }
        let vw = cw - 2.0 * (ROW + GAP);
        let t = fit(value, vw - 4.0, 15.0);
        let d = text_size(&t, 15);
        put_text(&t, (cx + ROW + GAP + (vw - d.width) / 2.0).round(), y + 18.0, 15.0, col(3));
        if self.tip_button(Rect::new(cx + cw - ROW, y, ROW, ROW), "+", plus_tip, plus, false) {
            act = Some(plus);
        }
        act
    }

    /// Buttons sharing a row of the panel evenly. Returns the action clicked.
    fn button_row(&mut self, y: f32, items: &[(&str, Action, bool)]) -> Option<Action> {
        let n = items.len() as f32;
        let bw = (self.inner_w() - (n - 1.0) * GAP) / n;
        let mut act = None;
        for (i, &(name, a, on)) in items.iter().enumerate() {
            if self.tool_button(Rect::new(PAD + i as f32 * (bw + GAP), y, bw, ROW), name, a, on) {
                act = Some(a);
            }
        }
        act
    }

    /// A section's title; returns where the section starts.
    fn section(&self, y: f32, text: &str, right: &str) -> f32 {
        self.section_title(y + 6.0, text, right);
        y + 6.0 + 18.0 + GAP
    }

    /// The doors part of the points panel: where the chosen point leads.
    fn door_rows(&mut self, mut y: f32) -> (f32, Option<Action>) {
        if self.doc.doors.into.is_empty() {
            return (y, None);
        }
        let lang = self.lang;
        let (cx, cw) = (PAD + LABEL_W, self.inner_w() - LABEL_W);
        let place = 70.0;
        let p = self.point as u8;
        let mut act = None;
        let target = match self.exit_on(p) {
            Some((_, leads)) => Some((tr(lang, "Exit to"), self.leads_name(leads))),
            None if self.world_map == OnMap::World && self.is_world_point(p) => Some((
                tr(lang, "Door"),
                match self.door_of(p) {
                    Some(to) => format!("{} {}", tr(lang, "into"), self.leads_name(to)),
                    None => tr(lang, "none").to_string(),
                },
            )),
            None => None,
        };
        if let Some((name, value)) = target {
            put_text(name, PAD, y + 18.0, 15.0, col(3));
            if self.tip_button(Rect::new(cx, y, place, ROW), tr(lang, "Choose"), tr(lang, "Choose on the map"), Action::ChooseTarget, false) {
                act = Some(Action::ChooseTarget);
            }
            put_text(&fit(&value, cw - place - 8.0, 14.0), cx + place + 8.0, y + 18.0, 14.0, INFO);
            y += ROW + GAP;
        } else {
            for (k, i) in self.exit_sites().into_iter().enumerate().take(2) {
                let at = self.exit_at(i).0;
                let a = if k == 0 { Action::MakeExit1 } else { Action::MakeExit2 };
                put_text(&format!("{} {}", tr(lang, "Exit"), k + 1), PAD, y + 18.0, 15.0, col(3));
                if self.tip_button(Rect::new(cx, y, place, ROW), tr(lang, "Here"), tr(lang, "Make it the exit: move it to this point"), a, false) {
                    act = Some(a);
                }
                let now = if at == doors::NONE { tr(lang, "on no point").to_string() } else { format!("{} {at:02X}", tr(lang, "now on")) };
                put_text(&fit(&now, cw - place - 8.0, 14.0), cx + place + 8.0, y + 18.0, 14.0, INFO);
                y += ROW + GAP;
            }
        }
        (y, act)
    }

    /// Show the map `step` places further in the map selector.
    pub(super) fn step_map(&mut self, step: i32) {
        let maps = if self.picker.is_some() { self.picking_maps() } else { self.all_maps() };
        let i = maps.iter().position(|&m| m == self.world_map).unwrap_or(0) as i32;
        self.show_map(maps[(i + step).rem_euclid(maps.len() as i32) as usize]);
        self.choose_on_new_zone();
    }

    /// On a new zone, choose its entry point if the chosen point is not one of its own.
    fn choose_on_new_zone(&mut self) {
        if let Some(k) = self.new_zone_shown() {
            let z = &self.doc.new_zones[k];
            if z.point(self.point as u8).is_none() {
                self.point = z.entry as usize;
            }
        }
    }

    /// A new zone made from the map shown, then shown.
    pub fn new_zone(&mut self) {
        let from = self.world_map;
        let start = self.doc.map_start(from);
        let made = self.doc.add_zone(from);
        let refused = made.is_err();
        self.status = match made {
            Ok(k) => {
                self.clipboard = None;
                self.show_map(OnMap::NewZone(k));
                self.choose_on_new_zone();
                self.offer_start(OnMap::NewZone(k), start);
                format!("{} {}", self.map_name(OnMap::NewZone(k)), tr(self.lang, "made; it has no door yet (Points tab)."))
            }
            Err(ZoneRefusal::TooManyZones(max)) => format!("{} {max}.", tr(self.lang, "No more new zones: there can be")),
            Err(_) => tr(self.lang, "New zones are made from the Tree, Pumpkin, Macro, Space or Mario zone, or from a new zone.").into(),
        };
        if refused {
            self.mark(Severity::Warning);
        }
    }

    /// Show Mario's start: its map, the Points tab, the point chosen.
    pub fn go_to_start(&mut self) {
        let Some(map) = self.doc.start_map() else {
            self.status = tr(self.lang, "Mario's start is missing: choose a point and press Mario's start (I) in the Points tab.").into();
            self.mark(Severity::Warning);
            return;
        };
        self.cancel_picking();
        self.show_map(map);
        self.world_tab = TAB_POINTS;
        self.point = self.doc.start.point as usize;
        // the point in the middle of the view, with room above for its label
        if let Some(&(_, x, y, _)) = self.map_points().iter().find(|s| s.0 == self.doc.start.point) {
            let view = vec2(screen_width() - self.panel, screen_height() - STATUS_H) / self.scale();
            let at = vec2(x as f32 - view.x / 2.0, y as f32 - view.y / 2.0);
            self.cam = at.clamp(Vec2::ZERO, (self.map_px() - view).max(Vec2::ZERO)).floor();
        }
        self.status = format!("{} {} {:02X}.", tr(self.lang, "Mario's start:"), self.map_name(map), self.doc.start.point);
    }

    /// A map's short name for the map list.
    fn map_short(&self, map: OnMap) -> String {
        match map {
            OnMap::World => tr(self.lang, "World").to_string(),
            OnMap::Zone(z) => zone_short(self.lang, z).to_string(),
            OnMap::NewZone(k) => match self.doc.new_zones.get(k) {
                Some(n) if !n.name.is_empty() => n.name.clone(),
                _ => format!("{} {}", tr(self.lang, "New zone"), k + 1),
            },
        }
    }

    /// Where Mario's start is on the shown map: a ring and a label that stand out, in both tabs.
    fn draw_start_mark(&self) {
        let Some(p) = self.doc.map_start(self.world_map) else { return };
        let Some(&(_, x, y, _)) = self.map_points().iter().find(|s| s.0 == p) else { return };
        let c = self.on_screen(x, y);
        let r = (4.0 * self.scale()).max(6.0) + 9.0;
        let red = Color::new(0.9, 0.1, 0.1, 1.0);
        draw_circle_lines(c.x, c.y, r + 1.5, 6.0, WHITE);
        draw_circle_lines(c.x, c.y, r, 4.0, red);
        let t = tr(self.lang, "MARIO STARTS HERE");
        let d = text_size(t, 16);
        let (bw, bh) = (d.width + 14.0, 22.0);
        let (bx, by) = (c.x - bw / 2.0, c.y - r - bh - 10.0);
        draw_triangle(vec2(c.x - 7.0, by + bh), vec2(c.x + 7.0, by + bh), vec2(c.x, by + bh + 9.0), red);
        draw_rectangle(bx, by, bw, bh, red);
        draw_rectangle_lines(bx, by, bw, bh, 2.0, WHITE);
        put_text(t, bx + 7.0, by + 16.0, 16.0, WHITE);
    }

    /// Delete the new zone shown; its base is shown.
    fn delete_zone(&mut self) {
        let Some(k) = self.new_zone_shown() else {
            self.status = tr(self.lang, "Only new zones can be deleted; the game's are only edited.").into();
            self.mark(Severity::Warning);
            return;
        };
        let name = self.map_name(OnMap::NewZone(k));
        let base = self.doc.new_zones[k].base;
        self.doc.remove_zone(k);
        self.clipboard = None;
        self.show_map(OnMap::Zone(base));
        self.status = format!("{name} {}", tr(self.lang, "deleted; doors and exits into it lead nowhere."));
    }

    /// The Points tab's actions.
    pub(super) fn run_points(&mut self, a: Action) {
        let n = world::POINT_COUNT;
        let zone = self.new_zone_shown();
        if self.picker.is_some() {
            return;
        }
        match a {
            Action::NewZone => self.new_zone(),
            Action::BlankZone => self.open_blank_zone_chooser(),
            Action::GoToStart => self.go_to_start(),
            Action::GameStart if self.doc.start != Start::default() => {
                self.doc.set_start(Start::default());
                self.go_to_start();
                self.status = format!("{} {} 00.", tr(self.lang, "Mario's start: every new game begins on"), self.map_name(OnMap::World));
                self.mark(Severity::Done);
            }
            Action::DeleteZone => self.delete_zone(),
            Action::PrevPoint | Action::NextPoint if zone.is_some() => {
                let list: Vec<usize> = self.map_points().iter().map(|s| s.0 as usize).collect();
                if !list.is_empty() {
                    let i = list.iter().position(|&p| p == self.point).unwrap_or(0) as i32;
                    let step = if a == Action::NextPoint { 1 } else { -1 };
                    self.point = list[(i + step).rem_euclid(list.len() as i32) as usize];
                }
            }
            Action::PrevPoint => self.select_original_point((self.point + n - 1) % n),
            Action::NextPoint => self.select_original_point((self.point + 1) % n),
            Action::PrevOpens | Action::NextOpens => {
                let step = if a == Action::NextOpens { 1 } else { -1 };
                match zone {
                    Some(k) => self.doc.zone_step_level(k, self.point as u8, step),
                    None => self.doc.step_point_level(self.point, step),
                }
            }
            Action::ChooseTarget => self.start_picking(),
            Action::MakeExit1 => self.make_exit(0),
            Action::MakeExit2 => self.make_exit(1),
            Action::SetStart => match self.doc.start_at(self.world_map, self.point as u8) {
                Some(s) => {
                    self.doc.set_start(s);
                    self.status = format!("{} {} {:02X}.", tr(self.lang, "Mario's start: every new game begins on"), self.map_name(self.world_map), self.point);
                    self.mark(Severity::Done);
                }
                None => {
                    self.status = tr(self.lang, "Mario's start goes on a point of the world map, a game zone or a new zone (not an added point).").into();
                    self.mark(Severity::Warning);
                }
            },
            Action::MakeEntry => match (zone, self.world_map) {
                (Some(k), _) => self.doc.zone_set_entry(k, self.point as u8),
                (None, OnMap::Zone(z)) if self.can_be_game_entry() => self.doc.set_game_entry(z, self.point as u8),
                _ => {}
            },
            Action::DeletePathRight | Action::DeletePathLeft | Action::DeletePathUp | Action::DeletePathDown => {
                let d = [Action::DeletePathRight, Action::DeletePathLeft, Action::DeletePathUp, Action::DeletePathDown].iter().position(|&x| x == a).unwrap_or(0);
                match zone {
                    Some(k) => self.doc.zone_set_way(k, self.point as u8, d, None),
                    None if self.point < map_routes::ALL => self.doc.set_way(self.point, d, None),
                    None => {}
                }
            }
            Action::ShutRight | Action::ShutLeft | Action::ShutUp | Action::ShutDown => {
                let d = Self::SHUT_ACTIONS.iter().position(|&x| x == a).unwrap_or(0);
                if self.doc.can_shut(zone, self.point as u8) {
                    self.doc.toggle_shut(zone, self.point as u8, d);
                } else {
                    self.status = tr(self.lang, "Only new points can shut their paths until their level is cleared.").into();
                    self.mark(Severity::Warning);
                }
            }
            Action::NewPath => {
                let drawing = self.route_draw.is_some();
                let can_draw = self.point < map_routes::ALL && self.chosen_at().is_some();
                self.placing_point = false;
                self.route_draw = if drawing || !can_draw { None } else { Some(Vec::new()) };
                if !can_draw {
                    self.status = tr(self.lang, "Choose a point on the map shown first.").into();
                    self.mark(Severity::Warning);
                }
            }
            Action::NewPoint if zone.is_some_and(|k| self.doc.new_zones[k].points.len() >= self.doc.new_zones[k].room()) && !self.placing_point => {
                self.status = self.zone_full_note(zone.unwrap_or(0));
                self.mark(Severity::Warning);
            }
            Action::NewPoint if self.takes_new_points() => {
                self.route_draw = None;
                self.placing_point = !self.placing_point;
            }
            Action::DeletePoint => match zone {
                Some(k) => {
                    if !self.doc.zone_remove_point(k, self.point as u8) {
                        self.status = tr(self.lang, "A new zone keeps at least one point.").into();
                        self.mark(Severity::Warning);
                    }
                    self.choose_on_new_zone();
                }
                None if self.doc.routes.added.contains(&(self.point as u8)) => {
                    self.doc.remove_point(self.point as u8);
                }
                None => {}
            },
            _ => {}
        }
    }

    /// Choose a point and show its map (for the command line).
    pub fn choose_point(&mut self, p: usize) {
        self.select_original_point(p);
    }

    /// Choose a point; the map it is drawn on is shown.
    fn select_original_point(&mut self, p: usize) {
        self.point = p;
        self.route_draw = None;
        if self.doc.routes.added.contains(&(p as u8)) {
            self.show_map(if added_points::index(p as u8).is_some() { self.doc.added_on() } else { OnMap::World });
        } else if let Some(s) = original_points::SPOTS.iter().find(|s| s.point as usize == p) {
            self.show_map(s.map);
        }
    }

    /// The mouse on the picture, on the 4-pixel steps paths are made of, counted from `from`.
    fn snapped(&self, m: Vec2, from: (i32, i32)) -> (i32, i32) {
        let w = self.world_pos(m);
        let s = |v: f32, o: i32| o + ((v - o as f32) / 4.0).round() as i32 * 4;
        (s(w.x, from.0), s(w.y, from.1))
    }

    /// Why new zone `k` takes no more points.
    fn zone_full_note(&self, k: usize) -> String {
        let z = &self.doc.new_zones[k];
        format!(
            "{} {} {}",
            tr(self.lang, "This zone already has"),
            z.room(),
            tr(self.lang, "points, as many as a zone can have (the game picks the map by point number). Delete one (Del) to put it somewhere else."),
        )
    }

    /// Put a new point where the mouse is (the world map, or the new zone shown).
    fn place_point(&mut self, m: Vec2) {
        self.placing_point = false;
        if let Some(k) = self.new_zone_shown() {
            let w = self.world_pos(m);
            self.status = match self.doc.zone_add_point(k, w.x.round() as i32, w.y.round() as i32 + FEET) {
                Ok(p) => {
                    self.point = p as usize;
                    format!("{} {p:02X}", tr(self.lang, "Point added:"))
                }
                Err(ZoneRefusal::NoNumberLeft(_)) => self.zone_full_note(k),
                Err(_) => String::new(),
            };
            return;
        }
        // world points sit 2 pixels below the 4-pixel grid; a zone's places are Mario's feet
        let (x, y) = match self.world_map {
            OnMap::World => self.snapped(m, (0, 2)),
            _ => {
                let w = self.world_pos(m);
                (((w.x / 4.0).round() * 4.0) as i32, ((w.y / 4.0).round() * 4.0) as i32 + FEET)
            }
        };
        self.status = match self.doc.add_point(self.world_map, x, y) {
            Some(p) => {
                self.point = p as usize;
                format!("{} {p:02X}", tr(self.lang, "Point added:"))
            }
            None => tr(self.lang, "This map has as many points as it can have.").into(),
        };
    }

    /// Play from a point on or off: the Points tab, and the next click on a point starts the
    /// game there.
    pub(super) fn toggle_play_pick(&mut self) {
        if !self.play_pick && self.game_open() {
            return;
        }
        self.play_pick = !self.play_pick;
        if !self.play_pick {
            self.status.clear();
            return;
        }
        self.world_tab = TAB_POINTS;
        self.route_draw = None;
        self.placing_point = false;
        self.status = tr(self.lang, "Click the point to play from: the copy is saved and the game begins there. Esc: cancel.").into();
    }

    /// Clicks on the map in the Points tab: choose a point, put a new one, draw a path, or
    /// drag a new zone's point somewhere else; with Play from a point, play from the one
    /// clicked.
    fn points_clicks(&mut self, m: Vec2) {
        let clicked = is_mouse_button_pressed(MouseButton::Left) && m.x >= self.panel && m.y < screen_height() - STATUS_H;
        // keys typed into a name are not for the map
        let key = |k: KeyCode| self.naming.is_none() && is_key_pressed(k);
        if self.play_pick {
            if key(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                self.toggle_play_pick();
            } else if let Some(s) = self.point_at(m).filter(|_| clicked) {
                self.play_pick = false;
                self.point = s.0 as usize;
                self.play_map_here();
            }
            return;
        }
        if self.placing_point {
            if key(KeyCode::Escape) || is_mouse_button_pressed(MouseButton::Right) {
                self.placing_point = false;
            } else if clicked {
                self.place_point(m);
            }
            return;
        }
        let (escape, backspace) = (key(KeyCode::Escape), key(KeyCode::Backspace));
        if let Some(corners) = &mut self.route_draw {
            if escape || is_mouse_button_pressed(MouseButton::Right) {
                self.route_draw = None;
                return;
            }
            if backspace {
                corners.pop();
            }
            if !clicked {
                return;
            }
            let Some(start) = self.chosen_at() else {
                self.route_draw = None;
                return;
            };
            match self.point_at(m) {
                Some((p, x, y, _)) if p as usize != self.point && (p as usize) < map_routes::ALL => {
                    let mut rel: Vec<(i32, i32)> = self.route_draw.take().unwrap_or_default().iter().map(|c| (c.0 - start.0, c.1 - start.1)).collect();
                    rel.push((x - start.0, y - start.1));
                    let made = match self.new_zone_shown() {
                        Some(k) => self.doc.zone_draw_path(k, self.point as u8, p, &rel),
                        None => self.doc.draw_path(self.point, p, &rel),
                    };
                    self.status = match made {
                        Some((d, back)) => {
                            let back = back.map(|b| format!(", {} {p:02X} {} > {:02X}", tr(self.lang, "and"), DIRS[b], self.point)).unwrap_or_default();
                            format!("{} {:02X} {} > {p:02X}{back}", tr(self.lang, "Path"), self.point, DIRS[d])
                        }
                        None => tr(self.lang, "That path has no steps.").into(),
                    };
                }
                Some(_) => {}
                None => {
                    let c = self.snapped(m, start);
                    if let Some(corners) = &mut self.route_draw {
                        corners.push(c);
                    }
                }
            }
            return;
        }
        if let Some(p) = self.drag_point {
            if !is_mouse_button_down(MouseButton::Left) {
                let offset = self.drag_offset();
                self.drag_point = None;
                let from = self.drag_from.take();
                match (self.new_zone_shown(), from, offset) {
                    (Some(k), _, _) => {
                        let w = self.world_pos(m);
                        self.doc.zone_move_point(k, p, w.x.round() as i32, w.y.round() as i32 + FEET);
                    }
                    (None, Some((place, _)), Some((dx, dy))) if (dx, dy) != (0, 0) => {
                        self.doc.map_snapshot();
                        self.doc.move_point(self.world_map, p, place, dx, dy);
                    }
                    _ => {}
                }
            }
            return;
        }
        if clicked {
            if let Some(s) = self.point_at(m) {
                self.point = s.0 as usize;
                let movable = self.new_zone_shown().is_some() || !(self.world_map != OnMap::World && self.is_world_point(s.0));
                if movable && (s.0 as usize) < map_routes::ALL {
                    self.drag_point = Some(s.0);
                    self.drag_from = self.doc.routes.places.get(s.0 as usize).map(|&pl| (pl, self.world_pos(m)));
                }
            }
        }
    }

    /// Tile under the mouse on the shown map.
    pub(super) fn world_cell(&self, m: Vec2) -> Option<(usize, usize)> {
        if m.x < self.panel || m.y > screen_height() - STATUS_H {
            return None;
        }
        let w = self.world_pos(m);
        let (x, y) = ((w.x / 8.0).floor() as i32, (w.y / 8.0).floor() as i32);
        let (mw, mh) = EditorDocument::map_size(self.world_map);
        if x >= 0 && y >= 0 && (x as usize) < mw && (y as usize) < mh {
            Some((x as usize, y as usize))
        } else {
            None
        }
    }

    pub(super) fn update_world(&mut self) {
        // a new zone gone (undone project, deleted): show the world map
        if matches!(self.world_map, OnMap::NewZone(_)) && self.new_zone_shown().is_none() {
            self.show_map(OnMap::World);
        }
        let m: Vec2 = mouse_position().into();
        let ctrl = super::shortcuts::command_down();
        let (space, _, _) = self.navigate(m, ctrl, self.map_px());
        if self.pan.is_some() || space {
            return;
        }
        if self.picker.is_some() && self.world_tab != TAB_POINTS {
            self.cancel_picking();
        }
        if self.world_tab == TAB_POINTS {
            if self.picker.is_some() {
                self.picker_clicks(m);
            } else {
                self.points_clicks(m);
            }
            return;
        }
        let cell = self.world_cell(m).map(|(x, y)| (x as i32, y as i32));
        self.edit_grid(cell);
    }

    pub(super) fn draw_world(&mut self) {
        clear_background(col(3));
        let s = self.scale() * 8.0;
        let (cw, ch) = (screen_width() - self.panel, screen_height() - STATUS_H);
        let (mw, mh) = EditorDocument::map_size(self.world_map);
        if let Some(atlas) = self.map_atlas() {
            let x0 = (self.cam.x / 8.0).floor().max(0.0) as i32;
            let y0 = (self.cam.y / 8.0).floor().max(0.0) as i32;
            let nx = (cw / s).ceil() as i32 + 1;
            let ny = (ch / s).ceil() as i32 + 1;
            for y in y0..(y0 + ny).min(mh as i32) {
                for x in x0..(x0 + nx).min(mw as i32) {
                    let t = self.doc.tile(self.world_map, x as usize, y as usize);
                    let p = self.on_screen(x * 8, y * 8);
                    draw_texture_ex(
                        atlas,
                        p.x.floor(),
                        p.y.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new((t % 16) as f32 * 8.0, (t / 16) as f32 * 8.0, 8.0, 8.0)),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            let end = self.on_screen(mw as i32 * 8, mh as i32 * 8);
            if self.grid && s >= 12.0 && self.world_tab == TAB_PICTURE {
                let gc = Color::new(0.0, 0.0, 0.0, 0.12);
                for x in x0..=(x0 + nx).min(mw as i32) {
                    let px = self.on_screen(x * 8, 0).x;
                    draw_line(px, 0.0, px, end.y.min(ch), 1.0, gc);
                }
                for y in y0..=(y0 + ny).min(mh as i32) {
                    let py = self.on_screen(0, y * 8).y;
                    draw_line(self.panel, py, end.x.min(screen_width()), py, 1.0, gc);
                }
            }
            let m: Vec2 = mouse_position().into();
            if self.world_tab == TAB_POINTS {
                self.draw_points();
                self.draw_start_mark();
                self.draw_picker();
            } else {
                let cell = self.world_cell(m);
                self.draw_grid_overlay(atlas, cell.map(|(x, y)| (x as i32, y as i32)));
                self.draw_start_mark();
                if let Some((x, y)) = cell {
                    let t = self.doc.tile(self.world_map, x, y);
                    put_text(&format!("{x},{y}  #{t:02X}"), self.panel + 8.0, ch - 8.0, 20.0, WHITE);
                }
            }
        }
        let (act, y) = self.world_panel_top();
        let act = match self.world_tab {
            TAB_PICTURE => {
                self.tile_palette(y);
                act
            }
            _ if self.picker.is_some() => self.picker_panel(y).or(act),
            _ => self.scrolled_points_panel(y).or(act),
        };
        self.status_line(cw, ch);
        self.test_game_popover();
        self.draw_tip();
        if let Some(a) = act {
            self.run(a);
        }
    }

    /// Restore the map shown: picture and points (`EditorDocument::restore_map`).
    pub(super) fn restore_map(&mut self) {
        let map = self.world_map;
        let name = self.map_name(map);
        self.clipboard = None;
        self.route_draw = None;
        self.placing_point = false;
        let origin = match map {
            OnMap::NewZone(k) => self.doc.zone_restore_origin(k).map(|o| self.origin_text(o)),
            _ => None,
        };
        if !self.doc.restore_map(map) {
            return;
        }
        self.choose_on_new_zone();
        let how = match map {
            _ if origin.is_some() => format!("{} {}", tr(self.lang, "restored as imported from"), origin.unwrap_or_default()),
            OnMap::NewZone(k) => format!("{} {} {}", tr(self.lang, "restored as the"), self.map_name(OnMap::Zone(self.doc.new_zones[k].base)), tr(self.lang, "is in the game")),
            _ => tr(self.lang, "restored as it is in the game").to_string(),
        };
        self.status = format!("{name} {how}: {}", tr(self.lang, "picture and points (save to keep it; undo brings them back)."));
    }

    /// Undo the last map change, a picture's or the points', whatever the tab.
    pub(super) fn world_undo(&mut self) {
        if !self.doc.map_undo() {
            self.status = tr(self.lang, "Nothing to undo.").into();
        }
    }

    pub(super) fn world_redo(&mut self) {
        if !self.doc.map_redo() {
            self.status = tr(self.lang, "Nothing to redo.").into();
        }
    }

    /// Title, map choice, new zones, tabs, tools, undo and save. Returns the action chosen and
    /// where the panel continues.
    fn world_panel_top(&mut self) -> (Option<Action>, f32) {
        draw_rectangle(0.0, 0.0, self.panel, screen_height(), col(0));
        draw_rectangle(self.panel - 3.0, 0.0, 3.0, screen_height(), col(3));
        let lang = self.lang;
        let w = self.inner_w();
        let mut act = None;
        let mut pick = |a: Action, clicked: bool| {
            if clicked {
                act = Some(a);
            }
        };
        put_text(tr(lang, "ZONE EDITOR"), PAD, 25.0, 20.0, col(3));
        pick(Action::WorldEditor, self.tool_button(Rect::new(PAD + w - 104.0, 8.0, 104.0, 24.0), tr(lang, "Levels"), Action::WorldEditor, false));
        let mut y = 40.0;
        // the map picker: < World map (M) v >
        pick(Action::PrevMap, self.tool_button(Rect::new(PAD, y, 30.0, ROW), "<", Action::PrevMap, false));
        pick(Action::NextMap, self.tool_button(Rect::new(PAD + w - 30.0, y, 30.0, ROW), ">", Action::NextMap, false));
        let mut name = self.map_name(self.world_map);
        if self.doc.map_start(self.world_map).is_some() {
            name.push_str(" (M)");
        }
        if self.doc.mods.map(self.world_map) {
            name.push_str("  mod");
        }
        let mid = Rect::new(PAD + 34.0, y, w - 68.0, ROW);
        let shown = format!("{}  {}", fit(&name, mid.w - 30.0, 15.0), if self.maps_open { "^" } else { "v" });
        if button(mid, &shown, self.maps_open, 15.0) {
            self.maps_open = !self.maps_open;
        }
        y += ROW + GAP;
        if self.maps_open {
            let third = (w - 2.0 * GAP) / 3.0;
            let maps = self.all_maps();
            let start = self.doc.start_map();
            for (i, &m) in maps.iter().enumerate() {
                let r = Rect::new(PAD + (i % 3) as f32 * (third + GAP), y + (i / 3) as f32 * (MAP_BUTTON_H + 2.0), third, MAP_BUTTON_H);
                let mark = if start == Some(m) { " (M)" } else { "" };
                let room = third - 10.0 - text_size(mark, 15).width;
                let label = format!("{}{mark}", fit(&self.map_short(m), room, 15.0));
                if button(r, &label, m == self.world_map, 15.0) {
                    self.cancel_picking();
                    self.show_map(m);
                    self.choose_on_new_zone();
                }
                if self.doc.mods.map(m) {
                    draw_triangle(vec2(r.x + r.w, r.y), vec2(r.x + r.w - 8.0, r.y), vec2(r.x + r.w, r.y + 8.0), col(2));
                }
                if let OnMap::NewZone(_) = m {
                    user_tag(r, m == self.world_map);
                    if r.contains(mouse_position().into()) {
                        self.tip = Some(format!("{}  ({})", self.map_name(m), tr(lang, "made in the editor")));
                    }
                }
            }
            y += maps.len().div_ceil(3) as f32 * (MAP_BUTTON_H + 2.0);
            if !self.doc.new_zones.is_empty() {
                y += user_tag_legend(PAD, y + 2.0, tr(lang, "made in the editor"));
            }
        }
        let items = [(tr(lang, "Copy zone"), Action::NewZone, self.world_map != OnMap::World), (tr(lang, "Blank zone"), Action::BlankZone, true), (tr(lang, "Delete zone"), Action::DeleteZone, self.new_zone_shown().is_some())];
        let (next, a) = self.picker_actions(y, self.maps_open, items);
        y = next;
        if let Some(a) = a {
            pick(a, true);
        }
        let (next, a) = self.toolbar(y, &[(tr(lang, "Save"), Action::Save, false), (tr(lang, "Play"), Action::Play, false)]);
        y = next - 6.0;
        if let Some(a) = a {
            pick(a, true);
        }
        pick(Action::PlayFromHere, self.play_button(Rect::new(PAD, y, w, ROW), tr(lang, "Play from a point"), Action::PlayFromHere, self.play_pick));
        y += ROW + GAP + 6.0;
        // Mario's start: go there, or put it back on the game's own
        let missing = !self.doc.start_ok();
        let go = if missing { tr(lang, "Mario's start is missing!") } else { tr(lang, "Go to Mario's start") };
        let half = (w - GAP) / 2.0;
        pick(Action::GoToStart, self.tool_button(Rect::new(PAD, y, half, ROW), go, Action::GoToStart, missing));
        let game = Rect::new(PAD + half + GAP, y, half, ROW);
        let tip = tr(lang, "Put Mario's start back where the original game begins (world map point 00)");
        if self.doc.start == Start::default() {
            self.tip_for(game, tip, Action::GameStart);
            unavailable_button(game, tr(lang, "Original start"), 15.0);
        } else {
            pick(Action::GameStart, self.tip_button(game, tr(lang, "Original start"), tip, Action::GameStart, false));
        }
        y += ROW + GAP + 6.0;
        if let Some(k) = self.new_zone_shown() {
            self.name_field(Rect::new(PAD, y, w, 24.0), Named::Zone(k));
            y += 24.0 + GAP + 6.0;
        }
        y = self.map_music_rows(y);
        // Picture / Points, then the picture's tools
        let half = (w - GAP) / 2.0;
        let picture = self.world_tab == TAB_PICTURE;
        if self.tool_button(Rect::new(PAD, y, half, ROW), tr(lang, "Picture"), Action::SwitchTab, picture) {
            self.world_tab = TAB_PICTURE;
        }
        if self.tool_button(Rect::new(PAD + half + GAP, y, half, ROW), tr(lang, "Points"), Action::SwitchTab, !picture) {
            self.world_tab = TAB_POINTS;
        }
        y += ROW + GAP;
        if picture {
            let qw = (w - 3.0 * GAP) / 4.0;
            let tool = self.tool.kind();
            let tools = [
                (tr(lang, "Pencil"), Action::Pencil, tool == ToolKind::Pencil),
                (tr(lang, "Box"), Action::BoxFill, tool == ToolKind::Box),
                (tr(lang, "Select"), Action::Select, tool == ToolKind::Select),
                (tr(lang, "Grid"), Action::Grid, self.grid),
            ];
            for (i, (name, a, on)) in tools.into_iter().enumerate() {
                pick(a, self.tool_button(Rect::new(PAD + i as f32 * (qw + GAP), y, qw, ROW), name, a, on));
            }
            y += ROW + GAP;
        }
        y += 2.0;
        draw_rectangle(PAD, y, w, 2.0, col(3));
        y += 8.0;
        (act, y)
    }

    /// The shown map's music: a chooser over `map_music::choices`, then Listen and how loud it
    /// plays. Returns where the panel continues.
    fn map_music_rows(&mut self, mut y: f32) -> f32 {
        let lang = self.lang;
        let w = self.inner_w();
        let (cx, cw) = (PAD + LABEL_W, w - LABEL_W);
        let music = self.doc.map_music(self.world_map);
        let choices = map_music::choices();
        let i = choices.iter().position(|&(m, _)| m == music);
        let name = match i.map(|i| choices[i].1) {
            Some(MusicNamed::Map(map)) => self.map_name(match map {
                Map::World => OnMap::World,
                Map::Zone(z) => OnMap::Zone(z),
            }),
            Some(MusicNamed::Level(l)) => format!("{} {l:02}", tr(lang, "as level")),
            None => format!("${music:02X}"),
        };
        put_text(tr(lang, "Music"), PAD, y + 18.0, 15.0, col(3));
        let d = stepper(Rect::new(cx, y, cw, ROW), "<", ">", &name);
        let playing = self.music_playing();
        if d != 0 {
            let chosen = choices[(i.unwrap_or(0) as i32 + d).rem_euclid(choices.len() as i32) as usize].0;
            self.doc.set_map_music(self.world_map, chosen);
            if playing {
                self.toggle_music(chosen);
                self.toggle_music(chosen);
            }
        }
        y += ROW + GAP;
        let music = self.doc.map_music(self.world_map);
        if button(Rect::new(PAD, y, LABEL_W - GAP, ROW), if playing { tr(lang, "Stop") } else { tr(lang, "Listen") }, playing, 15.0) {
            self.toggle_music(music);
        }
        let v = self.prefs.music_volume as i32;
        let d = stepper(Rect::new(cx, y, cw, ROW), "-", "+", &format!("{} {v}", tr(lang, "editor vol")));
        let nv = (v + d).clamp(*MUSIC_VOLUMES.start() as i32, *MUSIC_VOLUMES.end() as i32) as u32;
        if nv != self.prefs.music_volume {
            self.set_music_volume(nv);
        }
        y + ROW + GAP + 6.0
    }

    /// A point's circle with the level it opens and its tag; the chosen one ringed.
    fn draw_point(&self, p: u8, (x, y): (i32, i32), fill: Color, tag: &str) {
        let r = (4.0 * self.scale()).max(6.0);
        let c = self.on_screen(x, y);
        let level = self.level_of(p as usize);
        if (level as usize) < self.doc.levels.len() {
            let fs = (r * 1.3).clamp(13.0, 22.0);
            draw_circle(c.x, c.y, r + 2.0, BLACK);
            draw_circle(c.x, c.y, r, fill);
            let t = format!("{level:02}");
            let d = text_size(&t, fs as u16);
            put_text(&t, c.x - d.width / 2.0, c.y + d.offset_y / 2.0, fs, WHITE);
        } else {
            draw_circle(c.x, c.y, r * 0.6 + 1.5, BLACK);
            draw_circle(c.x, c.y, r * 0.6, Color::new(0.8, 0.8, 0.8, 1.0));
        }
        let chosen = p as usize == self.point;
        marked_text(tag, c.x + r + 2.0, c.y - r * 0.4, 15.0, if chosen { MARK_SELECTED } else { MARK });
        if chosen {
            draw_circle_lines(c.x, c.y, r + 5.0, 5.0, BLACK);
            draw_circle_lines(c.x, c.y, r + 5.0, 3.0, MARK_SELECTED);
        } else {
            draw_circle_lines(c.x, c.y, r + 2.5, 1.5, MARK);
        }
    }

    /// The points on the shown map: the level each opens, the paths Mario walks out of each
    /// (the chosen point's highlighted), the chosen one ringed.
    fn draw_points(&self) {
        let r = (4.0 * self.scale()).max(6.0);
        let pts = self.map_points();
        let zone = self.new_zone_shown();
        for &(p, x, y, _) in &pts {
            let Some(ways) = self.ways_of(p as usize) else { continue };
            if zone.is_none() && self.world_map != OnMap::World && self.is_world_point(p) {
                continue; // its paths are on the world map
            }
            let chosen = p as usize == self.point;
            for (d, w) in ways.iter().enumerate() {
                let Some(w) = w else { continue };
                let track: Vec<Vec2> = w.track().iter().map(|t| self.on_screen(x + t.0, y + t.1)).collect();
                let (c, th) = if chosen { (Color::new(1.0, 0.85, 0.1, 0.95), 3.0) } else { (Color::new(0.1, 0.1, 0.1, 0.75), 2.0) };
                for s in track.windows(2) {
                    draw_line(s[0].x, s[0].y, s[1].x, s[1].y, th, c);
                }
                if chosen && track.len() > 1 {
                    let a = track[(track.len() / 2).max(1)];
                    let t = format!("{} > {:02X}", DIRS[d], w.to);
                    put_text(&t, a.x + 5.0, a.y - 3.0, 16.0, BLACK);
                    put_text(&t, a.x + 4.0, a.y - 4.0, 16.0, Color::new(1.0, 0.95, 0.5, 1.0));
                }
            }
        }
        // the path being drawn, to the mouse
        if let (Some(corners), Some(start)) = (&self.route_draw, self.chosen_at()) {
            let m = self.snapped(mouse_position().into(), start);
            let mut line = vec![start];
            line.extend(corners.iter().copied());
            line.push(m);
            for s in line.windows(2) {
                let (a, b) = (self.on_screen(s[0].0, s[0].1), self.on_screen(s[1].0, s[1].1));
                draw_line(a.x, a.y, b.x, b.y, 3.0, WHITE);
            }
        }
        if self.placing_point || self.drag_point.is_some() {
            let dragged = self.drag_point.and_then(|p| Some((pts.iter().find(|s| s.0 == p)?, self.drag_offset()?)));
            let (x, y) = if zone.is_some() {
                let w = self.world_pos(mouse_position().into());
                (w.x as i32, w.y as i32)
            } else if let Some((s, (dx, dy))) = dragged {
                (s.1 + dx, s.2 + dy)
            } else {
                self.snapped(mouse_position().into(), (0, 2))
            };
            let c = self.on_screen(x, y);
            draw_circle_lines(c.x, c.y, r, 4.0, BLACK);
            draw_circle_lines(c.x, c.y, r, 2.0, MARK_SELECTED);
        }
        if let Some(k) = zone {
            let entry = self.doc.new_zones[k].entry;
            for &(p, x, y, _) in &pts {
                let mut tag = self.point_tag(p).unwrap_or(format!("{p:02X}"));
                if p == entry {
                    tag = format!("{tag} {}", tr(self.lang, "(entry)"));
                }
                if self.doc.map_start(self.world_map) == Some(p) {
                    tag = format!("{tag} {}", tr(self.lang, "(Mario's start)"));
                }
                self.draw_point(p, (x, y), Color::new(0.95, 0.45, 0.1, 1.0), &tag);
            }
            return;
        }
        for (i, sp) in original_points::SPOTS.iter().enumerate() {
            if sp.map != self.world_map {
                continue;
            }
            let mut tag = match (self.point_tag(sp.point), original_points::door(i)) {
                (Some(t), _) => t,
                (None, Some(to)) if self.doc.doors.into.is_empty() => format!("{:02X} > {}", sp.point, self.map_name(to.map)),
                _ => format!("{:02X}", sp.point),
            };
            if self.game_entry_shown() == Some(sp.point) {
                tag = format!("{tag} {}", tr(self.lang, "(entry)"));
            }
            if self.doc.map_start(self.world_map) == Some(sp.point) {
                tag = format!("{tag} {}", tr(self.lang, "(Mario's start)"));
            }
            if self.doc.point_notes(sp.point).contains(&Note::FlightOut) && tag == format!("{:02X}", sp.point) {
                tag = format!("{tag} > {} {}", self.map_name(OnMap::Zone(original_notes::SPACE_ZONE)), tr(self.lang, "(when cleared)"));
            }
            let (_, x, y, _) = self.spot_at(sp);
            self.draw_point(sp.point, (x, y), Color::new(0.95, 0.45, 0.1, 1.0), &tag);
        }
        for &(p, x, y, new) in &pts {
            if new {
                let tag = format!("{} {}", self.point_tag(p).unwrap_or(format!("{p:02X}")), tr(self.lang, "(new)"));
                self.draw_point(p, (x, y), Color::new(0.2, 0.7, 0.3, 1.0), &tag);
            }
        }
    }

    /// What the original game does with the chosen game point, and whether it was changed.
    fn original_rows(&self, y: f32) -> f32 {
        let p = self.point as u8;
        let mut lines: Vec<String> = self.doc.point_notes(p).iter().map(|&n| self.note_text(n)).collect();
        if self.doc.point_changed(p) {
            lines.push(tr(self.lang, "Changed from the original game (Restore puts the map back as it was).").to_string());
        }
        if lines.is_empty() {
            return y;
        }
        let mut y = self.section(y, tr(self.lang, "IN THE ORIGINAL GAME"), "");
        let w = self.inner_w();
        for line in lines.iter().flat_map(|l| wrap_px(l, w, 14.0)) {
            marked_text(&line, PAD, y + 12.0, 14.0, MARK);
            y += 16.0;
        }
        y + 4.0
    }

    fn note_text(&self, n: Note) -> String {
        let zone = |z: usize| self.map_name(OnMap::Zone(z));
        match n {
            Note::Start => tr(self.lang, "Where a new game starts; clearing its level walks Mario on to 13.").to_string(),
            Note::FlightOut => tr(self.lang, "Clearing the level opened here flies Mario to the Space Zone (he lands on 3C).").to_string(),
            Note::FlightIn => tr(self.lang, "How Mario enters the Space Zone: he lands here flying after clearing the level of world point 05 (or 1A).").to_string(),
            Note::Entrance { zone: z, twin } => format!(
                "{} {}. {} {twin:02X} {}",
                tr(self.lang, "Entrance of the"),
                zone(z),
                tr(self.lang, "After leaving the zone Mario stands here as point"),
                tr(self.lang, "(same place and paths, changed together; its level follows this one's).")
            ),
            Note::ZoneReturn { zone: z, twin } => format!(
                "{} {}: {} {twin:02X}, {}",
                tr(self.lang, "Where Mario stands after leaving the"),
                zone(z),
                tr(self.lang, "the same place and paths as"),
                tr(self.lang, "whose level it follows.")
            ),
            Note::UnusedDoor => tr(self.lang, "The game also keeps number 12 here, with its door into the Space Zone (A on 12), but Mario stops on 13, so that door never runs: he reaches the Space Zone by clearing world point 05's level.").to_string(),
            Note::TurtleWalk => tr(self.lang, "Leads into the Turtle Zone: the game walks Mario on its own to the hidden point 0D.").to_string(),
            Note::NeverStood => tr(self.lang, "No path leads here or out and it opens no level: Mario never stands on it.").to_string(),
        }
    }

    const SHUT_ACTIONS: [Action; 4] = [Action::ShutRight, Action::ShutLeft, Action::ShutUp, Action::ShutDown];

    /// The Points panel below `top`, scrolled with the wheel when it is taller than the window.
    fn scrolled_points_panel(&mut self, top: f32) -> Option<Action> {
        let bottom = screen_height() - 6.0;
        let m: Vec2 = mouse_position().into();
        if m.x < self.panel && m.y > top {
            let wheel = mouse_wheel().1;
            if wheel != 0.0 {
                self.points_scroll -= wheel.signum() * SCROLL_STEP;
            }
        }
        self.points_scroll = self.points_scroll.clamp(0.0, (self.points_content - (bottom - top)).max(0.0));
        let point = self.point;
        clip(Some(Rect::new(0.0, top, self.panel, bottom - top)));
        let (act, end) = self.points_panel(top - self.points_scroll);
        clip(None);
        self.points_content = end + self.points_scroll - top;
        if self.points_content > bottom - top {
            let (frac, at) = ((bottom - top) / self.points_content, self.points_scroll / self.points_content);
            draw_rectangle(self.panel - 8.0, top + at * (bottom - top), 3.0, frac * (bottom - top), col(2));
        }
        // a click above the panel's top is on the buttons drawn there, not on a row scrolled under them
        if m.y < top {
            self.point = point;
            return None;
        }
        act
    }

    /// The chosen point: the level it opens, where it leads, its paths; the points of the map.
    /// Returns the action chosen and where the panel ends.
    fn points_panel(&mut self, mut y: f32) -> (Option<Action>, f32) {
        let lang = self.lang;
        let w = self.inner_w();
        let zone = self.new_zone_shown();
        if !self.doc.start_ok() {
            y += message_lines(tr(lang, "Mario's start is missing: choose a point and press Mario's start. The project cannot be saved without it."), PAD, y + 12.0, w, 14.0, Severity::Warning) + 4.0;
        }
        // the chosen point
        y = self.section(y, tr(lang, "POINT"), "");
        let new = self.doc.routes.added.contains(&(self.point as u8));
        let on_zone = zone.is_some_and(|k| self.doc.new_zones[k].point(self.point as u8).is_some());
        let drawn = if on_zone {
            Some(self.map_name(self.world_map))
        } else if zone.is_some() {
            None
        } else if new {
            Some(tr(lang, "new point").to_string())
        } else {
            original_points::SPOTS.iter().find(|s| s.point as usize == self.point).map(|s| self.map_name(s.map))
        };
        let shown = format!("{:02X}  {}", self.point, drawn.unwrap_or_else(|| tr(lang, "(not drawn)").to_string()));
        let mut act = self.stepper(y, tr(lang, "Point"), &shown, (Action::PrevPoint, tr(lang, "Previous point")), (Action::NextPoint, tr(lang, "Next point")));
        y += ROW + GAP;
        let cur = self.level_of(self.point);
        let level = (cur as usize) < self.doc.levels.len();
        let label = match cur as usize {
            l if level && l >= cart_model::levels::LEVELS => match self.doc.level_name(l) {
                "" => format!("{} {l:02} ({} {:02})", tr(lang, "level"), tr(lang, "new, as"), self.doc.base_of(l)),
                name => format!("{l:02} {name} ({} {:02})", tr(lang, "as"), self.doc.base_of(l)),
            },
            l if level => format!("{} {l:02}", tr(lang, "level")),
            _ if cur == world::NO_LEVEL => tr(lang, "no level").to_string(),
            _ => format!("{} ${cur:02X}", tr(lang, "special")),
        };
        act = act.or(self.stepper(y, tr(lang, "Opens"), &label, (Action::PrevOpens, tr(lang, "Previous level")), (Action::NextOpens, tr(lang, "Next level"))));
        y += ROW + GAP;
        let (ny, door) = self.door_rows(y);
        y = ny;
        act = act.or(door);
        // what the point is for: Mario's start, the zone's entry
        let mut roles: Vec<(&str, Action, bool)> = Vec::new();
        if self.doc.start_at(self.world_map, self.point as u8).is_some() {
            roles.push((tr(lang, "Mario's start"), Action::SetStart, self.doc.map_start(self.world_map) == Some(self.point as u8)));
        }
        if let Some(k) = zone.filter(|_| on_zone) {
            roles.push((tr(lang, "Zone entry"), Action::MakeEntry, self.doc.new_zones[k].entry as usize == self.point));
        } else if !new && self.can_be_game_entry() {
            roles.push((tr(lang, "Zone entry"), Action::MakeEntry, self.game_entry_shown() == Some(self.point as u8)));
        }
        if !roles.is_empty() {
            y += 2.0;
            act = act.or(self.button_row(y, &roles));
            y += ROW + GAP;
        }
        if zone.is_none() && !new {
            y = self.original_rows(y);
        }
        // the ways out of the point
        y = self.section(y, tr(lang, "PATHS"), tr(lang, "pad > point"));
        if let Some(ways) = self.ways_of(self.point).cloned() {
            let half = (w - GAP) / 2.0;
            for (d, way) in ways.iter().enumerate() {
                let r = Rect::new(PAD + (d % 2) as f32 * (half + GAP), y + (d / 2) as f32 * (ROW + GAP), half, ROW);
                draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, col(2));
                match way.as_ref().map(|w| w.to) {
                    Some(to) => {
                        put_text(&format!("{}  >  {to:02X}", DIRS[d]), r.x + 8.0, r.y + 18.0, 15.0, col(3));
                        let a = [Action::DeletePathRight, Action::DeletePathLeft, Action::DeletePathUp, Action::DeletePathDown][d];
                        if self.tip_button(Rect::new(r.x + r.w - ROW, r.y, ROW, ROW), "x", tr(lang, "Delete this path"), a, false) {
                            act = Some(a);
                        }
                    }
                    None => {
                        put_text(&format!("{}  >  -", DIRS[d]), r.x + 8.0, r.y + 18.0, 15.0, col(2));
                    }
                }
            }
            y += 2.0 * (ROW + GAP);
            if self.doc.can_shut(zone, self.point as u8) {
                put_text(tr(lang, "Shut until its level is cleared:"), PAD, y + 13.0, 14.0, INFO);
                y += 18.0;
                let shut = self.doc.shut_of(zone, self.point as u8);
                let qw = (w - 3.0 * GAP) / 4.0;
                for (d, &a) in Self::SHUT_ACTIONS.iter().enumerate() {
                    let on = shut & added_points::DIR_BITS[d] != 0;
                    if self.tip_button(Rect::new(PAD + d as f32 * (qw + GAP), y, qw, ROW), &DIRS[d].to_string(), tr(lang, "Shut this way"), a, on) {
                        act = Some(a);
                    }
                }
                y += ROW + GAP;
            }
        }
        let drawing = self.route_draw.is_some();
        act = act.or(self.button_row(y, &[(tr(lang, "New path"), Action::NewPath, drawing)]));
        y += ROW + GAP;
        // the map's points
        let (used, most) = self.doc.point_count(self.world_map);
        let title = format!("{} {}", tr(lang, "POINTS ON"), self.map_short(self.world_map).to_uppercase());
        y = self.section(y, &title, &format!("{used} / {most}"));
        let mut edits: Vec<(&str, Action, bool)> = Vec::new();
        if self.takes_new_points() {
            edits.push((tr(lang, "New point"), Action::NewPoint, self.placing_point));
        }
        if new || on_zone {
            edits.push((tr(lang, "Delete point"), Action::DeletePoint, false));
        }
        if !edits.is_empty() {
            act = act.or(self.button_row(y, &edits));
            y += ROW + GAP + 2.0;
        }
        let list: Vec<usize> = self.map_points().iter().map(|s| s.0 as usize).collect();
        let per_row = ((w + GAP) / (POINT_BUTTON.0 + GAP)).floor().max(1.0) as usize;
        let bw = (w - (per_row - 1) as f32 * GAP) / per_row as f32;
        for (k, &p) in list.iter().enumerate() {
            let r = Rect::new(PAD + (k % per_row) as f32 * (bw + GAP), y + (k / per_row) as f32 * (POINT_BUTTON.1 + 2.0), bw, POINT_BUTTON.1);
            let l = self.level_of(p);
            let text = if (l as usize) < self.doc.levels.len() { format!("{p:02X} > {l:02}") } else { format!("{p:02X} > -") };
            if button(r, &text, p == self.point, 15.0) {
                self.point = p;
            }
        }
        y += list.len().div_ceil(per_row) as f32 * (POINT_BUTTON.1 + 2.0);
        let note = if drawing {
            tr(lang, "Drawing a path: click the corners, then the point it goes to. The pad direction is the way it starts; the way back is added too if that point has none that way. Backspace: last corner off. Esc or right click: cancel.")
        } else if self.placing_point {
            tr(self.lang, "Click on the map where the new point goes. Esc: cancel.")
        } else if zone.is_some() {
            tr(self.lang, "A new zone plays with its base zone's graphics. Click a point to choose it, drag it to move it (its paths become straight); New point adds one (up to 20 in the Tree Zone's, 21 in the others). Entry: Mario starts there. Exit to: where its exits lead. Its door is chosen on a world map point. Plays only in this port.")
        } else {
            tr(self.lang, "Click a point on the map to choose it; with - + choose the level it opens. Door: a world point can enter a zone or a new zone (A or START on it). Exit to: a zone's exit leads to a world point or straight into another zone; Choose on the map (U) picks where by clicking. Exit 1 / 2: Here moves that exit to the chosen zone point. Paths are drawn in yellow; New path draws one, New point puts one on the world map (6 numbers free). Play (F5) saves and tries it.")
        };
        let mut y = self.section(y, tr(lang, "HELP"), "");
        for line in wrap_px(note, w, 14.0) {
            put_text(&line, PAD, y + 12.0, 14.0, INFO);
            y += 16.0;
        }
        (act, y + 8.0)
    }
}
