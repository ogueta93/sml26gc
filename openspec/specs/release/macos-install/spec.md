# release/macos-install Specification

## Purpose

The macOS form of the editor and the game (two app bundles): which macOS they run on, how a
person opens them the first time, where their data lives and how they start each other.

## Requirements

### Requirement: Two app bundles
On macOS the game and the editor SHALL each be an app bundle, `SML2 6GC.app` (the game) and `SML2 6GC Editor.app` (the editor), with their own icon made from the project's icon PNG, their name, the version, and a universal executable that runs natively on Apple Silicon and on Intel Macs. Each bundle SHALL be signed ad hoc, so Apple Silicon runs it.

#### Scenario: Opening the game from the Finder
- **WHEN** the person double-clicks `SML2 6GC.app` after the first-time step (below)
- **THEN** the game's window opens with its icon in the Dock, and it asks for the base cartridge the first time

#### Scenario: An Intel Mac
- **WHEN** the package is opened on an Intel Mac with macOS 11
- **THEN** both apps start without Rosetta or any other installation

### Requirement: The minimum macOS
The apps SHALL run on macOS 11 Big Sur and every later version. Each bundle SHALL declare macOS 11 as its minimum, so an older macOS says the app needs a newer system instead of failing to start.

#### Scenario: macOS 12 on Apple Silicon
- **WHEN** the package built on the newest macOS is unpacked on a Mac with macOS 12 and the game is opened
- **THEN** it starts, with no error about a missing system function

#### Scenario: macOS 10.15
- **WHEN** the game is opened on macOS 10.15
- **THEN** the Finder says it needs a newer macOS, and nothing else happens

### Requirement: Opening an unsigned app the first time
The package's read-me SHALL say that the apps are not signed with an Apple developer identity, that macOS refuses them the first time, and how to open them: System Settings, Privacy & Security, Open Anyway, or the `xattr` command that removes the quarantine mark.

#### Scenario: First double-click
- **WHEN** the person double-clicks the downloaded game and macOS says it cannot verify it
- **THEN** the read-me's steps open it, and later double-clicks open it at once

### Requirement: The apps' data in Application Support
The game and the editor opened from their app bundles SHALL keep their settings, base cartridge choice, projects, saves, caches and exports in `~/Library/Application Support/sml26gc`, the same folder for both, creating it when missing, with the same layout as the portable folder; they SHALL never write inside a bundle. Each user of the Mac SHALL have their own. Programs built and run from the repository (not in a bundle) SHALL keep their files next to themselves, as on the other systems.

#### Scenario: First run of the game
- **WHEN** the game is opened on a Mac where it never ran
- **THEN** it creates `~/Library/Application Support/sml26gc`, asks for the base cartridge, and keeps the choice there

#### Scenario: The editor uses the game's choice
- **WHEN** the base cartridge was chosen in the game and the editor is then opened
- **THEN** the editor uses that cartridge without asking

#### Scenario: Replacing the apps
- **WHEN** the person replaces both apps with those of a newer version
- **THEN** their projects, saves and settings are still there

### Requirement: The editor starts the game
When both bundles are in the same folder, the editor's Play SHALL start the game of the bundle beside it, and the game SHALL start the editor of the bundle beside it. The read-me SHALL say to keep both apps in one folder.

#### Scenario: Play from the editor
- **WHEN** both apps are in Applications and Play is chosen in the editor
- **THEN** the game starts on that level
