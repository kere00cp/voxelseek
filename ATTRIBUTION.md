# Attribution & Legal Notices

VoxelSeek is built from open components. This file is the canonical record of
what came from where and under which license.

## Code

| Component | Source | License |
|---|---|---|
| VoxelSeek game code (fork) | This repository | **MIT** (see `LICENSE`) |
| Upstream base: **MineClaude** | https://github.com/snek-git/mineclaude | **MIT** — declared in the upstream README and repository metadata. Upstream ships no LICENSE file; the MIT terms are reproduced in `LICENSE` with credit to "MineClaude contributors". |
| DeepSeek harness plugin (`src/harness/`) | This repository (original work) | **MIT** |

## Textures (resource packs in `assets/packs/`)

| Pack | Source | License |
|---|---|---|
| `programmer-art` (default) | [deathcap/ProgrammerArt](https://github.com/deathcap/ProgrammerArt) — clean-room replacement textures in the style of Minecraft's classic "programmer art" | **CC BY 4.0** — credit: *deathcap and contributors* |
| `minetest-classic` | [luanti-org/minetest_game](https://github.com/luanti-org/minetest_game) (`default` and `farming` mods) | **CC BY-SA 4.0** — credit: *Luanti (Minetest) contributors* |
| `kenney-prototype` | [Kenney Prototype Textures](https://kenney.nl/assets/prototype-textures) via [Calinou/kenney-prototype-textures](https://github.com/Calinou/kenney-prototype-textures) | **CC0 1.0** — credit appreciated: *Kenney (kenney.nl)* |

Each pack directory contains its own `pack.toml` and `LICENSE.txt`.
If you build the game with a non-default pack (`VOXEL_PACK=...`) and
redistribute the result, you must comply with that pack's license
(e.g. CC BY-SA 4.0 requires the same license and attribution for the textures).

## Sounds

Sound effects in `assets/sounds/` originate from open-licensed sound packs
collected by the upstream MineClaude project (OpenGameArt CC0 packs and
CC BY 3.0 footsteps — see `research/assets.md` for the original research
notes). Where attribution is required by CC BY 3.0, credit goes to the
respective OpenGameArt authors as listed in that document.

## Trademarks

- **Minecraft** is a trademark of Mojang Synergies AB / Microsoft Corporation.
  VoxelSeek is an independent open-source project. **NOT AN OFFICIAL MINECRAFT
  PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH MOJANG OR MICROSOFT.** No
  Minecraft code, textures, sounds, or other assets are used or distributed
  here; all textures are clean-room or openly licensed replacements.
- **DeepSeek** is a trademark/brand of 深度求索 (Hangzhou DeepSeek Artificial
  Intelligence Basic Technology Research Co., Ltd.). The harness plugin talks
  to the official DeepSeek API with the user's own API key. This project is
  not affiliated with or endorsed by DeepSeek; use of the API is subject to
  DeepSeek's own terms of service.
- **Luanti / Minetest** textures are used under CC BY-SA 4.0 as detailed above.

## Your responsibilities as a user

- Keep `LICENSE` and this file (and the per-pack `LICENSE.txt` files) when
  redistributing the game, source or binary.
- Do not use the name "Minecraft", Mojang or Microsoft branding to promote
  derivative builds.
- Provide your own DeepSeek API key and comply with DeepSeek's API terms.
