# VoxelSeek

An **open-source, Minecraft-inspired voxel survival game** written in Rust
with the [Bevy](https://bevyengine.org) engine — featuring an embedded
**DeepSeek harness** (an AI assistant that can chat *and* act in your world),
a sandboxed **mod interface**, a graphical **launcher**, and
**open-licensed resource packs**.

![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![Bevy](https://img.shields.io/badge/Bevy_0.18-232326?style=flat&logo=bevy&logoColor=white)
![License](https://img.shields.io/badge/License-MIT-green)

> **NOT AN OFFICIAL MINECRAFT PRODUCT. NOT APPROVED BY OR ASSOCIATED WITH
> MOJANG OR MICROSOFT.** "Minecraft" is a trademark of Mojang Synergies AB.
> VoxelSeek contains no Minecraft code or assets — all textures are
> clean-room or openly licensed (see [ATTRIBUTION.md](ATTRIBUTION.md)).

VoxelSeek is a fork of [MineClaude](https://github.com/snek-git/mineclaude)
(MIT) — a voxel game built by an agent team — extended with the DeepSeek
harness, the mod system, the launcher, and the resource-pack system.

## Features

**Game** (from upstream MineClaude)

- Infinite procedurally generated terrain with biomes, caves, ores, trees
- Mining, building, crafting (40+ recipes), tools, armor, farming, hunger
- Hostile and passive mobs, day/night cycle, beds, chests, furnaces, doors
- Greedy-meshed chunk rendering with a texture atlas shader

**DeepSeek harness** (new)

- Press **T** in-game to open the console and talk to DeepSeek
- The assistant can *act* through a small, auditable tool set:
  `place_block`, `break_block`, `fill`, `give_item`, `teleport`,
  `set_time_of_day`, `get_surroundings`, `list_inventory`, …
- Function-calling loop with up to 8 chained tool rounds per message
- Runs against the official DeepSeek API with **your own** API key
  (env vars `DEEPSEEK_API_KEY`, optional `DEEPSEEK_BASE_URL`, `DEEPSEEK_MODEL`)
- Details: [docs/harness.md](docs/harness.md)

**Mod interface** (new)

- Sandboxed [Rhai](https://rhai.rs) scripts in `mods/<id>/main.rhai`
- Hooks: `on_load`, `on_tick`, `on_block_placed`, `on_block_broken`, `on_command`
- API: `set_block`, `fill`, `give`, `teleport`, `set_time`, `get_block`,
  `player_pos`, `chat`, `log`
- Console commands: type `/yourcommand args` in the T console
- A bundled example mod builds a castle around you with `/castle`
- Details: [docs/mods.md](docs/mods.md)

**Launcher** (new)

- Native GUI (`voxelseek-launcher`): edit settings, pick a resource pack,
  manage mods, store the DeepSeek key, build & launch the game
- Details: [docs/launcher.md](docs/launcher.md)

**Resource packs** (new)

- Three open-licensed packs included, switchable at runtime:
  | Pack | License |
  |---|---|
  | `programmer-art` (default) | CC BY 4.0 |
  | `minetest-classic` | CC BY-SA 4.0 |
  | `kenney-prototype` | CC0 1.0 |
- Add your own pack in `assets/packs/<id>/` — see [docs/resource-packs.md](docs/resource-packs.md)

## Quick start

```bash
# 1. Build the game and the launcher
cargo build --release

# 2. (optional) Enable the DeepSeek assistant
#    Windows (PowerShell):  $env:DEEPSEEK_API_KEY = "sk-..."
#    Linux/macOS:           export DEEPSEEK_API_KEY=sk-...

# 3. Play
cargo run --release --package voxelseek            # the game
cargo run --release --package voxelseek-launcher   # or via the launcher
```

Requires a stable Rust toolchain and a GPU with Vulkan/Metal/DX12 support.

## Controls

| Key | Action |
|-----|--------|
| WASD / Space / Ctrl / Shift | Move / jump / sprint / sneak |
| Left / Right click | Break / place & interact |
| E | Inventory |
| 1–9, scroll | Hotbar |
| **T** | **DeepSeek console** |
| Esc | Pause / close console |
| F3 | Debug overlay |

## Repository layout

```
src/            # game code (fork of MineClaude, MIT)
src/harness/    # DeepSeek harness: console UI, API client, tool set
src/mods/       # Rhai mod runtime + events + sandboxed world API
launcher/       # voxelseek-launcher (egui)
mods/           # mods live here (example-hello bundled)
assets/packs/   # open-licensed resource packs (each with LICENSE.txt)
docs/           # harness / mods / launcher / resource-pack guides
research/       # design notes written while porting the engine — not a spec
```

## Legal

- Code: **MIT** (see [LICENSE](LICENSE))
- Upstream **MineClaude**: MIT declared in its README, **no LICENSE file in the
  repository** — the situation is recorded verbatim in
  [LICENSE.upstream-mineclaude](LICENSE.upstream-mineclaude). If the upstream
  author objects or later publishes a different license, that license wins.
- Textures & sounds: open licenses, per-pack details in
  [ATTRIBUTION.md](ATTRIBUTION.md)
- Screenshots in `assets/screenshots/` come from the upstream MineClaude
  project (MIT).
- DeepSeek is a third-party service; API use is subject to DeepSeek's terms.
  The assistant can modify your local world — review the tool list in
  [docs/harness.md](docs/harness.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).
