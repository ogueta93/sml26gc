# SML2 6GC Emulator and Level Editor

Two programs for Super Mario Land 2: 6 Golden Coins, written in Rust, that use the cartridge file
**Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1).gb**, made from your own cartridge:

- **`sml26gc`**, the game: an emulated Game Boy runs the original cartridge's program unchanged,
  with a resizable window or full screen, colour palettes, keyboard and controllers with any
  bindings, a pause menu in 11 languages, fast forward and, when wanted, cheats, save states and
  rewind. It plays the original game and every project made in the editor, each with its own
  saves. Manual: [`docs/GAME.md`](docs/GAME.md).
- **`sml26gc_editor`**, the level editor: edits the game's levels, objects, world map and zones
  in projects (new levels, new zones, map points, checkpoints, secret exits, level settings),
  each saved as an edited copy of the cartridge that the game's own program plays. Manual:
  [`docs/EDITOR.md`](docs/EDITOR.md).

<p align="center">
  <a href="https://github.com/user-attachments/assets/8e903758-367d-45dd-a2b7-1f1c42a80132"><img height="200" alt="The editor placing objects in one of the game's levels" src="https://github.com/user-attachments/assets/8e903758-367d-45dd-a2b7-1f1c42a80132"></a>
  <a href="https://github.com/user-attachments/assets/8ba4f9d2-a19a-44c6-83d7-51c9340cd75c"><img height="200" alt="The edited level played in the game" src="https://github.com/user-attachments/assets/8ba4f9d2-a19a-44c6-83d7-51c9340cd75c"></a>
  <br>
  <sub>A level changed in the editor, then played in the game (click to enlarge). Recorded with
  the author's own cartridge backup: the game graphics shown belong to Nintendo and appear only
  to illustrate the tools; no game data is included in or distributed with this project
  (<a href="LEGAL.md">LEGAL.md</a>).</sub>
</p>

No game is included: you use a cartridge file from your own backup of a cartridge you own. The
programs accept exactly one dump, revision 1 of the game, recognised by its contents whatever the
file is called:

| Cartridge file | Revision | SHA-1 |
|---|---|---|
| Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1).gb | 1 | `96e3a314561fb394cdf51101f9178a32713c2313` |

The game was made in three revisions with the same title: revision 0 (often named without
"(Rev 1)", SHA-1 `bba408539ecbf8d322324956d859bc86e2a9977b`) and revision 2 are refused, as is
any modified file. A file's revision is the byte at address $014C of its header (`01` for
revision 1); its SHA-1 says whether it is exactly the supported dump (`sha1sum FILE` on Linux,
`shasum -a 1 FILE` on macOS, `Get-FileHash -Algorithm SHA1 FILE` in Windows PowerShell). Cartridges, saves and projects never go into this
repository (see `.gitignore`). Not affiliated with or endorsed by Nintendo.

## How it is made

This project is developed with the help of AI coding agents, directed and reviewed by its
author. Which agent is used is not named, on purpose: any agent, or none, is welcome. What counts
is the code, and the same checks apply to every change, whoever or whatever wrote it: the rules
in [`AGENTS.md`](AGENTS.md), the tests (with the regression fingerprint, which holds the emulator
to its recorded behaviour) and the specs in `openspec/specs/`.

## Playing

The programs run on Windows 11, Linux (x86-64; Debian 11, Ubuntu 20.04 and later
distributions) and macOS 11 Big Sur or later (Apple Silicon and Intel). Nothing needs installing besides them; the first time, either program asks for
your cartridge file.

- **Windows**: unpack `sml26gc-<version>-windows-x64.zip` in a folder you can write to, such as
  Documents, and open `sml26gc.exe` or `sml26gc_editor.exe`.
- **Linux**: unpack `sml26gc-<version>-linux-x64.tar.gz` in your home folder and run
  `./sml26gc` or `./sml26gc_editor` in the `sml26gc` folder (any distribution); or, on Debian,
  Ubuntu, Mint and the like, install `sml26gc_<version>_amd64.deb`
  (`sudo apt install ./sml26gc_<version>_amd64.deb`) and open them from the application menu.
- **macOS**: unpack `sml26gc-<version>-macos-universal.zip`, move `SML2 6GC.app` and
  `SML2 6GC Editor.app` together into Applications, and open each the first time as its read-me
  says (the apps are not signed with an Apple developer identity).

What a computer needs and what to do when a program does not start: the read-me in the package,
[`packaging/windows/README.txt`](packaging/windows/README.txt),
[`packaging/linux/README.txt`](packaging/linux/README.txt) or
[`packaging/macos/README.txt`](packaging/macos/README.txt).

## Documentation

| Document | For |
|---|---|
| [`docs/GAME.md`](docs/GAME.md) | Playing: what the game adds to the original, controls, menus, cheats |
| [`docs/EDITOR.md`](docs/EDITOR.md) | Editing: projects, levels, objects, maps, zones, test games, files |
| [`packaging/windows/README.txt`](packaging/windows/README.txt), [`packaging/linux/README.txt`](packaging/linux/README.txt), [`packaging/macos/README.txt`](packaging/macos/README.txt) | Running the release on a computer: requirements and problems |
| [`docs/DEVELOPMENT.md`](docs/DEVELOPMENT.md) | Developers: tools to install, building, checks, the Windows, Linux and macOS packages, how the emulator and the editor were made |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | The code's layout, dependencies and the contract between the two programs |
| [`docs/MECHANICS.md`](docs/MECHANICS.md) | The game's rules, read from its program |
| [`AGENTS.md`](AGENTS.md) | Rules for changing the repository |
| [`CHANGELOG.md`](CHANGELOG.md) | What each version brings |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Contributing: where the rules are |
| [`LEGAL.md`](LEGAL.md) | Licence, what is and is not included, legal grounds, responsibility |

## Licence

Copyright 2026 ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors. Free software under the GNU General Public License, version 3 or later
([`LICENSE`](LICENSE)); the fonts are under the SIL Open Font License 1.1. Which licence covers
each file: [`REUSE.toml`](REUSE.toml). Legal notice: [`LEGAL.md`](LEGAL.md).
