# Publishing

## To GitHub (recommended)

1. Create an **empty** repository on GitHub (do not initialize it with a
   README or license), e.g. `voxelseek`.
2. Run the helper script from the repo root:

   ```powershell
   powershell -ExecutionPolicy Bypass -File scripts/publish.ps1 -GitHubUser <your-username>
   ```

   or manually:

   ```bash
   git remote add origin https://github.com/<your-username>/voxelseek.git
   git push -u origin main
   ```

## Release checklist

- [ ] `cargo fmt --all` and `cargo clippy --workspace` pass
- [ ] `cargo test --workspace` passes
- [ ] `cargo build --release` works and the game starts
- [ ] No API keys or `voxelseek.toml` / `launcher.toml` committed
      (both are git-ignored — double-check with `git status`)
- [ ] ATTRIBUTION.md is up to date for any new assets
- [ ] Screenshots (if any) show only open-licensed textures

## Distributing binaries

A typical Windows release bundle looks like:

```
voxelseek/
├── voxelseek.exe            # the game
├── voxelseek-launcher.exe   # the launcher
├── assets/                  # packs (atlases are compiled in build.rs —
│   └── packs/               #   ship the per-pack atlas.png files!)
├── mods/
│   └── example-hello/
├── LICENSE
├── ATTRIBUTION.md
└── README.md
```

Build with `cargo build --release`, then copy `target/release/voxelseek.exe`
and `target/release/voxelseek-launcher.exe` next to the `assets/` and `mods/`
folders. The game resolves assets relative to its working directory.
