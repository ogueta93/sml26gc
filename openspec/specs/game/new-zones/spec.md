# game/new-zones Specification

## Purpose
New zones are played in this port as stand-ins for one of the game's zones: the game runs its
own zone code while the port puts the new zone's picture, points, paths, levels and exits in
place, so a project can have more zones than the cartridge.

## Requirements

### Requirement: Entering a new zone
When Mario uses a world map point that is a door into a new zone, the game SHALL show that
zone: its picture, with the base zone's graphics and the new zone's own music, and Mario on the
zone's first point.

#### Scenario: Door to a new zone
- **WHEN** world point $05 is the door into new zone 1 (made from the Tree Zone) and Mario presses A on it
- **THEN** new zone 1's picture is shown with the Tree Zone's tiles and music, and Mario stands on its first point

#### Scenario: A new zone with its own music
- **WHEN** new zone 1 (made from the Tree Zone) was given the Space Zone's map music and Mario enters it
- **THEN** new zone 1 is shown with the Tree Zone's tiles and plays the Space Zone's map music

### Requirement: Walking and playing in a new zone
Inside a new zone, Mario SHALL walk its own paths between its own points, and each point SHALL
open the level the new zone gives it (one of the 32 or a new level).

#### Scenario: A point's level
- **WHEN** a point of new zone 1 opens new level 35 and Mario presses A on it
- **THEN** level 35 is played and, cleared or not, Mario comes back to that point of new zone 1

### Requirement: Leaving a new zone
A new zone's exits SHALL take Mario where the zone says: a world map point or another zone.

#### Scenario: Exit to the world map
- **WHEN** new zone 1's exit leads to world point $05 and Mario walks out through it
- **THEN** the world map is shown with Mario on point $05

### Requirement: Base zone unchanged
The base zone SHALL play exactly as before, with its own picture, points, levels, exits and
progress, before and after a new zone made from it is played.

#### Scenario: Back to the base zone
- **WHEN** Mario leaves new zone 1 and enters the Tree Zone
- **THEN** the Tree Zone is the game's (or as edited), and its cleared points are the ones Mario cleared there

### Requirement: Progress of a new zone
Which points of a new zone are cleared SHALL be kept apart from every other zone's and SHALL
survive turning the game off: the port SHALL keep it in a file next to the battery save.

#### Scenario: Clear a level in a new zone
- **WHEN** Mario clears the level of a point in new zone 1, saves, and the game is started again
- **THEN** that point of new zone 1 is cleared and the matching point of the Tree Zone is not

### Requirement: Resuming inside a new zone
A game saved while Mario is inside a new zone SHALL start again inside that new zone.

#### Scenario: Saved in a new zone
- **WHEN** the game is saved with Mario on a point of new zone 1 and started again
- **THEN** new zone 1 is shown with Mario on that point

### Requirement: Only this port
New zones SHALL play only in this port; the edited copy on another emulator SHALL play the
game's zones as edited, and a new zone's door SHALL lead nowhere there.

#### Scenario: Other emulator
- **WHEN** the edited copy is played on another emulator and Mario presses A on a door into a new zone
- **THEN** nothing happens, as on a point with no door

### Requirement: Starting inside a new zone
When a project's start for Mario is in a new zone, a new game SHALL begin inside that new zone
on that point, as if Mario had entered it through its door.

#### Scenario: A new game in a new zone
- **WHEN** a project's start is on a point of a new zone and a new game is started
- **THEN** the game shows the new zone with Mario on that point
