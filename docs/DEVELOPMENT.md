# Development

How to set up a computer to work on the project, build and check it, make the Windows, Linux,
macOS and web packages, and how the emulator and the editor were made. The rules for changing the code are in
[`AGENTS.md`](../AGENTS.md); the code's layout in [`ARCHITECTURE.md`](ARCHITECTURE.md).

## What you need

Development happens on Windows 11 or Linux, on x86-64, or on macOS (Apple Silicon or Intel);
each platform's package is built on that platform.

### Windows

| Tool | Why | How to get it |
|---|---|---|
| Rust (stable, MSVC toolchain `stable-x86_64-pc-windows-msvc`) | The language of every program: edition 2024, Rust 1.100 or later (`rust-version` in `Cargo.toml`; older toolchains are refused) | [rustup](https://rustup.rs): run `rustup-init.exe`, keep the default (MSVC) toolchain; `rustup update` later |
| Visual Studio 2022 Build Tools, workload **Desktop development with C++** (MSVC v143 and a Windows 11 SDK) | The linker (`link.exe`) Rust uses on Windows, and the resource compiler (`rc.exe`) that puts the icons and version details in the executables | [Visual Studio downloads](https://visualstudio.microsoft.com/downloads/), "Build Tools for Visual Studio 2022" (rustup offers it when it is missing) |
| Git | The repository and its workflow (`AGENTS.md`) | [git-scm.com](https://git-scm.com) (Git Bash runs `tools/research/all_commands.sh`) |
| Your own Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) cartridge file (revision 1 only, see the README) | Running the programs, the golden fingerprint test and the research tool | Your backup of your cartridge. Never put it, or anything made from it, in the repository |
| Optional: Node.js and the OpenSpec CLI (`npm install -g @fission-ai/openspec`) | Planning changes as specs (`openspec/`) | [nodejs.org](https://nodejs.org) |
| Optional: the Linux target (`rustup target add x86_64-unknown-linux-gnu`) | `cargo xtask check-portable` | rustup |

### Linux

Any distribution where the tools below install; the commands are for Ubuntu 24.04 (Debian and
Mint alike) and Fedora.

| Tool | Why | How to get it |
|---|---|---|
| Rust (stable, through rustup) | The language of every program: Rust 1.100 or later | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh`, then `rustup update` later. The distribution's own `rustc` is usually too old |
| A C toolchain and `pkg-config` | The linker, and the build scripts of the sound and controller crates | `build-essential pkg-config` / `gcc pkgconf-pkg-config` |
| The ALSA and udev headers | Building `cpal` (sound) and `gilrs` (controllers) | `libasound2-dev libudev-dev` / `alsa-lib-devel systemd-devel` |
| Git | The repository and its workflow (`AGENTS.md`) | `git` |
| Docker, with your user in the `docker` group | Building the Linux packages (below, "Building for Linux") | `docker.io` / `moby-engine`; then `sudo usermod -aG docker $USER` and log out and in |
| `dpkg-deb` | Writing the `.deb` | Part of `dpkg` on Debian and Ubuntu; `dpkg` on Fedora |
| Your own Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) cartridge file | As on Windows | As on Windows |
| Optional: Node.js and the OpenSpec CLI (`npm install -g @fission-ai/openspec`) | Planning changes as specs (`openspec/`) | Node.js from the distribution or [nvm](https://github.com/nvm-sh/nvm) |

In one go:

    sudo apt install build-essential pkg-config libasound2-dev libudev-dev git docker.io
    sudo dnf install gcc pkgconf-pkg-config alsa-lib-devel systemd-devel git moby-engine dpkg

### macOS

A recent macOS (the package is built for macOS 11 and later whatever macOS builds it: "Building
for macOS"), on Apple Silicon or Intel.

| Tool | Why | How to get it |
|---|---|---|
| Xcode Command Line Tools | The linker, `lipo`, `codesign` and `ditto` (Xcode itself is not needed) | `xcode-select --install` |
| Rust (stable, through rustup) with both Apple targets | The language of every program: Rust 1.100 or later; both targets for the universal package | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh`, then `rustup target add aarch64-apple-darwin x86_64-apple-darwin`; `rustup update` later |
| Git | The repository and its workflow (`AGENTS.md`) | Comes with the Command Line Tools |
| Your own Super Mario Land 2 - 6 Golden Coins (USA, Europe) (Rev 1) cartridge file | As on Windows | As on Windows (`shasum -a 1 FILE` gives its SHA-1) |
| Optional: Node.js and the OpenSpec CLI (`npm install -g @fission-ai/openspec`) | Planning changes as specs (`openspec/`) | [nodejs.org](https://nodejs.org) |

No system library is needed: sound (CoreAudio), controllers (IOKit), the window (OpenGL) and the
file panels are part of macOS. Do not let Homebrew libraries into the build: the package refuses
executables that link one.

### All

Every other dependency (macroquad, cpal, gilrs, png...) is a Rust crate Cargo downloads by itself.
No Python, CMake or other build system is needed.

For the web game, on any of them: the WebAssembly target (`rustup target add
wasm32-unknown-unknown`), and to try it, any static web server (`python3 -m http.server`).

## Build and run

    cargo build --release
    target\release\sml26gc.exe ["path\to\game.gb"] [--project NAME | --test] [--level N] [--cheats]
    target\release\sml26gc_editor.exe

On Linux the same, with `target/release/sml26gc` and `target/release/sml26gc_editor`. They need
what any player's computer needs (`packaging/linux/README.txt`, "What the computer needs"); built
this way they need the glibc of the computer that built them, which is fine for your own use but
not for giving them to others (use the package, below).

On macOS the same paths as on Linux; built this way they are not app bundles, so they behave as
the unpacked programs of the other systems (below), with Cmd for the editor's shortcuts.

The emulator is too slow unoptimised, so even `cargo build` (debug) is optimised; use `--release`
for playing. `.cargo/config.toml` links the C runtime into every Windows build.

Both programs keep their files next to themselves, in `target/release/`: the base cartridge
choice (`base_cartridge.txt`), `settings.cfg`, `editor.cfg`, the projects (`editor\projects\`),
the saves (`saves\`), `exports\` and `cache\`. `cargo clean` deletes them with the build: copy
them out first.

## Tools

| Command | What it does |
|---|---|
| `cargo test --release --workspace` | Every test, with the golden fingerprint (`regression/`, see its README): needs the cartridge (`SML2_ROM`, or where the game finds it) |
| `$env:GOLDEN_UPDATE=1; cargo test --release -p regression --test golden` (Linux: `GOLDEN_UPDATE=1 cargo test ...`) | Rewrites the fingerprint, only for an intended behaviour change |
| `target\release\research.exe COMMAND ...` (Linux: `target/release/research`) | The reverse-engineering tool (`tools/research`): disassembly, memory watch and trace, ROM views, checks that play edited data. The cartridge comes from the command line, `RESEARCH_ROM`, the base cartridge, a `rom` folder or `%USERPROFILE%\Games` |
| `sh tools/research/all_commands.sh EXE DIR` | Runs a fixed set of research commands into `DIR`, to compare two builds file by file |
| `sml26gc.exe --research menushots DIR` | The pause menu's screens in every language, as pictures |
| `sml26gc.exe --self-test out.json` | Checks the game's rules read from the cartridge (`MECHANICS.md`) against the running game |
| `sml26gc.exe --dump DIR --script ... --frames N --every N` | Screens of a scripted run |
| `sml26gc.exe --screenshot FILE [--start-screen]`, `sml26gc_editor.exe --screenshot FILE [--project NAME --level N]` | One picture of the window, with default settings (unattended checks) |
| `cargo xtask targets`, `cargo xtask package TARGET`, `cargo xtask check-portable` | The release tool, below ("Building for Windows", "Building for Linux", "Building for macOS", "Building for the web") |
| `cargo xtask release-notes vX.Y.Z` | Prints the changelog's section of that version, the text of its GitHub release; refuses a tag that is not the workspace's version (below, "Workflow and releases") |

Never run `cargo fmt`: the code is formatted by hand (`AGENTS.md`).

## Working with OpenSpec after cloning

Changes are planned as specs with OpenSpec (optional, see `AGENTS.md`, "OpenSpec"). A clone holds
the specs (`openspec/specs/`) and the settings (`openspec/config.yaml`); the tool and the
commands for your coding assistant are set up on each computer:

1. Install the tool (it needs Node.js): `npm install -g @fission-ai/openspec`. From then on
   `openspec list --specs`, `openspec show <spec>` and `openspec validate --specs --strict` work
   on the specs, and `openspec new change <name>` / `openspec archive <name>` plan and close a
   change.
2. Give your assistant its OpenSpec commands, if you use one: in the repository's folder,
   `openspec init --tools <assistant>` (for example `claude`, `cursor`, `github-copilot`,
   `codex`; `openspec init --help` lists them all). It keeps the existing `openspec/` folder.
3. Commit none of what this leaves on your computer: the changes being planned
   (`openspec/changes/`) are ignored by git, and the files written for your assistant are your
   own. After `openspec init`, check `git status` and leave out anything it added or changed.

Archiving a change updates `openspec/specs/`: that, with the code, is what a feature branch
commits.

## Building for Windows

The package another person unpacks and runs, on Windows with an x86-64 processor, or with an
ARM one for `windows-arm64` (each package is built on its own processor):

    cargo xtask package windows-x64
    cargo xtask package windows-arm64

1. Builds the game and the editor with the `dist` profile (link-time optimisation, no debug
   symbols) for `x86_64-pc-windows-msvc` (or `aarch64-pc-windows-msvc`), in
   `target\<triple>\dist\` (Cargo's build folder, named after the profile; never run the
   programs from it).
2. Checks that neither executable needs the Visual C++ runtime DLLs (`VCRUNTIME140.dll`,
   `api-ms-win-crt-*`): `.cargo/config.toml` links the C runtime in (`+crt-static`).
3. Writes `dist\windows-x64\sml26gc\`, emptied first: `sml26gc.exe`, `sml26gc_editor.exe`,
   `README.txt` (`packaging/windows/`) and `NO-CARTRIDGE.txt` (`packaging/`), with the product
   and program names filled in, `LICENSE.txt` (the GPL), `LEGAL.md`, `CHANGELOG.md`, `manuals\` (`docs/GAME.md`,
   `docs/EDITOR.md`) and `licenses\` (the fonts' and those of every Rust crate compiled in, from
   `cargo metadata`).
4. Zips that folder to `dist\sml26gc-<version>-<target>.zip`.

The packages run on Windows 10 and 11: nothing in the code needs Windows 11, and Rust's standard
library supports Windows 10 and later.

What goes into the executables:

- The icons: `assets/icons/sml26gc.png` and `sml26gc_editor.png` (see its README), made into
  the `.ico` and the window icon by `crates/app-resources` at build time. A missing PNG is a
  build warning; a missing `rc.exe` (no Windows SDK) leaves the executables without icon and
  version details, with a warning.
- The version details: the product name (`PRODUCT_NAME` in `.cargo/config.toml`), the version
  (the workspace's `version` in `Cargo.toml`) and the not-affiliated line.
- The fonts and every translation catalogue (nothing is read beside the executable but the
  person's own files).
- The window subsystem: no console window; `host::console::attach()` gives the command-line
  modes the terminal they were started from.

A package is published only as part of a release (below, "Workflow and releases"). What a
computer needs to run it, and what to do when it does not start, is in
[`packaging/windows/README.txt`](../packaging/windows/README.txt).

Build errors and their cause:

| Error | Cause and fix |
|---|---|
| `linker 'link.exe' not found` | The Build Tools are missing: install the C++ workload |
| `warning: ... no icon or version details` | No Windows SDK (`rc.exe`): add a Windows 11 SDK in the Visual Studio Installer |
| `... needs VCRUNTIME140.dll` from the xtask | `.cargo/config.toml` was not applied (built from another folder, or `RUSTFLAGS` set in the environment, which overrides it) |
| `error: failed to remove file ... .exe` | The program is open: close it and build again |

## Building for Linux

The packages another person unpacks or installs, on Linux with Docker, on an x86-64 computer
or, for `linux-arm64`, a 64-bit ARM one (each is built natively on its own processor):

    cargo xtask package linux-x64
    cargo xtask package linux-arm64

1. Builds the image `sml26gc-build-linux:<rust-version>-<arch>` from `packaging/linux/Dockerfile`:
   Ubuntu 18.04 for the computer's architecture, with the C toolchain, the ALSA and udev headers and Rust at the workspace's
   `rust-version`, installed by rustup. Docker keeps it, so later runs reuse it at once; raising
   `rust-version` makes a new one.
2. Builds the game and the editor with the `dist` profile inside that image, as your own user
   (every file it writes is yours), with the repository mounted and the build folder
   `target/linux-container/` (apart from your own `target/release/`). The downloaded crates stay
   in the Docker volume `sml26gc-cargo` between runs.
3. Checks each executable needs only the libraries `packaging/linux/README.txt` names (glibc,
   ALSA, udev) and no glibc newer than 2.27, and prints the version it needs.
4. Writes `dist/<target>/sml26gc/`, as for Windows but with `README.txt` from
   `packaging/linux/`, then two packages:
   - `dist/sml26gc-<version>-<target>.tar.gz`: that folder, the programs executable.
   - `dist/sml26gc_<version>_<arch>.deb` (`amd64` or `arm64`), made with `dpkg-deb` from `target/deb/`: the programs in
     `/usr/lib/sml26gc/` with the file `installed` that sends their data to
     `~/.local/share/sml26gc` (`host::paths::data_dir`), the commands in `/usr/bin`, the launchers
     (`packaging/linux/*.desktop`) and 256x256 icons scaled from `assets/icons/`, the documents in
     `/usr/share/doc/sml26gc/` (with `packaging/linux/copyright`), and its dependencies.
     `PRODUCT_MAINTAINER` in `.cargo/config.toml` is its maintainer.

Why a container: an executable linked on a computer needs at least that computer's glibc (built
on Ubuntu 24.04 it would need 2.39 and not start on Ubuntu 22.04). Ubuntu 18.04's glibc 2.27 lets
one `.tar.gz` and one `.deb` run on Ubuntu 18.04, Debian 10, RHEL 8 and every later
distribution, the systems in use when macOS 11 (the macOS package's minimum) came out. To change
the base, edit the `FROM` line of the Dockerfile and `LINUX_GLIBC` in `xtask/src/main.rs`,
together, and the versions in `packaging/linux/README.txt`. Ubuntu 18.04's packages stay on
`archive.ubuntu.com` and `ports.ubuntu.com` while its extended support lasts (2028); after that,
the Dockerfile's sources must point at `old-releases.ubuntu.com`.

To free the space: `docker image rm sml26gc-build-linux:<rust-version>-<arch>`, `docker volume rm
sml26gc-cargo`, and `target/linux-container/` (it goes with `cargo clean`).

Build errors and their cause:

| Error | Cause and fix |
|---|---|
| `... Docker does not answer` | Docker is not installed or your user cannot use it: install it, `sudo usermod -aG docker $USER`, log out and in |
| `permission denied while trying to connect to the Docker daemon socket` | The same: your user is not in the `docker` group yet (log out and in after adding it) |
| `the linux-arm64 package is built on linux on aarch64` | The command was run on another system or processor: run it on that one (or let the release run on GitHub build it) |
| `... needs libXYZ.so, which the Linux read-me does not name` | A dependency now links another system library: add it to the read-me, the `.deb`'s `Depends` and the allowed list in `xtask/src/main.rs`, or avoid it |
| `... needs glibc 2.xx, newer than the 2.27 ...` | The executables were not built in the container (or the Dockerfile's base changed): run the command again |
| `can't find crate for ...` after the Dockerfile's base changed | `target/linux-container/` holds builds from the old image: delete it and run again |
| `dpkg-deb is needed` | Install `dpkg` |
| `Temporary failure resolving 'archive.ubuntu.com'` while building the image | No network: check the connection |

A package is published only as part of a release (below). What a computer needs to run it is in
[`packaging/linux/README.txt`](../packaging/linux/README.txt).

## Building for macOS

The package another person unpacks and opens, on a Mac:

    cargo xtask package macos-universal

1. Builds the game and the editor with the `dist` profile twice, for `aarch64-apple-darwin`
   (Apple Silicon) and `x86_64-apple-darwin` (Intel), with `MACOSX_DEPLOYMENT_TARGET=11.0`: the
   compiler and the linker then refuse, or link weakly, anything newer than macOS 11.
2. Joins each program's two builds into one universal executable with `lipo`, in
   `target/universal-apple-darwin/dist/`.
3. Checks each executable: both architectures are there, each one's minimum macOS
   (`LC_BUILD_VERSION`'s `minos`, what `vtool -show-build` prints) is at most 11.0, and it links
   only `/usr/lib/` and `/System/Library/Frameworks/` (what `otool -L` lists). It prints the
   minimum found, which the read-me states.
4. Writes `dist/macos-universal/sml26gc/`, as for Windows but with `README.txt` from
   `packaging/macos/`, and the executables in two app bundles: `SML2 6GC.app` and
   `SML2 6GC Editor.app`, each with `Contents/MacOS/<executable>`, `Info.plist` (identifier from
   `PRODUCT_BUNDLE_ID` in `.cargo/config.toml`, the version, `LSMinimumSystemVersion` 11.0) and
   `Contents/Resources/<executable>.icns`, made from `assets/icons/`. Each bundle is signed ad hoc
   (`codesign -s -`): Apple Silicon runs no unsigned code.
5. Zips that folder with `ditto` (which keeps the bundles' modes and signatures) to
   `dist/sml26gc-<version>-macos-universal.zip`.

The oldest macOS is 11 Big Sur: every Apple Silicon Mac and the Intel Macs from about 2013-2014.
It is in two places, changed together: the `macos` limit of `macos-universal` in
`xtask/src/main.rs` (it sets the deployment target, the check and `LSMinimumSystemVersion`), and
the versions and Macs named in `packaging/macos/README.txt`. A program built on a newer macOS can
still call a function only that macOS has (through a dependency), linked weakly: the check cannot
see it, so try each release's package on the oldest macOS you can (a Mac, or a macOS 12 virtual
machine in UTM or Parallels on Apple Silicon).

The apps are signed ad hoc, not with an Apple developer identity, and not notarised (that needs a
paid Apple developer account): macOS refuses them the first time, and the read-me says how to
open them.

Build errors and their cause:

| Error | Cause and fix |
|---|---|
| `the macos-universal package is built on macos` | The command was run on Windows or Linux: run it on a Mac |
| `error[E0463]: can't find crate for 'core'` ... `x86_64-apple-darwin` | A target is missing: `rustup target add aarch64-apple-darwin x86_64-apple-darwin` |
| `xcrun: error: invalid active developer path`, `lipo` or `codesign` not found | The Command Line Tools are missing: `xcode-select --install` |
| `... needs macOS 1x.0, newer than the 11.0 ...` | Built without the deployment target (`MACOSX_DEPLOYMENT_TARGET` set to something else in the environment), or a dependency needs a newer macOS: unset it, or avoid that dependency |
| `... needs /opt/homebrew/...` (or `/usr/local/...`) | A dependency linked a Homebrew library: build without it on the `PATH`/`PKG_CONFIG_PATH`, or avoid that dependency |
| `... has no X86_64 code` | `lipo` joined builds of one architecture only: run the command again |

A package is published only as part of a release (below). What a Mac needs to run it is in
[`packaging/macos/README.txt`](../packaging/macos/README.txt).

## Building for the web

The game as a web page, built on any of the three systems:

    cargo xtask package web

1. Builds the game alone with the `dist` profile for `wasm32-unknown-unknown`, without the
   bundled fonts (`fonts`'s `bundled` feature off): `target/wasm32-unknown-unknown/dist/sml26gc.wasm`.
2. Writes `dist/web/sml26gc/`: the `.wasm`; from `packaging/web/` the page (`index.html`),
   miniquad's loader (`gl.js`), the page's script (`sml26gc.js`) and `manifest.webmanifest`; the
   CJK fonts in `fonts/` (downloaded by the page when their language is chosen); the icons the
   manifest and `index.html` name, made from `assets/icons/sml26gc.png`; and the texts and
   licences as for the other packages, with only the game's manual.
3. Zips that folder to `dist/sml26gc-<version>-web.zip`.

To try it, serve the folder and open it in a browser (it does not run from `file://`):

    cd dist/web/sml26gc && python3 -m http.server

then `http://localhost:8000/`; from a phone on the same network, the computer's address and
port 8000 (fullscreen and the storage need HTTPS or localhost, so a phone may need the package on
an HTTPS host). The browser's developer tools show the page's messages and its storage
(IndexedDB, `sml26gc`). The program's half of the page is `crates/host/src/web.rs` and
`game/src/adapters/platform.rs`; `gl.js` comes from the `miniquad` crate of `Cargo.lock` and is
replaced when it is updated.

| Error | Cause and fix |
|---|---|
| `the wasm32-unknown-unknown target is not installed` | `rustup target add wasm32-unknown-unknown` |
| The page stays black, the console says an import is missing | `gl.js` and `sml26gc.js` do not match the program: copy `gl.js` from the `miniquad` crate in use |

## Workflow and releases

The rules are in [`AGENTS.md`](../AGENTS.md), "Git workflow"; this is why they are so.

- **Two long-lived branches.** `main` only ever holds released versions, so whatever is on it
  can be built and given to someone; `develop` gathers finished features until the next release.
- **One branch per change, from `develop`, even for an urgent fix.** Every change goes through
  the same steps (its spec, its tests, the changelog), so a hurried fix cannot skip them.
  A release then carries everything already on `develop`, so its number is chosen by all it
  holds, not only by the fix.
- **Archive, changelog and tests on the branch.** The branch arrives at `develop` complete: the
  specs say what the code now does, the changelog says what users will notice, the tests pass.
  Writing the changelog when each feature ends is easier and truer than rebuilding it from
  memory at release time, and its "Unreleased" section tells which number the release needs.
- **`--no-ff`, then delete the branch.** The merge commit keeps the feature visible as one unit
  in the history, so the branch itself is no longer needed; old branches only hide the ones that
  matter.
- **The version changes once, at release time.** It is what the executables, the zip and the tag
  show, so it changes in the commit that prepares the release and nowhere else. It is
  `MAJOR.MINOR.PATCH`: PATCH when a release only fixes, MINOR when it adds features, MAJOR when
  projects or saves of an earlier version no longer work (while it is `0.x`, the programs are
  still maturing).
- **The tag is the version and points at the merge on `main`.** A published zip can always be
  traced to the exact code it was built from. A release may have a name too: it is its title on
  GitHub, never its tag.

Publishing release X.Y.Z, once the user has asked for it and agreed the number:

1. On `develop`: set `version = "X.Y.Z"` in the workspace `Cargo.toml`, build (it updates
   `Cargo.lock`), turn `## [Unreleased]` in `CHANGELOG.md` into `## [X.Y.Z] - YYYY-MM-DD` with a
   new empty "Unreleased" above it, run the tests, commit "Release X.Y.Z", push.
2. `git checkout main`, `git merge --no-ff develop`, `git tag -a vX.Y.Z -m "Version X.Y.Z"`,
   `git push origin main vX.Y.Z`.
3. Pushing the tag starts the release run on GitHub (below, "GitHub Actions"): it builds every
   package on its own system and makes a **draft** release of the tag with them and the
   changelog's section as its text. Check the draft (every file there, the text right; when you
   can, a package started on its system) and choose Publish release.
4. Publishing it puts its web package on <https://ogueta93.github.io/sml26gc/>.

When GitHub cannot build a package, it can be built by hand with the same command on its own
system and processor (`cargo xtask package <target>`, sections above), and attached to the draft
(`gh release upload vX.Y.Z <file>`); `cargo xtask release-notes vX.Y.Z` prints the text.

## GitHub Actions

Three workflows in `.github/workflows/`, on GitHub's standard runners (free for a public
repository), with no secret but each run's own token and never the cartridge:

| Workflow | When | What |
|---|---|---|
| `checks.yml` | Every push to `develop`, every pull request | `cargo test --release --workspace` with warnings as errors, on Windows 11 (`windows-latest`), a Windows 10 base (`windows-2022`), Linux and macOS; the tests that need the cartridge skip |
| `release.yml` | A pushed tag `v*` | Checks the tag is the workspace's version and takes the changelog's section (`cargo xtask release-notes`), builds the six packages each on its own runner (`windows-latest`, `windows-11-arm`, `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-latest`), then makes the draft release; any failure leaves no release |
| `pages.yml` | A published release, or by hand (Actions, "Run workflow") | Puts the web package of the newest published release on the project's GitHub Pages site |

A failed run is shown on the commit, the pull request or the tag (a red cross) and in the
Actions tab: open the run, then the failed job, for the error. To try a release run again after
fixing its cause, delete the tag on GitHub and locally (`git push origin :vX.Y.Z`, `git tag -d
vX.Y.Z`), and tag and push again. The Pages site needs the repository's Settings, Pages, Source
set to "GitHub Actions" (once).

## How the emulator was made, and why

The game is not rewritten: the original cartridge's program runs, unchanged, on a Game Boy
written in Rust (`crates/gb-core`): the SM83 processor, the memory map and the cartridge's bank
controller, the picture processor drawing tiles and sprites line by line, the four sound
channels, the timer and the buttons. Every rule, speed, level and sound is then exactly the
game's own, with nothing to copy into the project: the person's cartridge file is the game.

What the port adds lives around the emulated machine, never inside the game's program unless
needed: the window, scaling and palettes, sound output, keyboard and controller with any
bindings, the pause menu in eleven languages, settings, the start screen, save states and rewind
(snapshots of the whole machine), fast forward, and the cheats, which change the running game's
memory or apply small patches to the copy in memory (`cart-model`'s `cheats`).

To know what to change, the cartridge's program was read with the research tool (an SM83
disassembler, write-watches naming the instruction that wrote a value, traces), and what was
learned is written down: `MECHANICS.md` (the rules) and `cart-model` (where levels, objects,
maps, routes and doors are, with ROM addresses). The golden fingerprint (`regression/`) holds the
emulator and that knowledge to their recorded behaviour, so any change that alters them shows.

## How the editor was made, and why

The editor changes the game's own data, in the game's own formats, and the game's own program
plays it: a level is the compressed block map, the object list and the header the cartridge
already uses; the world map is its picture, its points, the walk scripts between them and the
doors. Edits are never drawn by the port: they are written into a copy of the cartridge, which
any faithful Game Boy emulator could run. This keeps the original behaviour (physics, enemies,
music) for every edited level, and lets the editor show each level exactly as the game draws it.

- **Reading**: `cart-model` decodes the levels, objects and maps from the cartridge; the pictures
  of blocks and objects are captured from the running game (the editor runs the emulator in the
  background), so they are the game's own graphics in their real colours.
- **Room**: the original 512 KB cartridge is full, so the edited copy is 1 MB: twin banks give
  levels and objects more room, and new levels (32 and up) and new zones live in the added banks.
- **New behaviour**: what the game cannot do by itself (checkpoints with several bells, secret
  exits that lead anywhere, more map points, new zones, a different starting point) is added by
  small patches ("hooks") in the copy's code, each documented in `cart-model` with its address.
- **Projects**: what the person edits is kept as a project file (text, over the base
  cartridge), and the copy is made from it whenever it is missing or older; the base cartridge is
  never written. The game lists the projects on its start screen; the editor starts test games
  of the level being edited.
- **Checks**: the editor warns about what would not work in the game (too many objects, objects
  where the game never wakes them, missing goals), and the research tool's checks play edited data
  in the emulator to prove routes, doors and new zones work.

The code follows a hexagonal architecture: the rules (`domain`) and use cases (`application`)
know no files, windows or sound; adapters do (`ARCHITECTURE.md`).
