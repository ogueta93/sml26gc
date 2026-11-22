# Changelog

What each version of the programs brings, as a person using them notices it. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) (see `docs/DEVELOPMENT.md`, "Workflow and releases").

## [Unreleased]

## [1.2.0] - 2026-11-22

### Added

- macOS 11 Big Sur and later, on Apple Silicon and Intel Macs: the game and the editor as two
  apps (`sml26gc-<version>-macos-universal.zip`), your files in
  `~/Library/Application Support/sml26gc`. On macOS the file dialogs are the system's, the
  editor's shortcuts use Cmd in place of Ctrl, Cmd+Ctrl+F also turns full screen on and off, and
  the language follows the Mac's preferred languages.

### Fixed

- An object placed in one of the game's levels to the left of objects already there now appears
  when the level is played; before, it could be missing (new levels were not affected).

## [1.1.0] - 2026-11-21

### Added

- Linux (x86-64; Debian 11, Ubuntu 20.04 and later distributions): the game and the editor as a
  folder to unpack and run (`sml26gc-<version>-linux-x64.tar.gz`), or installed from
  `sml26gc_<version>_amd64.deb` with launchers in the application menu and your files in
  `~/.local/share/sml26gc`. On Linux the file dialogs are the desktop's own, the editor draws its
  menu bar in its window, and the system's language comes from the locale.

### Changed

- The programs, the read-me and the manuals name the one cartridge file accepted with its
  revision, Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), and say that revisions 0
  and 2 are refused. The accepted file is the same as before.
- The copyright notices (read-mes, legal notice, the programs' file details) name the author:
  ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors.

## [1.0.0] - 2026-11-21

### Added

- The game (`sml26gc`): plays Super Mario Land 2: 6 Golden Coins from your own cartridge file on
  an emulated Game Boy, and every project made in the editor, each with its own saves. Start
  screen with the projects; any window size or full screen; classic, green or grey colours;
  keyboard and controllers with changeable bindings; pause menu; fast forward; a cheats mode
  with cheats, save states and rewind.
- The editor (`sml26gc_editor`): projects over your cartridge; edits the 32 levels (blocks,
  objects, start, goal, secret exits, checkpoints, time, music, gravity, auto-scroll, camera,
  animated tiles), new and blank levels, enemies brought from other levels, the world map and
  the zones (pictures, points, paths, doors, Mario's start), new zones; test games of what is
  edited; level, zone, world map and project files.
- Both programs in English, Spanish, Japanese, Portuguese, French, German, Italian, Indonesian,
  Russian, Simplified Chinese and Korean; the system's language until another is chosen.
- A package for Windows 11 that runs without installing anything, with a read-me, the user
  manuals, the changelog, the licence and the legal notice.
