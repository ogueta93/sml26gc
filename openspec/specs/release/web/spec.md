# release/web Specification

## Purpose

The programs running in a web browser: the web package and how it is served, which browsers it
supports, where the cartridge and the data are kept there, and what the browser cannot do.

## Requirements

### Requirement: The web package is static files
The web package SHALL be a folder of static files (a page for the game, the WebAssembly program, its loader, fonts, manifest, icons and the notices) that plays when served by any web server over HTTP(S), with no program running on the server. Opening the page straight from disk (`file://`) SHALL show a message saying it must be served, and how.

#### Scenario: Served from a folder
- **WHEN** the unpacked web package is served with `python3 -m http.server` and `http://localhost:8000/` is opened in a browser
- **THEN** the game's start screen appears and asks for the base cartridge

#### Scenario: Opened from disk
- **WHEN** the package's `index.html` is opened by double-clicking it
- **THEN** the page says it must be served by a web server and names the simplest way to do it

### Requirement: Supported browsers
The web game SHALL play in the current versions of Chrome, Edge, Firefox and Safari on desktop, Safari on iOS and iPadOS 16.4 or later, and Chrome on Android 10 or later. On a browser without WebAssembly or WebGL, the page SHALL say which browsers it needs instead of showing an empty page.

#### Scenario: A phone
- **WHEN** the served page is opened in Chrome on an Android phone
- **THEN** the start screen appears and the game can be played by touch

#### Scenario: An old browser
- **WHEN** the page is opened in a browser without WebAssembly
- **THEN** the page says the game needs a current Chrome, Edge, Firefox or Safari

### Requirement: The cartridge stays in the browser
In the browser, the base cartridge SHALL be read from a file the person chooses or drops on the page, checked by its SHA-1 as on the desktop, and kept only in that browser's storage for that site. It SHALL never be sent over the network. The web package and its server SHALL hold no cartridge and no data taken from one.

#### Scenario: Choosing the cartridge
- **WHEN** the person chooses their Rev 1 cartridge file on the start screen
- **THEN** it is accepted, the page makes no network request carrying it, and the next visit does not ask for it again

#### Scenario: Another revision
- **WHEN** a Rev 0 cartridge file is dropped on the page
- **THEN** it is refused with the same message as on the desktop, and nothing is kept

### Requirement: Data kept in the browser
In the browser, every file the desktop programs keep in their data folder (base cartridge, settings, battery saves, new zone progress, and later the editor's) SHALL be kept in the browser's storage for that site, survive closing the page and the browser, and SHALL be asked to be kept persistently where the browser allows. The page SHALL warn that clearing the site's data removes them.

#### Scenario: Coming back
- **WHEN** a game is saved in the game's save slot, the browser is closed and the page opened again
- **THEN** the save is there and the game goes on from it

#### Scenario: Site data cleared
- **WHEN** the person clears the site's data in the browser and opens the page
- **THEN** the game asks for the base cartridge again, as on a first visit

### Requirement: The game saves without quitting
In the browser the game SHALL write its battery save and new zone progress to the browser's storage at least every five seconds while playing, as on the desktop, since a page can be closed at any moment without the game being told.

#### Scenario: Switching apps on a phone
- **WHEN** the game is saved in its save slot, and ten seconds later the person switches to another app and the browser discards the page
- **THEN** opening the page again finds the save

### Requirement: Several tabs
Two pages of the same site open at once SHALL see each other's saved files: a file written in one SHALL be read by the other from its next read on, and each file SHALL hold the last version written.

#### Scenario: The same site twice
- **WHEN** the language is changed on the start screen in one tab and the start screen is shown in another tab
- **THEN** the other tab uses the new language from its next start screen on

### Requirement: What the browser cannot do
In the browser the game SHALL offer nothing it cannot do there: no Quit (the page is closed instead) and no command-line modes. Opening the editor SHALL be offered where the editor's page runs (see `editor/browser`) and open it in a new tab. Settings that only exist on the desktop SHALL be hidden there.

#### Scenario: The pause menu in the browser
- **WHEN** the pause menu is opened in the web game
- **THEN** it has no Quit row

#### Scenario: Edit from the start screen on a computer
- **WHEN** a project is chosen on the web game's start screen on a computer and Edit is chosen
- **THEN** the editor's page opens in a new tab with that project, and the game's tab stays on its start screen

### Requirement: Arguments from the address
The web game SHALL take, from its page's address, the arguments the desktop game takes from its command line that make sense in the browser (`project`, `level`, `start`, `point`, `test`, `lives`, `coins`, `power`, `lang`, `cheats`), with the same meaning.

#### Scenario: A level from the address
- **WHEN** the page is opened as `index.html?project=Islands&level=3`
- **THEN** the game starts project Islands in level 3, as `sml26gc --project Islands --level 3` does

### Requirement: Fonts on demand
The web game SHALL download the font of Japanese, Chinese or Korean only when that language is used, and SHALL show the texts once it has arrived; the other languages SHALL need no extra download.

#### Scenario: Choosing Japanese
- **WHEN** Japanese is chosen on the start screen of the web game
- **THEN** the Japanese font is downloaded once and the start screen shows Japanese texts, and the next visit does not download it again unless the browser removed it

### Requirement: A fullscreen button
Where the browser lets a page go fullscreen, the web game SHALL show a fullscreen button on the start screen and beside the on-screen menu button. Touching it SHALL hide the browser's bars and the system's navigation bar, and the same button, with an exit icon while fullscreen, SHALL leave it. Leaving fullscreen through the browser or the system SHALL show the enter icon again. The game SHALL go on without stopping either way.

#### Scenario: Fullscreen on an Android phone
- **WHEN** the fullscreen button is touched during a level in Chrome on Android
- **THEN** the browser's address bar and the system's navigation bar disappear, the picture and controls take the whole screen, and the button shows the exit icon

#### Scenario: Leaving with the system's back gesture
- **WHEN** the person leaves fullscreen with Android's back gesture
- **THEN** the page goes back to its normal view, the game goes on, and the button shows the enter icon

#### Scenario: An iPhone
- **WHEN** the web game is opened in Safari on an iPhone, which does not let a page go fullscreen
- **THEN** no fullscreen button is shown, and the start screen says how to add the game to the home screen to play without the browser's bars

#### Scenario: A desktop browser
- **WHEN** the fullscreen button or F11 is used in a desktop browser
- **THEN** the page goes fullscreen, and Esc or the button leaves it

### Requirement: Installed on the home screen
The web game SHALL carry a web app manifest and icons, so a phone or computer can add it to its home screen or app list and open it fullscreen, in portrait or landscape, with the game's name and icon.

#### Scenario: Added to an iPhone's home screen
- **WHEN** the page is added to the home screen in Safari on iOS and opened from there
- **THEN** it opens without the browser's bars, with the game's icon and name
