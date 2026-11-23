# game/touch-controls Specification

## Purpose

Playing the game on a touch screen, a phone above all: the on-screen controls, their layouts in
portrait and landscape, how the buttons respond to fingers, when they show, and their settings.

## Requirements

### Requirement: On-screen controls
On a touch screen the game SHALL show on-screen controls for every Game Boy button (a D-pad, A, B, Start and Select) and a menu button that opens the pause menu, as the menu binding does. They SHALL play together with the keyboard and a controller.

#### Scenario: Jumping by touch
- **WHEN** the game is played on a phone and the on-screen A is touched in a level
- **THEN** Mario jumps, as with the A binding of the keyboard

#### Scenario: The menu by touch
- **WHEN** the on-screen menu button is touched on the world map
- **THEN** the pause menu opens, and the on-screen D-pad, A and B move through it, choose and go back

### Requirement: Portrait layout
With the screen taller than wide, the Game Boy screen SHALL be at the top, as wide as the screen allows, and the controls SHALL be below it, as on a handheld: the D-pad on the left, B and A on the right (A higher and further right), Select and Start between them, and the menu button at the top right of the controls' area. No control SHALL cover the Game Boy screen.

#### Scenario: A phone held upright
- **WHEN** the game is played on a phone held in portrait
- **THEN** the Game Boy screen fills the width at the top and the controls are below it, covering nothing of the picture

### Requirement: Landscape layout
With the screen wider than tall, the Game Boy screen SHALL be as large as the height allows, in the middle, with the D-pad on the left side and B and A on the right side, Select and Start at the bottom, and the menu button at the top right. Where the controls overlap the picture they SHALL be translucent.

#### Scenario: Turning the phone
- **WHEN** the phone is turned from portrait to landscape during a level
- **THEN** the game goes on without stopping and the controls move to the sides of the picture

### Requirement: The D-pad follows the finger
The D-pad SHALL read a direction from where the finger is relative to its centre, in eight directions (diagonals press both directions), with a small dead zone in the middle, and SHALL follow the finger as it slides without being lifted, also when the finger began on the D-pad and slides beyond its drawn edge.

#### Scenario: Sliding from right to down-right
- **WHEN** a finger holds the D-pad's right and slides down to its lower right without lifting
- **THEN** Mario goes from walking right to holding right and down

### Requirement: Fingers slide between buttons
A, B, Start and Select SHALL be held while a finger is on them, a finger sliding from one to another SHALL release the first and press the second, a finger between A and B SHALL press both, and each finger SHALL count on its own, so the D-pad and A or B can be held at the same time. Each button SHALL respond to touches somewhat beyond its drawn edge.

#### Scenario: Running and jumping
- **WHEN** one finger holds the D-pad's right and another holds B, then touches between B and A
- **THEN** Mario runs right and jumps while running

### Requirement: When the controls show
Set to Auto, the on-screen controls SHALL show when the screen is touched and hide when a key or a controller button is pressed; Always SHALL show them on any screen; Never SHALL hide them. The setting SHALL be in the pause menu's Controls with Auto by default, kept in `settings.cfg`.

#### Scenario: A controller on a tablet
- **WHEN** a controller's button is pressed on a tablet showing the on-screen controls with Auto
- **THEN** the controls hide and the Game Boy screen takes the whole space; touching the screen shows them again

### Requirement: Their size and vibration
The pause menu's Controls SHALL set the on-screen controls' size (Small, Medium, Large; Medium by default) and whether pressing one vibrates (On by default, where the device can vibrate), kept in `settings.cfg`.

#### Scenario: Larger controls
- **WHEN** the size is set to Large
- **THEN** the controls are drawn and respond larger at once, and still cover nothing of the picture in portrait

#### Scenario: Vibration on Android
- **WHEN** the on-screen A is pressed in Chrome on Android with vibration On
- **THEN** the phone gives a short vibration; with vibration Off it does not

### Requirement: Controls shows the inputs there are
The pause menu's Controls SHALL show the bindings only of the inputs the device has: the keyboard's where there is one (always on a computer; in a browser, where a mouse or trackpad is the main pointer, or once a key has been pressed), the controller's while one is connected, the choice between them only when both are there, and the on-screen controls' rows only on a touch screen (or once the screen has been touched). An input that appears while the menu is open SHALL appear in it at once.

#### Scenario: A phone without keyboard or controller
- **WHEN** Controls is opened on a phone with nothing connected
- **THEN** it shows only the on-screen controls' rows (when they show, their size, vibration), and no key or controller bindings

#### Scenario: A controller connected to the phone
- **WHEN** a controller is connected to that phone and Controls is opened
- **THEN** the controller's bindings and Restore defaults show above the on-screen controls' rows

### Requirement: The port's buttons
The on-screen controls SHALL also offer the port's actions that work at that moment, each as a binding of the same name does: fast forward (held) always; rewind (held), save state and load state with the cheats mode on; restart level in a level, with the cheats mode on or in a test game. They SHALL be smaller than the menu button and apart from the Game Boy's buttons, in a fixed order (rewind, fast forward, save, load, restart): in portrait in a row left of the fullscreen and menu buttons, covering nothing of the picture; in landscape above the D-pad. They SHALL not show while the pause menu is open.

#### Scenario: Saving and loading by touch
- **WHEN** the cheats mode is on and the on-screen save state button is touched in a level, and later the load state button
- **THEN** the game is saved, a note says so, and it comes back to that moment

#### Scenario: Without the cheats mode
- **WHEN** a game without the cheats mode is played by touch
- **THEN** only the fast forward button shows among the port's buttons

### Requirement: The picture stretched
Display's Scale SHALL offer sharp (whole pixels), fitted (as large as fits at the picture's shape) and stretched (all the room, out of shape); a settings file of an earlier version SHALL keep its choice. Stretched with the on-screen controls SHALL fill the room they leave: in portrait down to the row of small buttons, moved down next to the D-pad and the buttons; in landscape between the controls of either side. No control SHALL cover the picture in portrait.

#### Scenario: A phone across
- **WHEN** Scale is set to stretched on a phone held in landscape with the controls shown
- **THEN** the picture is wider than its shape, from the D-pad's side to the buttons' side, and no control lies over it

#### Scenario: A phone upright
- **WHEN** Scale is set to stretched on a phone held in portrait
- **THEN** the picture fills the width and reaches down to the small buttons, now just above the D-pad

### Requirement: Prompts name what plays
The pause menu's footers, the start-up tag and the save state notes SHALL name the buttons of the input that plays: a connected controller's (it comes first, even with a keyboard there), else the on-screen controls' while they show (A and B; the notes leave the keys out), else the keyboard's keys. With a controller connected, Controls SHALL open on the controller's bindings.

#### Scenario: A controller on a computer
- **WHEN** a controller is connected to a computer and the pause menu is opened
- **THEN** its footer names the controller's buttons bound to A and B instead of Enter and Esc, and Controls opens on the controller's bindings

#### Scenario: Playing by touch
- **WHEN** the pause menu is opened from the on-screen menu button on a phone
- **THEN** its footer names A and B

### Requirement: Turned off by mistake
When the on-screen controls are set to Never on a touch screen where no key has been pressed and no controller is connected, a small button SHALL be shown in a corner; touching it SHALL set them back to Auto and show them, so the game can always be played and the setting changed again.

#### Scenario: Never chosen on a phone
- **WHEN** the on-screen controls are set to Never in Controls on a phone with nothing connected
- **THEN** a small button shows in a corner, and touching it brings the controls back, with the setting at Auto
