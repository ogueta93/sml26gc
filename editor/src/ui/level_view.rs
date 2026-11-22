//! The level view: blocks drawn with the game's own graphics, zoom and camera, grid, the
//! whole-level overview, painting with the chosen tool (the side panel: `level_panel`).
use super::widgets::*;
use super::{Editor, Mode, LEVEL_SIZE};
use cart_model::levels::{COLS, ROWS};
use macroquad::prelude::*;
use super::inset::{screen_width, screen_height, mouse_position, is_mouse_button_down};

impl Editor {
    /// Where the whole-level overview sits (top right of the level view).
    pub(super) fn overview_rect(&self) -> Rect {
        let k = ((screen_width() - self.panel - 20.0) / COLS as f32).min(3.0);
        let (w, h) = (COLS as f32 * k, ROWS as f32 * k);
        Rect::new(screen_width() - w - 10.0, 10.0, w, h)
    }

    /// The block under the mouse (none over the panel, status bar or overview).
    pub(super) fn cell_at(&self, m: Vec2) -> Option<(i32, i32)> {
        if m.x < self.panel || m.y > screen_height() - STATUS_H {
            return None;
        }
        if self.show_overview && self.overview_rect().contains(m) {
            return None;
        }
        let w = self.world_pos(m);
        let (c, r) = ((w.x / BLOCK).floor() as i32, (w.y / BLOCK).floor() as i32);
        if c >= 0 && r >= 0 && (c as usize) < COLS && (r as usize) < ROWS {
            Some((c, r))
        } else {
            None
        }
    }

    pub(super) fn update_level(&mut self) {
        self.look_now();
        let m: Vec2 = mouse_position().into();
        let ctrl = super::shortcuts::command_down();
        let (space, view, max) = self.navigate(m, ctrl, LEVEL_SIZE);
        // Click or drag on the overview: centre the view there.
        let ov = self.overview_rect();
        if self.show_overview && ov.contains(m) && is_mouse_button_down(MouseButton::Left) {
            let k = ov.w / COLS as f32;
            let target = vec2((m.x - ov.x) / k, (m.y - ov.y) / k) * BLOCK;
            self.cam = (target - view * 0.5).clamp(Vec2::ZERO, max);
            return;
        }
        self.pump_mario();
        self.pump_looks();
        if self.pan.is_some() || space {
            return;
        }
        // a click that started the game or put a checkpoint is not also the start of a stroke
        if self.hold_click {
            if is_mouse_button_down(MouseButton::Left) {
                return;
            }
            self.hold_click = false;
        }
        if self.update_play_here(m) || self.update_checkpoint(m) {
            self.hold_click = true;
            return;
        }
        if self.update_goal(m) || self.update_start(m) {
            return;
        }
        if self.mode == Mode::Objects {
            self.update_objects(m);
            return;
        }
        let cell = self.cell_at(m);
        self.edit_grid(cell);
    }

    pub(super) fn draw_level(&mut self) {
        clear_background(col(3));
        let s = self.scale() * BLOCK;
        let (cw, ch) = (screen_width() - self.panel, screen_height() - STATUS_H);
        // --- the level
        if let Some(atlas) = &self.atlases[self.doc.base_of(self.cur)] {
            let c0 = (self.cam.x / BLOCK).floor() as i32;
            let r0 = (self.cam.y / BLOCK).floor() as i32;
            let ncol = (cw / s).ceil() as i32 + 1;
            let nrow = (ch / s).ceil() as i32 + 1;
            let map = &self.doc.levels[self.cur].content.map;
            for r in r0..(r0 + nrow).min(ROWS as i32) {
                for c in c0..(c0 + ncol).min(COLS as i32) {
                    let id = map[r as usize * COLS + c as usize] & 0x7F;
                    let x = self.panel + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_texture_ex(
                        atlas,
                        x.floor(),
                        y.floor(),
                        WHITE,
                        DrawTextureParams {
                            source: Some(Rect::new((id % 16) as f32 * 16.0, (id / 16) as f32 * 16.0, 16.0, 16.0)),
                            dest_size: Some(vec2(s.ceil(), s.ceil())),
                            ..Default::default()
                        },
                    );
                }
            }
            if self.grid && s >= 16.0 {
                let gc = Color::new(0.0, 0.0, 0.0, 0.15);
                for c in c0..=c0 + ncol {
                    let x = self.panel + (c as f32 * BLOCK - self.cam.x) * self.scale();
                    draw_line(x, 0.0, x, ch, 1.0, gc);
                }
                for r in r0..=r0 + nrow {
                    let y = (r as f32 * BLOCK - self.cam.y) * self.scale();
                    draw_line(self.panel, y, screen_width(), y, 1.0, gc);
                }
            }
            // where the level ends
            let end_x = self.panel + (COLS as f32 * BLOCK - self.cam.x) * self.scale();
            let end_y = (ROWS as f32 * BLOCK - self.cam.y) * self.scale();
            draw_rectangle_lines(self.panel - self.cam.x * self.scale(), -self.cam.y * self.scale(), end_x - (self.panel - self.cam.x * self.scale()), end_y + self.cam.y * self.scale(), 3.0, col(3));
            let m: Vec2 = mouse_position().into();
            if self.mode == Mode::Blocks {
                let cell = self.cell_at(m);
                self.draw_grid_overlay(atlas, cell);
                if let Some((c, r)) = cell {
                    let id = self.doc.block(self.cur, c, r);
                    put_text(&format!("{c},{r}  #{id:02X}"), self.panel + 8.0, ch - 8.0, 20.0, WHITE);
                }
            }
        } else {
            put_text(tr(self.lang, "Loading the level from the game..."), self.panel + 30.0, 60.0, 26.0, col(0));
        }
        self.finish_level_frame(cw, ch);
    }

    /// Objects, overview, panel and status bar, over the level.
    fn finish_level_frame(&mut self, cw: f32, ch: f32) {
        self.draw_objects();
        self.draw_goals();
        self.draw_checkpoints();
        self.draw_start();
        self.draw_play_here();
        if self.mode == Mode::Objects && self.placing.is_none() {
            self.object_tip(ch);
        }
        self.draw_overview(cw, ch);
        let (act, pick) = self.level_panel();
        self.level_status_bar(cw, ch);
        self.test_game_popover();
        self.draw_tip();
        if let Some(a) = act {
            self.run(a);
        }
        if let Some(l) = pick {
            self.select_level(l);
        }
    }

    /// The whole level, one pixel per block (M), with the part on screen outlined.
    fn draw_overview(&mut self, cw: f32, ch: f32) {
        if !self.show_overview {
            return;
        }
        if self.overview_stale || self.overview.is_none() {
            if let Some(art) = self.art.level(self.doc.base_of(self.cur)) {
                self.overview = Some(make_overview(art, &self.doc.levels[self.cur].content.map));
                self.overview_stale = false;
            }
        }
        if let Some(t) = &self.overview {
            let r = self.overview_rect();
            let k = r.w / COLS as f32;
            draw_rectangle(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, col(3));
            draw_texture_ex(t, r.x, r.y, WHITE, DrawTextureParams { dest_size: Some(vec2(r.w, r.h)), ..Default::default() });
            let v = vec2(cw, ch) / self.scale() / BLOCK * k;
            let p = self.cam / BLOCK * k;
            draw_rectangle_lines(r.x + p.x, r.y + p.y, v.x.min(r.w), v.y.min(r.h), 2.0, Color::from_rgba(255, 255, 255, 230));
            put_text(tr(self.lang, "MAP (M)"), r.x + 4.0, r.y + r.h + 16.0, 16.0, col(0));
        }
    }
}
