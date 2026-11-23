# release/ci Specification

## Purpose

The checks GitHub runs on every change to the project and the release it builds from a version
tag, on the free runners of a public repository, without the cartridge.

## Requirements

### Requirement: Every change is checked on every desktop system
On every push to `develop` and every pull request, GitHub SHALL build the workspace and run its
tests in release on Windows 11, on a Windows 10 base, on Linux and on macOS, and SHALL mark the
push or pull request as failed when a build has a warning or a test fails. The tests that need
the cartridge SHALL be skipped there, since no cartridge is ever given to GitHub.

#### Scenario: A pull request that breaks macOS
- **WHEN** a pull request changes code that builds on Linux and Windows but not on macOS
- **THEN** the pull request shows the macOS check as failed, with the compiler's error in its log

#### Scenario: A warning
- **WHEN** a push to `develop` adds an unused variable
- **THEN** the checks fail and name the warning

#### Scenario: No cartridge
- **WHEN** the checks run the tests
- **THEN** the tests that need the cartridge (the golden fingerprint included) are skipped, and the others pass or fail as they do on the developer's computer

### Requirement: A version tag builds a draft release
Pushing a tag `vX.Y.Z` SHALL build, each on its own system, the packages of every package target
(`windows-x64`, `windows-arm64`, `linux-x64`, `linux-arm64`, `macos-universal`, `web`) with the
package command, and SHALL create a draft GitHub release of that tag. The draft SHALL hold every
package file and have the changelog's section of that version as its text. It SHALL NOT be
published by GitHub: the developer reviews it and publishes it.

#### Scenario: Tagging 1.3.0
- **WHEN** `v1.3.0` is pushed on the merge into `main` whose workspace version is `1.3.0`
- **THEN** a draft release `v1.3.0` appears on GitHub with the Windows, Linux, macOS and web packages of both architectures where there are two, and the changelog's `1.3.0` section as its text

### Requirement: A failed release run leaves no release
When the tag's version is not the workspace's version, the changelog has no section for it, or
any package fails to build, the release run SHALL create no release and SHALL say why.

#### Scenario: A tag that is not the version
- **WHEN** `v1.4.0` is pushed while the workspace's version is `1.3.0`
- **THEN** no release is created, and the run says the tag and the version differ

#### Scenario: A package that fails
- **WHEN** the macOS package fails to build after a tag is pushed
- **THEN** no release is created, and the run shows the macOS build's error

### Requirement: The latest release plays online
When a release is published on GitHub, its web package SHALL be put on the repository's GitHub
Pages site (`https://ogueta93.github.io/sml26gc/` for this one; a fork's own), replacing what
was there: the game at that address
and the editor at `editor.html`. A draft, or a release that is not the newest, SHALL NOT change
the site. The site SHALL also be put up again from the latest published release when asked by
hand. `README.md` SHALL link to it in a "Try it here" section.

#### Scenario: Publishing 1.3.0
- **WHEN** the developer publishes the draft release `v1.3.0`
- **THEN** a few minutes later `https://ogueta93.github.io/sml26gc/` plays version 1.3.0, and its editor is at `https://ogueta93.github.io/sml26gc/editor.html`

#### Scenario: A draft
- **WHEN** the tag `v1.4.0` has built its draft release and it is not yet published
- **THEN** the site still plays 1.3.0

#### Scenario: No cartridge online
- **WHEN** someone opens the site
- **THEN** the page asks for their own cartridge, as the web package does, and the site holds only the web package's files

### Requirement: A fork runs the same workflows
The workflows SHALL name no account, repository or address of their own, so that a fork runs
them unchanged once its Actions and its Pages setting are enabled, with its own runners and its
own Pages site.

#### Scenario: A fork publishes a release
- **WHEN** someone forks the repository, enables its Actions and Pages, pushes a version tag and publishes the draft
- **THEN** the fork's packages are built on its own runners, its draft release is made in the fork, and its web package goes to `https://<owner>.github.io/<repository>/`

### Requirement: Free and without the cartridge
The checks and the release builds SHALL run only on the standard runners GitHub gives public
repositories at no cost, and SHALL NOT need any secret other than the token GitHub gives each
run. No cartridge, save or edited copy SHALL be stored in the repository's settings or used by
any run.

#### Scenario: Running the release
- **WHEN** the release run builds the packages
- **THEN** it uses only GitHub's standard runners and the run's own token, and no file of the game
