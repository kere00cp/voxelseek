# Mod interface

Mods are sandboxed [Rhai](https://rhai.rs) scripts. No compiler needed —
drop a folder into `mods/` and start the game.

## Layout

```
mods/
└── my-mod/
    ├── mod.toml
    └── main.rhai
```

`mod.toml`:

```toml
id = "my-mod"            # required, unique
name = "My Mod"
version = "0.1.0"
author = "you"
description = "what it does"
```

## Hooks

Define any of these in `main.rhai`:

```rhai
fn on_load() { }                        // once at startup
fn on_tick(dt) { }                      // every frame, dt in seconds (float)
fn on_block_placed(x, y, z, block) { }  // ints + snake_case block name
fn on_block_broken(x, y, z, block) { }
fn on_command(cmd, args) { }            // "/cmd a b c" typed in the console
```

## API

| Function | Description |
|---|---|
| `log(msg)` | Write to the game log |
| `chat(msg)` | Show a line in the in-game console overlay |
| `set_block(x, y, z, block) -> bool` | Queue a block placement |
| `fill(x1, y1, z1, x2, y2, z2, block) -> int` | Queue a box fill (≤ 4096 blocks); returns volume, `-1` bad block, `-2` too big |
| `give(item, count) -> int` | Give items (count clamped to 1–64); `-1` on unknown item |
| `teleport(x, y, z)` | Teleport the player |
| `set_time(t)` | Set time of day, `t` wraps into [0, 1) |
| `time_of_day() -> float` | Current time of day |
| `player_pos() -> [float; 3]` | Player position |
| `get_block(x, y, z) -> string` | Block name near the player (±16 blocks), `"unknown"` outside |

World mutations are **queued** and applied on the main thread after your
handler returns — it is fine to call many of them in one handler.

See `mods/example-hello/main.rhai` for a working example (`/castle`).

## Sandbox

- No filesystem, network, FFI or environment access.
- Each handler call is capped at 500k Rhai operations and 32 call levels.
- All world edits go through the same validation as the DeepSeek tools
  (no bedrock placement, no entombing the player, 4096-block fill cap).

## Disabling mods

Use the launcher (Mods tab) or add the id to `disabled_mods` in
`voxelseek.toml`:

```toml
disabled_mods = ["my-mod"]
```

## Security note

Rhai scripts are sandboxed, but treat mods from strangers like any code:
review `main.rhai` before enabling it. Mods cannot break out of the sandbox
by design, but they *can* grief your world (fill/delete blocks) — that's
what mods are for.
