# Architecture

A Cargo workspace with a hexagonal architecture: pure domain crates in the middle, programs
with ports and adapters around them. Behaviour is guarded by the golden fingerprint in
`regression/`.

## Crates and dependencies

| Folder | Role |
|---|---|
| `crates/gb-core` | Domain: the Game Boy (CPU, bus, PPU, APU, cartridge MBCs). No I/O, no dependencies |
| `crates/cart-model` | Domain: what is known about this cartridge (levels, objects, world map, patches, RAM map) and use cases on a running console |
| `crates/project-model` | Domain: a project of the editor (what is edited and its rules, level, zone, world map and project files) and the cartridge a project file makes over the base cartridge |
| `crates/host` | Shared adapters: the base cartridge, file locations, a project's cartridge made from its file, battery saves, settings file, PNG output, the system's language |
| `crates/i18n` | Languages and a translation dictionary keyed by the English text (the catalogues: `editor/i18n/editor.txt`, `crates/host/i18n/host.txt`, and the game's `game/i18n/`: `start.txt` the start screen, `menu.txt` the pause menu, `keys.txt` the key names). No dependencies |
| `crates/fonts` | The fonts a language needs beyond the built-in one (`fonts/`, SIL Open Font License), as bytes. Depends on `i18n` only |
| `game/` | Application: the game (`sml26gc.exe`) |
| `editor/` | Application: the editor (`sml26gc_editor.exe`) |
| `crates/app-resources` | Build time only: the programs' icons (`assets/icons/`) and Windows version details, for their build scripts |
| `tools/research/` | Reverse-engineering CLI (`research.exe`) |
| `regression/` | Golden fingerprint test |
| `xtask/` | The release tool (`cargo xtask`): a platform's package folder and zip |

```
 game ─────┐                                       tools/research ─┐
 editor ───┼──► host ──► project-model ──► cart-model ──► gb-core ◄─┘
           └──────────────────►┘
```

- `gb-core` knows nothing about the game, files, macroquad or cpal.
- `cart-model` works on `&[u8]` ROM images and `GameBoy`; no files, no UI.
- `project-model` works on projects and ROM images; no files, no UI. The game uses it only to
  make a project's cartridge from its file (through `host::projects`).
- `host` knows nothing about macroquad or cpal.
- `i18n` is used by `host` and the applications; it knows no files and no system.
- `fonts` is used by the applications; it holds data only (no window library).
- `game` and `editor` do not depend on each other; they share only the contract below.

## Platforms and packaging

What depends on the operating system (window menus, file dialogs, the system language, the
shortcut key, the console, starting the other program, icons and version details) is only in
`adapters/` folders, `crates/host`, `crates/app-resources`, build scripts and `xtask/`, each piece
with a Windows branch and one for other systems (with a macOS one where it differs);
`crates/host/tests/platform_boundary.rs` checks it.
Every data file lives in `host::paths::data_dir()`: the executables' folder (the programs are
portable), `$XDG_DATA_HOME/sml26gc` when the file `installed` sits beside the executables (only
the Linux `.deb` puts it there), or `~/Library/Application Support/sml26gc` when the executable is
in a macOS app bundle (`<name>.app/Contents/MacOS/`), or `/sml26gc` in the browser. Every file is read
and written through `host::storage`: `std::fs` on the desktop; in the browser a map of files in
memory, filled by the page from IndexedDB before the program starts, each change sent back to it
and to the site's other open pages (`host::web`, the page's functions). `host::clock::now()` is
the time (the browser has no system clock). `game_exe()`/`editor_exe()` find the other
program beside the running one, or in its app bundle beside this one's
(`GAME_BUNDLE`/`EDITOR_BUNDLE`, the names `xtask/src/macos.rs` gives them).

- File dialogs: Win32 in the programs' adapters (`game/src/adapters/system_window.rs`,
  `editor/src/adapters/file_dialogs.rs`); elsewhere `host::file_dialog` through `rfd` (macOS: the
  system's panels; Linux: the desktop portal or zenity), which also says when no dialog can open
  (`FileDialogs::available`, always true on macOS).
- The system language: the Windows UI language, or `LC_ALL` / `LC_MESSAGES` / (on macOS, the
  user's preferred languages through CoreFoundation) / `LANG` (`host::system_language`).
- The shortcut key: Ctrl, or Cmd on macOS (`host::shortcut_key`); the editor's `ui::shortcuts`
  reads that key and writes its name where its table and messages say Ctrl. The game's keyboard
  adapter takes Cmd+Ctrl+F as F11 on macOS, which keeps F11 for itself.
- The editor's menu bar: the Win32 one (`editor/src/adapters/menu_bar.rs`) when the system gives
  the window one, else `ui::drawn_menu_bar` along the top of the window. The `MenuBar` port says
  its `height` and reads its `input` before the views; `ui::inset` moves the views below it (a
  camera, and its own `screen_width`/`screen_height`/mouse/key functions, which the view modules
  import by name in place of macroquad's) and keeps clicks and keys from them while a menu is open.

Windows builds link the C runtime in (`.cargo/config.toml`) and use the window subsystem; the
programs call `host::console::attach()` so their command-line modes print to the terminal they
were started from. The product's name, short id, source address and the `.deb`'s maintainer are
`PRODUCT_*` in `.cargo/config.toml`; the version is the workspace's.

In the browser (`wasm32`), the game runs in a page (`packaging/web/`): miniquad's loader (`gl.js`)
and the page's script (`sml26gc.js`), a miniquad plugin that gives the program the `sml_*`
functions `host::web` imports (storage, time, the file chooser, fonts on demand, sound, the
controller, fullscreen, vibration). The game's composition root takes what differs from
`game/src/adapters/platform.rs` (sound and controller adapters, arguments, quitting, the editor,
fullscreen); the web adapters are `web_audio.rs` and `web_pad.rs`. The on-screen controls are a
domain module (`game/src/domain/touch.rs`: layouts, fingers, visibility) and an adapter
(`macroquad_touch.rs`) whose buttons reach the session as a controller, through the controller's
bindings. A tap that must open the file chooser or enter fullscreen is handled by the page
itself, inside the browser's event, on the rectangle the program gives it each frame.

`cargo xtask package TARGET` builds `game` and `editor` (for `web`, the game) with the `dist` profile for the target's
triple: on Windows into `target/<triple>/dist/`; for Linux in a Debian 11 container
(`packaging/linux/Dockerfile`, `xtask/src/linux.rs`) into `target/linux-container/`; for macOS
for both Apple triples, joined by `lipo` into `target/universal-apple-darwin/dist/`
(`xtask/src/macos.rs`). It checks the executables' libraries (Windows: no C runtime DLLs; Linux:
only the allowed ones, glibc 2.31 at most; macOS: only the system's, both architectures, macOS 11
at most), assembles `dist/TARGET/sml26gc/` (executables, on macOS in their signed app bundles,
`packaging/` texts, font licences, the third-party crates' licences from `cargo metadata`) and
writes the target's outputs: the zip, the `.tar.gz` and the `.deb`, or the macOS zip (`ditto`);
for the web, the page, the fonts and the icons (`xtask/src/web.rs`) and the zip. A new platform is one entry of `TARGETS` in `xtask/src/main.rs` and
its adapters' branches.

## `gb-core`

Everything goes through `GameBoy<O: BusObserver>` (alias `Gb`): `run_frame`, `set_buttons`,
`read`/`write` through the bus, `screen`, `audio`, `video`, `cart`. `Buttons` is a typed set.

- `BusObserver` is generic (static dispatch); `NoObserver` compiles away. Research tools use it
  to watch memory accesses.
- No `dyn` in the CPU loop or memory accesses; the MBC is an `enum` (a closed set of hardware).

## `cart-model`

Each module documents the ROM addresses and formats it relies on.

| Module | Contents |
|---|---|
| `rom` | `offset(bank, addr)`, `word`, `set_word` |
| `ram_map` | Named RAM addresses and `GameMode` |
| `levels` | Level headers, map locations, map compression (`decode`/`encode`) |
| `level_goals` | Where a level ends: the finish block $4A and the secret exit block $4B in its map, the bell (object $16) above the finish, the boss levels that have none |
| `secret_exits` | Where a level's secret exit takes Mario: a hook at 0F:6F62 and its table at 0F:7E40 (level to target point; a new level takes its base's entry while played) |
| `level_start` | Where Mario begins a level: header bytes $00-$07 (his y and x, the camera's), the camera worked out from his place |
| `checkpoint` | Midway bells (object $17) and their restart places (0:377B); up to 4 per level through a hook (touch, bell, map flag bits 4-5, restart table in bank $21) |
| `level_settings` | Header bytes of a level's start time ($13), music ($0E) and background shades ($0F) |
| `level_tables` | A level's entries in the bank-0 tables of gravity, auto-scroll, camera margin and animated tiles |
| `level_graphics` | Which levels draw the same objects (enemy sprites 0:362A and the object catalogue split at level 10) |
| `enemy_sets` | A level's enemies chosen from several levels: the sprite block built for them, where the copy keeps it (the low copies of the map banks' maps) and the 0:362A entry |
| `blank_level` | An empty map and start made from a game level's (its sky, a floor under Mario's start) |
| `objects` | Object lists: 3-byte format, reading, re-packing |
| `patch` | The 1 MB edited copy (twin banks $20-$3F) that gives levels and objects more room |
| `layout` | Each zone's space for its maps and packing edited maps back |
| `extra_levels` | Levels 32+, stored in banks $38-$3F and played in place of a base level |
| `new_zones` | New zones, stored in bank $31 and played in place of a base zone (point numbers, entry code, progress kept apart) |
| `start_point` | Where every new game begins (the new save slot's first map byte) and the new zone it is in |
| `world` | World map picture and the level each map point opens |
| `zone_maps` | The zones' one-screen map pictures |
| `original_notes` | What the original game does with some map points (Space Zone flight, entrance twins, unused door) |
| `original_points` | Where the game draws its map points (measured) |
| `map_routes` | Paths between map points (walk scripts), where Mario stands on each, new points; moved to bank $36 in the edited copy, room for 256 numbers |
| `added_points` | Points numbered from $80 on every map: hooks in bank $0F (level, flag, map pick), their flag bytes at $A1A0, the game zones' kept in bank $24 |
| `doors` | Which world points enter which zone and where each zone exit leads |
| `cheats` | What each cheat does in the running game: the invincible patch (0:2E95, 0:1C58), a summoned item in a free object slot, finishing the level through the bell and goal or its secret exit, going to a level, opening the whole map, the active objects, the timer, lives, coins and golden coins, the info panel's values |
| `art` | Pictures captured from the running game (blocks, map tiles, object looks) |
| `live` | What the running game has loaded |
| `driver` | `GameDriver`: boot to the map, enter a level, back to the map, show a map point |
| `error` | `CartError` and what a save used |

## `game/`

```
game/src/
├─ domain/        keys, settings (text format), i18n, overlay (bitmap font), pause menu,
│                 cheats (which there are, the lasting ones, the info and object tags),
│                 touch (the on-screen controls' layouts and fingers)
├─ application/   preferences (settings + saving), session (one frame of play; new levels;
│                 cheats, save states and rewind), zones (new zones swapped in and out,
│                 their progress)
├─ ports.rs       Keyboard, Gamepad, AudioOut, Window, SettingsStore, SaveStore, ZoneProgressStore
├─ adapters/      macroquad keyboard/display/picker/touch, cpal and web audio, gilrs and web
│                 controller, file stores, Win32, platform (what differs in the browser)
├─ checks.rs, selftest.rs   headless checks (--research, --self-test, --dump)
└─ main.rs        composition root
```

`domain/` and `application/` use no macroquad, cpal, `host` or `std::fs`; tests replace the
ports with fakes.

## `editor/`

```
editor/src/
├─ domain/        document (everything being edited, dirty flags, commit to the ROM image),
│                 history (generic undo/redo), tools (Tool trait: pencil, box, select, paste),
│                 stamp (a rectangle of cells: brush and clipboard), prefs (editor.cfg text),
│                 files (level, zone and project files: text format); document/zones
│                 (new zones), document/restore and document/check (level warnings),
│                 look_cache (captured object pictures and tiles as text), object_catalogue
│                 (the game's objects once each), object_names (the editor's names for them)
├─ application/   save (commit + write through a port), art_service (captured graphics),
│                 exchange (level, zone and project files through the dialogs), music
│                 (a level's music played by a console of the copy), look_loader (object
│                 pictures and tiles captured on background threads, one per spare core)
├─ ports.rs       EditedCopyStore, GameLauncher, PrefsStore, FileDialogs, TextFiles, MenuBar,
│                 SoundOut, LookStore
├─ adapters/      file repository (edited copy, editor.cfg, text files), Win32 file dialogs
│                 and menu bar, the game's launcher, cpal speakers
├─ ui/            macroquad panels: level, objects, palettes, world map, status bar, widgets;
│                 grid_edit (painting and clipboard shared by both views), shortcuts (one
│                 table for keys and button labels), menu (the File menu), files_view
│                 (file actions and their messages), names_view (naming new levels and
│                 zones), target_picker (where a door or exit leads, chosen on the maps),
│                 level_panel (the level side panel: picker, toolbar, tabs, inspector),
│                 testing_view (checkpoints and play here drawn on the level)
└─ main.rs        composition root
```

The world map editor has two tabs: Picture (paint the world, zone and new zone maps) and
Points (the level each point opens, paths, new points, doors, exits, entries).

## `tools/research/`

A table of commands (`COMMANDS` in `main.rs`) over the cartridge: disassembly, memory watch and
trace, ROM views, and checks that play the edited data in the emulator (`routecheck`,
`doorcheck`, `newzonecheck`, `sparecheck`...). `all_commands.sh` runs a fixed set and saves the output to compare builds.

## Contract between game and editor

| Item | Format |
|---|---|
| Start the game | `sml26gc.exe <rom> [--project NAME \| --test [--lives N] [--coins N] [--power N]] [--level N [--start X,Y] \| --point P] [--lang ID] [--cheats]` (no rom: the start screen; `--test`: no saves, those lives and coins, 99 and 999 when not given, Mario's power 0-3; `--start`: Mario's left x and feet y for that start only; `--point`: a map point in hex, `PP@K` in new zone K; `--lang`: the language of that game only) |
| Start the editor (from the game) | `sml26gc_editor.exe [--project NAME \| --new-project]` (the project opened, or the project screen typing a new one) |
| Base cartridge (checked by SHA-1) | `base_cartridge.txt` |
| A project (its file, and the cartridge made from it over the base cartridge when missing or older) | `editor/projects/<name>/<name>.nmbproject`, `editor/projects/<name>/<base name> - <name>.gb` |
| Battery saves | `saves/vanilla/game.sav`, `saves/<project>/game.sav` |
| New zone and added point progress (game only) | `saves/vanilla/zones.txt`, `saves/<project>/zones.txt` |
| Settings (the editor reads the language) | `settings.cfg` |
| Editor settings (palette zoom, listening volume) | `editor.cfg` |
| What imported new levels and zones restore to (editor only) | `editor/projects/<name>/restore_points.txt` |
| Captured object pictures (editor only) | `cache/looks-<hash of the cartridge>.txt` |
| Level, zone, world map and project files (editor only) | `exports/levels/*.nmblevel`, `exports/zones/*.nmbzone`, `exports/world/*.nmbworld`, `exports/projects/<name>/<name>.nmbproject` (any folder may be chosen) |

Paths and names live in `host::paths`, so both programs compute them the same way.

## Tests

1. Golden fingerprint (`regression/`): must match `golden.txt`; an intended behaviour change
   updates the file on purpose.
2. Unit tests of the pure domain (codecs, history, tools, menu, routes, doors...); tests that
   need the cartridge skip themselves without it.
3. Research checks that play edits in the emulator.
