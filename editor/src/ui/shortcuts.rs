//! Every keyboard shortcut of the editor: the keys that run each panel action and the text
//! its button shows, from one table. The table writes Ctrl for the system's shortcut key
//! (`host::shortcut_key`: Cmd on macOS), which reads and shows the one of the system.
//!
//! Keys are read by their place on the keyboard (US layout), so only letters, comma, period
//! and named keys are used: they sit in the same place on a Spanish keyboard. Plus and minus
//! accept the keys that carry them on both layouts and the keypad.
use macroquad::prelude::{is_key_down, KeyCode};
use super::inset::is_key_pressed;
use host::shortcut_key::{ShortcutKey, SHORTCUT_KEY};

/// Where the editor is: which keys mean what depends on it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ctx {
    Blocks,
    Objects,
    Picture,
    Points,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Undo,
    Redo,
    Save,
    Play,
    Grid,
    Pencil,
    BoxFill,
    Select,
    Overview,
    BlocksObjects,
    PrevLevel,
    NextLevel,
    NewLevel,
    NewBlankLevel,
    ObjectCatalogue,
    RecaptureObjects,
    /// Leave the project for the project screen.
    Projects,
    DeleteLevel,
    WorldEditor,
    PrevMap,
    NextMap,
    SwitchTab,
    Restore,
    ZoomOut,
    ZoomIn,
    Copy,
    Cut,
    Paste,
    ClearSelection,
    Escape,
    PlaceStart,
    PlaceFinish,
    PlaceSecret,
    SecretLeads,
    PlaceCheckpoint,
    PlayFromHere,
    /// Play again from the last place Play here was used in the level shown.
    PlayHereAgain,
    DeleteObject,
    PrevPoint,
    NextPoint,
    PrevOpens,
    NextOpens,
    ChooseTarget,
    MakeExit1,
    MakeExit2,
    DeletePathRight,
    DeletePathLeft,
    DeletePathUp,
    DeletePathDown,
    ShutRight,
    ShutLeft,
    ShutUp,
    ShutDown,
    NewPath,
    NewPoint,
    DeletePoint,
    MakeEntry,
    /// Mario's start on the chosen point.
    SetStart,
    /// Show the map and point of Mario's start.
    GoToStart,
    /// Mario's start back where the original game begins.
    GameStart,
    NewZone,
    /// An empty new zone: its graphics chosen from a base zone.
    BlankZone,
    DeleteZone,
    ExportLevel,
    ImportLevel,
    ReplaceLevel,
    ExportZone,
    ImportZone,
    ReplaceZone,
    ExportProject,
}

impl Action {
    /// Actions that open a file dialog.
    pub fn uses_files(self) -> bool {
        matches!(self, ExportLevel | ImportLevel | ReplaceLevel | ExportZone | ImportZone | ReplaceZone | ExportProject)
    }
}

pub struct Shortcut {
    pub action: Action,
    ctxs: &'static [Ctx],
    ctrl: bool,
    shift: bool,
    keys: &'static [KeyCode],
    pub label: &'static str,
}

use Action::*;
use Ctx::*;
use KeyCode as K;

const ALL: &[Ctx] = &[Blocks, Objects, Picture, Points];
const LEVEL: &[Ctx] = &[Blocks, Objects];
const WORLD: &[Ctx] = &[Picture, Points];
const PAINT: &[Ctx] = &[Blocks, Picture];
const PALETTES: &[Ctx] = &[Blocks, Objects, Picture];

const fn s(action: Action, ctxs: &'static [Ctx], ctrl: bool, shift: bool, keys: &'static [KeyCode], label: &'static str) -> Shortcut {
    Shortcut { action, ctxs, ctrl, shift, keys, label }
}

pub const TABLE: &[Shortcut] = &[
    s(Undo, ALL, true, false, &[K::Z], "Ctrl+Z"),
    s(Redo, ALL, true, false, &[K::Y], "Ctrl+Y"),
    s(Redo, ALL, true, true, &[K::Z], "Ctrl+Shift+Z"),
    s(Save, ALL, true, false, &[K::S], "Ctrl+S"),
    s(Play, ALL, false, false, &[K::F5], "F5"),
    s(WorldEditor, ALL, false, false, &[K::F2], "F2"),
    s(Restore, ALL, true, true, &[K::R], "Ctrl+Shift+R"),
    s(Grid, PALETTES, false, false, &[K::G], "G"),
    s(Pencil, PAINT, false, false, &[K::B], "B"),
    s(BoxFill, PAINT, false, false, &[K::R], "R"),
    s(Select, PAINT, false, false, &[K::E], "E"),
    s(Copy, PAINT, true, false, &[K::C], "Ctrl+C"),
    s(Cut, PAINT, true, false, &[K::X], "Ctrl+X"),
    s(Paste, PAINT, true, false, &[K::V], "Ctrl+V"),
    s(ClearSelection, PAINT, false, false, &[K::Delete], "Del"),
    s(Escape, PALETTES, false, false, &[K::Escape], "Esc"),
    s(PlaceStart, LEVEL, false, false, &[K::I], "I"),
    s(PlaceFinish, LEVEL, false, false, &[K::F], "F"),
    s(PlaceSecret, LEVEL, false, true, &[K::F], "Shift+F"),
    s(SecretLeads, LEVEL, false, false, &[K::U], "U"),
    s(PlaceCheckpoint, LEVEL, false, false, &[K::K], "K"),
    s(PlayFromHere, ALL, false, true, &[K::F5], "Shift+F5"),
    s(PlayHereAgain, LEVEL, true, false, &[K::F5], "Ctrl+F5"),
    s(ZoomOut, PALETTES, true, false, &[K::Minus, K::Slash, K::KpSubtract], "Ctrl+-"),
    s(ZoomIn, PALETTES, true, false, &[K::Equal, K::RightBracket, K::KpAdd], "Ctrl++"),
    s(ExportProject, ALL, true, true, &[K::E], "Ctrl+Shift+E"),
    s(Projects, ALL, true, true, &[K::O], "Ctrl+Shift+O"),
    s(Overview, LEVEL, false, false, &[K::M], "M"),
    s(BlocksObjects, LEVEL, false, false, &[K::Tab, K::O], "Tab"),
    s(PrevLevel, LEVEL, false, false, &[K::PageUp], "PgUp"),
    s(NextLevel, LEVEL, false, false, &[K::PageDown], "PgDn"),
    s(NewLevel, LEVEL, true, false, &[K::N], "Ctrl+N"),
    s(NewBlankLevel, LEVEL, true, false, &[K::B], "Ctrl+B"),
    s(ObjectCatalogue, LEVEL, false, false, &[K::F3], "F3"),
    s(DeleteLevel, LEVEL, true, false, &[K::Delete], "Ctrl+Del"),
    s(ExportLevel, LEVEL, true, false, &[K::E], "Ctrl+E"),
    s(ImportLevel, LEVEL, true, true, &[K::N], "Ctrl+Shift+N"),
    s(ReplaceLevel, LEVEL, true, false, &[K::I], "Ctrl+I"),
    s(ExportZone, WORLD, true, false, &[K::E], "Ctrl+E"),
    s(ImportZone, WORLD, true, true, &[K::N], "Ctrl+Shift+N"),
    s(ReplaceZone, WORLD, true, false, &[K::I], "Ctrl+I"),
    s(DeleteObject, &[Objects], false, false, &[K::Delete, K::Backspace], "Del"),
    s(PrevMap, WORLD, false, false, &[K::PageUp], "PgUp"),
    s(NextMap, WORLD, false, false, &[K::PageDown], "PgDn"),
    s(SwitchTab, WORLD, false, false, &[K::Tab], "Tab"),
    s(PrevPoint, &[Points], false, false, &[K::Comma], ","),
    s(NextPoint, &[Points], false, false, &[K::Period], "."),
    s(PrevOpens, &[Points], false, false, &[K::J], "J"),
    s(NextOpens, &[Points], false, false, &[K::K], "K"),
    s(ChooseTarget, &[Points], false, false, &[K::U], "U"),
    s(MakeExit1, &[Points], false, false, &[K::T], "T"),
    s(MakeExit2, &[Points], false, true, &[K::T], "Shift+T"),
    s(DeletePathRight, &[Points], false, true, &[K::Right], "Shift+Right"),
    s(DeletePathLeft, &[Points], false, true, &[K::Left], "Shift+Left"),
    s(DeletePathUp, &[Points], false, true, &[K::Up], "Shift+Up"),
    s(DeletePathDown, &[Points], false, true, &[K::Down], "Shift+Down"),
    s(ShutRight, &[Points], true, false, &[K::Right], "Ctrl+Right"),
    s(ShutLeft, &[Points], true, false, &[K::Left], "Ctrl+Left"),
    s(ShutUp, &[Points], true, false, &[K::Up], "Ctrl+Up"),
    s(ShutDown, &[Points], true, false, &[K::Down], "Ctrl+Down"),
    s(NewPath, &[Points], false, false, &[K::P], "P"),
    s(NewPoint, &[Points], false, false, &[K::N], "N"),
    s(DeletePoint, &[Points], false, false, &[K::Delete], "Del"),
    s(MakeEntry, &[Points], false, false, &[K::H], "H"),
    s(SetStart, &[Points], false, false, &[K::I], "I"),
    s(GoToStart, WORLD, false, false, &[K::M], "M"),
    s(NewZone, WORLD, true, false, &[K::N], "Ctrl+N"),
    s(BlankZone, WORLD, true, false, &[K::B], "Ctrl+B"),
    s(DeleteZone, WORLD, true, false, &[K::Delete], "Ctrl+Del"),
];

/// Is the system's shortcut key held (Ctrl, or Cmd on macOS)?
pub fn command_down() -> bool {
    match SHORTCUT_KEY {
        ShortcutKey::Control => is_key_down(K::LeftControl) || is_key_down(K::RightControl),
        ShortcutKey::Command => is_key_down(K::LeftSuper) || is_key_down(K::RightSuper),
    }
}

/// The modifier keys, which the system's dialogs would keep from the editor while held.
pub const MODIFIERS: [KeyCode; 8] = [K::LeftControl, K::RightControl, K::LeftSuper, K::RightSuper, K::LeftShift, K::RightShift, K::LeftAlt, K::RightAlt];

/// The actions whose keys went down this frame.
pub fn pressed(ctx: Ctx) -> Vec<Action> {
    let ctrl = command_down();
    let shift = is_key_down(K::LeftShift) || is_key_down(K::RightShift);
    TABLE.iter().filter(|s| s.ctxs.contains(&ctx) && s.ctrl == ctrl && s.shift == shift && s.keys.iter().any(|&k| is_key_pressed(k))).map(|s| s.action).collect()
}

/// The keys of an action, as its button shows them.
pub fn keys(action: Action) -> &'static str {
    static LABELS: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let labels = LABELS.get_or_init(|| TABLE.iter().map(|s| with_shortcut_key(s.label, SHORTCUT_KEY)).collect());
    TABLE.iter().position(|s| s.action == action).map_or("", |i| &labels[i])
}

/// `text` (a label or a message of the catalogue) with the system's shortcut key where it
/// says Ctrl (Strg in German).
pub fn on_this_system(text: &str) -> String {
    with_shortcut_key(text, SHORTCUT_KEY)
}

fn with_shortcut_key(text: &str, key: ShortcutKey) -> String {
    match key {
        ShortcutKey::Control => text.to_string(),
        ShortcutKey::Command => text.replace("Ctrl+", "Cmd+").replace("Strg+", "Cmd+"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shortcut_key_in_labels_and_messages() {
        assert_eq!(with_shortcut_key("Ctrl+Shift+E", ShortcutKey::Command), "Cmd+Shift+E");
        assert_eq!(with_shortcut_key("(Strg+Z holt sie zurueck).", ShortcutKey::Command), "(Cmd+Z holt sie zurueck).");
        assert_eq!(with_shortcut_key("Ctrl+Z", ShortcutKey::Control), "Ctrl+Z");
        assert_eq!(keys(Action::Save), format!("{}+S", SHORTCUT_KEY.label()));
        assert_eq!(keys(Action::Play), "F5");
    }

    #[test]
    fn no_combination_does_two_things_and_panning_keys_stay_free() {
        for ctx in [Blocks, Objects, Picture, Points] {
            let mut seen = Vec::new();
            for s in TABLE.iter().filter(|s| s.ctxs.contains(&ctx)) {
                for &k in s.keys {
                    let combo = (s.ctrl, s.shift, k);
                    assert!(!seen.contains(&combo), "{ctx:?}: {combo:?} twice");
                    seen.push(combo);
                    if !s.ctrl {
                        assert!(![K::W, K::A, K::S, K::D, K::Space].contains(&k), "{k:?} pans the view");
                        assert!(s.shift || ![K::Left, K::Right, K::Up, K::Down].contains(&k), "{k:?} pans the view");
                    }
                }
            }
        }
    }
}
