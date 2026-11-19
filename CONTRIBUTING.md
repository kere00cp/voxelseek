# Contributing to VoxelSeek

Thanks for your interest! A few ground rules that keep this project safe to
use and redistribute.

## Legal ground rules (important)

1. **No Mojang assets.** Never submit textures, sounds, models, or code
   taken from Minecraft or derived from Mojang's artwork. "Clean-room" or
   openly licensed (CC0 / CC BY / CC BY-SA) assets only, with provenance
   documented in [ATTRIBUTION.md](ATTRIBUTION.md).
2. **No "Minecraft" branding.** Don't use the name or logo to promote the
   project or derivative builds.
3. **Keep licenses intact.** Third-party components keep their own licenses;
   add attribution when required.
4. **New code is MIT.** By submitting a PR you license your contribution
   under the project's MIT license.

## Code guidelines

- Rust stable; `cargo fmt --check` and `cargo clippy` should pass.
- Keep the DeepSeek harness (`src/harness/`) and mod runtime (`src/mods/`)
  decoupled: world mutations go through `harness::tools` so there is a
  single validated execution path.
- Sandbox invariants for mods must hold: no fs/network/FFI for scripts,
  operations caps, main-thread execution of world edits.
- Run the game once before submitting (`cargo run --features bevy/dynamic_linking`).

## Adding resource packs / mods

- Resource packs: see [docs/resource-packs.md](docs/resource-packs.md).
- Example mods are welcome in `mods/` (with `mod.toml`); keep them small
  and well-commented.

## Reporting issues

Include: OS/GPU, `rustc --version`, whether you used a texture pack or mods,
and steps to reproduce. For harness issues, include the tool call log lines
from the console (never your API key!).
