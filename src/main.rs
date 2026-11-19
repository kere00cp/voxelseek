use bevy::prelude::*;
use bevy::window::WindowResolution;

mod block;
mod world;
mod player;
mod ui;
mod lighting;
mod audio;
mod inventory;
mod save;
mod entity;
mod harness;
mod settings;
mod mods;

fn main() {
    let game_settings = settings::GameSettings::load();

    App::new()
        .add_plugins(DefaultPlugins
            .set(ImagePlugin {
                default_sampler: bevy::image::ImageSamplerDescriptor {
                    mag_filter: bevy::image::ImageFilterMode::Nearest,
                    min_filter: bevy::image::ImageFilterMode::Nearest,
                    mipmap_filter: bevy::image::ImageFilterMode::Nearest,
                    ..default()
                },
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "VoxelSeek".to_string(),
                    resolution: WindowResolution::new(
                        game_settings.window_width as u32,
                        game_settings.window_height as u32,
                    ),
                    ..default()
                }),
                ..default()
            }))
        .insert_resource(game_settings)
        .init_state::<GameState>()
        .add_plugins((
            world::WorldPlugin,
            player::PlayerPlugin,
            ui::UiPlugin,
            lighting::LightingPlugin,
            audio::GameAudioPlugin,
            inventory::InventoryPlugin,
            save::SavePlugin,
            entity::EntityPlugin,
            harness::HarnessPlugin,
            mods::ModsPlugin,
        ))
        .run();
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    Playing,
    Paused,
    Inventory,
}
