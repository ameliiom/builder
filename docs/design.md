# Builder: design

Place a block, then another further on: a line, a wall or a floor of that block fills the space
between, as you placed it (stairs keep their facing), paid from your inventory. A preview in the
game's own outline shows what will go down before you place the second block.

## 1. Outcome

A wall, a floor or a long row takes two clicks instead of dozens, for the two of you building.

## 2. Prior art (searched 2026-11-25)

| What | Where | Kind | Use |
|---|---|---|---|
| Horizon mods (GitHub `horizon-mod.toml`, 38 repos) | GitHub | no line, wall or floor tool | build ours |
| Effortless Building | Java (NeoForge) | modes (line, wall, floor, cube, mirror), preview, undo, a mode wheel | design only |
| Construction Wand | Java | a wand item that extends a face | not taken: a new item, against vanilla-first |

## 3. How it plays

- **Mode**: press **Z** (rebindable; B opens Horizon's emote wheel, measured) to step Off → Line → Wall → Floor; the action bar names the mode, as
  vanilla shows a message there. Off is the game's own placing.
- **Two placements**: the first block you place marks the start; while you aim for the second,
  the shape is outlined; place the second and the space between fills.
- **As you placed it**: every block copies the first one's state (stairs, logs and slabs keep
  their facing and half).
- **Paid**: from the stack in hand, then the same block from the inventory (Hands refills too);
  short of blocks, the shape stops where they run out.
- **Safe**: only into air or replaceable blocks (grass, snow), never where a player or mob stands,
  and only where you may build (spawn protection, other mods' claims).
- **Undo**: Ctrl+Z within 10 seconds takes the last shape back into your inventory.

## 4. Comforts from Skills (nodes added to Skills' Building tree)

From 0.3.0 the comforts are two rows Builder adds to `skills:building` (`data/builder/skills/building.json`,
`"extends": "skills:building"`), not a tree of their own: Building already means "placing blocks", and
one meaning has one tree (owner's correction, 2026-11-26). Node ids carry the prefix `builder_` and sit
in rows 3 and 4, under Building's own three rows. The only experience source Builder adds is its
statistic `builder:placed` (0.1 a block): `PlayerRef::place_block` fires no place event, so Skills'
`placed` source never sees a shape's blocks. The old Construction tree's progress resets: no file defines
it any more, so its points no longer count. Measured in the game (2026-11-26): the Building tab
showed Builder's five nodes in rows 4 and 5 under its own (`docs/building-tree.png`), no
Construction tab; unlocking Bigger Shapes I gave `builder.size.16`, then Walls And Floors gave
`builder.shapes`, and the points went from 23 to 22 after the first. A test keeps the file an addition: it must extend
`skills:building`, define no title and prefix every node id.

| Always | Earned |
|---|---|
| Line up to 8 | **Builder** I/II/III: shapes up to 16, 64, 256 blocks · **Walls and Floors** (the Wall and Floor modes) · **Undo** |

## 5. Measured (owner's PC, Horizon f037744, a spike, 2026-11-25)

| What | Result |
|---|---|
| One oak stair placed facing north, line of 8 more | 8 placed in **0.27 ms**, all facing north (the first one's state copied) |
| Paid from the hand | 16 stairs → 7 (the one placed by hand and 8 more) |
| `PlayerRef::place_block` | writes the block with neighbour updates (fences connect); it does no checks, no sound, no payment, so the mod does those (air, may-build, entities, sound, items) |

## 6. Built (0.2.0), measured in the game (2026-11-25)

| Case | Result |
|---|---|
| Z with no comfort | off and line only (Wall and Floor need the node) |
| Line of stone bricks, no comfort (8), a cow standing at one block | 8 placed: the cap kept the 8 nearest the first block and the cow's block stayed empty; 8 bricks paid |
| Floor 5×5 with Walls And Floors and Bigger Shapes II | all 25 in place, 25 bricks paid |
| U right after | the 23 the shape placed gone, 23 bricks back in the inventory; the 2 placed by hand stay |
| Preview | the block where the next would go outlined in the game's thin dark outline while the shape waits |
| Stairs (skeleton) | 7 in a row, all facing as the first |

The mode key moved from B to **Z**: B opens Horizon's emote wheel and Y another mod's screen (both
seen in the game); Z, J and U opened nothing. Undo is U.

## 7. Risks and answers

| Risk | Answer |
|---|---|
| Building into a friend's base or spawn | `may_use_item_at` per block (permissions, spawn protection, claims) |
| Blocks lost or duplicated | pay before each block is written; refunds on a refused write; undo returns exactly what was placed |
| Lag on a huge floor | 256 at most, 64 a tick |
| A block that needs support (torch, door) | only full blocks and stairs, slabs, logs, glass, fences, walls; the rest place one at a time |
