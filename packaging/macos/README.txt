{product}

An emulator that plays Super Mario Land 2: 6 Golden Coins from your own cartridge file, and a
level editor that makes edited copies of it (levels, objects, the world map, new zones).
No game is included: see NO-CARTRIDGE.txt. The manuals are in the manuals folder; what each
version brings, in CHANGELOG.md.

Not affiliated with or endorsed by Nintendo.


STARTING

  SML2 6GC.app          plays the game, or a project made in the editor (manuals/GAME.md)
  SML2 6GC Editor.app   edits levels, objects and the world map (manuals/EDITOR.md)

1. Double-click the zip: the Finder unpacks it into a folder {id}.
2. Drag both apps into Applications (or both into any other folder of yours). Keep them in the
   same folder: the editor starts the game from the app beside it, and the game the editor.
3. Open each app the first time as "THE FIRST TIME" below says. From then on, open them as any
   other app (Launchpad, Spotlight, the Dock).

The first time, a program asks for your Super Mario Land 2 - 6 Golden Coins (USA, Europe)
(Rev 1) cartridge file (.gb); both programs then use it. The file is only read, never changed.


THE FIRST TIME

The apps are not signed with an Apple developer identity (that needs a paid Apple account), so
macOS refuses to open them the first time ("cannot be opened because Apple cannot check it for
malicious software", or "cannot verify that it is free of malware"). To open them:

  - Double-click the app and close the message. Open System Settings (System Preferences on
    macOS 11 and 12), Privacy & Security, scroll down to the message about the app and choose
    "Open Anyway", then "Open". Do it for both apps.
    On macOS 11 to 14 you can instead Control-click (or right-click) the app, choose Open, and
    then Open in the message.
  - Or, in Terminal, once for both apps (after moving them to Applications):
      xattr -dr com.apple.quarantine "/Applications/SML2 6GC.app" "/Applications/SML2 6GC Editor.app"

Opening an app that is not signed is your choice: these apps come from {source}, and their
source code is there for anyone to check.


WHAT THE MAC NEEDS

  - macOS {macos} (Big Sur) or later, on Apple Silicon (M1 and later) or Intel: every Mac that
    runs macOS 11, that is MacBook Air from 2013, MacBook Pro from late 2013, MacBook from 2015,
    iMac from 2014, Mac mini from 2014, iMac Pro, Mac Pro from 2013, and every later Mac. The
    apps run natively on both kinds of processor (no Rosetta).
  - Optional: a game controller (Xbox, PlayStation and other USB or Bluetooth controllers).

Nothing needs installing: the fonts and translations are inside the programs.


IF SOMETHING GOES WRONG

To see why a program does not start, open Terminal and run it from there:
  "/Applications/SML2 6GC.app/Contents/MacOS/{game}"
Its messages appear in Terminal. The game also takes the cartridge file as its argument:
  "/Applications/SML2 6GC.app/Contents/MacOS/{game}" /path/to/game.gb

macOS refuses to open the app, or says it is damaged and should be moved to the Trash
  The app is not signed with an Apple developer identity (see THE FIRST TIME above). "Damaged"
  is what macOS says of such an app downloaded from the internet: the xattr command of THE
  FIRST TIME fixes it.

"You can't use this version of the application with this version of macOS"
  The Mac runs a macOS older than {macos}. Update macOS (Apple menu, System Settings, Software
  Update), if the Mac can.

Play in the editor does not start the game (a message names a path)
  The two apps are not in the same folder, or are still opened from the Downloads folder (macOS
  then runs them from a hidden copy). Move both apps into Applications, together, and open them
  from there.

"This folder cannot be written" (in red)
  The folder of the apps' files (YOUR FILES, below) cannot be written: check its permissions in
  the Finder (Get Info, Sharing & Permissions), or delete it to start afresh.

The cartridge is refused
  Only one dump is supported: Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), that
  is revision 1 of the game, recognised by its contents (SHA-1
  96e3a314561fb394cdf51101f9178a32713c2313), whatever its file name. Revision 0 (often named
  without "(Rev 1)"), revision 2 and modified files are refused. In Terminal, shasum -a 1 FILE
  gives a file's SHA-1.

No sound
  Check the Mac's sound output (System Settings, Sound) and that the game's volume (pause menu,
  Sound) is not 0.

The controller does nothing
  Connect it (cable or Bluetooth, System Settings, Bluetooth) before opening the game. Its
  buttons are changed in the pause menu: Controls, Input: Controller.

F11 shows the desktop, or a function key changes the volume or brightness
  On many Mac keyboards the function keys need Fn (Fn+F11), and macOS keeps F11 for Show
  Desktop. Full screen also works with Cmd+Ctrl+F. To make F1, F2... work without Fn: System
  Settings, Keyboard, Keyboard Shortcuts, Function Keys.


YOUR FILES

Everything the programs keep is in your Library folder, the same for both apps:
  ~/Library/Application Support/{id}
settings (settings.cfg, editor.cfg), the cartridge chosen (base_cartridge.txt), the editor's
projects (editor/projects), the games' saves (saves) and exported files (exports). In the
Finder, Go, Go to Folder, and type the path above. To back up everything, copy that folder.
Replacing the apps with a newer version keeps it; deleting the apps does not delete it.


KEYS

In the editor, Cmd takes the place of Ctrl in every shortcut the manual gives (Cmd+S saves,
Cmd+Z undoes). Cmd+Q quits either program.

Game keys (changeable in the pause menu):

  Arrows      move               X        A (jump)
  Z           B (run, fire)      Return   Start
  Delete      Select             Esc      pause menu
  F11 or Cmd+Ctrl+F  full screen    Left Control (held)  fast forward

The full lists are in the manuals.


LICENCE AND LEGAL NOTICE

Copyright 2026 ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors. These programs
are free software under the GNU General Public License, version 3 or later (LICENSE.txt): you
may share and change them under the same licence.
Their source code: {source}
LEGAL.md says what they contain, on what grounds they are made, and who is responsible for what.
The licences of the fonts and libraries they include are in the licenses folder.
