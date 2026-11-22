# The game (sml26gc)

`sml26gc` (`sml26gc.exe` on Windows) plays Super Mario Land 2: 6 Golden Coins from your own cartridge file, and the
projects made in the editor. The cartridge's own program runs on an emulated Game Boy, so the
game plays exactly as the original; everything below is added around it.

## What it adds to the original

| | Original Game Boy cartridge | sml26gc |
|---|---|---|
| The game | | The same program, rules, levels, music and saves, unchanged |
| Edited games | | Plays the projects made in the editor (new levels, zones, map points, checkpoints...), each with its own saves |
| Screen | 160x144, green | Any window size or full screen, sharp (whole pixels) or fitted; classic, green or grey colours |
| Controls | The console's buttons | Keyboard and any controller at once, every button can be changed |
| Pause | Start | A menu over the game: the game's own pause, controls, display, sound, language, back to the map, file select, quit |
| Language | English | The menus in 11 languages (the game's own texts stay as they are) |
| Speed | | Fast forward while a key is held |
| Help | | Cheats, save states (save and load anywhere) and rewind, when the "Debug / cheats" box is on |
| Saves | The cartridge's battery | The same save file, kept beside the program (`saves\`), one per game or project |

## Starting

The first time, the game asks for your Super Mario Land 2 - 6 Golden Coins (USA, Europe)
(Rev 1) cartridge file (`.gb`, any file name; revision 1 only, see the README). It is checked by its contents at every start and shared
with the editor; it is only read, never changed.

The start screen then shows what can be played, one at a time, as a cartridge with its title
screen running:

- **VANILLA**: the original game.
- Every project made in the editor, by its name.

| Key / button | What it does |
|---|---|
| Left / Right (arrows, wheel, the round buttons) | The previous / next one (the last one played comes first) |
| Enter, Play | Plays the one shown |
| E | Opens the project in the editor |
| N | Opens the editor to make a new project |
| C, the "Debug / cheats" box | Turns on the cheats mode (kept for next time) |
| L, the language list (top right) | The language of the screens and menus (until one is chosen, the system's) |
| O, Cartridge... | Chooses another base cartridge file |
| Esc, Quit | Closes the game |

A project's cartridge is made again from its project file when it is missing or older, so a
project saved in the editor plays at once.

## Controls

| Game Boy | Keyboard (default) | Controller (default) |
|---|---|---|
| D-pad | Arrows | D-pad or left stick |
| A (jump) | X | B / right button |
| B (run, fire) | Z | A / bottom button |
| Start | Enter | Start |
| Select | Backspace | X / left button |

| Port | Keyboard | Controller |
|---|---|---|
| Pause menu (and back in it) | Esc | Back / View / Share, or the guide button |
| Fast forward (held) | Left Ctrl | RT |
| Save state / load state (cheats mode) | F6 / F7 | LB / RB |
| Rewind (held, cheats mode) | R | LT |
| Restart level (cheats mode, test games) | F8 | |
| Full screen | F11 (macOS: also Cmd+Ctrl+F) | |

A controller (on Windows XInput or Windows.Gaming.Input, on Linux and macOS any the system sees) plays as
soon as it is connected, together with the keyboard. Every binding is changed in the pause menu, Controls.

## The pause menu

Esc opens it (in a level, Enter does too); the frame's footer shows the keys. Esc goes back.

| Item | What it does |
|---|---|
| Original pause | In a level: the game's own pause screen |
| Debug / cheats | With the cheats mode on: the cheats, below |
| Controls | Input (keyboard or controller), then each binding: Enter, then the new key or button. Restore defaults puts them back |
| Display | Colours (classic, green, grey, with a sample), full screen, scale (sharp: whole pixels; fitted: fills the window) |
| Sound | Volume, 0 to 10 |
| Language | English, Español, 日本語, Português, Français, Deutsch, Italiano, Bahasa Indonesia, Русский, 简体中文, 한국어 |
| Restart level | Cheats mode or a test game: the level again, without losing a life |
| Back to map | In a level: leaves it for the map |
| File select | The game's file select (to change save slot) |
| Quit | Closes the game |

Settings are kept in `settings.cfg`, with your other files (below).

## Cheats mode

On with the start screen's "Debug / cheats" box, and always on in games started from the
editor. It never changes the cartridge or the project, only the game being played.

| Cheat | What it does |
|---|---|
| Invincible | On/off: enemies and spikes do not hurt Mario (falling off still does) |
| Finish level | Ends the level as its goal with the bell rung: bonus game, then the map |
| Secret exit | Ends the level through its secret exit, to where it leads (levels that have one) |
| Go to level | Any level of the game or project (new ones by name), entered from its map point |
| Summon item | Star, mushroom, fire flower, carrot or heart (a life) in front of Mario |
| Freeze timer | On/off: the level's timer stops |
| 99 lives, 999 coins | Sets the counter |
| 6 golden coins | The six bosses beaten: Wario's castle opens (their levels close, as in the game) |
| Open the whole map | Every level cleared, secret exits too: every path opens |
| Info panel | On/off: level, Mario's position, map point, game mode and active objects |
| Objects view | On/off: each active object's type over it |

Finish level, Secret exit and Summon item work only in a level. Also in this mode:

- **Save state / load state** (F6 / F7): the whole game at that moment, kept until the game
  closes.
- **Rewind** (hold R): plays the last 10 seconds backwards.

When a level begins, a tag lists these keys.

## Games from the editor

The editor's Play (F5), Play here (Shift+F5) and Play from a point start this program on the
project: with the cheats mode on, no saves kept, the lives, coins and Mario's power set in the
editor, in the editor's language, and Restart level instead of Back to map when one level is
tried.

## Your files

All beside the program: `base_cartridge.txt` (the cartridge chosen), `settings.cfg`,
`saves/vanilla/` (the original game's battery save) and `saves/<project>/` (each project's, and
its progress in new zones). Keep the folder where it can be written; the game warns when it
cannot. Installed on Linux from the `.deb`, they are in `~/.local/share/sml26gc/` instead; on
macOS, in `~/Library/Application Support/sml26gc/`, shared by both apps.

## Command line

    sml26gc ["path/to/cartridge.gb"] [--project NAME] [--cheats] [--level N [--start X,Y]]

Without a cartridge the start screen opens. `--project` plays that project with its saves;
`--level N` starts level N (with `--start`, Mario at that x and feet y). The editor adds more
options for its test games.
