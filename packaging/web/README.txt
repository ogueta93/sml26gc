SML2 6GC in the browser

An emulator that plays Super Mario Land 2: 6 Golden Coins from your own cartridge file, in a
web browser, on a computer, a phone or a tablet, and an editor that makes your own levels and
maps from it, on a computer. No game is included: see NO-CARTRIDGE.txt. The manuals are in the
manuals folder; what each version brings, in CHANGELOG.md.

Not affiliated with or endorsed by Nintendo.


BROWSERS

A current Chrome, Edge, Firefox or Safari, on the computer or on the phone (on iPhone and iPad,
iOS 16.4 or later). The browser needs WebAssembly and WebGL, which all of them have.


SERVING IT

Browsers do not run the page opened straight from the folder (index.html says so if you try).
It must be served by a web server, which can be your own computer:

  1. Open a terminal in this folder (the one with index.html).
  2. Run
       Windows:          py -m http.server
       Linux, macOS:     python3 -m http.server
     (Python 3 is needed; any other static file server works as well.)
  3. Open http://localhost:8000/ in the browser.

To play it from other devices, or to let others play it, upload the whole folder to any static
web host (GitHub Pages, Netlify, your own server...) served over HTTPS: browsers keep data and
allow fullscreen only on HTTPS pages (or on localhost). Nothing runs on the server; the files
are only sent.


THE CARTRIDGE AND YOUR DATA

The first time, the page asks for your Super Mario Land 2 - 6 Golden Coins (USA, Europe)
(Rev 1) cartridge file (.gb): choose it with the start screen's button, or drop it on the
page. It is read by the browser and kept in it; it is never sent anywhere, not even to the
server the page comes from.

The cartridge, settings, saves and the editor's projects are kept in the browser's storage for
that site, and stay when the browser is closed. To keep a copy of a project elsewhere, export
it in the editor (File, Export project). Clearing the site's data (or the browser's history with "site
data") removes them all, and the page asks for the cartridge again. Each browser, and each
address the page is served from, keeps its own.


THE EDITOR

editor.html (or Edit and New project on the game's start screen) opens the editor in a tab of
its own. It needs a mouse or trackpad and a keyboard: on a phone or tablet the page says so. It
uses the same cartridge as the game, and the projects saved in it are on the game's start
screen. Files it exports are downloaded; File, Export cartridge downloads a project's edited
copy (.gb). The manual is manuals/EDITOR.md.


ON A PHONE OR TABLET

Touch the screen to show the on-screen controls (they hide again when a keyboard or controller
is used). The four-corners button enters fullscreen, without the browser's bars.

The page can be added to the home screen, and opens from there as an app, without bars:
  Android (Chrome):  the menu (three dots), Add to Home screen or Install app.
  iPhone, iPad (Safari): Share, Add to Home Screen. Safari on the iPhone has no fullscreen
                         button, so this is the way to play without bars there.


IF SOMETHING GOES WRONG

The page stays empty, or says the browser cannot run the game
  Serve it as above (not from the folder), over HTTPS when it is not localhost, and use one of
  the browsers above, up to date.

No sound
  Browsers start the sound only after a tap or a key: touch the screen or press a key. On an
  iPhone, the ring/silent switch mutes pages too: turn the ringer on. Check the game's volume
  (pause menu, Sound).

The cartridge is refused
  Only one dump is supported: Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1), that
  is revision 1 of the game, recognised by its contents (SHA-1
  96e3a314561fb394cdf51101f9178a32713c2313), whatever its file name. Revision 0 (often named
  without "(Rev 1)"), revision 2 and modified files are refused.

The controller does nothing
  Browsers show a controller to the page only after one of its buttons is pressed while the
  page is open: press a button. Its buttons are changed in the pause menu: Controls, Input:
  Controller.

Esc leaves fullscreen
  In fullscreen the browser takes Esc for itself. The pause menu also opens with Start (in a
  level) and with the on-screen menu button.


GAME KEYS (changeable in the pause menu)

  Arrows      move               X        A (jump)
  Z           B (run, fire)      Enter    Start
  Backspace   Select             Esc      pause menu
  F11         full screen        Left Ctrl (held)  fast forward

The full lists are in the manual.


LICENCE AND LEGAL NOTICE

Copyright 2026 ogueta93 (https://github.com/ogueta93) and the SML2 6GC authors. This program is
free software under the GNU General Public License, version 3 or later (LICENSE.txt): you may
share and change it under the same licence.
Its source code: {source}
LEGAL.md says what it contains, on what grounds it is made, and who is responsible for what.
The licences of the fonts and libraries it includes are in the licenses folder.
