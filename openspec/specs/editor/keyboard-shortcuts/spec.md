# editor/keyboard-shortcuts Specification

## Purpose

Every action of the editor's side panel can be done from the keyboard, and the panel shows
which key does it.

## Requirements

### Requirement: A shortcut for every panel action
Every action a panel button does SHALL also have a keyboard shortcut, active only in the view
and tab where the button is shown. Shortcut keys SHALL sit in the same place on US and Spanish
keyboards.

#### Scenario: Shortcut outside its tab
- **WHEN** the Picture tab is shown and the user presses N
- **THEN** no new point is placed

### Requirement: Existing keys kept
Keys in use before this change SHALL keep their meaning: arrows/WASD and Space pan, G grid,
B Pencil, R Box, M overview, Tab Blocks/Objects, Del/Backspace delete the selected object, F5
play, Ctrl+S save, Ctrl+Z undo, Ctrl+Y or Ctrl+Shift+Z redo.

#### Scenario: Pan keys
- **WHEN** the user holds D in any view
- **THEN** the view pans right and no action runs

### Requirement: Level and map navigation shortcuts
The editor SHALL provide these shortcuts:

| Action | Shortcut |
|---|---|
| Previous / next level (level editor); previous / next map (map editor) | PageUp / PageDown |
| New level (copy) | Ctrl+N |
| Delete level (new levels only) | Ctrl+Delete |
| World map editor / Back to the levels | F2 |
| Picture / Points tab | Tab |
| Restore | Ctrl+Shift+R |

#### Scenario: New level from the keyboard
- **WHEN** the level editor is shown and the user presses Ctrl+N
- **THEN** a new level is made as with the New level (copy) button

#### Scenario: Next map
- **WHEN** the world map editor is shown and the user presses PageDown
- **THEN** the next map is shown as with the > button

#### Scenario: Restore from the keyboard
- **WHEN** the Pumpkin Zone is shown and the user presses Ctrl+Shift+R
- **THEN** the Pumpkin Zone is restored as with the Restore button

### Requirement: Tool, palette and clipboard shortcuts
The editor SHALL provide these shortcuts in Blocks mode and the Picture tab:

| Action | Shortcut |
|---|---|
| Select tool | E |
| Palette zoom out / in | Ctrl+Minus / Ctrl+Plus |
| Copy / cut / paste | Ctrl+C / Ctrl+X / Ctrl+V |
| Fill the selection with the background | Delete |
| Clear the selection or leave Paste | Esc |

#### Scenario: Select tool key
- **WHEN** Blocks mode is shown and the user presses E
- **THEN** the Select tool is chosen

### Requirement: Points tab shortcuts
The editor SHALL provide these shortcuts in the Points tab:

| Action | Shortcut |
|---|---|
| Previous / next point | , / . |
| Level the point opens: previous / next | J / K |
| Door or Exit to: choose on the map | U |
| Make it the exit (first / second exit of the zone) | T / Shift+T |
| Make it the entry (new zones) | H |
| Delete the path right, left, up, down | Shift+Right, Shift+Left, Shift+Up, Shift+Down |
| New path / New point / Delete point | P / N / Delete |

#### Scenario: Next point
- **WHEN** point $04 is chosen and the user presses .
- **THEN** point $05 is chosen, as with the Point + button

### Requirement: No shortcut does two things
Within one view and tab, each key combination SHALL trigger at most one action.

#### Scenario: Delete in Blocks and Objects mode
- **WHEN** a selection exists in Blocks mode and the user presses Delete
- **THEN** the selection is filled with the background and no object is deleted

### Requirement: Shortcuts shown on the buttons
Every button with a shortcut SHALL show it: in the world map editor in its label (for
example "Undo (Ctrl+Z)"), and in the level editor's side panel in a tooltip while the mouse
is on the button, so its buttons keep short names. For the small - and + buttons of the world
map editor the keys SHALL be shown beside the row's label. A label too long for its button
SHALL be drawn smaller so it fits inside it.

#### Scenario: Undo label
- **WHEN** the world map editor is shown, or the mouse rests on the level editor's Undo button
- **THEN** the world map editor's Undo button reads "Undo (Ctrl+Z)" (Spanish: "Deshacer (Ctrl+Z)"), and the level editor shows that text in a tooltip over a button that reads "Undo"

#### Scenario: Point row
- **WHEN** the Points tab is shown
- **THEN** the Point row shows ", ." next to its - and + buttons

### Requirement: Level and project file shortcuts
The editor SHALL run the level and project file actions from the keyboard, and their File
menu entries SHALL show the keys. Level file keys SHALL work in the level editor, project file
keys in both editors.

| Action | Keys |
|---|---|
| Export level | Ctrl+E |
| Import level as a new level | Ctrl+Shift+N |
| Replace this new level from a file | Ctrl+I |
| Export project | Ctrl+Shift+E |
| Import project | Ctrl+Shift+I |

#### Scenario: Export a level from the keyboard
- **WHEN** the level editor is shown and the user presses Ctrl+E
- **THEN** the Save dialog for a level file opens

#### Scenario: Project from the world map editor
- **WHEN** the world map editor is shown and the user presses Ctrl+Shift+E
- **THEN** the Save dialog for a project file opens

### Requirement: New zone and zone file shortcuts
The world map editor SHALL run the new zone and zone file actions from the keyboard while a
zone map is shown, with the same keys as their level counterparts, and their buttons and File
menu entries SHALL show the keys.

| Action | Keys |
|---|---|
| New zone (copy of the zone shown) | Ctrl+N |
| Delete the new zone shown | Ctrl+Delete |
| Export zone | Ctrl+E |
| Import zone as a new zone | Ctrl+Shift+N |
| Replace this new zone from a file | Ctrl+I |

#### Scenario: New zone from the keyboard
- **WHEN** the Macro Zone is shown in the world map editor and the user presses Ctrl+N
- **THEN** a new zone made from the Macro Zone is added and shown

#### Scenario: On the world map
- **WHEN** the world map is shown and the user presses Ctrl+E
- **THEN** nothing is exported and the status bar says zones are exported from a zone map

### Requirement: Level start and goal shortcuts
The level editor SHALL provide these shortcuts in both its modes (Blocks and Objects), shown
on their buttons like the others; pressing the key of the mode already chosen, or Esc,
SHALL leave it and go back to the tool chosen before:

| Action | Shortcut |
|---|---|
| Place start | I |
| Place finish | F |
| Place secret exit | Shift+F |

#### Scenario: Place the start from the keyboard
- **WHEN** level 05 is shown in Objects mode and the user presses I, then clicks on the level
- **THEN** Mario's start is placed where the user clicked

#### Scenario: Leave placing
- **WHEN** "Place finish" was chosen with F and the user presses Esc
- **THEN** the tool chosen before is back and nothing was placed

### Requirement: An ordered level panel
The level editor's side panel SHALL keep a fixed head (the level picker, whose grid of all
levels folds away; a toolbar with undo, redo, save, play and play here; Blocks / Objects with
the mode's tools; two tabs) and give the rest of its height to the chosen tab: the palette or
the level's inspector.

#### Scenario: Room for the palette
- **WHEN** the Palette tab is chosen in a window 820 pixels tall
- **THEN** the palette fills the panel below its head

### Requirement: The level inspector
The inspector tab SHALL group the level's controls in titled sections (level, goals,
settings, warnings) whose rows put their label in one column and their controls in the next,
SHALL scroll with the mouse wheel when it does not fit, and its tab SHALL show the shown
level's warnings count.

#### Scenario: A level with warnings
- **WHEN** the shown level has two warnings
- **THEN** the Level tab reads "Level ! 2" and its Warnings section lists both

### Requirement: New blank level shortcut
In the level editor, Ctrl+B SHALL open the New blank level chooser, as the File menu entry
does. Ctrl+N SHALL keep making a copy of the level shown.

#### Scenario: Blank level from the keyboard
- **WHEN** the level editor is shown and the user presses Ctrl+B
- **THEN** the New blank level chooser opens

### Requirement: Object catalogue shortcut
In the level editor, F3 SHALL open the object catalogue, as its button in the Objects palette
does, and Esc SHALL close it.

#### Scenario: Catalogue from the keyboard
- **WHEN** the level editor is shown and the user presses F3
- **THEN** the object catalogue opens

### Requirement: Cmd on macOS
On macOS, every shortcut written with Ctrl SHALL be made with Cmd instead (Cmd+S saves, Cmd+Z undoes, Cmd+E exports), Ctrl together with the mouse wheel SHALL be Cmd with the wheel, and every place the editor shows a shortcut (menus, buttons, tooltips, messages) SHALL write Cmd where the other systems write Ctrl. Ctrl alone SHALL not make those shortcuts on macOS. The other systems keep Ctrl.

#### Scenario: Saving on a Mac
- **WHEN** the user presses Cmd+S in the level editor on macOS
- **THEN** the project is saved, as Ctrl+S does on Windows

#### Scenario: Ctrl on a Mac
- **WHEN** the user presses Ctrl+S in the level editor on macOS
- **THEN** nothing is saved

#### Scenario: Labels on a Mac
- **WHEN** the user points at the Undo button on macOS
- **THEN** its tooltip shows Cmd+Z

### Requirement: Keys the browser keeps
In the web editor, an action whose shortcut a browser keeps for itself (Ctrl+N and Ctrl+Shift+N, Cmd+N and Cmd+Shift+N on a Mac) SHALL use Alt in place of Ctrl or Cmd, and its buttons, tooltips and menu entries SHALL show that key. Every other shortcut SHALL keep its key and SHALL NOT also do the browser's own action for it.

#### Scenario: A new level in the browser
- **WHEN** Alt+N is pressed in the level editor of the web editor
- **THEN** a new level is made as Ctrl+N does on the desktop, and the New level button shows Alt+N

#### Scenario: Saving in the browser
- **WHEN** Ctrl+S is pressed in the web editor
- **THEN** the project is saved and the browser does not offer to save the page
