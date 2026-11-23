# Mechanics sheet

The game's rules as its own program has them, read from the cartridge's machine code with the
research tool (`tools/research`: an SM83 disassembler, a memory write-watch naming the
instruction that wrote each value, and per-frame traces). The game's `--self-test` reads these
values from the cartridge and checks that the running game reaches each one. All addresses are
bank 0 (`0:xxxx`), which is always mapped.

## Player state (in the cartridge's battery RAM, which this game also uses as work RAM)

| Memory | Meaning | Found by |
|---|---|---|
| $FFC2/$FFC3 | player X in the level (16-bit) | written at 0:1864 by the horizontal mover |
| $FFC0/$FFC1 | player Y in the level (16-bit, down = larger) | written at 0:158E by the vertical mover |
| $A200 | horizontal speed index; $80 = standing, above = right, below = left | write-watch: 0:16EE, 0:172B |
| $A201 | jump/fall table index; counts down one per frame | write-watch: 0:153A, 0:1683 |
| $A214 | $FF on the ground, $00 in the air | trace |
| $FF80 | buttons this frame: bit0 A, bit1 B, bit4 Right, bit5 Left, bit6 Up, bit7 Down | code at 0:171B, 0:1528 |
| $FF97 | frame counter | 0:1849 |

## Walking and running (0:16CC friction, 0:1701 input, 0:1826 mover)

| Rule | Value | Source |
|---|---|---|
| Speed index while a direction is held | +8 / −8 per frame | 0:1729 `add a,$08`, 0:177F `sub $08` |
| Friction on the ground (always, after input) | 4 per frame toward $80 | 0:16EC `sub $04`, 0:16F5 `add $04` |
| Net acceleration | 4 index steps per frame → walk top speed in 15 frames | derived; self-test: $90 after 4 frames |
| Walk cap (no B) | $BF right / $48 left → $BB / $4C after friction | 0:175C, 0:17B1 |
| Run cap (B held) | $F3 right / $08 left → $EF / $0C after friction | 0:176A, 0:17BF |
| In the air | no friction (0:16DD returns when $A214 = 0); cap $B8 when not on the ground or in a special state | 0:16D9, 0:1741 |
| Pixels moved per frame | table at $19FF[speed index + (frame & 7)] (signed); a second table at $1AFF is used in the special physics state ($A26D set) | 0:182C, 0:1842, 0:1845-0:1852 |
| Walk top speed | 1.125 px/frame = 67.2 px/s | $19FF rows $BB-$C2; self-test measured 1.125 |
| Run top speed | 1.5 px/frame = 89.6 px/s | $19FF rows $EF-$F6; self-test measured 1.500 |
| Fastest table entries (reachable only in special states) | 1.5 px/frame | $19FF rows $D8-$FF |

## Jumping and falling (0:1501 vertical mover, 0:1650 jump start)

| Rule | Value | Source |
|---|---|---|
| Vertical movement per frame | Y += table $18AF[index], index −1 each frame first; $18FB when $A26D is set, $1947 when $A287 is set | 0:1533-0:15A0 |
| Jump start index, running or Up held | $4B | 0:1681 |
| Jump start index, speed between $48 and $BF (standing or walking) without Up | $30 | 0:168D-0:16A0 |
| Jump start index in the $A26D state | $48 | 0:16A9 |
| Long-jump shortcut | when the index drops to $30 it jumps straight to the apex $18 | 0:153D-0:1549 |
| Releasing A while rising | index ≥ $21 is cut to $21 (shorter jump) | 0:151B-0:1530 |
| Head hits a ceiling | Y restored, index set to $18 (start falling) | 0:15B6-0:15DF |
| Landing | index ≤ $18 on the ground → $18 | 0:15A2-0:15B1 |
| Fall speed | builds from 0 to 3 px/frame (table $18AF index $17 → $00), then stays at 3 | $18AF rows 0-1; index stops at 0 (0:1537) |
| Standing/walking jump height (A held) | 49 px | table sum $19-$2F; self-test measured 49 |
| Running jump height (A held) | 69 px | table sum $31-$4A; self-test measured 69 |
| Shortest jump (A tapped) | about 11 px | table: first frame 3 px, then $20 → $19 (estimated: tap timing not measured) |
| Rise speed | 3 px/frame at the start of a jump | $18AF index $20-$4A mostly −3 |
| Bounce off an enemy | index $28, or $26 at walking speed | 0:16AF-0:16C8 |

## Levels (read from the code; used by the level editor)

| What | Where / rule | Source |
|---|---|---|
| Level number | $A269 (0-31) | 01:558C |
| Level headers | word table at 01:55CB → 20-byte headers (file $560B + 20·n), copied to $A800 | 01:5589-01:55A3 |
| Map bank / map number | header $0D / low nibble of header $12 | 0:0412, 0:0424, 0:0481 |
| Block definitions | 128 blocks × 4 tiles from bank 8, address in header $0B-$0C, copied to $A600 | 0:03F7-0:0410 |
| Map table | word table at $42F0 of the map bank | 0:0361 |
| Map compression | byte < $80 = one block; $80+v, n = block v written 1+n times (n=0 → 256); runs stop at each 256-block row | 0:0361-0:03AE |
| Map size in memory | 48 rows × 256 columns of 16×16-pixel blocks, $B000-$DFFF | 0:0383, 0:0A6C-0:0A92 |
| Drawing a block | tiles top-left, top-right, bottom-left, bottom-right from $A600 + 4·id | 0:0A92-0:0B22 |
| Game mode byte | $FF9B: $1A file select, $0C map, $16/$0B/$02/$03 loading a level, $04 playing, $08 paused, $09 losing a life, $14 back to the map, $05 game over | write-watch on $FF9B |

Checked: all 32 levels decode exactly as the game itself fills $B000-$DFFF, and re-compressing
gives the same bytes back (`--research levels`).

The cartridge has no free space in its level banks, so the editor's copy is made 1 MB: banks
$20-$3F copy $00-$1F and a 3-place patch (0:037C, 0:03A9, stub at 0:39BD in unused $FF filler)
makes the decoder read map data from bank $20 + the level's bank while the screen is off, then
switch back. Checked: the expanded copy loads all 32 levels exactly like the original, and much
bigger edited levels load exactly as edited without changing the other levels of their zone
(`--research storecheck`).

## Objects in levels (enemies, items; used by the level editor)

| What | Where / rule | Source |
|---|---|---|
| Object list | bank 3, word table at $6037 indexed by level → 3-byte objects, ended by $FF; lists sit back to back at file $E077 (2879 bytes) | 03:6BC7-03:6BDB |
| X | (b & $0F)·256 + (c & $1F)·8 px | 03:6BDD-03:6BE9 |
| Y | ((d >> 5) & 3)·256 + (d & $1F)·8 px; d bit 7 is a flag kept with the object | 03:6BEA-03:6BFD |
| Type (0-127) | b.4→bit 6, b.7→bit 5, b.6→bit 4, b.5→bit 3, c bits 5-7 → bits 0-2 | 03:6BFE-03:6C13 |
| In memory | 6 bytes each from $AB06: X page, X, flag, Y page, Y, type | 03:6BD5 |
| Loader | called once per level at 0:0581 with the screen off; uses only its code (03:6BB6-03:6C60), the table and the lists | read-watch |

Checked (`--research objcheck`): for all 32 levels the game's records match our reading and
re-encoding gives the same bytes; edited lists (moved, removed, added, new types) load exactly.
In the editor's 1 MB copy, 0:0581 calls a stub at 0:39C8 that sets the upper bank bit around the
loader, so it reads from bank $23, whose $4000-$6036 (8247 bytes) holds the lists; checked that all
32 levels keep their objects and that 40 extra objects load as stored.

## World map (used by the world map editor)

| What | Where / rule | Source |
|---|---|---|
| Picture | 64×64 tiles (512×512 px), uncompressed, bank $0D $7000-$7FFF as four 32×32 quadrants (TL, TR, BL, BR) | 0F:6AE5 via 0:3E22 (read-watch: 0D:7000-7FFF) |
| Map tile graphics | bank $0D $4000-$57FF | read-watch at map load |
| Level each point opens | byte table 0F:4218, 130 points indexed by $A840; $FE = no level | 0F:41A6-0F:41B5 → $A269 |
| Point positions / paths | spread through per-point code in bank $0F (0F:438D…) | not editable yet |
| Each map's music | the `nn` of `ld a,nn / ld ($A468),a` in the map's set-up code: Tree 0F:5186 ($10), Pumpkin 0F:5297 ($1E), Macro 0F:5401 ($05), Space 0F:552B ($1C), Turtle 0F:5694 ($12), Mario 0F:5840 ($1B), Wario's Castle 0F:5758 ($0E), bonus game 0F:58D6 ($0D), world map 0F:6A9B and 0F:71FD ($06) | write-watch on $A468 while each map is shown |

Checked (`--research worldcheck`): the 32×32 window the game puts in VRAM on the map matches our
reading of the picture (1016/1024 tiles; the rest are animated), and setting point 0 to level 6
makes Start on that point load level 6.

## Status bar

| Value | Number | Source |
|---|---|---|
| Position | bottom 8 lines: lives, coins, enemies defeated, timer | frame dump; self-test |
| Level timer start (level 1) | 399 | frame dump (estimated as a fixed value for other levels) |

## Hardware timing (what the game runs on)

| Value | Number | Source |
|---|---|---|
| CPU clock | 4,194,304 Hz | DMG specification |
| Cycles per frame | 70,224 (154 lines × 456 dots) → 59.73 Hz | DMG specification |
| Screen | 160 × 144, 4 shades | DMG specification |
| Mode 3 length | 172 dots, fixed | estimated (real length varies) |
| Sprites per line | 10, priority by X then OAM index | DMG specification |
| Timer rates | 4096 / 262144 / 65536 / 16384 Hz | DMG specification |
| Sound frame sequencer | 512 Hz | DMG specification |
| Cartridge | MBC1 + 8 KB battery RAM, 512 KB ROM | ROM header $147=$03, $148=$04, $149=$02 |
| Start-up registers | A=01 F=B0 BC=0013 DE=00D8 HL=014D SP=FFFE PC=0100 | DMG boot ROM hand-over state |

## Controls

| Game Boy | Keyboard |
|---|---|
| D-pad | Arrows / WASD |
| A (jump) | X / K / Space |
| B (run, fireball) | Z / J / Left Shift |
| Start | Enter |
| Select | Backspace / Right Shift / Tab |

## Not emulated

- Link cable, boot logo, exact mode-3 timing.
