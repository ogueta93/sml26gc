{product}

An emulator that plays Super Mario Land 2: 6 Golden Coins from your own cartridge file, and a
level editor that makes edited copies of it (levels, objects, the world map, new zones).
No game is included: see NO-CARTRIDGE.txt. The manuals are in the manuals folder; what each
version brings, in CHANGELOG.md.

Not affiliated with or endorsed by Nintendo.


STARTING

  {game}           plays the game, or a project made in the editor (manuals/GAME.md)
  {editor}    edits levels, objects and the world map (manuals/EDITOR.md)

Unpack the whole folder somewhere you can write, such as your home folder (this works on every
distribution: Debian, Ubuntu, Mint, Fedora, openSUSE, Arch...):

  tar -xzf {id}-<version>-linux-x64.tar.gz
  cd {id}
  ./{game}

or open the programs from the file manager (double-click; some file managers ask whether to
run it). The first time, a program asks for your Super Mario Land 2 - 6 Golden Coins (USA,
Europe) (Rev 1) cartridge file (.gb); both programs then use it. The file is only read, never
changed.
The game also takes the cartridge file as its argument: ./{game} /path/to/game.gb

To install the programs instead, with launchers in your desktop's application menu, use the
.deb package (Debian, Ubuntu, Linux Mint, Raspberry Pi OS and the like) of your processor,
_amd64 on x86_64 and _arm64 on ARM, from the folder that holds it:

  sudo apt install ./{id}_<version>_amd64.deb     installs it and any library missing
  sudo dpkg -i {id}_<version>_amd64.deb           installs only the package; if it reports
                                                  missing libraries, sudo apt -f install
                                                  installs them

Then open "SML2 6GC" or "SML2 6GC Editor" from the application menu, or type {game} or
{editor} in a terminal. To remove them (your files stay):

  sudo apt remove {id}      or      sudo dpkg -r {id}

Installing a newer version the same way replaces the older one.


WHAT THE COMPUTER NEEDS

  - Linux on {cpu},
    with glibc {glibc} or newer: Ubuntu 18.04, Linux Mint 19, Debian 10, RHEL, AlmaLinux and
    Rocky Linux 8, Fedora 28, openSUSE Leap 15.3, Raspberry Pi OS (64-bit) or any later version,
    Arch Linux.
  - A desktop with X11, or Wayland with Xwayland (every common desktop has it).
  - A graphics card with its driver supporting OpenGL 3.1 (or at least 2.1): any card of the
    last ten years with the distribution's Mesa drivers, or NVIDIA's.
  - These libraries, which every desktop installation already has:
      library                     Debian, Ubuntu, Mint            Fedora
      ALSA (sound)                libasound2 (libasound2t64)      alsa-lib
      udev (controllers)          libudev1                        systemd-libs
      OpenGL                      libgl1                          libglvnd-glx, mesa-dri-drivers
      X11                         libx11-6, libxi6                libX11, libXi
      keyboard (X11)              libxkbcommon0                   libxkbcommon
      D-Bus (file dialogs)        libdbus-1-3                     dbus-libs
  - Optional: a sound device (without one the game plays silently), a game controller, and a
    file dialog: the desktop portal (xdg-desktop-portal with the one of your desktop, as GNOME,
    KDE and most others install) or zenity.

Nothing else needs installing: the fonts and translations are inside the programs.


IF SOMETHING GOES WRONG

To see why a program does not start, open a terminal in this folder and run it from there:
./{game}
Its messages appear in the terminal.

"Permission denied" when starting a program
  The file lost its executable mark (unpacked with a tool that drops it, or copied from a
  USB stick formatted for Windows). Run: chmod +x {game} {editor}
  On a drive mounted with "noexec", move the folder to your home folder.

"error while loading shared libraries: libasound.so.2" (or libudev.so.1, libGL.so.1...)
  A library of the table above is missing (minimal installations). Install its package:
  sudo apt install libasound2 libudev1 libgl1 libx11-6 libxi6   (Debian, Ubuntu, Mint)
  sudo dnf install alsa-lib systemd-libs libglvnd-glx libX11 libXi   (Fedora)

"version `GLIBC_2.xx' not found"
  The distribution is older than the ones listed above. Use a newer one.

The window does not open, or closes at once, or the message mentions OpenGL or GLX
  The graphics driver is missing or too old.
  - Install the distribution's Mesa drivers (sudo apt install libgl1-mesa-dri, or
    sudo dnf install mesa-dri-drivers), or your card maker's driver (NVIDIA).
  - In a virtual machine, turn on 3D acceleration (VirtualBox: VMSVGA with "Enable 3D
    Acceleration"; VMware: "Accelerate 3D graphics") and install its guest tools.
  - Over a remote desktop the computer's graphics card may not be used: run it at the computer.

apt ends with "N: Download is performed unsandboxed as root ... couldn't be accessed by user
'_apt'"
  Only a note, not an error: the package is installed. apt reads packages as its own user,
  _apt, which cannot read your home folder, so it read it as root. To avoid the note, copy the
  .deb to /tmp first and install it from there, or use sudo dpkg -i.

"No file dialog is available" (in red)
  Neither a desktop portal nor zenity is installed (some minimal window managers). Install one:
  sudo apt install zenity (or xdg-desktop-portal-gtk), sudo dnf install zenity.
  Meanwhile, give the cartridge to the game as its argument: ./{game} /path/to/game.gb

"This folder cannot be written" (in red)
  The folder is read-only for you (for example under /opt, or another user's folder): settings,
  projects and saves would be lost. Move the whole folder to your home folder.

The cartridge is refused
  Only one dump is supported: Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), that
  is revision 1 of the game, recognised by its contents (SHA-1
  96e3a314561fb394cdf51101f9178a32713c2313), whatever its file name. Revision 0 (often named
  without "(Rev 1)"), revision 2 and modified files are refused. sha1sum FILE gives a file's
  SHA-1.

No sound
  Check the desktop's sound settings have an output device, and that the game's volume (pause
  menu, Sound) is not 0. The game plays through ALSA, which PipeWire and PulseAudio serve.

The controller does nothing
  Connect it (cable or Bluetooth) and check the desktop sees it. The game reads controllers from
  /dev/input/event*: desktops give the person at the computer access to them; if yours does
  not (some minimal systems), add your user to the "input" group (sudo usermod -aG input $USER,
  then log out and in). Its buttons are changed in the pause menu: Controls, Input: Controller.


YOUR FILES

Unpacked from the .tar.gz, everything the programs keep is written in this folder, next to
them: settings (settings.cfg, editor.cfg), the cartridge chosen (base_cartridge.txt), the
editor's projects (editor/projects), the games' saves (saves) and exported files (exports). To
move or back up everything, move or copy the whole folder. To update to a new version, unpack it
over this folder (from the same place, with the same tar command): the programs are replaced and
your files stay. To remove the programs, delete the folder, after copying out anything you want
to keep.

Installed from the .deb, the same files are in ~/.local/share/{id} (or $XDG_DATA_HOME/{id}),
one for each user of the computer. To move from the unpacked folder to the installed programs,
copy those files and folders from the unpacked folder into ~/.local/share/{id}.


GAME KEYS (changeable in the pause menu)

  Arrows      move               X        A (jump)
  Z           B (run, fire)      Enter    Start
  Backspace   Select             Esc      pause menu
  F11         full screen        Left Ctrl (held)  fast forward

The full lists are in the manuals.


LICENCE AND LEGAL NOTICE

Copyright 2026 ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors. These programs
are free software under the GNU General Public License, version 3 or later (LICENSE.txt): you
may share and change them under the same licence.
Their source code: {source}
LEGAL.md says what they contain, on what grounds they are made, and who is responsible for what.
The licences of the fonts and libraries they include are in the licenses folder.
