# editor/projects Specification

## Purpose
The editor works inside a project: a named folder that keeps its own edited cartridge, over
the one original cartridge both programs check.

## Requirements

### Requirement: The base cartridge
The editor SHALL need the path of the original cartridge file, whatever its name, and SHALL
accept only the cartridge the tools support (checked by the whole file's SHA-1). The path SHALL
be shared with the game: chosen in either program, the other uses it. At every start the
editor SHALL check it again and, while it is missing or not the right cartridge, SHALL ask for
it and open nothing else.

#### Scenario: First start
- **WHEN** the editor starts and no base cartridge was ever chosen
- **THEN** it asks for the cartridge file and opens no project until a valid one is chosen

#### Scenario: Wrong file
- **WHEN** a Game Boy file that is not the supported cartridge is chosen
- **THEN** it is refused, the editor says it is not the supported cartridge, and still asks

#### Scenario: Chosen in the game first
- **WHEN** the base cartridge was chosen in the game and the editor starts
- **THEN** the editor checks that path and goes on without asking

### Requirement: Working in a project
The editor SHALL open only a project: at start it SHALL offer to create a new project (by
name) or open one of `editor/projects`, the last one used chosen first. A new project SHALL
start as the original game.

#### Scenario: A new project
- **WHEN** the user creates the project "Islands"
- **THEN** the editor shows the game unchanged, in the project "Islands"

### Requirement: Where a project is kept
A project SHALL be the folder `editor/projects/<name>/` with its project file `<name>.nmbproject`
and its edited cartridge `<original file name> - <project name>.gb`, made from the file over the
base cartridge. Saving SHALL write both with every change (levels, new levels, world map, zone
maps, new zones, points, paths, doors), and opening the project SHALL go on where it was left,
making the cartridge again from the file when it is missing or older than the file.

#### Scenario: The project's cartridge
- **WHEN** the project "Islands" is created over "Super Mario Land 2 - 6 Golden Coins (USA, Europe).gb" and saved
- **THEN** `editor/projects/Islands/Super Mario Land 2 - 6 Golden Coins (USA, Europe) - Islands.gb` holds its changes

#### Scenario: Only the project file
- **WHEN** a project folder holds only its project file and the project is opened
- **THEN** its cartridge is made from the file over the base cartridge and the editor shows the project

#### Scenario: Going on later
- **WHEN** a level of "Islands" is edited, saved, the editor closed and "Islands" opened again
- **THEN** the level is as it was saved

### Requirement: Switching, renaming, duplicating and deleting
The project screen (at start, and from File, Projects...) SHALL open a project, make a new one,
rename one (its folder, its cartridge and its game saves follow), duplicate one under a new name,
and delete one after asking; it SHALL show when each was last saved. Project names SHALL be
unique, printable ASCII and usable as folder names.

#### Scenario: Rename
- **WHEN** "Islands" is renamed "Sky Islands"
- **THEN** its folder, its cartridge name and its saves folder use the new name, and the game lists it so

#### Scenario: Name taken
- **WHEN** a project is created or renamed with a name another project has
- **THEN** it is refused and the editor says so

### Requirement: Unsaved changes
Leaving a project with unsaved changes (switching, or closing the editor) SHALL ask whether
to save, discard or cancel.

#### Scenario: Closing with changes
- **WHEN** a level was edited and the editor window is closed
- **THEN** the editor asks; Cancel keeps it open with the changes

### Requirement: The supported cartridge's name
The supported cartridge SHALL be named in the editor's texts with its revision, as Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), both when the editor asks for it and when it refuses another file; the refusal SHALL say that other revisions of the game are refused too.

#### Scenario: Asking for the cartridge
- **WHEN** the editor starts and no base cartridge was ever chosen
- **THEN** it asks for the Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) cartridge file

#### Scenario: Another revision
- **WHEN** a file of revision 0 of Super Mario Land 2 - 6 Golden Coins (USA, Europe) is chosen
- **THEN** it is refused, and the message names Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) and says other revisions are refused

### Requirement: Saved objects in the game's order
Saving SHALL store every level's objects sorted left to right by their x, the order the game walks its lists in, whatever order they were placed in; objects at the same x SHALL keep their order. A level whose objects did not change SHALL keep the same bytes.

#### Scenario: An object placed to the left of others
- **WHEN** a mushroom is placed in level 00 at x 272, to the left of an object at x 472 in the same 256-pixel page, the project is saved and the level is played
- **THEN** the mushroom comes to life when Mario reaches it

#### Scenario: An unchanged level
- **WHEN** the project is saved with no change to level 01's objects
- **THEN** level 01's object list has the same bytes as in the original cartridge
