//! The views sit below a menu bar drawn in the window, when there is no native one: they draw
//! through a camera moved down by its height, and read the screen's size, the mouse and the keys
//! through these functions, which the view modules import by name so that they take the place
//! of macroquad's (an explicit import wins over the prelude's glob). While the bar takes the
//! mouse or the keys (a menu is open), the views get no clicks, wheel or keys.
use crate::ports::{MenuBar, Taken};
use macroquad::prelude as mq;
use macroquad::prelude::{KeyCode, MouseButton};
use std::cell::Cell;

thread_local! {
    static TOP: Cell<f32> = const { Cell::new(0.0) };
    static TAKEN: Cell<Taken> = const { Cell::new(Taken { mouse: false, keys: false }) };
}

/// Where the views start, in window pixels.
pub fn top() -> f32 {
    TOP.get()
}

/// Start a frame of the views below `menu`, which reads its input first.
pub fn begin(menu: &mut dyn MenuBar) {
    TAKEN.set(menu.input());
    let top = menu.height();
    TOP.set(top);
    if top > 0.0 {
        let (w, h) = (mq::screen_width(), mq::screen_height());
        seal_drawing();
        mq::set_camera(&mq::Camera2D { target: mq::vec2(w / 2.0, h / 2.0 - top), zoom: mq::vec2(2.0 / w, 2.0 / h), ..Default::default() });
    }
}

/// End the frame: `menu` drawn over the views.
pub fn end(menu: &mut dyn MenuBar) {
    seal_drawing();
    mq::set_default_camera();
    menu.draw();
}

/// A camera change sends the frame's drawing so far. Its last texture must not be a font's:
/// miniquad 0.4 remembers a deleted texture as still bound, and a font's glyph texture growing
/// later in the frame gets the deleted one's name, so its letters are never uploaded and every
/// text turns into blocks until the program ends (seen on macOS).
fn seal_drawing() {
    mq::draw_rectangle(0.0, 0.0, 1.0, 1.0, mq::Color::new(0.0, 0.0, 0.0, 0.0));
}

pub fn screen_width() -> f32 {
    mq::screen_width()
}

pub fn screen_height() -> f32 {
    mq::screen_height() - top()
}

/// Far outside the views while the bar has the mouse, so nothing shows as under it.
pub fn mouse_position() -> (f32, f32) {
    if TAKEN.get().mouse {
        return (-1.0e6, -1.0e6);
    }
    let (x, y) = mq::mouse_position();
    (x, y - top())
}

pub fn is_mouse_button_pressed(b: MouseButton) -> bool {
    !TAKEN.get().mouse && mq::is_mouse_button_pressed(b)
}

pub fn is_mouse_button_down(b: MouseButton) -> bool {
    !TAKEN.get().mouse && mq::is_mouse_button_down(b)
}

pub fn is_mouse_button_released(b: MouseButton) -> bool {
    !TAKEN.get().mouse && mq::is_mouse_button_released(b)
}

pub fn mouse_wheel() -> (f32, f32) {
    if TAKEN.get().mouse { (0.0, 0.0) } else { mq::mouse_wheel() }
}

pub fn is_key_pressed(k: KeyCode) -> bool {
    !TAKEN.get().keys && mq::is_key_pressed(k)
}

pub fn get_char_pressed() -> Option<char> {
    let c = mq::get_char_pressed();
    if TAKEN.get().keys { None } else { c }
}
