# Resource packs

VoxelSeek ships three open-licensed texture packs. The game builds one
16×16 atlas per pack at compile time (`build.rs`) and picks the pack at
runtime from `voxelseek.toml` (or the launcher).

| Pack id | Look | License |
|---|---|---|
| `programmer-art` | classic (default) | CC BY 4.0 — [deathcap/ProgrammerArt](https://github.com/deathcap/ProgrammerArt) |
| `minetest-classic` | Luanti/Minetest style | CC BY-SA 4.0 — [minetest_game](https://github.com/luanti-org/minetest_game) |
| `kenney-prototype` | flat prototype colors | CC0 1.0 — [Kenney](https://kenney.nl/assets/prototype-textures) |

Each pack directory contains `pack.toml` (metadata) and `LICENSE.txt`.

## Selecting a pack

In `voxelseek.toml` (or the launcher's Game Settings tab):

```toml
texture_pack = "minetest-classic"
```

Tiles missing from a pack fall back to `programmer-art`, so partial packs
are valid.

## Making your own pack

```
assets/packs/my-pack/
├── pack.toml
├── LICENSE.txt          # the license of YOUR textures
└── blocks/
    ├── stone.png        # 16x16 PNG
    ├── dirt.png
    └── …                # any subset; missing tiles fall back
```

`pack.toml`:

```toml
name = "My Pack"
id = "my-pack"
version = "1.0.0"
license = "CC0-1.0"       # SPDX id
author = "you"
source = "https://…"
attribution = "…"
```

Tile filenames must match the names in `build.rs` (`TILE_SOURCES`), e.g.
`grass_top.png`, `grass_side.png`, `oak_log_side.png`, `torch_on.png`.
Animated tiles (water) are vertical spritesheets; only the first 16×16
frame is used.

Grass/leaves/water tiles are stored grayscale and tinted at build time —
keep them grayscale for correct biome colors.

Rebuild the game (`cargo build`) and the new pack appears in the launcher.

## License rules

- Only commit packs with a license that allows redistribution
  (CC0, CC BY, CC BY-SA, or your own work).
- **Never** commit textures extracted from Minecraft or derived from
  Mojang's art — they are copyrighted and may not be redistributed.
- CC BY-SA packs (like `minetest-classic`) require derivative atlases to
  keep the same license — keep `LICENSE.txt` next to the textures.
