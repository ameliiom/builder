# Builder

For [Horizon](https://playhorizon.dev) (Minecraft: Java Edition 26.3 in Rust). Place a block, then another further on: a line, a wall or a floor of that block fills the space between, as you placed it (stairs keep their facing), paid from your inventory. English and Arabic.

## How it plays
- **Z** steps the mode: off, line, wall, floor (the action bar names it). Off is the game's own placing.
- Place the first block, aim where the last goes: the blocks in between are outlined. Place it and they fill.
- Only into air or grass and the like, never where someone stands, only where you may build. Out of blocks, it stops.
- **U** takes your last shape back into your inventory within 10 seconds.

## Comforts from Skills
With [Skills](https://github.com/ameliiom/skills) 0.2 or later, Builder adds its comforts to Skills' own **Building** tree (two new rows): lines of 8 from the start, then **Bigger Shapes** I/II/III (16, 64, 256 blocks), **Walls And Floors**, **Undo**. Building grows from every block you place by hand, and Builder adds the blocks its shapes place. From 0.3.0 the separate Construction tree is gone: progress earned in it resets, and your Building points pay for these nodes. Without Skills, the server setting **All comforts without Skills** opens them.

## Install
1. Copy `builder.hmod` into `%APPDATA%\horizon\horizon-native-mods\`.
2. Add the line `mod.builder:true` to `%APPDATA%\horizon\horizon-mods.txt`.
3. Build: the launcher's Mods page **Build**, or `horizon-launcher native-build --instance horizon --trust-native-code`. A native mod runs with your rights: install it only from a source you trust.

The server and every player need the mod. Built for Horizon Nightly `f037744` (2026-11-21).

## Licence
MIT, see [LICENSE](LICENSE). Not affiliated with the Horizon project.
