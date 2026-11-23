# editor/languages Specification

## Purpose
The editor's language: English, Spanish or Japanese, taken from the system until the person
chooses one, every text of the editor shown in it.

## Requirements

### Requirement: Three languages
Every text the editor shows (panels, buttons, tooltips, status bar, warnings, project screen,
choosers, menu bar, file dialogs) SHALL be in the language in use: English, Spanish,
Japanese, Portuguese, French, German, Italian, Indonesian, Russian, Simplified Chinese or
Korean. A text a language has no translation for SHALL show in English; the editor's
catalogue SHALL have every text in every language.

#### Scenario: Japanese
- **WHEN** the editor is in Japanese
- **THEN** the level editor's title, its buttons and the File menu read in Japanese

#### Scenario: French
- **WHEN** the editor is in French
- **THEN** the level editor's title, its buttons and the File menu read in French

#### Scenario: A missing translation
- **WHEN** a text has no Japanese translation and the editor is in Japanese
- **THEN** that text shows in English and the rest in Japanese

#### Scenario: A complete catalogue
- **WHEN** the editor's texts are checked
- **THEN** every catalogue entry has its eleven languages

### Requirement: The system's language first
Until a language is chosen, the editor SHALL use the system's when it is one of its eleven
(any country: Portuguese for Brazil or Portugal, Chinese for Simplified Chinese), English
otherwise. The editor SHALL NOT follow the game's language setting.

#### Scenario: A Spanish Windows
- **WHEN** the editor starts for the first time on a Windows set to Spanish (Mexico)
- **THEN** it is in Spanish

#### Scenario: A Korean Windows
- **WHEN** the editor starts for the first time on a Windows set to Korean
- **THEN** it is in Korean

#### Scenario: Another language
- **WHEN** the editor starts for the first time on a Windows set to Arabic
- **THEN** it is in English

#### Scenario: The game in another language
- **WHEN** the game's menu is set to Japanese and the editor has no language chosen on an English Windows
- **THEN** the editor is in English

### Requirement: Choosing the language
The language chosen in the Language menu SHALL apply at once, to the menu bar too, without
losing anything being edited, and SHALL be kept in `editor.cfg` for the next start.
Automatic SHALL go back to the system's language.

#### Scenario: Changed while editing
- **WHEN** the language is changed with unsaved changes in a level
- **THEN** the texts change at once and the changes are still there, unsaved

#### Scenario: Kept
- **WHEN** Japanese is chosen and the editor is closed and opened again
- **THEN** it opens in Japanese

### Requirement: Japanese is readable
In Japanese, Russian, Simplified Chinese and Korean every character the editor's texts use
SHALL be drawn (no empty boxes or question marks), at the sizes the editor uses, with a font
that has that script, and the texts SHALL fit their buttons as the English ones do (made
smaller when they do not, as for any label). The Latin languages SHALL be written without
accents, as Spanish is, and drawn with the editor's own font.

#### Scenario: Every character
- **WHEN** the editor is in Japanese
- **THEN** every character of every Japanese text has its glyph in the editor's Japanese font

#### Scenario: Korean and Chinese
- **WHEN** the editor's texts are checked
- **THEN** every character of every Korean, Chinese and Russian text has its glyph in the font that language is drawn with

#### Scenario: Only when needed
- **WHEN** the editor is in English, Spanish or another Latin language
- **THEN** the Chinese and Korean fonts are not loaded

### Requirement: The editor's texts in editable catalogues
Every translated text the editor shows, the game zones' names and the "saved ... ago" texts included, SHALL be in its plain-text catalogues (`editor/i18n/editor.txt`, and `crates/host/i18n/host.txt` for the messages it shares with the game); the code SHALL hold only the English.

#### Scenario: A zone's name
- **WHEN** the Korean line of the entry "Tree Zone" is changed in `editor/i18n/editor.txt`
- **THEN** the editor in Korean shows the new name, and no Rust file was edited

### Requirement: Texts stay readable

The editor's texts SHALL stay readable for as long as it runs, in every language, whatever
new letters or sizes it comes to draw.

#### Scenario: New letters after the menu bar has been drawn

- **WHEN** the editor draws letters or sizes it had not drawn before, after its menu bar and views have been drawn with text
- **THEN** every text keeps drawing as letters, in that frame and every later one, without quitting the editor
