# release/packaging Specification

## Purpose
How the editor and the game are made into a folder and a zip that another person can unpack and run
on a supported platform, with only what the programs need and the notices they must carry.

## Requirements

### Requirement: One command builds a platform's package
A single command run from the repository, naming a platform target (`windows-x64`, `linux-x64` or `macos-universal`), SHALL build the editor and the game for that target in release, assemble the package folder `dist/<target>/sml26gc/` from an empty folder each time, and write the target's packages (see "Each platform's packages"). An unknown target SHALL stop with a message listing the known ones, and nothing SHALL be written.

#### Scenario: Building the Windows package
- **WHEN** the developer runs the package command for `windows-x64`
- **THEN** `dist/windows-x64/sml26gc/` holds the editor and the game built in release, and `dist/sml26gc-<version>-windows-x64.zip` holds that folder

#### Scenario: Building the Linux packages
- **WHEN** the developer runs the package command for `linux-x64` on Linux
- **THEN** `dist/linux-x64/sml26gc/` holds the editor and the game built in release, `dist/sml26gc-<version>-linux-x64.tar.gz` holds that folder with both programs executable, and `dist/sml26gc_<version>_amd64.deb` installs them as `release/linux-install` says

#### Scenario: Building the macOS package
- **WHEN** the developer runs the package command for `macos-universal` on a Mac
- **THEN** `dist/macos-universal/sml26gc/` holds the two app bundles built in release, and `dist/sml26gc-<version>-macos-universal.zip` holds that folder with the bundles intact (executable, signed)

#### Scenario: An unknown target
- **WHEN** the package command is run for `amiga`
- **THEN** it says which targets it knows and writes nothing

### Requirement: The package holds only what runs
The package folder SHALL hold exactly: the game and editor executables (on macOS, inside their two app bundles), a read-me, the cartridge notice, the licence text, the legal notice, the changelog, the user manuals of the game and the editor, and the licences folder. It SHALL NOT hold source files, build output (debug symbols, intermediate files, other binaries such as the research tool), cartridges or anything made from one, projects, saves, settings files, caches or exports.

#### Scenario: The developer's own files stay out
- **WHEN** the package is built on a computer whose `target/release` holds projects, saves, `settings.cfg`, `editor.cfg`, `base_cartridge.txt` and `.pdb` files
- **THEN** none of them is in the package folder, the zip, the `.tar.gz` or the `.deb`

#### Scenario: Listing the package
- **WHEN** the Windows package folder is listed
- **THEN** it holds `sml26gc.exe`, `sml26gc_editor.exe`, `README.txt`, `NO-CARTRIDGE.txt`, `LICENSE.txt`, `LEGAL.md`, `CHANGELOG.md`, `manuals/` (`GAME.md`, `EDITOR.md`) and `licenses/`, and nothing else

#### Scenario: Listing the Linux package
- **WHEN** the Linux package folder is listed
- **THEN** it holds `sml26gc`, `sml26gc_editor`, `README.txt`, `NO-CARTRIDGE.txt`, `LICENSE.txt`, `LEGAL.md`, `CHANGELOG.md`, `manuals/` (`GAME.md`, `EDITOR.md`) and `licenses/`, and nothing else

#### Scenario: Listing the macOS package
- **WHEN** the macOS package folder is listed
- **THEN** it holds `SML2 6GC.app`, `SML2 6GC Editor.app`, `README.txt`, `NO-CARTRIDGE.txt`, `LICENSE.txt`, `LEGAL.md`, `CHANGELOG.md`, `manuals/` (`GAME.md`, `EDITOR.md`) and `licenses/`, and nothing else; each bundle holds only its executable, `Info.plist`, its icon and its signature

### Requirement: No cartridge is included
The package SHALL NOT contain any cartridge or data taken from one, and SHALL say so: `NO-CARTRIDGE.txt` (and the read-me) SHALL state that the programs include no game, that the person uses a cartridge file from their own backup of a cartridge they own, that the authors take no responsibility for where it was obtained, and that the product is not affiliated with or endorsed by Nintendo. The programs SHALL keep asking for the cartridge on first run, as they do now.

#### Scenario: First run from the package
- **WHEN** the unpacked game is started on a computer that never ran it
- **THEN** it asks for the base cartridge file, and the package's notice explains that the person provides it

### Requirement: The executables carry their icons and version
On Windows, each executable SHALL show its own icon (in Explorer, the taskbar and its window) made from the PNG the project keeps for it: `assets/icons/sml26gc.png` for the game and `assets/icons/sml26gc_editor.png` for the editor. Each SHALL carry version details: the product name, the program's name, the version, and a line saying it is not affiliated with Nintendo. While an icon file is missing, the build SHALL still succeed, without that icon, and say which file is missing.

#### Scenario: The icons are given
- **WHEN** `assets/icons/sml26gc.png` and `assets/icons/sml26gc_editor.png` exist and the package is built
- **THEN** `sml26gc.exe` and `sml26gc_editor.exe` each show their own icon in Explorer, and their Properties show the product name and the version

#### Scenario: An icon is missing
- **WHEN** `assets/icons/sml26gc_editor.png` does not exist and the package is built
- **THEN** the build succeeds, the editor has the default icon, and the build output names the missing file

### Requirement: The package carries its licences
The `licenses/` folder SHALL hold the licence of every font the programs include (SIL Open Font License, one file per font family) and a list of the Rust crates compiled into the executables with each crate's licence text. The package command SHALL make this list from the build's own dependency graph, so it never names a crate the executables do not include.

#### Scenario: A font's licence
- **WHEN** the package is built
- **THEN** `licenses/` holds the licences of BIZ UDGothic, Noto Sans CJK and Fusion Pixel Font, and the third-party crates list names `macroquad` with its licence

### Requirement: The product's names
The product SHALL be named "SML2 6GC Emulator and Level Editor" (saying it is an emulator with a level editor for Super Mario Land 2: 6 Golden Coins), with the short id `sml26gc`. The game's executable SHALL be `sml26gc` (`sml26gc.exe` on Windows) and the editor's `sml26gc_editor` (`sml26gc_editor.exe`); the editor SHALL start the game, and the game the editor, by these names from their own folder.

#### Scenario: The editor starts a test game
- **WHEN** Play is chosen in the editor of an unpacked package
- **THEN** `sml26gc.exe` from the same folder starts on that level

### Requirement: Product name and version in one place
The product name, the short id, the version and the list of platform targets SHALL each be defined once (the version is the workspace's); the package's folder and zip names and the executables' version details SHALL take them from there. The package's texts (read-me, cartridge notice, third-party list) SHALL NOT show the version.

#### Scenario: A new version
- **WHEN** the workspace version is changed to `0.2.0` and the package is built
- **THEN** the zip is named with `0.2.0`, both executables' Properties show `0.2.0`, and no text of the package names a version

### Requirement: The package says what a computer needs
Each platform's read-me SHALL say what a computer needs to run the programs and that nothing else needs installing, and SHALL say, for each way a program can fail to start or work, its cause and what to do.

#### Scenario: A computer without a graphics driver
- **WHEN** the game closes at once on a fresh Windows 11 without its graphics driver
- **THEN** the package's read-me names the cause and how to get the driver or the OpenGL Compatibility Pack

#### Scenario: What the read-me covers
- **WHEN** the read-me is read
- **THEN** it gives the Windows version, the graphics driver (OpenGL 3.1, at least 2.1), the optional sound device and controller, how to see a program's messages from a terminal, and what to do for SmartScreen, a missing or old graphics driver (the maker's driver, 3D acceleration in a virtual machine, the OpenGL Compatibility Pack, Remote Desktop), a missing Visual C++ runtime (the redistributable to download), a folder that cannot be written, a refused cartridge, no sound and a controller that does nothing

#### Scenario: What the Linux read-me covers
- **WHEN** the Linux package's read-me is read
- **THEN** it gives the oldest glibc (2.31) and example distributions (Debian 11, Ubuntu 20.04 and later, Fedora, Mint, Arch), the libraries the programs use with the Debian/Ubuntu and Fedora packages that provide them, the graphics driver (OpenGL 3.1, at least 2.1), the optional sound device, controller and file dialog (desktop portal or zenity), how to unpack and start the programs from a terminal or the file manager, how to install the `.deb` instead, and what to do for a program that is not executable, a missing library, a missing or old graphics driver, a folder that cannot be written, no file dialog, a refused cartridge, no sound and a controller that does nothing (access to `/dev/input`)

#### Scenario: What the macOS read-me covers
- **WHEN** the macOS package's read-me is read
- **THEN** it gives the oldest macOS (11 Big Sur), that Apple Silicon and Intel Macs are both supported and which Macs that means, how to unpack the zip and move both apps to Applications or any one folder, how to open an unsigned app the first time (Open Anyway in Privacy & Security, or `xattr -dr com.apple.quarantine`), where the files are kept and how to back them up, the keys (Cmd in the editor; F11 or Cmd+Ctrl+F for full screen, Fn with function keys), how to see a program's messages from Terminal, and what to do for an app macOS refuses or calls damaged, an app that needs a newer macOS, Play that does not start the game, a refused cartridge, no sound and a controller that does nothing

### Requirement: Documentation for players and developers
The repository SHALL document, each in one place: the game's and the editor's user manuals, how to prepare a computer to develop on each supported platform (the language, tools and system libraries and how to install them), how to build, run, check and package on each, and how and why the emulator and the editor were made. The README SHALL describe both programs briefly, name the supported platforms and link to each document.

#### Scenario: A new developer
- **WHEN** someone with a fresh Windows 11 reads the development guide
- **THEN** it names every tool to install (Rust with the MSVC toolchain, the Visual Studio Build Tools with the C++ workload and a Windows SDK, Git, and the optional OpenSpec CLI), where to get each, and the commands to build, test and package

#### Scenario: A new developer on Linux
- **WHEN** someone with a fresh Ubuntu 24.04 reads the development guide
- **THEN** it names every tool and library to install (Rust through rustup, `build-essential`, `pkg-config`, `libasound2-dev`, `libudev-dev`, Git, Docker for packaging, and the optional OpenSpec CLI) with the command that installs them, their equivalents on Fedora, and the commands to build, run, test and package for Linux

### Requirement: Licence
The project's own work SHALL be licensed under GPL-3.0-or-later, copyright ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors, with each file's licence stated in `REUSE.toml` and the texts in `LICENSES/` (the fonts keep OFL-1.1). Each package SHALL carry the licence text and the address of the repository that holds the source code, and the executables' version details SHALL name the copyright holder and the licence.

#### Scenario: Someone receives the zip
- **WHEN** a person unpacks a package
- **THEN** it holds `LICENSE.txt` (the GPL) and `LEGAL.md`, and its read-me gives the copyright, the licence and the address of the source code

#### Scenario: Which licence a file has
- **WHEN** a font file or a source file is looked up in `REUSE.toml`
- **THEN** the font is OFL-1.1 with its authors, and the source file GPL-3.0-or-later, copyright ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors

#### Scenario: The author's name travels with the code
- **WHEN** someone reads the copyright notice of the repository, a package or an executable
- **THEN** it names ogueta93 with the address of their GitHub profile

### Requirement: Legal notice
A legal notice (`LEGAL.md`), also in each package, SHALL state what is and is not included, the legal grounds of the emulator and the editor, that the project is not commercial, who is responsible for what, that there is no warranty, and how a rights holder reaches the author.

#### Scenario: What the legal notice covers
- **WHEN** `LEGAL.md` is read
- **THEN** it says no game or Nintendo material is included, gives the grounds (study and interoperability of programs, the user's own backup, changes for one's own use, names used only to say what the programs work with), says nobody makes money from the project, that each person answers for their use and their cartridge file's origin, and gives the author's contact

### Requirement: Each platform's packages
The Windows target SHALL write `dist/sml26gc-<version>-windows-x64.zip` holding the package folder. The Linux target SHALL write `dist/sml26gc-<version>-linux-x64.tar.gz` holding the package folder with both programs executable, and `dist/sml26gc_<version>_amd64.deb` holding the same files placed as `release/linux-install` says, plus its launchers and icons.

#### Scenario: The Linux packages' contents
- **WHEN** the Linux packages are built
- **THEN** unpacking the `.tar.gz` gives `sml26gc/` with `sml26gc` and `sml26gc_editor` executable, and the `.deb`'s file list holds the same executables, texts, manuals and licences plus two launchers and two icons

### Requirement: The Linux executables' libraries are checked
When building the Linux packages, the package command SHALL stop, naming the library, if either executable needs a shared library outside the set the Linux read-me names, and SHALL stop if they need a glibc newer than 2.31, naming the version and the symbol's library. The oldest glibc they need SHALL be printed and stated in the read-me and the `.deb`'s dependencies.

#### Scenario: An unexpected library
- **WHEN** a dependency makes the editor link against `libgtk-3.so.0` and the Linux package is built
- **THEN** the command stops and names `libgtk-3.so.0` and the executable that needs it, and no package is written

#### Scenario: The glibc version
- **WHEN** the Linux package is built on Ubuntu 24.04
- **THEN** the command prints a glibc of at most 2.31, and the `.deb` depends on `libc6` of at least that version

#### Scenario: Executables built against a newer glibc
- **WHEN** the Linux executables were linked against glibc 2.39
- **THEN** the command stops, says they need glibc 2.39 where 2.31 is the limit, and writes no package

### Requirement: Linux packages built in a Debian 11 container
The package command for `linux-x64` SHALL build the game and the editor inside a Docker container of Debian 11 (glibc 2.31) holding the C toolchain, the ALSA and udev headers and the workspace's Rust version, building that image first when missing or outdated. The files it writes SHALL belong to the developer, not root. Without a usable Docker it SHALL stop before building, saying how to install Docker and get access to it.

#### Scenario: Packaging on Ubuntu 24.04
- **WHEN** the developer runs the package command for `linux-x64` on Ubuntu 24.04 with Docker
- **THEN** the executables are built in the Debian 11 container, the packages are written to `dist/`, and every file written is owned by the developer

#### Scenario: Docker missing
- **WHEN** the package command for `linux-x64` is run where `docker` is not installed, or the developer cannot use it
- **THEN** it says Docker is needed, how to install it and join the `docker` group, and writes nothing

#### Scenario: A new Rust version
- **WHEN** the workspace's `rust-version` is raised and the Linux package is built
- **THEN** the build image is rebuilt with that Rust before building

### Requirement: The macOS bundles carry their icons and version
On macOS, each app bundle SHALL show its own icon in the Finder and the Dock, made from the same PNG as on Windows (`assets/icons/sml26gc.png`, `assets/icons/sml26gc_editor.png`), and its Get Info window SHALL show the version and the copyright line.

#### Scenario: The macOS bundles' icons
- **WHEN** the macOS package is unpacked in the Finder
- **THEN** `SML2 6GC.app` and `SML2 6GC Editor.app` each show their own icon, and Get Info shows the version

### Requirement: The macOS package's zip
The macOS target SHALL write `dist/sml26gc-<version>-macos-universal.zip` holding the package folder, made so that the bundles keep their executable marks and signatures when unpacked by the Finder.

#### Scenario: Unpacking the macOS zip
- **WHEN** the macOS zip is double-clicked in the Finder
- **THEN** it gives `sml26gc/` with both app bundles, whose signatures `codesign --verify` accepts

### Requirement: The macOS executables are checked
When building the macOS package, the package command SHALL build each program for Apple Silicon and Intel with macOS 11 as its minimum and join them into one universal executable. It SHALL stop, naming the executable and what it found, if an architecture is missing, if an architecture's minimum macOS is newer than 11.0 (the limit, defined once in the target), or if an executable links a library outside `/usr/lib/` and `/System/Library/Frameworks/`. It SHALL print the minimum macOS it found.

#### Scenario: The minimum version
- **WHEN** the macOS package is built on the newest macOS
- **THEN** the command prints a minimum of macOS 11.0 for both architectures and writes the zip

#### Scenario: A library from Homebrew
- **WHEN** a dependency makes the game link `/opt/homebrew/lib/libSDL2.dylib` and the macOS package is built
- **THEN** the command stops, names that library and the executable that needs it, and writes no zip

#### Scenario: A newer minimum
- **WHEN** the executables were linked with macOS 14 as their minimum
- **THEN** the command stops, says they need macOS 14.0 where 11.0 is the limit, and writes no zip

#### Scenario: Packaging off a Mac
- **WHEN** the package command for `macos-universal` is run on Linux
- **THEN** it says the macOS package is built on macOS and writes nothing
