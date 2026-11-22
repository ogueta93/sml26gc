# editor/menu-bar Specification

## Purpose
The editor window's menu bar, like other programs have: the File menu gathers saving and the
level and project files, each entry showing its shortcut; the Tools menu opens the object
catalogue and captures the object pictures again.

## Requirements

### Requirement: File menu
The editor window SHALL have a menu bar above the panels with a File menu (Spanish: Archivo)
holding, in order: Save; Export level, Import level as a new level, Replace this level from a
file; Export zone, Import zone as a new zone, Replace this zone from a file; Export project,
Import project. Each entry SHALL show its shortcut and run the same action as the shortcut.

#### Scenario: Export a level from the menu
- **WHEN** the level editor is shown and the user opens File and chooses Export level
- **THEN** the Save dialog for a level file opens, as with Ctrl+E

#### Scenario: Shortcuts shown
- **WHEN** the user opens the File menu
- **THEN** each entry shows its keys at the right, for example "Export project... Ctrl+Shift+E"

#### Scenario: Export a zone from the menu
- **WHEN** a zone map is shown in the world map editor and the user chooses Export zone
- **THEN** the Save dialog for a zone file opens, as with Ctrl+E there

### Requirement: Unavailable entries greyed out
File menu entries that cannot run where the editor is SHALL be greyed out and do nothing:
the level file entries in the world map editor, the zone file entries in the level editor and
on the world map, Replace on the game's 32 levels and Replace on the game's zones.

#### Scenario: World map editor
- **WHEN** the world map editor shows the world map and the user opens the File menu
- **THEN** Save and the project entries can be chosen, and the level and zone file entries are greyed out

#### Scenario: Original level
- **WHEN** level 07 is shown and the user opens the File menu
- **THEN** Replace is greyed out, and it is available again when a new level is shown

#### Scenario: Zone map
- **WHEN** the Macro Zone is shown and the user opens the File menu
- **THEN** Export zone and Import zone can be chosen and Replace zone is greyed out

### Requirement: No file buttons in the panels
The level and project file actions SHALL be reached from the File menu and the keyboard only;
the side panels SHALL NOT have buttons for them.

#### Scenario: Level panel
- **WHEN** the level editor is shown
- **THEN** its side panel has no export or import buttons and its palette starts where it did before this change

### Requirement: Tools menu
The window SHALL have a Tools menu beside File with "Capture the object pictures again": it
SHALL forget every captured object picture and tile, the cache too, and capture them all again
in the background, the progress box showing how far it is.

#### Scenario: Pictures captured again
- **WHEN** Tools, Capture the object pictures again is chosen
- **THEN** the progress box appears and counts every picture and enemy tile again

### Requirement: Catalogue in the Tools menu
The Tools menu SHALL have "Object catalogue" (with its shortcut, F3) as its first entry, which
opens the catalogue as F3 does.

#### Scenario: Opened from the menu
- **WHEN** Tools, Object catalogue is chosen while a level is shown
- **THEN** the object catalogue opens on that level

### Requirement: Catalogue only in the level editor
Tools, Object catalogue SHALL be greyed out and do nothing outside the level editor (the world
map and zone maps).

#### Scenario: World map
- **WHEN** the world map editor is shown and the Tools menu is opened
- **THEN** Object catalogue is greyed out

### Requirement: Project entries
The File menu SHALL have Projects... (the project screen: new, open, rename, duplicate, delete,
import) and Export project. The world map has no entries of its own: the zone entries export
it and replace it (`editor/zone-files`).

#### Scenario: Level editor
- **WHEN** a level is shown and the File menu is opened
- **THEN** Projects... and Export project can be chosen, and no world map entry is listed

### Requirement: Language menu
The window SHALL have a Language menu after Tools (Spanish: Idioma, Japanese: 言語) with
Automatic (the system's language), English, Español and 日本語, each language written in
itself; the one in use SHALL be ticked (Automatic when no language was chosen). Choosing one
SHALL change the editor's language (`editor/languages`), the menu bar's own labels included.
The menu SHALL be there in the editor and on the project screen.

#### Scenario: Japanese from the menu
- **WHEN** Language, 日本語 is chosen in the level editor
- **THEN** 日本語 is ticked, the panels and the File and Tools menus read in Japanese, and the menu's title reads 言語

#### Scenario: Automatic
- **WHEN** Language, Automatic is chosen on a Windows set to Spanish
- **THEN** Automatic is ticked and the editor is in Spanish

### Requirement: Menu bar inside the window on Linux
On Linux and macOS, where the editor's window has no native menu bar of its own, the editor SHALL draw its menu bar along the top of its own window, above the panels, with the same menus, entries, shortcuts shown (with Cmd in place of Ctrl on macOS), greyed-out and ticked entries as on Windows, in the editor's language. The panels SHALL start below it, and nothing the editor shows SHALL be hidden under it.

#### Scenario: Opening File with the mouse
- **WHEN** the user clicks File in the editor on Linux
- **THEN** the File menu opens below it, listing its entries with their shortcuts, and clicking Export level opens the Save dialog for a level file, as Ctrl+E does

#### Scenario: Closing a menu
- **WHEN** a menu is open and the user clicks outside it or presses Esc
- **THEN** the menu closes and nothing is chosen, and the click does not edit the level under it

#### Scenario: Greyed-out entries
- **WHEN** the world map editor is shown on Linux and the File menu is opened
- **THEN** the level and zone file entries are greyed out and clicking them does nothing

#### Scenario: Moving between menus
- **WHEN** the File menu is open and the pointer moves over Tools
- **THEN** the Tools menu opens in its place

#### Scenario: Language ticked
- **WHEN** the Language menu is opened on Linux after choosing Español
- **THEN** Español is ticked and the bar reads Archivo, Herramientas, Idioma

#### Scenario: Shortcuts shown on macOS
- **WHEN** the user opens File in the editor on macOS
- **THEN** the bar is drawn inside the window and the entries show their keys with Cmd, for example "Export project... Cmd+Shift+E"
