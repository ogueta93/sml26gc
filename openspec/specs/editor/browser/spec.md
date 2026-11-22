# editor/browser Specification

## Purpose

The level editor running in a web browser on a computer: its page, the devices it runs on, how
files go in and out of the browser, test games in the game's page, and leaving with unsaved
changes.

## Requirements

### Requirement: The editor's page
The web package SHALL have the editor's page beside the game's. It SHALL do everything the desktop editor does with projects, levels, zones and the world map, over the same base cartridge, projects and settings that the game's page uses in that browser.

#### Scenario: A project made in the browser
- **WHEN** a project "Sky" is made and saved in the editor's page, and the game's start screen is shown in another tab
- **THEN** the game's start screen offers Sky, and playing it plays the edited levels

#### Scenario: The base cartridge chosen once
- **WHEN** the base cartridge was chosen in the game's page and the editor's page is opened
- **THEN** the editor does not ask for it

### Requirement: A computer with a mouse and keyboard
The editor's page SHALL run on a device whose main pointer is a mouse or trackpad (a fine pointer), or once a key has been pressed there. On a device with only a touch screen, it SHALL say that the editor needs a computer with a mouse and keyboard and link to the game's page, and the game's start screen SHALL NOT offer Edit or New project there.

#### Scenario: Opening the editor on a phone
- **WHEN** the editor's page is opened in Chrome on an Android phone
- **THEN** it says the editor needs a computer with a mouse and keyboard and offers a link to play the game

#### Scenario: A laptop with a touch screen
- **WHEN** the editor's page is opened on a laptop with a touch screen and a trackpad
- **THEN** the editor opens

### Requirement: Files out are downloads
In the browser, exporting a level, zone, world map, project file or the project's cartridge SHALL download it with the name the desktop editor would suggest, and the editor's status bar SHALL say it was exported.

#### Scenario: Exporting a level
- **WHEN** Export level is chosen for level "Cave" in the web editor
- **THEN** the browser downloads `Cave.nmblevel`, which the desktop editor can import

#### Scenario: Exporting the cartridge
- **WHEN** Export cartridge is chosen for project "Sky" in the web editor
- **THEN** the browser downloads `Sky.gb`; the game's page does not need it, as it plays Sky's cartridge kept in the browser

### Requirement: The project between the browser and the desktop
Export project in the web editor SHALL download the project's file, which holds everything edited in the project; importing it on the project screen, in the browser or in the desktop editor, SHALL make the same project, its cartridge made again from the base cartridge. A project file exported by the desktop editor SHALL import in the browser the same way.

#### Scenario: A backup brought back
- **WHEN** project "Sky" is exported in the web editor, the browser's data for the site is cleared, the base cartridge is chosen again and `Sky.nmbproject` is imported on the project screen
- **THEN** Sky is back with every edited level, zone and the world map, and plays as before

#### Scenario: From the browser to the desktop
- **WHEN** `Sky.nmbproject` exported in the web editor is imported in the desktop editor
- **THEN** the desktop editor has Sky as it was in the browser

### Requirement: Files in are chosen or dropped
In the browser, importing or replacing a level, zone, world map or project, or choosing the base cartridge, SHALL open the browser's file chooser for that kind of file. A file dropped on the editor's page SHALL be taken by the action its kind names, where that view offers it. The action SHALL go on when the file arrives.

#### Scenario: Importing a level
- **WHEN** Import level is chosen in the web editor and a `.nmblevel` file is chosen
- **THEN** the level is added to the project, as on the desktop

#### Scenario: Dropping a project file
- **WHEN** a `.nmbproject` file is dropped on the project screen of the web editor
- **THEN** it is imported as a project, as with Import project

#### Scenario: Cancelling
- **WHEN** the file chooser is closed without choosing
- **THEN** nothing changes and the editor goes on

### Requirement: Test games in the game's page
In the browser, Play, Play here and Play from a point SHALL open the game's page in a new tab on the project's cartridge with the same test game as on the desktop: the same arguments, given in its address. While that tab is open, a new test game SHALL replace it, so at most one test game is open.

#### Scenario: Play here
- **WHEN** Play here is chosen at a place in level 3 in the web editor
- **THEN** a tab opens with the game in level 3, Mario at that place, with the test game's lives, coins and power, and no saves

#### Scenario: Playing again
- **WHEN** Play is chosen again while the test game's tab is open
- **THEN** that tab shows the new test game and no other tab opens

### Requirement: Unsaved changes when the page closes
In the browser, closing or reloading the editor's page, or leaving it, while the project has unsaved changes SHALL make the browser ask whether to leave; with no unsaved changes it SHALL close without asking.

#### Scenario: Closing with changes
- **WHEN** a block is painted and the editor's tab is closed without saving
- **THEN** the browser asks whether to leave the page
