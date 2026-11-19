# Launcher

`voxelseek-launcher` is a small native (egui) launcher for the game.

## Run it

```bash
cargo run --release --package voxelseek-launcher
```

## What it does

- **DeepSeek settings** — API key (stored locally in `launcher.toml`,
  password-masked), base URL, model. Passed to the game as environment
  variables at launch; never sent anywhere except DeepSeek's API by the game.
- **Game settings** — resource pack, window size, render distance, FOV.
  Written to `voxelseek.toml` in the game directory (read by the game at
  startup, see `src/settings.rs`).
- **Mods** — lists `mods/<id>/` folders with enable/disable checkboxes
  (writes `disabled_mods` to `voxelseek.toml`) and a rescan button.
- **Build** — runs `cargo build --release --package voxelseek` in the
  background and reports the result.
- **Launch / Stop** — starts the game with the configured environment,
  tracks the process, and can kill it.

## Files

| File | Location | Content |
|---|---|---|
| `launcher.toml` | launcher's working directory | launcher state incl. API key |
| `voxelseek.toml` | game directory | game settings incl. disabled mods |

Both are git-ignored (`voxelseek.toml` may contain a path to your key —
do not commit it).
