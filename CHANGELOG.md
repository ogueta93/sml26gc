# Changelog

What each version of the programs brings, as a person using them notices it. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/) (see `docs/DEVELOPMENT.md`, "Workflow and releases").

## [Unreleased]

### Added

- The game in a web browser (`sml26gc-<version>-web.zip`), on a computer, a phone or a tablet:
  serve the folder with any web server and open it. The cartridge you choose (or drop on the
  page) stays in the browser, with your settings and saves; clearing the site's data removes
  them.
- On a phone or tablet, on-screen controls: a D-pad that follows the finger, A and B (both at
  once between them), Select, Start and the pause menu, upright like a handheld or across with
  the picture between them, and smaller buttons for fast forward, and with the cheats mode
  rewind, save and load state and restart level. In the pause menu, Controls: when they show,
  their size and vibration. Turned off with nothing else to play with, a small button brings
  them back.

- The editor in a web browser, in the same package (`editor.html`, or Edit and New project on
  the game's start screen), on a computer with a mouse or trackpad: it shares the cartridge and
  the projects with the game in that browser. Exported files are downloaded; imports use the
  file chooser or a file dropped on the page; test games open in a tab of their own. Alt+N and
  Alt+Shift+N take the place of Ctrl+N and Ctrl+Shift+N, which the browser keeps. Closing the
  page with unsaved changes asks first.
- Editor: File, Export cartridge writes the project's edited copy as a `.gb` file, to play it
  in another emulator or on another computer.
- Packages for ARM: `linux-arm64` (a Raspberry Pi 4 or 5 with a 64-bit system, ARM laptops;
  `.tar.gz` and `_arm64.deb`) and `windows-arm64` (Windows on ARM laptops), which run natively.
- The latest version plays online, in the browser, at https://ogueta93.github.io/sml26gc/ (the
  editor at `editor.html`).
- Editor: the world map and each zone, new zones too, can play another music: any the game
  plays on a map or in a level, chosen in the zone editor (Listen plays it). Zone, world map and
  project files keep it.

### Changed

- The Linux packages run on older distributions: glibc 2.27 instead of 2.31, so Ubuntu 18.04,
  Linux Mint 19, Debian 10, RHEL, AlmaLinux and Rocky Linux 8 and later. The `.deb` also
  depends on libxkbcommon0, which the programs need on X11.
- The Windows packages support Windows 10 as well as Windows 11.
- Editor: the object pictures and the enemies' tiles are captured several times faster, and
  the shown level's palette and enemies come first.
- Display, Scale has a third choice, stretched: the picture fills all the room (the window, or
  the room the on-screen controls leave), out of shape. A phone shows a larger picture with
  fitted than with sharp.
- The pause menu's Controls shows only the inputs there are: the controller's bindings while one
  is connected, the keyboard's on a computer. With a controller connected it opens on the
  controller's bindings.
- The pause menu's footer, the start-up tag and the save state notes name the controller's
  buttons while one is connected (it comes before the keyboard), and A and B with the on-screen
  controls.
- In the browser, a fullscreen button on the start screen and beside the on-screen menu button;
  on an iPhone, the start screen says how to add the game to the home screen to play without
  the browser's bars.

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
