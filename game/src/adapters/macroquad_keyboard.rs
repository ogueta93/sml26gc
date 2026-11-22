//! The keyboard through macroquad. On macOS, which keeps F11 for itself, Cmd+Ctrl+F (the
//! system's full screen keys) is also F11.
use crate::domain::keys::Key;
use crate::ports::Keyboard;
use macroquad::input::{get_last_key_pressed, is_key_down, is_key_pressed, KeyCode};

pub struct MacroquadKeyboard;

// The game's keys carry macroquad's names, so the translation is one list.
macro_rules! same_names {
    ($($k:ident)*) => {
        fn code(k: Key) -> KeyCode {
            match k { $(Key::$k => KeyCode::$k,)* }
        }
        fn key(c: KeyCode) -> Option<Key> {
            match c { $(KeyCode::$k => Some(Key::$k),)* _ => None }
        }
    };
}

same_names! {
    Up Down Left Right Space Enter KpEnter Backspace Tab LeftShift RightShift LeftControl RightControl LeftAlt
    A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
    Key0 Key1 Key2 Key3 Key4 Key5 Key6 Key7 Key8 Key9
    Kp0 Kp1 Kp2 Kp3 Kp4 Kp5 Kp6 Kp7 Kp8 Kp9
    Comma Period Minus Slash Escape F11 F1 F2 F3 F4 F5 F6 F7 F8 F9 F10
}

impl Keyboard for MacroquadKeyboard {
    fn pressed(&self, k: Key) -> bool {
        is_key_pressed(code(k)) || (k == Key::F11 && full_screen_keys())
    }
    fn held(&self, k: Key) -> bool {
        is_key_down(code(k))
    }
    fn last_pressed(&self) -> Option<Key> {
        get_last_key_pressed().and_then(key)
    }
}

#[cfg(target_os = "macos")]
fn full_screen_keys() -> bool {
    let held = |a, b| is_key_down(a) || is_key_down(b);
    held(KeyCode::LeftSuper, KeyCode::RightSuper) && held(KeyCode::LeftControl, KeyCode::RightControl) && is_key_pressed(KeyCode::F)
}

#[cfg(not(target_os = "macos"))]
fn full_screen_keys() -> bool {
    false
}
