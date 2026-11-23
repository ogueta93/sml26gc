//! Everything known about this cartridge from reverse engineering it. Works on ROM bytes
//! and on a running `Gb`; no files, no windows.
//!
//! | Module | What |
//! |---|---|
//! | `rom` | Banks and words in the ROM file |
//! | `levels` | Level headers, map locations, the map compression (decode/encode) |
//! | `level_goals` | Where a level ends: goal blocks $4A and $4B, the bell |
//! | `checkpoint` | Midway bells and where Mario starts again; several per level through a hook |
//! | `enemy_sets` | A level's enemies chosen from several levels: blocks, where the copy keeps them |
//! | `level_graphics` | Which levels draw the same objects (enemy sprites and object catalogue) |
//! | `level_settings` | A level's start time, music and background shades (header bytes) |
//! | `blank_level` | An empty map and start made from a game level's |
//! | `level_tables` | A level's gravity, auto-scroll, camera margin and animated tiles (bank-0 tables) |
//! | `level_start` | Where Mario begins a level (header bytes $00-$07) |
//! | `secret_exits` | Where each level's secret exit takes Mario (a hook in the map code) |
//! | `extra_levels` | Levels beyond the 32, played in place of a base level |
//! | `new_zones` | Zones beyond the eight, played in place of a base zone |
//! | `objects` | Object lists per level: 3-byte format, reading, re-packing |
//! | `world` | World map picture and which level each map point opens |
//! | `doors` | Doors between the world map and the zones (which point enters which zone, where exits lead) |
//! | `added_points` | Points numbered from $80 on every map (hooks in the map code) |
//! | `map_routes` | The paths between map points (walk scripts) and where Mario stands on each |
//! | `zone_maps` | The zones' own one-screen maps (pictures) |
//! | `original_notes` | What the original game does with some map points (flight, twins, unused door) |
//! | `original_points` | Where the game draws its own map points (measured) |
//! | `patch` | The 1 MB edited copy that gives levels and objects more room |
//! | `layout` | Each zone's space for its maps and packing edited maps back |
//! | `ram_map` | Where the running game keeps things; `GameMode` |
//! | `live` | Reading what the running game has loaded |
//! | `cheats` | What each cheat writes in the running game (invincible patch, items, counters, finish) |
//! | `art` | Pictures captured from the running game (blocks, map tiles, object looks) |
//! | `driver` | `GameDriver`: boot to the map, enter a level, back to the map... |
//! | `error` | `CartError` and what a save used |
pub mod added_points;
pub mod art;
pub mod blank_level;
pub mod checkpoint;
pub mod cheats;
pub mod doors;
pub mod driver;
pub mod enemy_sets;
pub mod error;
pub mod extra_levels;
pub mod layout;
pub mod level_goals;
pub mod level_graphics;
pub mod level_settings;
pub mod level_tables;
pub mod level_start;
pub mod secret_exits;
pub mod start_point;
pub mod levels;
pub mod live;
pub mod map_music;
pub mod map_routes;
pub mod new_zones;
pub mod objects;
pub mod original_notes;
pub mod original_points;
pub mod patch;
pub mod ram_map;
pub mod rom;
pub mod world;
pub mod zone_maps;

pub use driver::GameDriver;
pub use error::{BankUsage, CartError, ObjectsUsage};
pub use ram_map::GameMode;
