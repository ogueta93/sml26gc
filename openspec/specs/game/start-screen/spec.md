# game/start-screen Specification

## Purpose
The game's first screen: the base cartridge, checked, and what to play, each with its own
saves.

## Requirements

### Requirement: The base cartridge
The game SHALL need the path of the original cartridge file, whatever its name, accept only
the supported cartridge (checked by the whole file's SHA-1), share the path with the editor,
and check it at every start; while it is missing or wrong the game SHALL say so and ask for it,
and start nothing.

#### Scenario: Wrong cartridge
- **WHEN** the game starts and the base cartridge path points to another file
- **THEN** the start screen says it is not the supported cartridge and asks for it, and no game can start

### Requirement: What to play
With a valid base cartridge, the start screen SHALL offer VANILLA (the original cartridge) and
every project folder of `editor/projects` that has its project file, the last one played chosen
first. A project's cartridge SHALL NOT be checked against the base cartridge; when it is missing
or older than the project file, it SHALL be made from the file over the base cartridge before
the game starts, and a project file of another cartridge or damaged SHALL be refused with why.

#### Scenario: Two projects
- **WHEN** `editor/projects` has "Islands" with its project file and "Draft" with neither file nor cartridge
- **THEN** the list offers VANILLA and Islands

#### Scenario: A project file alone
- **WHEN** "Islands" holds only its project file and is chosen
- **THEN** its cartridge is made over the base cartridge and the game starts

### Requirement: Choosing is not playing
The start screen SHALL show one choice at a time in a selector (its kind, its name, and for a
project how many new levels it has and when it was saved), stepped with arrows, the arrow keys
or the wheel; choosing SHALL only select it, and Play (Enter) SHALL start the game.

#### Scenario: Looking through the projects
- **WHEN** the right arrow is pressed twice on the start screen
- **THEN** the third choice is shown and selected, and no game starts until Play

### Requirement: Editing from the start screen
The start screen SHALL open the editor on the chosen project ("edit in the editor", not for
VANILLA) or on a new project (the editor's project screen typing the new name), and SHALL show
projects made or renamed meanwhile.

#### Scenario: Edit Islands
- **WHEN** Islands is chosen and E is pressed
- **THEN** the editor opens with Islands, and the game's start screen stays

#### Scenario: A new project
- **WHEN** N is pressed, and the project "Sky" is made in the editor
- **THEN** the editor's project screen asks for the new name, and Sky shows in the game's list

### Requirement: Saves per project
Each choice SHALL keep its battery save and its new zone progress apart: `saves/vanilla/` for
the original and `saves/<project>/` for a project.

#### Scenario: Separate progress
- **WHEN** a level is cleared playing "Islands" and VANILLA is chosen afterwards
- **THEN** the original's progress does not have that level cleared

### Requirement: The supported cartridge's name
The supported cartridge SHALL be named in the game's texts with its revision, as Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), both when the game asks for it and when it refuses another file; the refusal SHALL say that other revisions of the game are refused too.

#### Scenario: Asking for the cartridge
- **WHEN** the game starts and no base cartridge was ever chosen
- **THEN** it asks for the Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) cartridge file

#### Scenario: Another revision
- **WHEN** a file of revision 0 of Super Mario Land 2 - 6 Golden Coins (USA, Europe) is chosen
- **THEN** it is refused, and the message names Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) and says other revisions are refused

### Requirement: The cartridge in the browser
In the web game, the start screen's Browse SHALL open the browser's file chooser, and a cartridge file dropped anywhere on the page SHALL be taken as if chosen. The file's contents, not its path, SHALL become the base cartridge, checked as on the desktop.

#### Scenario: Browse on a phone
- **WHEN** Browse is tapped on the start screen in Safari on an iPhone and the cartridge file is chosen from Files
- **THEN** it is checked, accepted, and the start screen offers VANILLA

#### Scenario: Dropping the file
- **WHEN** the cartridge file is dragged from a folder and dropped on the page in a desktop browser
- **THEN** it is checked and accepted as with Browse

### Requirement: The start screen on a phone
The start screen SHALL fit a screen of 360 by 640 points in portrait and in landscape, with every text readable and every button large enough to tap, and SHALL be used by tapping as by the mouse.

#### Scenario: A small phone upright
- **WHEN** the web game is opened on a phone 360 points wide held in portrait
- **THEN** the whole start screen is visible without zooming, and tapping Play starts the game chosen
