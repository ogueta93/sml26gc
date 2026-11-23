//! The inputs the computer or phone has now, as the adapters detect them: what the pause menu's
//! Controls offers to set.
use super::pad::Input;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Devices {
    pub keyboard: bool,
    pub controller: bool,
    pub touch: bool,
    /// The on-screen controls are on the screen now.
    pub touch_shown: bool,
}

/// Whose buttons the menu's footers and the start-up tag name: a connected controller's first
/// (it plays even with a keyboard there), else the on-screen controls' while shown, else the
/// keyboard's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Prompts {
    Keyboard,
    Controller,
    Touch,
}

impl Default for Devices {
    /// A computer: its keyboard.
    fn default() -> Devices {
        Devices { keyboard: true, controller: false, touch: false, touch_shown: false }
    }
}

impl Devices {
    /// The input whose bindings Controls shows: the one `chosen` when it is there, else the other
    /// one there, or None with neither.
    pub fn input(self, chosen: Input) -> Option<Input> {
        let has = |i: Input| match i {
            Input::Keyboard => self.keyboard,
            Input::Controller => self.controller,
        };
        let other = if chosen == Input::Keyboard { Input::Controller } else { Input::Keyboard };
        [chosen, other].into_iter().find(|&i| has(i))
    }

    pub fn prompts(self) -> Prompts {
        if self.controller {
            Prompts::Controller
        } else if self.touch_shown {
            Prompts::Touch
        } else {
            Prompts::Keyboard
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_input_shown_is_one_that_is_there() {
        let phone_with_pad = Devices { keyboard: false, controller: true, touch: true, touch_shown: true };
        assert_eq!(phone_with_pad.input(Input::Keyboard), Some(Input::Controller));
        assert_eq!(Devices::default().input(Input::Controller), Some(Input::Keyboard));
        let both = Devices { controller: true, ..Devices::default() };
        assert_eq!(both.input(Input::Controller), Some(Input::Controller));
        let phone = Devices { keyboard: false, controller: false, touch: true, touch_shown: true };
        assert_eq!(phone.input(Input::Keyboard), None);
    }

    #[test]
    fn the_controller_comes_first_in_the_prompts() {
        assert_eq!(Devices::default().prompts(), Prompts::Keyboard);
        assert_eq!(Devices { controller: true, touch_shown: true, ..Devices::default() }.prompts(), Prompts::Controller);
        assert_eq!(Devices { keyboard: false, touch: true, touch_shown: true, ..Devices::default() }.prompts(), Prompts::Touch);
    }
}
