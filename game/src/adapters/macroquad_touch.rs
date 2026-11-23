//! The on-screen controls through macroquad: this frame's fingers, the controls drawn around (or
//! over) the picture, a short vibration when one is pressed, and the controller the session
//! sees: the real one together with the touched buttons, through the controller's bindings.
use super::platform;
use crate::domain::devices::Devices;
use crate::domain::keys::{self, KEYS};
use crate::domain::pad::PadButton;
use crate::domain::settings::{Scale, Settings};
use crate::domain::touch::{Circle, Finger, Layout, Phase, Rect, TouchControls, TouchShow, Visibility};
use crate::ports::Gamepad;
use macroquad::prelude::*;

const VIBRATE_MS: u32 = 12;
/// The button that brings back the controls turned off: its size and its distance from the corner.
const BRING_BACK: f32 = 40.0;
const CORNER: f32 = 10.0;

#[derive(Default)]
pub struct Touch {
    controls: TouchControls,
    visibility: Visibility,
    layout: Option<Layout>,
    fullscreen: bool,
    /// A key went down, or the screen was touched, since the program started.
    key_seen: bool,
    touch_seen: bool,
    controller: bool,
    /// The controls are off and nothing else was used to play: the button that brings them back.
    bring_back: Option<Rect>,
    /// The fingers seen on the screen (a tap can start and end within one frame: only its end
    /// is seen).
    down: Vec<u64>,
}

impl Touch {
    pub fn new() -> Touch {
        Touch::default()
    }

    /// Read this frame's fingers. `key` and `button`: a key or a controller button went down
    /// (with Auto, that hides the controls until the next touch); `controller`: one is connected.
    /// `extras`: the port's actions that work now. True when the button that brings the controls
    /// back was touched.
    pub fn update(&mut self, st: &Settings, key: bool, button: bool, controller: bool, extras: &[usize]) -> bool {
        let dpi = screen_dpi_scale();
        let fingers: Vec<Finger> = touches()
            .iter()
            .map(|t| Finger {
                id: t.id,
                x: t.position.x / dpi,
                y: t.position.y / dpi,
                phase: match t.phase {
                    TouchPhase::Started => Phase::Began,
                    TouchPhase::Ended | TouchPhase::Cancelled => Phase::Ended,
                    TouchPhase::Moved | TouchPhase::Stationary => Phase::Moved,
                },
            })
            .collect();
        // a finger that began this frame, or a tap that began and ended within it
        let new: Vec<&Finger> = fingers.iter().filter(|f| f.phase == Phase::Began || (f.phase == Phase::Ended && !self.down.contains(&f.id))).collect();
        let began = !new.is_empty();
        for f in &fingers {
            self.down.retain(|&id| id != f.id);
            if f.phase != Phase::Ended {
                self.down.push(f.id);
            }
        }
        self.key_seen |= key;
        self.touch_seen |= began;
        self.controller = controller;
        self.visibility.update(began, key || button);
        let (w, h) = (screen_width(), screen_height());
        // from what was used, not from what the browser claims: no key pressed, no controller
        let stuck = self.devices().touch && !self.key_seen && !self.controller;
        self.bring_back = (st.touch == TouchShow::Never && stuck).then(|| Rect::new(w - CORNER - BRING_BACK, CORNER, BRING_BACK, BRING_BACK));
        if let Some(r) = self.bring_back {
            if new.iter().any(|f| r.grown(r.w * 0.3).contains(f.x, f.y)) {
                if st.vibrate {
                    platform::vibrate(VIBRATE_MS);
                }
                return true;
            }
        }
        self.layout = self.visibility.shown(st.touch).then(|| Layout::new(w, h, st.touch_size, platform::can_fullscreen()).with_extras(extras)).map(|l| if st.scale == Scale::Stretched { l.stretched() } else { l });
        match &self.layout {
            Some(layout) => self.controls.update(layout, &fingers),
            None => self.controls = TouchControls::default(),
        }
        if st.vibrate && self.controls.any_pressed() {
            platform::vibrate(VIBRATE_MS);
        }
        self.fullscreen = platform::is_fullscreen();
        platform::place_page_button(platform::PageButton::Fullscreen, self.layout.and_then(|l| l.fullscreen));
        false
    }

    /// The inputs there are: the keyboard (always on a computer; in a browser with a mouse or
    /// once a key went down), the controller while connected, the touch screen.
    pub fn devices(&self) -> Devices {
        let touch = platform::has_touch() || self.touch_seen;
        Devices { keyboard: platform::has_keyboard() || self.key_seen, controller: self.controller, touch, touch_shown: self.layout.is_some() }
    }

    /// Where the picture goes while the controls are shown.
    pub fn picture(&self) -> Option<Rect> {
        self.layout.map(|l| l.picture)
    }

    /// The controller the session reads: `pad` and the touched buttons, each Game Boy button as
    /// the controller button bound to it (`bindings`), the menu button as Guide.
    pub fn pad<'a>(&'a self, pad: &'a dyn Gamepad, bindings: [PadButton; KEYS]) -> TouchPad<'a> {
        TouchPad { pad, touch: &self.controls, bindings, active: self.layout.is_some() }
    }

    /// Draw the controls, after the picture.
    pub fn draw(&self) {
        if let Some(r) = self.bring_back {
            rounded(r, Color::new(0.3, 0.3, 0.32, 0.6));
            let (cx, cy, arm) = (r.x + r.w / 2.0, r.y + r.h / 2.0, r.w * 0.17);
            let ink = Color::new(0.9, 0.9, 0.92, 0.9);
            draw_rectangle(cx - arm / 2.0, cy - arm * 1.5, arm, arm * 3.0, ink);
            draw_rectangle(cx - arm * 1.5, cy - arm / 2.0, arm * 3.0, arm, ink);
        }
        let Some(l) = &self.layout else { return };
        let alpha = if l.over_picture { 0.5 } else { 1.0 };
        let shade = |v: f32| Color::new(v, v, v * 1.05, alpha);
        let (face, lit, ink) = (shade(0.3), shade(0.55), shade(0.88));
        let held = |k: usize| self.controls.held(k);

        let d = l.dpad;
        if !l.over_picture {
            draw_circle(d.x, d.y, d.r, shade(0.16));
        }
        let arm = d.r * 0.6;
        let reach = d.r * 0.9 - arm / 2.0;
        draw_rectangle(d.x - arm / 2.0, d.y - arm / 2.0, arm, arm, face);
        for (k, dx, dy) in [(keys::UP, 0.0, -1.0), (keys::DOWN, 0.0, 1.0), (keys::LEFT, -1.0, 0.0), (keys::RIGHT, 1.0, 0.0)] {
            let (cx, cy) = (d.x + dx * (arm / 2.0 + reach / 2.0), d.y + dy * (arm / 2.0 + reach / 2.0));
            let (w, h) = if dx == 0.0 { (arm, reach) } else { (reach, arm) };
            draw_rectangle(cx - w / 2.0, cy - h / 2.0, w, h, if held(k) { lit } else { face });
            draw_triangle_pointing(cx, cy, dx, dy, arm * 0.22, ink);
        }

        for (c, k, name) in [(l.a, keys::A, "A"), (l.b, keys::B, "B")] {
            draw_circle(c.x, c.y, c.r, if held(k) { lit } else { face });
            label(name, c, c.r * 0.9, ink);
        }
        for (r, k, name) in [(l.select, keys::SELECT, "SELECT"), (l.start, keys::START, "START")] {
            pill(r, if held(k) { lit } else { face });
            label(name, Circle { x: r.x + r.w / 2.0, y: r.y + r.h / 2.0, r: 0.0 }, r.h * 0.55, ink);
        }

        let m = l.menu;
        rounded(m, if self.controls.menu() { lit } else { face });
        for i in 0..3 {
            let y = m.y + m.h * (0.32 + 0.18 * i as f32);
            draw_line(m.x + m.w * 0.27, y, m.x + m.w * 0.73, y, (m.h * 0.08).max(2.0), ink);
        }
        if let Some(f) = l.fullscreen {
            rounded(f, face);
            fullscreen_icon(f, self.fullscreen, ink);
        }
        for &(k, r) in l.extras.iter().flatten() {
            rounded(r, if held(k) { lit } else { face });
            extra_icon(k, r, ink);
        }
    }
}

/// The picture of one of the port's actions: rewind and fast forward as double arrows, the save
/// states as an arrow into or out of a tray, restart as a turning arrow.
fn extra_icon(k: usize, r: Rect, ink: Color) {
    let (cx, cy, s) = (r.x + r.w / 2.0, r.y + r.h / 2.0, r.w);
    let t = (s * 0.08).max(2.0);
    match k {
        keys::REWIND | keys::FAST_FORWARD => {
            let dx = if k == keys::REWIND { -1.0 } else { 1.0 };
            for off in [-0.11, 0.11] {
                draw_triangle_pointing(cx + off * s, cy, dx, 0.0, s * 0.13, ink);
            }
        }
        keys::SAVE_STATE | keys::LOAD_STATE => {
            let (l, rt, bottom, top) = (cx - s * 0.26, cx + s * 0.26, cy + s * 0.24, cy + s * 0.06);
            draw_line(l, top, l, bottom, t, ink);
            draw_line(l - t / 2.0, bottom, rt + t / 2.0, bottom, t, ink);
            draw_line(rt, bottom, rt, top, t, ink);
            let (from, to) = if k == keys::SAVE_STATE { (cy - s * 0.3, cy + s * 0.02) } else { (cy + s * 0.12, cy - s * 0.2) };
            draw_line(cx, from, cx, to, t, ink);
            let dy = if k == keys::SAVE_STATE { 1.0 } else { -1.0 };
            draw_triangle_pointing(cx, to, 0.0, dy, s * 0.1, ink);
        }
        _ => {
            const STEPS: usize = 10;
            let radius = s * 0.22;
            let (from, to) = (-1.2f32, 3.6f32);
            let at = |a: f32| vec2(cx + radius * a.cos(), cy + radius * a.sin());
            for i in 0..STEPS {
                let (a, b) = (from + (to - from) * i as f32 / STEPS as f32, from + (to - from) * (i + 1) as f32 / STEPS as f32);
                draw_line(at(a).x, at(a).y, at(b).x, at(b).y, t, ink);
            }
            let end = at(from);
            draw_triangle_pointing(end.x, end.y, from.sin(), -from.cos(), s * 0.1, ink);
        }
    }
}

/// A text centred on `at`, `size` pixels high.
fn label(text: &str, at: Circle, size: f32, color: Color) {
    let size = size.round().max(8.0) as u16;
    let m = measure_text(text, None, size, 1.0);
    draw_text(text, at.x - m.width / 2.0, at.y + m.offset_y / 2.0, size as f32, color);
}

fn pill(r: Rect, color: Color) {
    rounded_by(r, r.h / 2.0, color);
}

fn rounded(r: Rect, color: Color) {
    rounded_by(r, r.w.min(r.h) * 0.2, color);
}

/// A rectangle with corners of radius `rad`, in pieces that do not overlap (the controls can be
/// see-through).
fn rounded_by(r: Rect, rad: f32, color: Color) {
    draw_rectangle(r.x + rad, r.y, r.w - 2.0 * rad, r.h, color);
    draw_rectangle(r.x, r.y + rad, rad, r.h - 2.0 * rad, color);
    draw_rectangle(r.x + r.w - rad, r.y + rad, rad, r.h - 2.0 * rad, color);
    let quarter = std::f32::consts::FRAC_PI_2;
    for (i, (x, y)) in [(r.x + r.w - rad, r.y + r.h - rad), (r.x + rad, r.y + r.h - rad), (r.x + rad, r.y + rad), (r.x + r.w - rad, r.y + rad)].into_iter().enumerate() {
        fan(x, y, rad, i as f32 * quarter, color);
    }
}

/// A quarter of a circle around `(x, y)` from angle `from` (y downwards).
fn fan(x: f32, y: f32, r: f32, from: f32, color: Color) {
    const STEPS: usize = 8;
    let at = |i: usize| {
        let a = from + std::f32::consts::FRAC_PI_2 * i as f32 / STEPS as f32;
        vec2(x + r * a.cos(), y + r * a.sin())
    };
    for i in 0..STEPS {
        draw_triangle(vec2(x, y), at(i), at(i + 1), color);
    }
}

/// A small triangle at `(x, y)` pointing along `(dx, dy)`.
fn draw_triangle_pointing(x: f32, y: f32, dx: f32, dy: f32, s: f32, color: Color) {
    let (px, py) = (-dy, dx);
    draw_triangle(vec2(x + dx * s, y + dy * s), vec2(x - dx * s + px * s, y - dy * s + py * s), vec2(x - dx * s - px * s, y - dy * s - py * s), color);
}

/// Four corners: pointing out to enter fullscreen, in to leave it.
pub(super) fn fullscreen_icon(r: Rect, leave: bool, color: Color) {
    let (cx, cy) = (r.x + r.w / 2.0, r.y + r.h / 2.0);
    let (far, near) = (r.w * 0.28, r.w * 0.1);
    let t = (r.w * 0.07).max(2.0);
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        let corner = if leave { (cx + sx * near, cy + sy * near) } else { (cx + sx * far, cy + sy * far) };
        let away = if leave { 1.0 } else { -1.0 };
        let len = far - near;
        draw_line(corner.0, corner.1, corner.0 + sx * away * len, corner.1, t, color);
        draw_line(corner.0, corner.1, corner.0, corner.1 + sy * away * len, t, color);
    }
}

/// The controller the session sees while the on-screen controls are there.
pub struct TouchPad<'a> {
    pad: &'a dyn Gamepad,
    touch: &'a TouchControls,
    bindings: [PadButton; KEYS],
    active: bool,
}

impl TouchPad<'_> {
    fn touched(&self, b: PadButton, went_down: bool) -> bool {
        if !self.active {
            return false;
        }
        if b == PadButton::Guide {
            return went_down && self.touch.menu();
        }
        (0..KEYS).filter(|&k| k != keys::MENU).any(|k| self.bindings[k] == b && if went_down { self.touch.pressed(k) } else { self.touch.held(k) })
    }
}

impl Gamepad for TouchPad<'_> {
    fn connected(&self) -> bool {
        self.active || self.pad.connected()
    }
    fn pressed(&self, b: PadButton) -> bool {
        self.pad.pressed(b) || self.touched(b, true)
    }
    fn held(&self, b: PadButton) -> bool {
        self.pad.held(b) || self.touched(b, false)
    }
    /// The real controller's only: binding a button is done on it.
    fn last_pressed(&self) -> Option<PadButton> {
        self.pad.last_pressed()
    }
}
