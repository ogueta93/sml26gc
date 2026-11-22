# The editor (sml26gc_editor)

`sml26gc_editor` (`sml26gc_editor.exe` on Windows) changes Super Mario Land 2: 6 Golden Coins in projects: each project is an
edited copy of your cartridge that the game's own program plays, so every edited level keeps the
original's physics, enemies and music. Your base cartridge is never changed.

## What it lets you do

- Edit the game's 32 levels, block by block, and their objects (enemies, items, platforms...).
- Add new levels (copies of a level, or blank ones played with a game level's graphics).
- Move where Mario starts, where a level ends, its secret exit and where it leads, and add
  midway bells (checkpoints).
- Change a level's time, music, background shades, gravity, auto-scroll, camera and animated
  tiles.
- Bring enemies from other levels into a level.
- Repaint the world map and the zones' maps, change which level each point opens, draw paths,
  add map points, set doors and exits, and where a new game begins.
- Add new zones, each with its own map, points, levels and progress.
- Play what is being edited at once, from the level's start or any point.
- Share levels, zones, world maps and whole projects as files.
- Use it in 11 languages.

## Projects

At start (and from File, Projects..., Ctrl+Shift+O) the project screen lists the projects, the
last one first, and creates, renames, duplicates, deletes or imports them. Each project is a
folder (`editor/projects/<name>/`) with its project file (what was edited) and the cartridge made
from it over the base cartridge; the cartridge is made again when missing or older, also by the
game. A project opens where it was last saved (the level, or the zone editor's map), a new one on
the world map. Leaving with unsaved changes asks to save, discard or cancel.

The first time, the editor asks for the base cartridge, shared with the game.

## The window

Two editors, each with a side panel:

- **Level editor**: the level picker (its grid unfolds with the middle button, with Copy, Blank
  and Delete under it), a toolbar (Undo, Redo, Restore; Save, Play, Play here), Blocks / Objects
  with their tools, and two tabs: Palette, and Level (start, goals, checkpoints, settings and
  warnings; it scrolls with the wheel).
- **Zone editor** (F2, Zones): the world map and the zones, laid out the same way: map picker,
  toolbar, Go to Mario's start, Picture / Points.

Every button has a shortcut, shown in its tooltip. On macOS, Cmd takes the place of Ctrl in every
shortcut (Cmd+S, Cmd+Z), and the editor shows it so. Ctrl+Z / Ctrl+Y undo and redo every change in
the order made, whatever the mode or tab. Ctrl+S saves the project's cartridge. Messages say
their kind: done in green (+), warnings in orange (!), errors in red (x). A text cut short
(ending in "..") shows whole in a tooltip. Palettes zoom with their - + buttons or Ctrl - /
Ctrl +; the side panel widens to fit (at most half the window); a cut palette scrolls with the
wheel or a right-drag.

The Language menu: English, Spanish, Japanese, Portuguese, French, German, Italian, Indonesian,
Russian, Simplified Chinese or Korean; Automatic follows the system's. Kept in `editor.cfg`.

The File, Tools and Language menus are in the window's menu bar. On Linux and macOS the editor draws it
along the top of its window: a click on a title opens its menu (then the pointer moves between
menus), a click on an entry chooses it, and a click elsewhere or Esc closes it.

## Levels

- **Changing level**: PgUp / PgDn, or the level picker. A level or map that differs from the
  game's is marked "mod"; levels and zones made in the editor carry a stripe.
- **Painting blocks**: Pencil (B) and Box (R) paint the palette selection; drag over the palette,
  or right-drag on the map, to take several cells at once. Select (E), then Ctrl+C / Ctrl+X /
  Del, and Ctrl+V to paste (Esc stops).
- **Objects** (Tab): the palette offers the types the level draws right (its own, and items that
  look the same there), filtered by category with < >; Show all adds the others, framed, as they
  may look broken. Click to place, drag to move, right-click to delete; Shift+click changes the
  selected object's type. Clicking the chosen type again, or right-clicking an empty place,
  leaves none chosen.
- **Start and goals**: Mario's start is drawn on the level; Place start (I) puts him on the block
  clicked, or drag him (the camera follows). Place finish (F) moves the goal (the bell above it
  goes along); boss levels have none. Secret exit (Shift+F) places a secret exit block, and Leads
  to (U) chooses on the maps where it takes Mario.
- **Checkpoints** (K): a midway bell at the cell clicked; after ringing it and losing a life,
  Mario starts again just left of it (drawn as his outline). Up to 4 per level; level 31 cannot
  have them.
- **Level settings** (Level tab): start time (100-900), music (Listen plays it, at the volume
  kept in `editor.cfg`), background shades, gravity (normal or space), auto-scroll (Mario caught
  against a wall loses a life), camera margin and animated tiles.
- **Warnings** (Level tab): no finish, a bell without its finish, more than 6 objects in one
  screen or 83 in all, objects on the screen the level begins on (the game wakes objects only at
  the screen's edge, so they never appear), Mario's start or a restart inside blocks, a map
  point opening a level that does not exist. A click shows the place; each level button shows its
  count.
- **New levels**: Ctrl+N copies the level shown; Ctrl+B (Blank level) shows the game's 32 levels
  with only their blocks and the objects they draw right, and makes an empty level (sky, a floor,
  Mario's start) that plays as the one chosen (tiles, objects, music, time). New levels have a
  name.
- **Object catalogue** (F3, or Catalogue in the Objects palette, or the Tools menu): every object
  of the game once, with name, picture, category and the levels that use it, filtered by Show
  (all, compatible, usable here, can be added) and category. An enemy, trap or platform of
  another level of the same group (levels 0-9 or 10-31) can be added to the level's enemy set
  when its tiles fit in the level's 56 enemy tiles (each cell says whether it fits, and with what
  it clashes). Remove from this level takes an object of the set out, with every object of that
  type; one Ctrl+Z brings them back. Objects brought from other levels have a corner mark in the
  palette.
- **Restore** (Ctrl+Shift+R): puts the level or map shown back as in the game (a new level or
  zone, as its base).

## World map and zones

- **Picture tab**: paint the world map and the zones' maps, as with blocks.
- **Points tab**: choose each point's level, draw paths, add points (46 new on the world map; up
  to 40 in all on each zone but the bonus game) and set doors and exits; where one leads is
  chosen on the maps (U). Points are dragged (their paths become straight). Points numbered from
  $80 work only with this game; their paths can be shut until the point's level is cleared
  (Ctrl+arrow keys). The tab says what the original game does with the chosen point and whether
  it was changed.
- **Mario's start** (I in the Points tab): the point where every new game begins, on the world
  map, a zone or a new zone, marked on its map; Go to Mario's start (M) shows it. A project has
  exactly one.
- **New zones**: the map name opens the list of every map, with Copy zone (Ctrl+N), Blank zone
  (Ctrl+B: each base zone's map and tiles to choose from) and Delete zone (Ctrl+Del). A new zone
  (from the Tree, Pumpkin, Macro, Space or Mario zone) has its own name, picture, points, paths,
  levels, exits and progress, plays with its base zone's graphics and music, and is entered from
  any world point.

## Playing what you edit

- **Play** (F5): saves and plays a test game of the level shown: cheats on, no saves kept, with
  the lives, coins and Mario's power set in the window the small button beside Play opens.
- **Play here** (Shift+F5), then a click on the level: Mario starts on the clicked block.
- **Play from a point** (zone editor): the game starts on the chosen map point.

The game speaks the editor's language. Only one test game runs at a time. To play the project
with its own saves, choose it on the game's start screen.

## Files

From the File menu (Save and Open dialogs start in `exports\`):

| Keys | On a level | On a zone map | On the world map |
|---|---|---|---|
| Ctrl+E | Export the level | Export the zone | Export the world map |
| Ctrl+Shift+N | Import a level as a new one | Import a zone as a new one | |
| Ctrl+I | Replace the new level shown | Replace the new zone shown | Replace it (picture, points, paths, doors) |

The game's 32 levels are never replaced, only edited. Ctrl+Shift+E exports the whole project;
the project screen imports one as a new project. Imports are kept when the project is saved.
Files made from a project are derived from your cartridge: for your own use.

## Object pictures

The pictures of the objects are captured from the game running in the background (the level
shown first) and kept in `cache\`, so later starts have them at once. Until it ends, a box at
the bottom right shows how far it is. Tools, Capture the object pictures again starts over.

## Your files

All beside the program: `editor.cfg` (language, zoom, listening volume, test game values), the
projects (`editor/projects/`), `exports/` and `cache/`; `base_cartridge.txt` is shared with the
game. Installed on Linux from the `.deb`, they are in `~/.local/share/sml26gc/` instead; on macOS,
in `~/Library/Application Support/sml26gc/`, shared by both apps.
