//! The on-screen controls for a touch screen: where the picture and each control go for a
//! screen's size (a handheld's layout upright, the controls beside the picture across), and
//! which Game Boy buttons the fingers on them hold. Positions are in the window's points.
use crate::domain::keys;

/// When the controls show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TouchShow {
    /// From a touch until a key or a controller button.
    #[default]
    Auto,
    Always,
    Never,
}

impl TouchShow {
    pub const ALL: [TouchShow; 3] = [TouchShow::Auto, TouchShow::Always, TouchShow::Never];

    pub fn id(self) -> &'static str {
        match self {
            TouchShow::Auto => "auto",
            TouchShow::Always => "always",
            TouchShow::Never => "never",
        }
    }

    pub fn from_id(id: &str) -> Option<TouchShow> {
        TouchShow::ALL.into_iter().find(|t| t.id() == id)
    }
}

/// How large the controls are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TouchSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl TouchSize {
    pub const ALL: [TouchSize; 3] = [TouchSize::Small, TouchSize::Medium, TouchSize::Large];

    pub fn id(self) -> &'static str {
        match self {
            TouchSize::Small => "small",
            TouchSize::Medium => "medium",
            TouchSize::Large => "large",
        }
    }

    pub fn from_id(id: &str) -> Option<TouchSize> {
        TouchSize::ALL.into_iter().find(|t| t.id() == id)
    }

    /// The D-pad's width as a part of the screen's shorter side.
    fn part(self) -> f32 {
        match self {
            TouchSize::Small => 0.30,
            TouchSize::Medium => 0.36,
            TouchSize::Large => 0.42,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }

    /// Grown by `m` on every side.
    pub fn grown(&self, m: f32) -> Rect {
        Rect::new(self.x - m, self.y - m, self.w + 2.0 * m, self.h + 2.0 * m)
    }

    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Circle {
    pub x: f32,
    pub y: f32,
    pub r: f32,
}

impl Circle {
    fn distance(&self, x: f32, y: f32) -> f32 {
        ((x - self.x).powi(2) + (y - self.y).powi(2)).sqrt()
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x - self.r, self.y - self.r, 2.0 * self.r, 2.0 * self.r)
    }
}

/// How much further than its drawing a button responds, as a part of its size.
const REACH: f32 = 1.3;
/// The D-pad's middle that presses nothing, as a part of its radius.
const DEAD_ZONE: f32 = 0.2;
/// Around the point between A and B, the part of a button's radius that presses both.
const BOTH_ZONE: f32 = 0.6;
/// The port's own actions on screen, in their order (`keys` indexes): rewind, fast forward,
/// save and load state, restart the level. Smaller than the menu button, apart from the Game
/// Boy's buttons.
pub const EXTRAS: [usize; 5] = [keys::REWIND, keys::FAST_FORWARD, keys::SAVE_STATE, keys::LOAD_STATE, keys::RESTART];
const EXTRA_SIZE: f32 = 0.85;
/// The least an action's button shrinks to, to fit the screen.
const EXTRA_MIN: f32 = 28.0;

/// Where everything goes on a screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout {
    pub picture: Rect,
    pub dpad: Circle,
    pub a: Circle,
    pub b: Circle,
    pub select: Rect,
    pub start: Rect,
    pub menu: Rect,
    /// None where the page cannot go fullscreen.
    pub fullscreen: Option<Rect>,
    /// The port's actions that work now (`EXTRAS`' `keys` indexes) and where they are.
    pub extras: [Option<(usize, Rect)>; EXTRAS.len()],
    /// The controls lie over the picture: drawn see-through.
    pub over_picture: bool,
}

impl Layout {
    /// The layout for a `w` by `h` screen; `fullscreen`: there is a fullscreen button.
    pub fn new(w: f32, h: f32, size: TouchSize, fullscreen: bool) -> Layout {
        let short = w.min(h);
        let d = (short * size.part()).clamp(96.0, 240.0);
        let ab = d * 0.42;
        let pad = d * 0.12;
        let pill = (d * 0.44, (d * 0.16).max(26.0));
        let small = (d * 0.24).max(40.0);
        let square = |x: f32, y: f32| Rect::new(x, y, small, small);
        if h > w {
            // the handheld: the picture on top, the controls below it
            let need = pad + small + d + pill.1 + 3.0 * pad;
            let mut pic_h = w * 144.0 / 160.0;
            if h - pic_h < need {
                pic_h = (h - need).max(h * 0.35);
            }
            let pic_w = (pic_h * 160.0 / 144.0).min(w);
            let picture = Rect::new((w - pic_w) / 2.0, 0.0, pic_w, pic_h);
            let top = pic_h;
            let menu = square(w - pad - small, top + pad);
            let shut = h - 2.0 * pad - pill.1;
            let cy = ((top + 2.0 * pad + small + shut) / 2.0).max(top + 2.0 * pad + small + d / 2.0 - small);
            let dpad = Circle { x: pad + d / 2.0, y: cy, r: d / 2.0 };
            let a = Circle { x: w - pad - ab / 2.0, y: cy - ab * 0.35, r: ab / 2.0 };
            let b = Circle { x: a.x - ab * 1.3, y: cy + ab * 0.35, r: ab / 2.0 };
            let py = h - pad - pill.1;
            Layout {
                picture,
                dpad,
                a,
                b,
                select: Rect::new(w / 2.0 - pill.0 - pad / 2.0, py, pill.0, pill.1),
                start: Rect::new(w / 2.0 + pad / 2.0, py, pill.0, pill.1),
                menu,
                fullscreen: fullscreen.then(|| square(menu.x - pad - small, menu.y)),
                extras: [None; EXTRAS.len()],
                over_picture: false,
            }
        } else {
            // across: the picture in the middle, the controls on its sides
            let pic_h = h;
            let pic_w = (h * 160.0 / 144.0).min(w);
            let picture = Rect::new((w - pic_w) / 2.0, (h - pic_h) / 2.0, pic_w, pic_h);
            let cy = h * 0.55;
            let dpad = Circle { x: pad + d / 2.0, y: cy, r: d / 2.0 };
            let a = Circle { x: w - pad - ab / 2.0, y: cy - ab * 0.35, r: ab / 2.0 };
            let b = Circle { x: a.x - ab * 1.3, y: cy + ab * 0.35, r: ab / 2.0 };
            let py = h - pad - pill.1;
            let menu = square(w - pad - small, pad);
            let mut layout = Layout {
                picture,
                dpad,
                a,
                b,
                select: Rect::new(dpad.x - pill.0 / 2.0, py, pill.0, pill.1),
                start: Rect::new((a.x + b.x) / 2.0 - pill.0 / 2.0, py, pill.0, pill.1),
                menu,
                fullscreen: fullscreen.then(|| square(menu.x - pad - small, menu.y)),
                extras: [None; EXTRAS.len()],
                over_picture: false,
            };
            layout.over_picture = layout.controls().iter().any(|r| r.overlaps(&picture));
            layout
        }
    }

    /// With the buttons of the port's actions in `shown` (of `EXTRAS`, in its order): upright, in
    /// a row left of the menu button; across, in rows above the D-pad.
    pub fn with_extras(mut self, shown: &[usize]) -> Layout {
        let keys: Vec<usize> = EXTRAS.iter().copied().filter(|k| shown.contains(k)).collect();
        let n = keys.len() as f32;
        if keys.is_empty() {
            return self;
        }
        let pad = self.dpad.x - self.dpad.r;
        let upright = self.picture.y + self.picture.h <= self.menu.y;
        let (x0, y0, room_w, room_h) = if upright {
            let end = self.fullscreen.map_or(self.menu.x, |f| f.x) - pad;
            (pad, self.menu.y, end - pad, self.menu.h)
        } else {
            (pad, pad, (2.0 * self.dpad.r).max(self.select.w), self.dpad.y - self.dpad.r - 2.0 * pad)
        };
        let mut size = self.menu.w * EXTRA_SIZE;
        let fits = |s: f32| {
            let gap = s * 0.25;
            let columns = ((room_w + gap) / (s + gap)).floor().max(1.0);
            let rows = (n / columns).ceil();
            (rows * s + (rows - 1.0) * gap <= room_h && (upright.then_some(rows <= 1.0).unwrap_or(true))).then_some(columns as usize)
        };
        while fits(size).is_none() && size > EXTRA_MIN {
            size -= 2.0;
        }
        let columns = fits(size).unwrap_or(keys.len());
        let gap = size * 0.25;
        for (i, &k) in keys.iter().enumerate() {
            let (c, r) = ((i % columns) as f32, (i / columns) as f32);
            self.extras[i] = Some((k, Rect::new(x0 + c * (size + gap), y0 + r * (size + gap) + if upright { (room_h - size) / 2.0 } else { 0.0 }, size, size)));
        }
        if !upright {
            self.over_picture = self.controls().iter().any(|r| r.overlaps(&self.picture));
        }
        self
    }

    /// The picture given all the room the controls leave (to be stretched into it): upright,
    /// down to the row of small buttons, which moves down next to the D-pad and the buttons;
    /// across, between the controls of either side.
    pub fn stretched(mut self) -> Layout {
        let pad = self.dpad.x - self.dpad.r;
        let w = 2.0 * self.picture.x + self.picture.w;
        if self.picture.y + self.picture.h <= self.menu.y {
            let cluster = [self.dpad, self.a, self.b].iter().map(|c| c.y - c.r).fold(f32::MAX, f32::min);
            let dy = (cluster - pad - self.menu.h - self.menu.y).max(0.0);
            self.menu.y += dy;
            if let Some(f) = self.fullscreen.as_mut() {
                f.y += dy;
            }
            for (_, r) in self.extras.iter_mut().flatten() {
                r.y += dy;
            }
            let row_top = self.extras.iter().flatten().map(|e| e.1.y).fold(self.menu.y, f32::min);
            self.picture = Rect::new(0.0, 0.0, w, row_top - pad);
        } else {
            let (mut left, mut right) = (0.0f32, w);
            for r in self.controls() {
                if r.x + r.w / 2.0 < w / 2.0 {
                    left = left.max(r.x + r.w);
                } else {
                    right = right.min(r.x);
                }
            }
            let room = Rect::new(left + pad, 0.0, right - left - 2.0 * pad, self.picture.h);
            if room.w > self.picture.w {
                self.picture = room;
            }
            self.over_picture = self.controls().iter().any(|r| r.overlaps(&self.picture));
        }
        self
    }

    /// Every control's drawn area.
    pub fn controls(&self) -> Vec<Rect> {
        let mut v = vec![self.dpad.bounds(), self.a.bounds(), self.b.bounds(), self.select, self.start, self.menu];
        v.extend(self.fullscreen);
        v.extend(self.extras.iter().flatten().map(|e| e.1));
        v
    }

    /// The Game Boy buttons (`keys` indexes) a finger at `(x, y)` presses among A, B, Start and
    /// Select: one, both A and B between them, or none.
    fn buttons_at(&self, x: f32, y: f32) -> Vec<usize> {
        let mid = Circle { x: (self.a.x + self.b.x) / 2.0, y: (self.a.y + self.b.y) / 2.0, r: self.a.r * BOTH_ZONE };
        if mid.distance(x, y) <= mid.r {
            return vec![keys::A, keys::B];
        }
        let near = [(self.a, keys::A), (self.b, keys::B)].into_iter().filter(|(c, _)| c.distance(x, y) <= c.r * REACH).min_by(|p, q| p.0.distance(x, y).total_cmp(&q.0.distance(x, y)));
        if let Some((_, k)) = near {
            return vec![k];
        }
        let reach = |r: &Rect| r.grown(r.h * (REACH - 1.0) * 2.0).contains(x, y);
        if reach(&self.select) {
            vec![keys::SELECT]
        } else if reach(&self.start) {
            vec![keys::START]
        } else {
            Vec::new()
        }
    }

    /// The directions (`keys` indexes) the D-pad gives for a finger at `(x, y)`, wherever it is:
    /// eight directions by the angle from the D-pad's middle, none in its dead zone.
    fn directions_at(&self, x: f32, y: f32) -> Vec<usize> {
        let (dx, dy) = (x - self.dpad.x, y - self.dpad.y);
        if (dx * dx + dy * dy).sqrt() < self.dpad.r * DEAD_ZONE {
            return Vec::new();
        }
        let sector = ((dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8);
        match sector {
            0 => vec![keys::RIGHT],
            1 => vec![keys::RIGHT, keys::DOWN],
            2 => vec![keys::DOWN],
            3 => vec![keys::LEFT, keys::DOWN],
            4 => vec![keys::LEFT],
            5 => vec![keys::LEFT, keys::UP],
            6 => vec![keys::UP],
            _ => vec![keys::RIGHT, keys::UP],
        }
    }

    /// What a finger beginning at `(x, y)` belongs to.
    fn control_at(&self, x: f32, y: f32) -> Option<Control> {
        if let Some((k, _)) = self.extras.iter().flatten().find(|(_, r)| r.grown(r.w * 0.1).contains(x, y)) {
            Some(Control::Extra(*k))
        } else if self.dpad.distance(x, y) <= self.dpad.r * 1.25 {
            Some(Control::DPad)
        } else if !self.buttons_at(x, y).is_empty() {
            Some(Control::Buttons)
        } else if self.menu.grown(self.menu.w * 0.2).contains(x, y) {
            Some(Control::Menu)
        } else if self.fullscreen.is_some_and(|r| r.grown(r.w * 0.2).contains(x, y)) {
            Some(Control::Fullscreen)
        } else {
            None
        }
    }
}

/// What a finger began on, and keeps for as long as it is down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    /// Follows the finger anywhere, past its edge too.
    DPad,
    /// A, B, Start or Select: whichever the finger is on now.
    Buttons,
    Menu,
    /// The page itself enters or leaves fullscreen.
    Fullscreen,
    /// One of the port's actions (a `keys` index), held while the finger stays.
    Extra(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Began,
    Moved,
    Ended,
}

/// A finger on the screen this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Finger {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub phase: Phase,
}

/// The fingers on the controls, frame after frame.
#[derive(Default)]
pub struct TouchControls {
    owners: Vec<(u64, Control)>,
    held: [bool; keys::KEYS],
    went_down: [bool; keys::KEYS],
    menu: bool,
}

impl TouchControls {
    /// Read this frame's fingers on `layout`.
    pub fn update(&mut self, layout: &Layout, fingers: &[Finger]) {
        let before = self.held;
        self.held = [false; keys::KEYS];
        self.menu = false;
        for f in fingers {
            let known = self.owners.iter().find(|(id, _)| *id == f.id).map(|o| o.1);
            // a tap whose start the same frame hid: it presses for this frame
            let tap = f.phase == Phase::Ended && known.is_none();
            let owner = match f.phase {
                Phase::Ended => {
                    self.owners.retain(|(id, _)| *id != f.id);
                    if tap { layout.control_at(f.x, f.y) } else { None }
                }
                Phase::Began => {
                    self.owners.retain(|(id, _)| *id != f.id);
                    let c = layout.control_at(f.x, f.y);
                    self.owners.extend(c.map(|c| (f.id, c)));
                    c
                }
                Phase::Moved => known,
            };
            self.menu |= (f.phase == Phase::Began || tap) && owner == Some(Control::Menu);
            let pressed = match owner {
                Some(Control::DPad) => layout.directions_at(f.x, f.y),
                Some(Control::Buttons) => layout.buttons_at(f.x, f.y),
                Some(Control::Extra(k)) => vec![k],
                _ => Vec::new(),
            };
            for k in pressed {
                self.held[k] = true;
            }
        }
        for k in 0..self.held.len() {
            self.went_down[k] = self.held[k] && !before[k];
        }
    }

    /// Binding `k` (a Game Boy button, or one of `EXTRAS`) is held.
    pub fn held(&self, k: usize) -> bool {
        self.held.get(k).copied().unwrap_or(false)
    }

    /// Went down this frame.
    pub fn pressed(&self, k: usize) -> bool {
        self.went_down.get(k).copied().unwrap_or(false)
    }

    /// Any button went down this frame (to vibrate).
    pub fn any_pressed(&self) -> bool {
        self.went_down.iter().any(|&p| p) || self.menu
    }

    /// The menu button was touched this frame.
    pub fn menu(&self) -> bool {
        self.menu
    }
}

/// Whether the controls are shown: with Auto, from a touch until a key or controller button.
#[derive(Default)]
pub struct Visibility {
    touch_last: bool,
}

impl Visibility {
    pub fn update(&mut self, touched: bool, other_input: bool) {
        if touched {
            self.touch_last = true;
        } else if other_input {
            self.touch_last = false;
        }
    }

    pub fn shown(&self, show: TouchShow) -> bool {
        match show {
            TouchShow::Auto => self.touch_last,
            TouchShow::Always => true,
            TouchShow::Never => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PHONE_UP: (f32, f32) = (390.0, 844.0);
    const PHONE_ACROSS: (f32, f32) = (844.0, 390.0);

    fn finger(id: u64, x: f32, y: f32, phase: Phase) -> Finger {
        Finger { id, x, y, phase }
    }

    fn held(t: &TouchControls) -> Vec<usize> {
        (0..=keys::SELECT).filter(|&k| t.held(k)).collect()
    }

    #[test]
    fn upright_the_picture_fills_the_width_and_nothing_covers_it() {
        for size in TouchSize::ALL {
            for (w, h) in [PHONE_UP, (360.0, 640.0), (768.0, 1024.0)] {
                let l = Layout::new(w, h, size, true);
                assert!(!l.over_picture);
                assert!(l.controls().iter().all(|r| !r.overlaps(&l.picture)), "{size:?} {w}x{h}");
                assert!(l.controls().iter().all(|r| r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w && r.y + r.h <= h), "{size:?} {w}x{h}: inside the screen");
            }
        }
        assert_eq!(Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false).picture.w, PHONE_UP.0);
    }

    #[test]
    fn across_the_controls_are_on_the_sides() {
        let l = Layout::new(PHONE_ACROSS.0, PHONE_ACROSS.1, TouchSize::Medium, true);
        assert_eq!(l.picture.h, PHONE_ACROSS.1);
        assert!(l.dpad.x < l.picture.x && l.a.x > l.picture.x + l.picture.w);
        assert!(l.select.y > l.dpad.y && l.menu.y < l.dpad.y);
        let wide = Layout::new(640.0, 400.0, TouchSize::Large, false);
        assert!(wide.over_picture, "controls over the picture are drawn see-through");
    }

    #[test]
    fn larger_controls_are_larger() {
        let (w, h) = PHONE_UP;
        let r = |s| Layout::new(w, h, s, false).dpad.r;
        assert!(r(TouchSize::Small) < r(TouchSize::Medium) && r(TouchSize::Medium) < r(TouchSize::Large));
    }

    #[test]
    fn a_jumps_and_the_menu_button_opens_the_menu() {
        let l = Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false);
        let mut t = TouchControls::default();
        t.update(&l, &[finger(1, l.a.x, l.a.y, Phase::Began)]);
        assert_eq!(held(&t), [keys::A]);
        assert!(t.pressed(keys::A) && t.any_pressed());
        t.update(&l, &[finger(1, l.a.x, l.a.y, Phase::Moved)]);
        assert!(t.held(keys::A) && !t.pressed(keys::A));
        t.update(&l, &[finger(1, l.a.x, l.a.y, Phase::Ended)]);
        assert!(held(&t).is_empty());
        t.update(&l, &[finger(2, l.menu.x + 5.0, l.menu.y + 5.0, Phase::Began)]);
        assert!(t.menu() && held(&t).is_empty());
    }

    #[test]
    fn the_d_pad_follows_the_finger_past_its_edge() {
        let l = Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false);
        let (cx, cy, r) = (l.dpad.x, l.dpad.y, l.dpad.r);
        let mut t = TouchControls::default();
        t.update(&l, &[finger(1, cx + r * 0.7, cy, Phase::Began)]);
        assert_eq!(held(&t), [keys::RIGHT]);
        t.update(&l, &[finger(1, cx + r * 0.7, cy + r * 0.7, Phase::Moved)]);
        assert_eq!(held(&t), [keys::DOWN, keys::RIGHT], "right and down");
        t.update(&l, &[finger(1, cx + r * 2.5, cy + r * 2.4, Phase::Moved)]);
        assert_eq!(held(&t), [keys::DOWN, keys::RIGHT], "beyond the drawn edge");
        t.update(&l, &[finger(1, cx + 1.0, cy - 1.0, Phase::Moved)]);
        assert!(held(&t).is_empty(), "the middle presses nothing");
        t.update(&l, &[finger(1, cx, cy - r * 0.8, Phase::Moved)]);
        assert_eq!(held(&t), [keys::UP]);
    }

    #[test]
    fn running_and_jumping_with_two_fingers() {
        let l = Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false);
        let mut t = TouchControls::default();
        t.update(&l, &[finger(1, l.dpad.x + l.dpad.r * 0.8, l.dpad.y, Phase::Began), finger(2, l.b.x, l.b.y, Phase::Began)]);
        assert_eq!(held(&t), [keys::RIGHT, keys::B]);
        let between = ((l.a.x + l.b.x) / 2.0, (l.a.y + l.b.y) / 2.0);
        t.update(&l, &[finger(1, l.dpad.x + l.dpad.r * 0.8, l.dpad.y, Phase::Moved), finger(2, between.0, between.1, Phase::Moved)]);
        assert_eq!(held(&t), [keys::RIGHT, keys::A, keys::B], "between A and B presses both");
        assert!(t.pressed(keys::A) && !t.pressed(keys::B));
        t.update(&l, &[finger(1, l.dpad.x + l.dpad.r * 0.8, l.dpad.y, Phase::Moved), finger(2, l.a.x, l.a.y, Phase::Moved)]);
        assert_eq!(held(&t), [keys::RIGHT, keys::A], "slid onto A: B let go");
    }

    #[test]
    fn a_finger_off_every_button_presses_nothing_and_comes_back() {
        let l = Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false);
        let mut t = TouchControls::default();
        t.update(&l, &[finger(1, l.start.x + 2.0, l.start.y + 2.0, Phase::Began)]);
        assert_eq!(held(&t), [keys::START]);
        t.update(&l, &[finger(1, l.picture.x + 10.0, l.picture.y + 10.0, Phase::Moved)]);
        assert!(held(&t).is_empty());
        t.update(&l, &[finger(1, l.select.x + 2.0, l.select.y + 2.0, Phase::Moved)]);
        assert_eq!(held(&t), [keys::SELECT]);
    }

    #[test]
    fn the_actions_keep_apart_from_everything() {
        for size in TouchSize::ALL {
            for (w, h) in [PHONE_UP, (360.0, 640.0), PHONE_ACROSS, (640.0, 360.0), (768.0, 1024.0), (1024.0, 768.0)] {
                let l = Layout::new(w, h, size, true).with_extras(&EXTRAS);
                let all = l.controls();
                assert_eq!(l.extras.iter().flatten().count(), EXTRAS.len());
                for (i, r) in all.iter().enumerate() {
                    assert!(r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= w && r.y + r.h <= h, "{size:?} {w}x{h}: {r:?} inside");
                    assert!(all.iter().skip(i + 1).all(|o| !r.overlaps(o)), "{size:?} {w}x{h}: {r:?} apart");
                }
                if h > w {
                    assert!(all.iter().all(|r| !r.overlaps(&l.picture)), "{size:?} {w}x{h}: off the picture");
                }
                let (_, ff) = l.extras.iter().flatten().find(|e| e.0 == keys::FAST_FORWARD).copied().unwrap();
                assert!(ff.w < l.menu.w && ff.w < l.a.r * 2.0, "smaller than the main buttons");
            }
        }
    }

    #[test]
    fn stretched_the_picture_takes_the_room_the_controls_leave() {
        for size in TouchSize::ALL {
            for (w, h) in [PHONE_UP, (360.0, 640.0), PHONE_ACROSS, (640.0, 360.0), (768.0, 1024.0), (1024.0, 768.0)] {
                let fitted = Layout::new(w, h, size, true).with_extras(&EXTRAS);
                let l = fitted.stretched();
                assert!(l.picture.w >= fitted.picture.w && l.picture.h >= fitted.picture.h, "{size:?} {w}x{h}: larger");
                let all = l.controls();
                for (i, r) in all.iter().enumerate() {
                    assert!(r.y + r.h <= h && all.iter().skip(i + 1).all(|o| !r.overlaps(o)), "{size:?} {w}x{h}: {r:?}");
                }
                if h > w {
                    assert!(all.iter().all(|r| !r.overlaps(&l.picture)), "{size:?} {w}x{h}: off the picture");
                }
            }
        }
        let across = Layout::new(PHONE_ACROSS.0, PHONE_ACROSS.1, TouchSize::Medium, true).stretched();
        assert!(across.picture.w > across.picture.h * 160.0 / 144.0 && !across.over_picture, "across, wider than its shape between the controls");
    }

    #[test]
    fn fast_forward_is_held_by_touch() {
        let l = Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false).with_extras(&[keys::FAST_FORWARD]);
        let (_, r) = l.extras[0].unwrap();
        let mut t = TouchControls::default();
        t.update(&l, &[finger(1, r.x + 4.0, r.y + 4.0, Phase::Began)]);
        t.update(&l, &[finger(1, r.x + 4.0, r.y + 4.0, Phase::Moved)]);
        assert!(t.held(keys::FAST_FORWARD) && !t.held(keys::A));
    }

    #[test]
    fn a_tap_within_one_frame_still_presses() {
        let l = Layout::new(PHONE_UP.0, PHONE_UP.1, TouchSize::Medium, false);
        let mut t = TouchControls::default();
        t.update(&l, &[finger(7, l.a.x, l.a.y, Phase::Ended)]);
        assert!(t.pressed(keys::A) && t.held(keys::A), "pressed for the frame");
        t.update(&l, &[]);
        assert!(!t.held(keys::A), "and let go the next");
        t.update(&l, &[finger(8, l.menu.x + 5.0, l.menu.y + 5.0, Phase::Ended)]);
        assert!(t.menu());
    }

    #[test]
    fn shown_with_a_touch_hidden_with_a_key() {
        let mut v = Visibility::default();
        assert!(!v.shown(TouchShow::Auto) && v.shown(TouchShow::Always));
        v.update(true, false);
        assert!(v.shown(TouchShow::Auto) && !v.shown(TouchShow::Never));
        v.update(false, true);
        assert!(!v.shown(TouchShow::Auto));
    }
}
