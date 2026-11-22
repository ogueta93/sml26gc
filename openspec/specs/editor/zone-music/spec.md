# editor/zone-music Specification

## Purpose

The music a map plays, chosen in the world map editor like a level's: for the world map, the
game's eight zone maps and each new zone, listened to in the editor and played by the game.

## Requirements

### Requirement: Choosing a map's music
The world map editor SHALL let the user choose the music of the map shown (the world map, each of the game's zone maps and each new zone) among the musics the game plays on its maps and in its levels, each named by a map or level that plays it. The chooser SHALL show the map's music as it is now.

#### Scenario: The Tree Zone's music
- **WHEN** the Tree Zone is shown in the world map editor
- **THEN** its music chooser shows the Tree Zone's own music

#### Scenario: A castle music for the Pumpkin Zone
- **WHEN** the Pumpkin Zone is shown and the music of a castle level is chosen
- **THEN** the chooser shows that music for the Pumpkin Zone and the other maps keep theirs

### Requirement: Playing the chosen music
After a save, the game SHALL play each map's chosen music whenever that map is shown: on entering it from a door, on coming back to it from a level, and on starting a game on it.

#### Scenario: Back from a level
- **WHEN** the Tree Zone is given the Space Zone's map music, the copy is saved, and a Tree Zone level is played and left
- **THEN** the Tree Zone map plays the Space Zone's map music

#### Scenario: The world map
- **WHEN** the world map is given the music of level 01, the copy is saved and a game is started
- **THEN** the world map plays level 01's music

### Requirement: A new zone's own music
A new zone SHALL have its own music, at first its base zone's (or the music of the zone it was copied from), and choosing it SHALL NOT change its base zone's.

#### Scenario: Two zones made from the Tree Zone
- **WHEN** a new zone made from the Tree Zone is given the Mario Zone's map music and the copy is saved
- **THEN** the game plays the Mario Zone's map music on the new zone and the Tree Zone's own music on the Tree Zone

#### Scenario: A copied zone
- **WHEN** a new zone is made as a copy of a new zone whose music was changed
- **THEN** the copy has the same music

### Requirement: Listening to a map's music
The world panel SHALL have a button to listen to the chosen music in the editor, played by the game's own sound code at the editor's listening volume, until it is pressed again, another map is shown or the copy is saved.

#### Scenario: Listen in the world map editor
- **WHEN** the listen button is pressed with the Macro Zone's music chosen
- **THEN** the editor plays the Macro Zone's music until the button is pressed again

### Requirement: A map's music kept like its other changes
A map's music SHALL be undone and redone, put back as the game has it by Restore (a new zone: its base zone's), counted as a change that needs saving, and kept in the edited copy and in project files.

#### Scenario: Undo
- **WHEN** the Macro Zone's music is changed and Ctrl+Z is pressed
- **THEN** the Macro Zone has its own music again

#### Scenario: Restore
- **WHEN** the Turtle Zone's music was changed and Restore is used on the Turtle Zone
- **THEN** the Turtle Zone has its own music again

#### Scenario: A project file
- **WHEN** a project whose world map has another music is exported and imported into another project
- **THEN** the world map of the second project has that music

### Requirement: Older copies and files
Edited copies, project files and zone or world map files written before maps had their own music SHALL still be read, every map with the game's music and every new zone with its base zone's.

#### Scenario: An older project file
- **WHEN** a project file written before this change is imported
- **THEN** every map has the game's music and the rest of the project is the file's
