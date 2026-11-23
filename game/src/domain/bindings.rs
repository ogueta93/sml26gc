//! The bindings as pressed in one frame: the keyboard always plays, and a connected controller
//! plays too, each with its own bindings (`Settings::input` only chooses which ones the
//! Controls screen shows).
use super::keys;
use super::pad::{self, PadButton};
use super::settings::Settings;
use crate::ports::{Gamepad, Keyboard};
use gb_core::Buttons;

pub struct Bindings<'a> {
    keys: &'a dyn Keyboard,
    pad: &'a dyn Gamepad,
    st: &'a Settings,
}

impl<'a> Bindings<'a> {
    pub fn new(keys: &'a dyn Keyboard, pad: &'a dyn Gamepad, st: &'a Settings) -> Bindings<'a> {
        Bindings { keys, pad, st }
    }

    /// Is a controller connected (and so playing)?
    pub fn controller(&self) -> bool {
        self.pad.connected()
    }

    /// Binding `i` (`keys::UP`..`keys::MENU`) went down this frame, on either input.
    pub fn pressed(&self, i: usize) -> bool {
        self.keys.pressed(self.st.keys[i]) || (self.controller() && self.pad.pressed(self.st.pad[i]))
    }

    /// Binding `i` is down on either input.
    pub fn held(&self, i: usize) -> bool {
        self.keys.held(self.st.keys[i]) || (self.controller() && self.pad.held(self.st.pad[i]))
    }

    /// The Game Boy buttons held on either input.
    pub fn buttons(&self) -> Buttons {
        let b = keys::buttons(self.keys, &self.st.keys);
        if self.controller() {
            keys::no_opposites(b | pad::pad_buttons(self.pad, &self.st.pad))
        } else {
            b
        }
    }

    /// The controller's guide button went down (it opens the menu anywhere).
    pub fn guide(&self) -> bool {
        self.controller() && self.pad.pressed(PadButton::Guide)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::keys::Key;

    struct Keys(Vec<Key>);
    impl Keyboard for Keys {
        fn pressed(&self, k: Key) -> bool {
            self.0.contains(&k)
        }
        fn held(&self, k: Key) -> bool {
            self.0.contains(&k)
        }
        fn last_pressed(&self) -> Option<Key> {
            None
        }
    }

    struct Pad(bool, Vec<PadButton>);
    impl Gamepad for Pad {
        fn connected(&self) -> bool {
            self.0
        }
        fn pressed(&self, b: PadButton) -> bool {
            self.1.contains(&b)
        }
        fn held(&self, b: PadButton) -> bool {
            self.1.contains(&b)
        }
        fn last_pressed(&self) -> Option<PadButton> {
            None
        }
    }

    #[test]
    fn keyboard_and_controller_play_together() {
        let st = Settings::default();
        let keys = Keys(vec![Key::Right]);
        let pad = Pad(true, vec![PadButton::East]);
        assert_eq!(Bindings::new(&keys, &pad, &st).buttons(), Buttons::RIGHT | Buttons::A);
        let unplugged = Pad(false, vec![PadButton::East]);
        assert_eq!(Bindings::new(&keys, &unplugged, &st).buttons(), Buttons::RIGHT, "an unplugged controller does nothing");
        assert!(Bindings::new(&Keys(vec![Key::Escape]), &unplugged, &st).pressed(keys::MENU));
        assert!(Bindings::new(&Keys(vec![]), &Pad(true, vec![PadButton::Select]), &st).pressed(keys::MENU));
    }
}
