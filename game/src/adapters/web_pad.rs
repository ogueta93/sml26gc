//! The controller in the browser: the first one the page sees (Gamepad API, standard mapping),
//! read once per frame.
use crate::domain::pad::PadButton;
use crate::ports::Gamepad;
use std::collections::HashSet;

/// How far a stick goes to count as pressed, and back to count as let go.
const STICK_ON: f32 = 0.5;
const STICK_OFF: f32 = 0.3;

/// The standard mapping's buttons, by index.
const BUTTONS: [PadButton; 17] = [
    PadButton::South,
    PadButton::East,
    PadButton::West,
    PadButton::North,
    PadButton::LeftShoulder,
    PadButton::RightShoulder,
    PadButton::LeftTrigger,
    PadButton::RightTrigger,
    PadButton::Select,
    PadButton::Start,
    PadButton::LeftThumb,
    PadButton::RightThumb,
    PadButton::DPadUp,
    PadButton::DPadDown,
    PadButton::DPadLeft,
    PadButton::DPadRight,
    PadButton::Guide,
];

#[derive(Default)]
pub struct WebPad {
    connected: bool,
    held: HashSet<PadButton>,
    pressed: HashSet<PadButton>,
    last: Option<PadButton>,
}

impl WebPad {
    pub fn new() -> WebPad {
        WebPad::default()
    }

    /// Read the controller for this frame (the page only gives it while it is shown).
    pub fn poll(&mut self, _in_front: bool) {
        self.pressed.clear();
        self.last = None;
        let Some(state) = host::web::pad_state() else {
            self.connected = false;
            self.held.clear();
            return;
        };
        self.connected = true;
        for (i, &b) in BUTTONS.iter().enumerate() {
            self.set(b, state[i] > 0.5);
        }
        // the left stick: x right, y down
        let (x, y) = (state[17], state[18]);
        self.stick(x, PadButton::StickRight, PadButton::StickLeft);
        self.stick(-y, PadButton::StickUp, PadButton::StickDown);
    }

    fn set(&mut self, b: PadButton, down: bool) {
        if !down {
            self.held.remove(&b);
        } else if self.held.insert(b) {
            self.pressed.insert(b);
            self.last = Some(b);
        }
    }

    /// An axis as two buttons: `plus` past `STICK_ON`, `minus` below `-STICK_ON`.
    fn stick(&mut self, v: f32, plus: PadButton, minus: PadButton) {
        for (b, on) in [(plus, v), (minus, -v)] {
            if on >= STICK_ON {
                self.set(b, true);
            } else if on < STICK_OFF {
                self.set(b, false);
            }
        }
    }
}

impl Gamepad for WebPad {
    fn connected(&self) -> bool {
        self.connected
    }
    fn pressed(&self, b: PadButton) -> bool {
        self.pressed.contains(&b)
    }
    fn held(&self, b: PadButton) -> bool {
        self.held.contains(&b)
    }
    fn last_pressed(&self) -> Option<PadButton> {
        self.last
    }
}
