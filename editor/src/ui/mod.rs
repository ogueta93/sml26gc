//! The editor's window (macroquad), split by panel. `Editor` holds what is on screen (the
//! level shown, camera, zoom, tool, selection, textures); what is being edited lives in
//! `EditorDocument`, and saving / graphics / starting the game go through the application
//! layer and the ports.
mod blank_view;
mod blank_zone_view;
mod catalogue_view;
pub mod drawn_menu_bar;
mod files_view;
mod grid_edit;
pub mod inset;
mod level_panel;
mod level_view;
mod menu;
mod names_view;
mod objects_view;
mod palette;
mod projects_view;
mod recent_view;
mod search_view;
pub mod shortcuts;
mod goal_view;
pub mod language_menu;
mod start_view;
mod status_bar;
mod target_picker;
mod test_game_view;
mod testing_view;
mod widgets;
mod world_view;

pub use projects_view::{BaseScreen, ProjectsChoice, ProjectsScreen};
pub use widgets::{col, set_text_language};
use widgets::INFO;

use crate::application::art_service::{ArtService, Capture};
use crate::application::look_loader::LookLoader;
use crate::application::music::MusicPlayer;
use crate::domain::look_cache;
use crate::application::save::{self, SaveError};
use crate::domain::document::{EditorDocument, Grid, Saved};
use crate::domain::place::Place as SavedPlace;
use crate::domain::prefs::{Prefs, PALETTE_ZOOMS};
use crate::domain::stamp::Stamp;
use crate::domain::tools::{self, GridCanvas, Pointer, Tool, ToolKind};
use crate::ports::{EditedCopyStore, FileDialogs, GameLauncher, LookStore, MenuBar, PrefsStore, SoundOut, TestGame, TextFiles};
use grid_edit::Brush;
use palette::PaletteView;
use shortcuts::{Action, Ctx};
use cart_model::levels::{COLS, LEVELS, ROWS};
use cart_model::map_routes::Place;
use cart_model::original_points::OnMap;
use cart_model::zone_maps::ZONES;
use macroquad::prelude::*;
use inset::{screen_width, screen_height, is_mouse_button_pressed, is_mouse_button_down, is_mouse_button_released, mouse_wheel, is_key_pressed};
use std::collections::HashMap;
use widgets::*;

#[derive(PartialEq, Clone, Copy)]
pub enum Mode {
    Blocks,
    Objects,
}

/// Tabs of the world map editor.
pub const TAB_PICTURE: usize = 0;
pub const TAB_POINTS: usize = 1;

pub struct Editor {
    lang: Lang,
    pub(crate) doc: EditorDocument,
    store: Box<dyn EditedCopyStore>,
    launcher: Box<dyn GameLauncher>,
    /// The project open, and it as last saved (a project file's text), to see unsaved changes.
    project: String,
    saved_project: String,
    /// Leaving asked while there are unsaved changes: Save, Discard or Cancel is shown.
    asking: Option<Leave>,
    /// Where the editor is going once this frame ends.
    leaving: Option<Leave>,
    /// The blank zone chooser, open on `new_zones::BASES[i]`.
    blank_zone_choice: Option<usize>,
    /// The zone editor's map list is open.
    maps_open: bool,
    /// A map brought in with Mario's start while the project has its own: which to keep is
    /// asked (the start offered, and where it is).
    start_offer: Option<(cart_model::start_point::Start, String)>,
    art: ArtService,
    atlases: Vec<Option<Texture2D>>,
    world_atlas: Option<Texture2D>,
    zone_atlases: Vec<Option<Texture2D>>,
    /// What each object type looks like in each level: texture and where its top-left sits
    /// relative to the object's position. None = draws nothing.
    looks: HashMap<(usize, u8), Option<(Texture2D, f32, f32)>>,
    pub(crate) cur: usize,
    /// The block and the map tile chosen to paint with: none until the user chooses.
    blocks: Option<Brush>,
    cam: Vec2,
    zoom: usize,
    tool: Box<dyn Tool>,
    grid: bool,
    status: String,
    pan: Option<(Vec2, Vec2)>,
    space_cache: (usize, usize),
    space_at: f64,
    obj_space: (usize, usize),
    overview: Option<Texture2D>,
    overview_stale: bool,
    show_overview: bool,
    pub(crate) mode: Mode,
    /// The object type chosen in the palette to place (None: clicks only pick objects).
    kind: Option<u8>,
    sel_obj: Option<usize>,
    drag_obj: Option<(usize, Vec2)>,
    in_world: bool,
    tiles: Option<Brush>,
    /// A palette selection being dragged: where it started (column, row).
    palette_drag: Option<(i32, i32)>,
    /// A right-button drag on the grid picking a brush: where it started.
    pick_from: Option<(i32, i32)>,
    /// Copied cells and the level or map they came from (pasted only there).
    clipboard: Option<(Grid, Stamp)>,
    /// The tool to go back to after pasting.
    prev_tool: ToolKind,
    prefs: Prefs,
    prefs_store: Box<dyn PrefsStore>,
    level_palette: PaletteView,
    map_palette: PaletteView,
    /// The side panel's width: as wide as its palette (see `fitted_panel`).
    panel: f32,
    pub(crate) world_tab: usize,
    /// Which map picture the world map editor shows: the world map or a zone map.
    pub(crate) world_map: OnMap,
    point: usize,
    world_cam: Vec2,
    level_cam: Vec2,
    /// A path being drawn from the chosen point: its corners so far (pixels of the picture).
    route_draw: Option<Vec<(i32, i32)>>,
    /// The next click on the world map puts a new point there.
    placing_point: bool,
    /// A new zone's point being dragged with the mouse.
    drag_point: Option<u8>,
    /// Where a drag of a world map or game zone point began: the point's place, the mouse.
    drag_from: Option<(Place, Vec2)>,
    dialogs: Box<dyn FileDialogs>,
    text_files: Box<dyn TextFiles>,
    /// A file action waiting for the keys and buttons to be let go (see `run_pending_file`).
    pending_file: Option<Action>,
    /// The last file action that asked for a file: run again when the file comes later.
    last_open: Option<Action>,
    /// Mario's picture at a level's start (None: not captured yet; Some(None): no picture).
    mario: Option<Option<(Texture2D, f32, f32)>>,
    /// What a click on the level places, when not painting or moving objects.
    placing: Option<start_view::Placing>,
    /// Mario's start being dragged.
    drag_start: bool,
    /// The finish or secret exit being dragged.
    drag_goal: Option<cart_model::level_goals::Goal>,
    /// A level pixel to bring into view on the next frame (true: even when it is in view).
    look: Option<(Vec2, bool)>,
    /// A new level's or zone's name being typed.
    naming: Option<names_view::Naming>,
    /// Where a door or exit leads, being chosen on the maps.
    picker: Option<target_picker::Picker>,
    menu: Box<dyn MenuBar>,
    /// Each File menu entry's state as last set (None: not set yet).
    menu_enabled: [Option<bool>; menu::FILE_MENU_LEN],
    tools_enabled: [Option<bool>; menu::TOOLS_MENU_LEN],
    /// A warning's place marked on the level until a time.
    mark: Option<(Vec2, f64)>,
    /// The level picker's grid is unfolded.
    levels_open: bool,
    /// The test game's settings are open, dropped from the button at `test_game_anchor`.
    test_game_open: bool,
    test_game_anchor: Rect,
    side_tab: level_panel::SideTab,
    /// The level inspector scrolled by, and how tall it was last drawn.
    side_scroll: f32,
    side_content: f32,
    /// The New blank level chooser is open on this game level.
    blank_choice: Option<usize>,
    /// The object catalogue, while it is open.
    catalogue: Option<catalogue_view::CatalogueView>,
    /// The game's objects once each, and how many pictures they were grouped with.
    catalogue_index: (usize, Vec<crate::domain::object_catalogue::CatalogueObject>),
    /// What kind of message the status is, and the text that kind was given to (a new
    /// message is information until marked).
    status_kind: Severity,
    status_for: String,
    /// The object palette shows only this category.
    palette_filter: Option<crate::domain::object_names::Category>,
    /// Per base level, the object types it draws right (`offered_types`).
    offered: HashMap<usize, Vec<(u8, Vec<usize>)>>,
    /// Each captured (level, type) picture that draws something.
    captured: HashMap<(usize, u8), cart_model::art::Look>,
    /// The object palette shows every type, not only those the level draws right.
    show_all_objects: bool,
    /// Play from a point is on: a click on a point of the map starts the game there.
    play_pick: bool,
    /// The last place each level was played from (Play here again).
    played_here: Vec<(usize, Vec2)>,
    /// The object palette's search: the text, and whether it is being typed (`search_view`).
    search: search_view::Search,
    /// Each palette's last chosen values (`recent_view`).
    recent: Vec<(recent_view::Recent, Vec<u8>)>,
    /// The world map editor's Points panel: how far it is scrolled, and its height.
    points_scroll: f32,
    points_content: f32,
    /// The tooltip of the button under the mouse this frame.
    tip: Option<String>,
    /// The left button went down for a click that is done (play here, a checkpoint): the
    /// level takes nothing more from it until it is let go.
    hold_click: bool,
    music: MusicPlayer,
    sound: Box<dyn SoundOut>,
    look_loader: LookLoader,
    look_store: Box<dyn LookStore>,
    /// Every object picture known, as the cache keeps them, and how many it has.
    look_data: Vec<look_cache::Entry>,
    /// The tiles of the game's levels' own objects (`art::object_tiles`), as captured.
    tile_data: Vec<look_cache::Tiles>,
    object_tiles: HashMap<(usize, u8), Vec<u8>>,
    looks_saved: usize,
}

/// What the editor works through, besides the document.
/// Where the editor goes when it leaves the project.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Leave {
    Projects,
    Quit,
}

pub struct Ports {
    pub store: Box<dyn EditedCopyStore>,
    pub launcher: Box<dyn GameLauncher>,
    pub prefs: Box<dyn PrefsStore>,
    pub dialogs: Box<dyn FileDialogs>,
    pub text_files: Box<dyn TextFiles>,
    pub menu: Box<dyn MenuBar>,
    pub sound: Box<dyn SoundOut>,
    pub looks: Box<dyn LookStore>,
}

/// No menu bar (an editor whose bar went to the next project's).
struct NoMenu;

impl MenuBar for NoMenu {
    fn add_menu(&mut self, _: &str, _: &[crate::ports::MenuEntry]) {}
    fn chosen(&mut self) -> Vec<u16> {
        Vec::new()
    }
    fn enable(&mut self, _: u16, _: bool) {}
    fn check(&mut self, _: u16, _: bool) {}
    fn clear(&mut self) {}
}

/// Painting into the level or map picture shown.
struct DocCanvas<'a> {
    doc: &'a mut EditorDocument,
    grid: Grid,
    changed: &'a mut bool,
}

impl GridCanvas for DocCanvas<'_> {
    fn begin_edit(&mut self) {
        self.doc.grid_snapshot(self.grid);
    }
    fn paint(&mut self, x: i32, y: i32, v: u8) {
        if self.doc.set_cell(self.grid, x, y, v) {
            *self.changed = true;
        }
    }
}

fn left_button() -> Pointer {
    Pointer {
        pressed: is_mouse_button_pressed(MouseButton::Left),
        down: is_mouse_button_down(MouseButton::Left),
        released: is_mouse_button_released(MouseButton::Left),
    }
}

impl Editor {
    pub fn new(mut doc: EditorDocument, ports: Ports, project: String, lang: Lang) -> Editor {
        let Ports { store, launcher, prefs: prefs_store, dialogs, text_files, menu, sound, looks: look_store } = ports;
        let restore_damaged = match store.read_restore_points().map(|t| crate::domain::files::read_restore_points(&t)) {
            Some(Ok(points)) => {
                doc.attach_restore_points(points);
                false
            }
            Some(Err(_)) => true,
            None => false,
        };
        let (cached, cached_tiles) = look_store.load().map(|t| look_cache::from_text(&t)).unwrap_or_default();
        let known = cached.iter().map(|e| (e.0, e.1)).collect();
        let known_tiles = cached_tiles.iter().map(|e| (e.0, e.1)).collect();
        let look_loader = LookLoader::start(doc.rom.clone(), doc.original.clone(), &known, &known_tiles);
        let mut status = format!("{} {project}", tr(lang, "Project"));
        if restore_damaged {
            status = format!("{status}. {}", tr(lang, "Its restore points file is damaged: imported levels and zones restore to their base."));
        }
        let saved_project = crate::domain::files::project_text(&doc.project_file());

        let mut ed = Editor {
            lang,
            doc,
            store,
            launcher,
            project,
            saved_project,
            asking: None,
            leaving: None,
            start_offer: None,
            blank_zone_choice: None,
            maps_open: false,
            art: ArtService::new(),
            atlases: (0..LEVELS).map(|_| None).collect(),
            world_atlas: None,
            zone_atlases: ZONES.iter().map(|_| None).collect(),
            looks: HashMap::new(),
            cur: 0,
            blocks: None,
            cam: Vec2::ZERO,
            zoom: 3,
            tool: tools::make(ToolKind::Pencil),
            grid: true,
            status,
            pan: None,
            space_cache: (0, 1),
            space_at: -1.0,
            obj_space: (0, 1),
            overview: None,
            overview_stale: true,
            show_overview: true,
            mode: Mode::Blocks,
            kind: None,
            sel_obj: None,
            drag_obj: None,
            in_world: false,
            tiles: None,
            palette_drag: None,
            pick_from: None,
            clipboard: None,
            prev_tool: ToolKind::Pencil,
            prefs: prefs_store.load(),
            prefs_store,
            level_palette: PaletteView::default(),
            map_palette: PaletteView::default(),
            panel: PANEL_MIN,
            world_tab: TAB_PICTURE,
            world_map: OnMap::World,
            point: 0,
            world_cam: Vec2::ZERO,
            route_draw: None,
            placing_point: false,
            drag_point: None,
            drag_from: None,
            level_cam: Vec2::ZERO,
            dialogs,
            text_files,
            pending_file: None,
            last_open: None,
            mario: None,
            placing: None,
            drag_start: false,
            drag_goal: None,
            look: None,
            naming: None,
            picker: None,
            menu,
            menu_enabled: [None; menu::FILE_MENU_LEN],
            tools_enabled: [None; menu::TOOLS_MENU_LEN],
            mark: None,
            levels_open: false,
            test_game_open: false,
            test_game_anchor: Rect::default(),
            side_tab: level_panel::SideTab::Palette,
            side_scroll: 0.0,
            side_content: 0.0,
            blank_choice: None,
            catalogue: None,
            catalogue_index: (usize::MAX, Vec::new()),
            palette_filter: None,
            status_kind: Severity::Info,
            status_for: String::new(),
            offered: HashMap::new(),
            captured: HashMap::new(),
            show_all_objects: false,
            recent: Vec::new(),
            search: Default::default(),
            played_here: Vec::new(),
            play_pick: false,
            points_scroll: 0.0,
            points_content: 0.0,
            tip: None,
            hold_click: false,
            music: MusicPlayer::default(),
            sound,
            look_loader,
            look_store,
            look_data: Vec::new(),
            tile_data: Vec::new(),
            object_tiles: HashMap::new(),
            looks_saved: 0,
        };
        for (l, k, look) in cached {
            ed.add_look(l, k, look);
        }
        for (l, k, t) in cached_tiles {
            ed.add_tiles(l, k, t);
        }
        ed.looks_saved = ed.look_data.len() + ed.tile_data.len();
        ed.build_menu();
        ed
    }

    /// Take the other object catalogue's objects out of the levels' sets (a copy saved when a
    /// level could take them), and say so.
    pub fn drop_other_catalogue(&mut self) {
        let lang = self.lang;
        let (dropped, objects) = self.doc.drop_other_catalogue();
        if dropped.is_empty() {
            return;
        }
        let levels: Vec<String> = dropped.iter().map(|l| format!("{l:02}")).collect();
        self.status = format!("{} {} ({objects} {}). {}", tr(lang, "Objects of the other object catalogue taken out of the enemies of levels"), levels.join(", "), tr(lang, "placed"), shortcuts::on_this_system(tr(lang, "A level plays its own catalogue's objects only; Ctrl+Z brings them back.")));
        self.mark(Severity::Warning);
    }

    /// One frame: the menu, shortcuts, input, then drawing.
    /// One frame: the views below the menu bar, the bar over them.
    pub fn frame(&mut self) {
        inset::begin(&mut *self.menu);
        self.views_frame();
        inset::end(&mut *self.menu);
    }

    fn views_frame(&mut self) {
        new_frame_texts();
        if self.asking.is_some() {
            self.unsaved_frame();
            self.draw_tip();
            return;
        }
        if self.start_offer.is_some() {
            self.start_offer_frame();
            self.draw_tip();
            return;
        }
        self.update_menu();
        if self.blank_choice.is_some() && !self.in_world {
            self.blank_frame();
            self.draw_tip();
            return;
        }
        if self.blank_zone_choice.is_some() && self.in_world {
            self.blank_zone_frame();
            self.draw_tip();
            return;
        }
        if self.catalogue.is_some() && !self.in_world {
            self.catalogue_frame();
            self.draw_tip();
            return;
        }
        if !self.update_naming() && !self.update_search() {
            for a in shortcuts::pressed(self.ctx()) {
                self.run(a);
            }
        }
        self.run_pending_file();
        self.panel = self.fitted_panel();
        // the open test game settings take the clicks off the view
        let view_input = !self.test_game_open;
        if self.in_world {
            if view_input {
                self.update_world();
            }
            self.draw_world();
        } else {
            if view_input {
                self.update_level();
            }
            self.draw_level();
        }
        if self.in_world {
            self.music.stop();
        }
        self.music.pump(&*self.sound, self.prefs.music_volume as f32 / *crate::domain::prefs::MUSIC_VOLUMES.end() as f32);
    }

    /// The status's kind: what `mark` gave this text, else information.
    pub(super) fn severity(&self) -> Severity {
        if self.status == self.status_for {
            self.status_kind
        } else {
            Severity::Info
        }
    }

    /// The status just written is `kind` (warning, error or done).
    pub(super) fn mark(&mut self, kind: Severity) {
        self.status_for = self.status.clone();
        self.status_kind = kind;
    }

    fn music_playing(&self) -> bool {
        self.music.playing()
    }

    /// Listen to `music` with the game's own sound, or stop.
    fn toggle_music(&mut self, music: u8) {
        if self.music.playing() {
            self.music.stop();
            return;
        }
        if let Err(e) = self.music.play(&self.doc.rom, music, self.sound.sample_rate()) {
            self.status = format!("{} {e}", tr(self.lang, "Could not play the music:"));
            self.mark(Severity::Error);
        }
    }

    fn scale(&self) -> f32 {
        ZOOMS[self.zoom]
    }

    fn set_tool(&mut self, kind: ToolKind) {
        if self.tool.kind() != kind {
            self.tool = tools::make(kind);
        }
    }

    /// Where the mouse points, in the shown picture's pixels.
    fn world_pos(&self, m: Vec2) -> Vec2 {
        (m - vec2(self.panel, 0.0)) / self.scale() + self.cam
    }

    fn ctx(&self) -> Ctx {
        match (self.in_world, self.world_tab == TAB_POINTS, self.mode) {
            (true, true, _) => Ctx::Points,
            (true, false, _) => Ctx::Picture,
            (false, _, Mode::Objects) => Ctx::Objects,
            (false, _, Mode::Blocks) => Ctx::Blocks,
        }
    }

    /// Do what a shortcut or a panel button asks.
    fn run(&mut self, a: Action) {
        match a {
            Action::Undo if self.in_world => self.world_undo(),
            Action::Undo => self.undo(),
            Action::Redo if self.in_world => self.world_redo(),
            Action::Redo => self.redo(),
            Action::Save => {
                self.save();
            }
            Action::Play if self.in_world => self.play_map(),
            Action::PlayFromHere if self.in_world => self.toggle_play_pick(),
            Action::Play => self.play(),
            Action::Grid => self.grid = !self.grid,
            Action::Pencil => self.set_tool(ToolKind::Pencil),
            Action::BoxFill => self.set_tool(ToolKind::Box),
            Action::Select => self.set_tool(ToolKind::Select),
            Action::Overview => self.show_overview = !self.show_overview,
            Action::BlocksObjects => self.set_mode(if self.mode == Mode::Blocks { Mode::Objects } else { Mode::Blocks }),
            Action::PrevLevel => self.select_level((self.cur + self.level_count() - 1) % self.level_count()),
            Action::NextLevel => self.select_level((self.cur + 1) % self.level_count()),
            Action::NewLevel => self.new_level(),
            Action::NewBlankLevel if !self.in_world => self.open_blank_chooser(),
            Action::ObjectCatalogue if !self.in_world => self.open_catalogue(),
            Action::Projects => self.ask_leave(Leave::Projects),
            Action::RecaptureObjects => self.recapture_objects(),
            Action::DeleteLevel => self.delete_level(),
            Action::WorldEditor => self.enter_world(!self.in_world),
            Action::PrevMap => self.step_map(-1),
            Action::NextMap => self.step_map(1),
            Action::SwitchTab => self.world_tab = if self.world_tab == TAB_PICTURE { TAB_POINTS } else { TAB_PICTURE },
            Action::Restore if self.in_world => self.restore_map(),
            Action::Restore => self.restore_original(),
            Action::ZoomOut => self.palette_zoom(-1),
            Action::ZoomIn => self.palette_zoom(1),
            Action::Copy => self.copy_selection(false),
            Action::Cut => self.copy_selection(true),
            Action::Paste => self.start_paste(),
            Action::ClearSelection => self.clear_selection_cells(),
            Action::Escape if self.placing.is_some() => self.placing = None,
            Action::Escape => self.escape_grid(),
            Action::PlaceStart => self.toggle_placing(start_view::Placing::Start),
            Action::PlaceFinish if !self.doc.ends_with_boss(self.cur) => self.toggle_placing(start_view::Placing::Finish),
            Action::PlaceSecret => self.toggle_placing(start_view::Placing::Secret),
            Action::PlaceCheckpoint => match self.doc.checkpoint_refused(self.cur) {
                Some(crate::domain::document::CheckpointRefused::NoRoom) => {
                    self.status = tr(self.lang, "The game keeps no restart place for this level: it cannot have checkpoints.").into();
                    self.mark(Severity::Warning);
                }
                _ => self.toggle_placing(start_view::Placing::Checkpoint),
            },
            Action::PlayFromHere => self.toggle_placing(start_view::Placing::PlayHere),
            Action::PlayHereAgain => match self.played_here.iter().find(|p| p.0 == self.cur).map(|p| p.1) {
                Some(w) => self.play_from_here(w),
                None => {
                    self.status = format!("{} ({})", tr(self.lang, "This level has not been played from here yet: use Here first"), shortcuts::keys(Action::PlayFromHere));
                    self.mark(Severity::Warning);
                }
            },
            Action::SecretLeads => self.start_secret_picking(),
            Action::DeleteObject => self.delete_selected_object(),
            a if a.uses_files() => self.queue_file(a),
            _ => self.run_points(a),
        }
    }

    /// Blocks or Objects, with its palette shown.
    pub fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.side_tab = level_panel::SideTab::Palette;
        self.sel_obj = None;
        self.leave_grid();
    }

    fn palette_zoom_now(&self) -> u32 {
        if self.in_world {
            self.prefs.map_palette_zoom
        } else {
            self.prefs.level_palette_zoom
        }
    }

    /// One step bigger or smaller for the palette shown.
    fn palette_zoom(&mut self, d: i32) {
        let z = (self.palette_zoom_now() as i32 + d).clamp(*PALETTE_ZOOMS.start() as i32, *PALETTE_ZOOMS.end() as i32) as u32;
        if z == self.palette_zoom_now() {
            return;
        }
        if self.in_world {
            self.prefs.map_palette_zoom = z;
        } else {
            self.prefs.level_palette_zoom = z;
        }
        self.prefs_store.save(&self.prefs);
    }

    /// Pan with arrows / WASD, middle mouse or Space + left mouse; zoom at the mouse with
    /// the wheel; keep the camera inside a picture of `size` pixels. Returns whether Space
    /// is held, the visible size and the furthest camera position.
    fn navigate(&mut self, m: Vec2, ctrl: bool, size: Vec2) -> (bool, Vec2, Vec2) {
        let speed = 600.0 * get_frame_time() / self.scale();
        let mut d = Vec2::ZERO;
        // keys typed into a name do not move the view
        let down = |k: KeyCode| self.naming.is_none() && is_key_down(k);
        // Shift + arrows and Ctrl + arrows are shortcuts
        let arrows = !(ctrl || is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift));
        if (arrows && down(KeyCode::Left)) || (!ctrl && down(KeyCode::A)) {
            d.x -= speed;
        }
        if (arrows && down(KeyCode::Right)) || (!ctrl && down(KeyCode::D)) {
            d.x += speed;
        }
        if (arrows && down(KeyCode::Up)) || (!ctrl && down(KeyCode::W)) {
            d.y -= speed;
        }
        if (arrows && down(KeyCode::Down)) || (!ctrl && down(KeyCode::S)) {
            d.y += speed;
        }
        let space = down(KeyCode::Space);
        self.cam += d;
        if m.x >= self.panel && (is_mouse_button_pressed(MouseButton::Middle) || (space && is_mouse_button_pressed(MouseButton::Left))) {
            self.pan = Some((m, self.cam));
        }
        if let Some((m0, c0)) = self.pan {
            if is_mouse_button_down(MouseButton::Middle) || (space && is_mouse_button_down(MouseButton::Left)) {
                self.cam = c0 - (m - m0) / self.scale();
            } else {
                self.pan = None;
            }
        }
        let wheel = mouse_wheel().1;
        if wheel != 0.0 && m.x >= self.panel {
            let before = self.world_pos(m);
            if wheel > 0.0 && self.zoom + 1 < ZOOMS.len() {
                self.zoom += 1;
            } else if wheel < 0.0 && self.zoom > 0 {
                self.zoom -= 1;
            }
            self.cam = before - (m - vec2(self.panel, 0.0)) / self.scale();
        }
        let view = vec2(screen_width() - self.panel, screen_height() - STATUS_H) / self.scale();
        let max = (size - view).max(Vec2::ZERO);
        // room over the map for the tabs while choosing a target
        let min = vec2(0.0, -self.picker_tabs_h() / self.scale());
        self.cam = self.cam.clamp(min, max);
        (space, view, max)
    }

    /// Let the game load the level (from the world map) and keep its graphics (a new
    /// level's are its base level's).
    fn ensure_art(&mut self, l: usize) {
        let l = self.doc.base_of(l);
        match self.art.ensure_level(&self.doc.rom, l) {
            Capture::Ready => return,
            Capture::Failed(e) => {
                self.status = e;
                return;
            }
            Capture::NotLoaded => {
                self.status = tr(self.lang, "The game didn't load this level; its graphics may be wrong.").into();
                self.mark(Severity::Warning);
            }
            Capture::Captured => {}
        }
        self.atlases[l] = self.art.level(l).map(make_atlas);
    }

    /// The shown map's tiles, as the game shows them (captured on that map's screen).
    fn ensure_world_art(&mut self) {
        if self.map_atlas().is_some() {
            return;
        }
        if !self.art.booted() {
            self.ensure_art(self.cur);
        }
        match self.world_map {
            OnMap::World => self.world_atlas = self.art.world().map(make_world_atlas),
            OnMap::Zone(z) => self.zone_atlases[z] = self.art.zone(z).map(make_world_atlas),
            OnMap::NewZone(k) => {
                let z = self.doc.new_zones[k].base;
                self.zone_atlases[z] = self.art.zone(z).map(make_world_atlas);
            }
        }
    }

    /// The tiles of the map picture shown.
    fn map_atlas(&self) -> Option<&Texture2D> {
        match self.world_map {
            OnMap::World => self.world_atlas.as_ref(),
            OnMap::Zone(z) => self.zone_atlases[z].as_ref(),
            OnMap::NewZone(k) => self.zone_atlases[self.doc.new_zones.get(k)?.base].as_ref(),
        }
    }

    /// Show another map picture (the world map or a zone map).
    pub fn show_map(&mut self, map: OnMap) {
        if map == self.world_map {
            return;
        }
        self.world_map = map;
        self.tiles = None;
        self.doc.show_added(map);
        self.cam = Vec2::ZERO;
        self.leave_grid();
        self.ensure_world_art();
    }

    /// Bring the view to the level pixel asked for (in `look`), a little left of and below its
    /// middle; unless it is already in view, when asked so.
    fn look_now(&mut self) {
        let Some((p, always)) = self.look.take() else { return };
        let view = vec2(screen_width() - self.panel, screen_height() - STATUS_H) / self.scale();
        if !always && Rect::new(self.cam.x, self.cam.y, view.x, view.y).contains(p) {
            return;
        }
        let at = vec2(p.x - 2.0 * BLOCK, p.y - view.y * 0.6);
        self.cam = at.clamp(Vec2::ZERO, (LEVEL_SIZE - view).max(Vec2::ZERO)).floor();
    }

    pub fn select_level(&mut self, l: usize) {
        if l != self.cur {
            self.music.stop();
            // its blocks and objects are its own: nothing chosen there yet
            self.blocks = None;
            self.kind = None;
            self.sel_obj = None;
        }
        self.cur = l;
        self.leave_grid();
        self.ensure_art(l);
        // Start the view where Mario begins the level.
        let s = self.doc.level_start(l);
        self.look = Some((vec2(s.mario_x as f32, s.mario_y as f32), true));
        self.overview_stale = true;
        let shared = self.doc.sharing(l);
        self.status = if shared.is_empty() {
            format!("{} {:02} {}", tr(self.lang, "Level"), l, self.doc.level_name(l))
        } else {
            format!("{} {:02}: {} {:?}", tr(self.lang, "Level"), l, tr(self.lang, "its map is also used by level(s)"), shared)
        };
    }

    pub fn enter_world(&mut self, on: bool) {
        if on == self.in_world {
            return;
        }
        self.leave_grid();
        if on {
            self.level_cam = self.cam;
            self.ensure_world_art();
            self.cam = self.world_cam;
            self.status = tr(self.lang, "World map: paint its tiles, or choose which level each point opens.").into();
        } else {
            self.world_cam = self.cam;
            self.cam = self.level_cam;
        }
        self.in_world = on;
    }

    /// Where the editor is: the level shown, or the zone editor's map.
    fn place(&self) -> SavedPlace {
        if self.in_world {
            SavedPlace::Map(self.world_map)
        } else {
            SavedPlace::Level(self.cur)
        }
    }

    /// Show where the editor was when the project was last saved; the world map when it was
    /// never saved there or that level or map is gone.
    pub fn open_at_saved_place(&mut self) {
        match self.store.read_place().as_deref().and_then(SavedPlace::from_text) {
            Some(SavedPlace::Level(l)) if l < self.level_count() => self.select_level(l),
            Some(SavedPlace::Map(m)) if self.all_maps().contains(&m) => {
                self.enter_world(true);
                self.show_map(m);
            }
            _ => self.enter_world(true),
        }
    }

    pub fn level_count(&self) -> usize {
        self.doc.levels.len()
    }

    /// A copy of the level shown, shown (for the check pictures).
    pub fn copy_level(&mut self) {
        self.new_level();
    }

    /// A new level, a copy of the one shown, and show it.
    fn new_level(&mut self) {
        match self.doc.add_level(self.cur) {
            Some(n) => {
                let base = self.doc.base_of(n);
                self.select_level(n);
                self.status = format!(
                    "{} {n:02}: {} {base:02} ({}). {}",
                    tr(self.lang, "New level"),
                    tr(self.lang, "plays as level"),
                    tr(self.lang, "its graphics, music and time"),
                    tr(self.lang, "Give it to a map point to reach it in the game.")
                );
            }
            None => {
                self.status = tr(self.lang, "There can't be more levels.").into();
                self.mark(Severity::Warning);
            }
        }
    }

    /// Delete the new level shown (the 32 of the game stay).
    fn delete_level(&mut self) {
        let l = self.cur;
        if l < LEVELS {
            return;
        }
        if self.doc.remove_level(l) {
            self.select_level(l.min(self.doc.levels.len() - 1));
            self.status = format!("{} {l:02} {}", tr(self.lang, "Level"), tr(self.lang, "deleted (save to keep it); the ones after it moved down a number."));
            self.mark(Severity::Done);
        }
    }

    fn undo(&mut self) {
        let base = self.doc.base_of(self.cur);
        if self.doc.undo(self.cur) {
            self.level_changed(base);
        }
    }

    fn redo(&mut self) {
        let base = self.doc.base_of(self.cur);
        if self.doc.redo(self.cur) {
            self.level_changed(base);
        }
    }

    /// After the shown level changed under the view; `base` is what it played as before (a
    /// replaced new level can play as another and look different).
    fn level_changed(&mut self, base: usize) {
        self.sel_obj = None;
        self.overview_stale = true;
        if self.doc.base_of(self.cur) != base {
            let l = self.cur;
            self.looks.retain(|k, _| k.0 != l);
            self.ensure_art(l);
        }
    }

    fn restore_original(&mut self) {
        let origin = self.doc.level_restore_origin(self.cur).map(|o| self.origin_text(o));
        self.doc.restore_level(self.cur);
        self.sel_obj = None;
        self.overview_stale = true;
        let l = self.cur;
        self.status = if let Some(origin) = origin {
            format!("{} {l:02} {} {origin} {}", tr(self.lang, "Level"), tr(self.lang, "restored as imported from"), tr(self.lang, "(save to keep it; undo brings it back)."))
        } else if l >= LEVELS {
            format!("{} {l:02} {} {:02} {}", tr(self.lang, "Level"), tr(self.lang, "restored as its base level"), self.doc.base_of(l), tr(self.lang, "is in the game (save to keep it; undo brings it back)."))
        } else {
            format!("{} {l:02} {}", tr(self.lang, "Level"), tr(self.lang, "restored as it is in the game (save to keep it; undo brings it back)."))
        };
    }

    /// Save the edited copy; the status bar says what happened.
    fn save(&mut self) -> bool {
        self.music.stop();
        let lang = self.lang;
        match save::save(&mut self.doc, &*self.store) {
            Ok(parts) => {
                if parts.iter().any(|p| matches!(p, Saved::Objects(_))) {
                    self.sel_obj = None;
                }
                let notes: Vec<String> = parts
                    .iter()
                    .map(|p| match p {
                        Saved::Bank(n) => n.to_string(),
                        Saved::Objects(n) => n.to_string(),
                        Saved::WorldMap => tr(lang, "world map").to_string(),
                        Saved::LevelPoints => tr(lang, "level points").to_string(),
                        Saved::Doors => tr(lang, "doors").to_string(),
                        Saved::Routes(n) => format!("{} ({n} bytes)", tr(lang, "map paths")),
                        Saved::NewLevels(n, used) => format!("{n} {} {used} bytes", tr(lang, "new levels")),
                        Saved::NewZones(n, used) => format!("{n} {} {used} bytes", tr(lang, "new zones")),
                        Saved::Starts(n) => format!("{n} {}", tr(lang, "level starts and settings")),
                        Saved::SecretExits(n) => format!("{n} {}", tr(lang, "secret exits")),
                    })
                    .collect();
                self.status = format!(
                    "{} {} {}",
                    tr(lang, "Saved"),
                    self.store.path().display(),
                    if notes.is_empty() { String::new() } else { format!("({})", notes.join(", ")) }
                );
                self.mark(Severity::Done);
                self.saved_project = crate::domain::files::project_text(&self.doc.project_file());
                let _ = self.store.write_place(&self.place().to_text());
                true
            }
            Err(SaveError::Layout(e)) => {
                self.status = format!("{}: {e}", tr(lang, "Not saved"));
                self.mark(Severity::Error);
                false
            }
            Err(SaveError::Write(e)) => {
                self.status = format!("{}: {e}", tr(lang, "Could not save"));
                self.mark(Severity::Error);
                false
            }
        }
    }

    /// Leave the project (`to`), asking first when there are unsaved changes.
    pub fn ask_leave(&mut self, to: Leave) {
        if self.unsaved() {
            self.asking = Some(to);
        } else {
            self.leaving = Some(to);
        }
    }

    /// Where the editor is going, once it has left the project.
    pub fn leaving(&self) -> Option<Leave> {
        self.leaving
    }

    /// The window's menu bar, to build the next project's editor on.
    pub fn take_menu(&mut self) -> Box<dyn MenuBar> {
        let mut m = std::mem::replace(&mut self.menu, Box::new(NoMenu));
        m.clear();
        m
    }

    /// Is the project different from what was last saved?
    pub fn unsaved(&self) -> bool {
        crate::domain::files::project_text(&self.doc.project_file()) != self.saved_project
    }

    /// Save, Discard or Cancel before leaving with unsaved changes.
    fn unsaved_frame(&mut self) {
        let lang = self.lang;
        clear_background(col(3));
        let (w, h) = (520.0, 150.0);
        let r = Rect::new((screen_width() - w) / 2.0, (screen_height() - h) / 2.0, w, h);
        draw_rectangle(r.x, r.y, r.w, r.h, col(0));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, col(3));
        put_text(&format!("{} {}", tr(lang, "Unsaved changes in"), self.project), r.x + 20.0, r.y + 36.0, 22.0, col(3));
        put_text(tr(lang, "Save them before leaving?"), r.x + 20.0, r.y + 64.0, 18.0, INFO);
        let bw = (w - 40.0 - 2.0 * 12.0) / 3.0;
        let at = |i: f32| Rect::new(r.x + 20.0 + i * (bw + 12.0), r.y + h - 56.0, bw, 36.0);
        if button(at(0.0), tr(lang, "Save"), true, 17.0) || is_key_pressed(KeyCode::Enter) {
            let to = self.asking.take();
            if self.save() {
                self.leaving = to;
            }
        } else if button(at(1.0), tr(lang, "Discard"), false, 17.0) {
            self.leaving = self.asking.take();
        } else if button(at(2.0), tr(lang, "Cancel"), false, 17.0) || is_key_pressed(KeyCode::Escape) {
            self.asking = None;
        }
    }

    /// A map brought in (imported, replaced or copied) holding Mario's start at point `start`:
    /// taken when the project has none; asked when it has one elsewhere.
    pub(super) fn offer_start(&mut self, map: OnMap, start: Option<u8>) {
        let Some(s) = start.and_then(|p| self.doc.start_at(map, p)) else { return };
        if s == self.doc.start {
            return;
        }
        if !self.doc.start_ok() {
            self.doc.set_start(s);
            return;
        }
        let other = self.doc.start_map().map(|m| self.map_name(m)).unwrap_or_default();
        self.start_offer = Some((s, format!("{} {:02X} ({} {other})", self.map_name(map), s.point, tr(self.lang, "the project's is on"))));
    }

    /// Which of two starts for Mario to keep.
    fn start_offer_frame(&mut self) {
        let lang = self.lang;
        let Some((s, what)) = self.start_offer.clone() else { return };
        clear_background(col(3));
        let (w, h) = (560.0, 170.0);
        let r = Rect::new((screen_width() - w) / 2.0, (screen_height() - h) / 2.0, w, h);
        draw_rectangle(r.x, r.y, r.w, r.h, col(0));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, col(3));
        put_text(tr(lang, "Two starts for Mario"), r.x + 20.0, r.y + 34.0, 22.0, col(3));
        let mut y = r.y + 60.0;
        for line in wrap_px(&format!("{} {what}. {}", tr(lang, "The map brought in has Mario's start at"), tr(lang, "A project has one start: which do you keep?")), w - 40.0, 16.0) {
            put_text(&line, r.x + 20.0, y, 16.0, INFO);
            y += 20.0;
        }
        let bw = (w - 40.0 - 12.0) / 2.0;
        if button(Rect::new(r.x + 20.0, r.y + h - 52.0, bw, 34.0), tr(lang, "The project's"), true, 16.0) || is_key_pressed(KeyCode::Escape) {
            self.start_offer = None;
        } else if button(Rect::new(r.x + 32.0 + bw, r.y + h - 52.0, bw, 34.0), tr(lang, "The map's"), false, 16.0) {
            self.doc.set_start(s);
            self.start_offer = None;
        }
    }

    /// Save, then try the level shown in the game.
    fn play(&mut self) {
        self.play_level(None);
    }

    /// Play from here: Mario standing on the block at level pixel `w`, this time only.
    /// Refused where his body would be in solid blocks.
    fn play_from_here(&mut self, w: Vec2) {
        let start = Self::start_at(w);
        if start.blocked(&self.doc.levels[self.cur].content.map) {
            self.status = tr(self.lang, "Mario cannot stand there (solid blocks or outside the level).").into();
            self.mark(Severity::Warning);
            return;
        }
        self.placing = None;
        self.played_here.retain(|p| p.0 != self.cur);
        self.played_here.push((self.cur, w));
        if self.play_level(Some((start.mario_x, start.mario_y + cart_model::level_start::FEET))) {
            self.status = format!("{}  ({}: {})", self.status, shortcuts::keys(Action::PlayHereAgain), tr(self.lang, "again from this place"));
        }
    }

    /// A test game is still open: another is not started (the status says so).
    pub(super) fn game_open(&mut self) -> bool {
        if !self.launcher.running() {
            return false;
        }
        self.status = tr(self.lang, "The game is already open: close it to play again.").into();
        self.mark(Severity::Warning);
        true
    }

    /// The test game Play and Here start.
    fn test_game(&self) -> TestGame {
        TestGame { lives: self.prefs.test_lives, coins: self.prefs.test_coins, power: self.prefs.test_power, lang: self.lang }
    }

    /// Save and start the game in the level shown. True when it started.
    fn play_level(&mut self, start: Option<(u16, u16)>) -> bool {
        if self.game_open() || !self.save() {
            return false;
        }
        let started = self.launcher.launch(self.store.path(), Some(self.cur), start, None, self.test_game());
        self.status = match &started {
            Ok(()) if self.cur >= LEVELS => format!("{} {:02} ({} {:02})", tr(self.lang, "Playing level"), self.cur, tr(self.lang, "as"), self.doc.base_of(self.cur)),
            Ok(()) => format!("{} {:02}", tr(self.lang, "Playing level"), self.cur),
            Err(e) => format!("{} {e}", tr(self.lang, "Could not start")),
        };
        if started.is_err() {
            self.mark(Severity::Error);
        }
        started.is_ok()
    }

    /// Save, then start the game on the edited copy's map, Mario on his start (to try the
    /// points).
    fn play_map(&mut self) {
        self.play_map_at(None);
    }

    /// Play from a point: the game on the map, Mario on the chosen point this time only.
    pub(super) fn play_map_here(&mut self) {
        match self.doc.start_at(self.world_map, self.point as u8) {
            Some(s) => self.play_map_at(Some(s)),
            None => {
                self.status = tr(self.lang, "The game can begin on a point of the world map, a game zone or a new zone (not an added point).").into();
                self.mark(Severity::Warning);
            }
        }
    }

    fn play_map_at(&mut self, point: Option<cart_model::start_point::Start>) {
        if self.game_open() || !self.save() {
            return;
        }
        let started = self.launcher.launch(self.store.path(), None, None, point, self.test_game());
        self.status = match &started {
            Ok(()) => tr(self.lang, "Game started with the edited copy.").into(),
            Err(e) => e.clone(),
        };
        if started.is_err() {
            self.mark(Severity::Error);
        }
    }

    /// For the check pictures: capture this level's object pictures first.
    pub fn capture_level_looks(&mut self) {
        self.wait_for_level_looks();
        self.pump_mario();
    }

    /// The object palette with every type (for the command line).
    pub fn show_all_objects(&mut self) {
        self.show_all_objects = true;
    }

    /// Show the level inspector tab, and unfold the level grid (for the command line).
    pub fn show_inspector(&mut self, levels_open: bool) {
        self.side_tab = level_panel::SideTab::Level;
        self.levels_open = levels_open;
    }

    /// Choose "Place start" (for the command line).
    pub fn place_start(&mut self) {
        self.toggle_placing(start_view::Placing::Start);
    }

    /// Choose "Place finish" (for the command line).
    pub fn place_finish(&mut self) {
        self.toggle_placing(start_view::Placing::Finish);
    }

    /// Choose where the level's secret exit leads (for the command line).
    pub fn choose_secret(&mut self) {
        self.start_secret_picking();
    }

}

// The level's size in pixels.
const LEVEL_SIZE: Vec2 = Vec2::new(COLS as f32 * BLOCK, ROWS as f32 * BLOCK);
