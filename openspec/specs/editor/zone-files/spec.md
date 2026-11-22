# editor/zone-files Specification

## Purpose
Zone files keep a zone outside the cartridge: exported from the world map editor, they come
back as a new zone or replace a new zone whole, with the base zone they were made with.

## Requirements

### Requirement: Exporting a zone
The world map editor SHALL export the zone shown (one of the game's or a new one), as it is
being edited, to a file the user names: its base zone, picture, points, paths, the level
each point opens, its exits and its music. A zone imported from the file, as a new zone or
replacing one, SHALL have the file's music.

#### Scenario: Export the Pumpkin Zone
- **WHEN** the Pumpkin Zone is shown and the user exports it
- **THEN** the file holds the Pumpkin Zone as edited, with the Pumpkin Zone as base, and the status bar says where it was written

#### Scenario: A zone's music travels with it
- **WHEN** the Pumpkin Zone, given a castle level's music, is exported and the file is imported as a new zone
- **THEN** the new zone plays the castle level's music and the Pumpkin Zone keeps its own

### Requirement: Importing a zone as a new zone
The world map editor SHALL import a zone file as a new zone, made from the file's base zone,
and SHALL show it. Levels a point opens that the project does not have SHALL open no level,
and exits that lead to points or zones the project does not have SHALL lead nowhere; the
status bar SHALL say so.

#### Scenario: New zone from a file
- **WHEN** a zone file exported from the Pumpkin Zone is imported
- **THEN** a new zone made from the Pumpkin Zone, like the exported one, is added and shown

### Requirement: Replacing a new zone from a file
The world map editor SHALL replace the new zone shown whole with a zone file, base included,
undoably. The game's zones SHALL NOT be replaced by a file; on them the action SHALL be
unavailable.

#### Scenario: Replace on a game zone
- **WHEN** the Tree Zone is shown
- **THEN** the File menu's Replace zone entry is greyed out

### Requirement: Rejecting unusable zone files
A zone import SHALL change nothing when the file is not a zone file, is damaged, names a base
that is not one of the game's zones, or has more points than its base allows; the status bar
SHALL say why.

#### Scenario: Level file as a zone
- **WHEN** the user imports a level file as a zone
- **THEN** nothing changes and the status bar says it is not a zone file

### Requirement: Zone file folders
Exporting a zone SHALL offer the folder `exports/zones` (any folder may be chosen), and a zone
imported as a new zone or replacing one SHALL be part of the open project once saved.

#### Scenario: Default folder
- **WHEN** a zone is exported
- **THEN** the Save dialog opens in `exports/zones`

### Requirement: The world map as a zone file
On the world map, Export zone SHALL write the whole world map (picture, point levels, paths,
places, added points, doors, shut directions, music) to a world map file, and Replace zone SHALL put
one in its place, as one change saved with the project. Points and exits leading to new levels
or new zones the project does not have SHALL lose their destination, and the editor SHALL say
how many. A world map file SHALL NOT be imported as a new zone.

#### Scenario: A world map to another project
- **WHEN** the world map of one project is exported with Export zone and, in another project, Replace zone is chosen on the world map with that file
- **THEN** the second project's world map is the file's, its music included, and saving keeps it

### Requirement: Mario's start in zone files
A zone or world map file SHALL keep Mario's start when the map holds it. Importing or replacing
from a file that has one, in a project that already has its start elsewhere, SHALL ask which to
keep (the project's or the file's); the other SHALL be dropped.

#### Scenario: Two starts
- **WHEN** a zone file holding Mario's start is imported as a new zone into a project whose start is on the world map
- **THEN** the editor asks which start to keep, and afterwards the project has exactly one

### Requirement: A zone file's origin
An exported zone file SHALL say where it came from: the project's name and the map exported
(the game zone, or the new zone and its name). Files without it SHALL still be read, with an
unknown origin. World map files are not concerned.

#### Scenario: Export marks the origin
- **WHEN** new zone 1 "Forest" of project "Demo" is exported
- **THEN** the file says it comes from new zone 1 "Forest" of project "Demo"

### Requirement: An imported zone's restore point
A zone file imported as a new zone, or replacing a new zone, SHALL keep the file's zone and
origin as that new zone's restore point (see Restore). The restore point SHALL be undone and
redone with the import or replacement, SHALL follow the zone when other new zones are removed,
and SHALL be kept with the project once saved. New zones made any other way SHALL have none.

#### Scenario: Restore point kept on save
- **WHEN** a zone file is imported as a new zone, the project is saved and opened again, and Restore is chosen on that zone
- **THEN** the zone is as the file had it

#### Scenario: A zone made from a game zone
- **WHEN** a new zone is made from the Pumpkin Zone and Restore is chosen on it
- **THEN** it is restored as the Pumpkin Zone is in the game
