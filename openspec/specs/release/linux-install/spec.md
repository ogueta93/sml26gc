# release/linux-install Specification

## Purpose

The installed form of the editor and the game on Linux (the `.deb` package): where the programs
go, how a person starts them, where their data lives and how they are removed.

## Requirements

### Requirement: Installing the Debian package
The `.deb` package SHALL be named `sml26gc_<version>_<arch>.deb` (`amd64` or `arm64`), with package name `sml26gc` and that architecture, and SHALL install the game and the editor side by side in `/usr/lib/sml26gc/`, the commands `sml26gc` and `sml26gc_editor` in `/usr/bin`, and the read-me, cartridge notice, legal notice, changelog, manuals and licences in `/usr/share/doc/sml26gc/`. It SHALL declare the system libraries the programs need as its dependencies.

#### Scenario: Installing on Ubuntu
- **WHEN** a person runs `sudo apt install ./sml26gc_<version>_amd64.deb` on Ubuntu 24.04
- **THEN** apt installs it with any missing library it depends on, and `sml26gc` and `sml26gc_editor` can be typed in a terminal

#### Scenario: Installing on an older Debian
- **WHEN** a person runs `sudo apt install ./sml26gc_<version>_amd64.deb` on Debian 10 or Ubuntu 18.04
- **THEN** it installs, its dependencies are met by that system's packages, and the programs start

#### Scenario: Installing on a Raspberry Pi
- **WHEN** a person runs `sudo apt install ./sml26gc_<version>_arm64.deb` on 64-bit Raspberry Pi OS
- **THEN** it installs, and the game and the editor start from the application menu

#### Scenario: The wrong architecture
- **WHEN** a person tries to install the `arm64` `.deb` on an x86-64 computer
- **THEN** apt refuses it for its architecture and nothing is installed

#### Scenario: The editor starts a test game
- **WHEN** Play is chosen in the installed editor
- **THEN** the installed game starts on that level

### Requirement: Launchers in the application menu
The installed package SHALL add one launcher for the game and one for the editor to the desktop's application menu, each with its own icon made from the project's icon PNG, its name, and no terminal window.

#### Scenario: Opening the game from the menu
- **WHEN** the person searches "SML2" in the desktop's application menu after installing
- **THEN** the game and the editor appear with their icons, and choosing one opens its window

### Requirement: Installed data in the user's folder
The installed programs SHALL keep their settings, base cartridge choice, projects, saves, caches and exports in `$XDG_DATA_HOME/sml26gc` (`~/.local/share/sml26gc` when the variable is unset), creating it when missing, with the same layout as the portable folder. Each user of the computer SHALL have their own.

#### Scenario: First run of the installed game
- **WHEN** the installed game is started by a user who never ran it
- **THEN** it creates `~/.local/share/sml26gc`, asks for the base cartridge, and keeps the choice there

#### Scenario: Moving from the portable folder
- **WHEN** a person copies the files of their portable `sml26gc/` folder (projects, saves, settings) into `~/.local/share/sml26gc`
- **THEN** the installed programs find those projects, saves and settings

### Requirement: Removing the package
Removing the package SHALL remove the programs, commands, launchers and documents it installed, and SHALL NOT remove any user's data folder.

#### Scenario: apt remove
- **WHEN** a person runs `sudo apt remove sml26gc`
- **THEN** the programs and launchers are gone and `~/.local/share/sml26gc` with its projects and saves is still there
