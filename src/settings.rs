//! Runtime game settings, loaded from `voxelseek.toml` next to the game
//! executable (or the working directory). The launcher writes this file;
//! the game reads it on startup. Missing file or keys fall back to defaults.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const SETTINGS_FILE: &str = "voxelseek.toml";

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameSettings {
    /// Texture pack id: a directory name under `assets/packs/`.
    pub texture_pack: String,
    /// Initial window size.
    pub window_width: f32,
    pub window_height: f32,
    /// Horizontal render distance in chunks (despawn distance is +2).
    pub render_distance: i32,
    /// Camera field of view in degrees.
    pub fov: f32,
    /// Ids of mods that the launcher/user disabled.
    pub disabled_mods: Vec<String>,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            texture_pack: "programmer-art".to_string(),
            window_width: 1280.0,
            window_height: 720.0,
            render_distance: 16,
            fov: 80.0,
            disabled_mods: Vec::new(),
        }
    }
}

impl GameSettings {
    /// Load settings from disk, falling back to defaults on any problem.
    pub fn load() -> Self {
        let mut settings = Self::load_from(std::path::Path::new(SETTINGS_FILE))
            .unwrap_or_default();
        settings.render_distance = settings.render_distance.clamp(2, 24);
        settings.fov = settings.fov.clamp(30.0, 120.0);
        // Validate the texture pack atlas exists (it is produced by build.rs).
        let atlas = format!("assets/packs/{}/atlas.png", settings.texture_pack);
        if !std::path::Path::new(&atlas).exists() {
            if settings.texture_pack != "programmer-art" {
                warn!(
                    "texture pack '{}' has no built atlas, falling back to 'programmer-art'",
                    settings.texture_pack
                );
            }
            settings.texture_pack = "programmer-art".to_string();
        }
        settings
    }

    fn load_from(path: &std::path::Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        match toml::from_str(&text) {
            Ok(s) => Some(s),
            Err(e) => {
                warn!("failed to parse {}: {e} — using defaults", path.display());
                None
            }
        }
    }

    /// Asset path of the selected pack's texture atlas.
    pub fn atlas_path(&self) -> String {
        format!("packs/{}/atlas.png", self.texture_pack)
    }
}
