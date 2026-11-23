{product}

An emulator that plays Super Mario Land 2: 6 Golden Coins from your own cartridge file, and a
level editor that makes edited copies of it (levels, objects, the world map, new zones).
No game is included: see NO-CARTRIDGE.txt. The manuals are in the manuals folder; what each
version brings, in CHANGELOG.md.

Not affiliated with or endorsed by Nintendo.


STARTING

  {game}           plays the game, or a project made in the editor (manuals\GAME.md)
  {editor}    edits levels, objects and the world map (manuals\EDITOR.md)

Unpack the whole folder somewhere you can write, such as Documents, and open a program. The
first time, it asks for your Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1)
cartridge file (.gb); both programs then use it. The file is only read, never changed.


WHAT THE COMPUTER NEEDS

  - Windows 10 or 11, 64-bit, on {cpu}.
  - A graphics card with its driver supporting OpenGL 3.1 (or at least 2.1): any card of the
    last ten years with the manufacturer's driver installed.
  - Optional: a sound device (without one the game plays silently) and a game controller
    (XInput or Windows.Gaming.Input, used as soon as it is connected).

Nothing needs installing: the programs carry everything else (no Visual C++ Redistributable,
.NET or DirectX download is needed).


IF SOMETHING GOES WRONG

To see why a program does not start, open a terminal in this folder (in Explorer, right-click
the folder, "Open in Terminal") and run it from there: .\{game}
Its messages appear in the terminal.

"Windows protected your PC" (SmartScreen)
  The programs are not signed. Choose "More info", then "Run anyway". If an antivirus moves a
  program to quarantine, restore it and allow it.

The window does not open, or closes at once, or the message mentions OpenGL or WGL
  The graphics driver is missing or too old (Windows' basic display driver has no OpenGL 3).
  - Install your card's driver from its maker (Intel, AMD or NVIDIA), not only Windows Update's.
  - In a virtual machine, turn on 3D acceleration (VirtualBox: VMSVGA with "Enable 3D
    Acceleration"; VMware: "Accelerate 3D graphics") and install its guest tools.
  - Without a usable driver (some virtual machines, ARM computers): install Microsoft's
    "OpenGL Compatibility Pack" from the Microsoft Store, which provides OpenGL through
    DirectX 12.
  - Over Remote Desktop the computer's graphics card may not be used: run it at the computer.

"VCRUNTIME140.dll was not found" (or MSVCP140.dll)
  It should not happen with this release (the C runtime is inside the programs). If it does,
  install Microsoft's Visual C++ Redistributable 2015-2022, x64:
  https://aka.ms/vs/17/release/vc_redist.x64.exe

"This folder cannot be written" (in red)
  The folder is read-only for you (for example under Program Files): settings, projects and
  saves would be lost. Move the whole folder to Documents or another folder of yours.

The cartridge is refused
  Only one dump is supported: Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), that
  is revision 1 of the game, recognised by its contents (SHA-1
  96e3a314561fb394cdf51101f9178a32713c2313), whatever its file name. Revision 0 (often named
  without "(Rev 1)"), revision 2 and modified files are refused. In PowerShell,
  Get-FileHash -Algorithm SHA1 FILE gives a file's SHA-1.

No sound
  Check that Windows has a default output device and that the game's volume (pause menu,
  Sound) is not 0.

The controller does nothing
  Connect it (cable or Bluetooth) and check it works in Windows' game controller settings.
  Its buttons are changed in the pause menu: Controls, Input: Controller.


YOUR FILES

Everything the programs keep is written in this folder, next to them: settings
(settings.cfg, editor.cfg), the cartridge chosen (base_cartridge.txt), the editor's projects
(editor\projects), the games' saves (saves) and exported files (exports). To move or back up
everything, move or copy the whole folder.


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
