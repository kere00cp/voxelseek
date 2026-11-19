//! In-game console UI and main-thread glue for the DeepSeek harness.

use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions, PrimaryWindow};
use serde_json::json;

use crate::inventory::chest::ChestOpen;
use crate::inventory::crafting::CraftingTableOpen;
use crate::inventory::furnace::FurnaceOpen;
use crate::inventory::inventory::Inventory;
use crate::lighting::day_night::DayNightCycle;
use crate::player::Player;
use crate::ui::death_screen::PlayerDead;
use crate::ui::inventory_screen::InventoryOpen;
use crate::ui::main_menu::InMainMenu;
use crate::ui::pause_menu::PauseState;
use crate::world::manager::{ChunkDataStore, ChunkManager};

use super::api::{self, ApiOutcome};
use super::tools;
use super::{ChatLine, HarnessChannel, HarnessConfig, HarnessConsole, HarnessHistory};

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

#[derive(Component)]
struct ConsoleRoot;

#[derive(Component)]
struct ConsoleLogText;

#[derive(Component)]
struct ConsoleInputText;

/// Spawn the (initially hidden) console overlay.
pub fn setup_console_ui(mut commands: Commands) {
    commands
        .spawn((
            ConsoleRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(12.0),
                top: Val::Px(48.0),
                width: Val::Percent(55.0),
                max_width: Val::Px(760.0),
                max_height: Val::Percent(45.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(6.0),
                display: Display::None,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.65)),
            ZIndex(90),
        ))
        .with_children(|parent| {
            parent.spawn((
                ConsoleLogText,
                Text::new(""),
                TextColor(Color::srgb(0.9, 0.9, 0.9)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
            parent.spawn((
                ConsoleInputText,
                Text::new(""),
                TextColor(Color::srgb(0.55, 1.0, 0.55)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
            ));
        });
}

/// Reflect console state in the UI.
pub fn update_console_ui(
    console: Res<HarnessConsole>,
    mut root_q: Query<&mut Node, With<ConsoleRoot>>,
    mut log_q: Query<&mut Text, (With<ConsoleLogText>, Without<ConsoleInputText>)>,
    mut input_q: Query<&mut Text, (With<ConsoleInputText>, Without<ConsoleLogText>)>,
) {
    if !console.is_changed() {
        return;
    }
    if let Ok(mut node) = root_q.single_mut() {
        node.display = if console.open { Display::Flex } else { Display::None };
    }
    if !console.open {
        return;
    }

    const MAX_LINES: usize = 12;
    let start = console.log.len().saturating_sub(MAX_LINES);
    let mut s = String::new();
    for line in &console.log[start..] {
        s.push_str(line.role);
        s.push_str(": ");
        s.push_str(&line.text);
        s.push('\n');
    }
    if console.busy {
        s.push_str("DeepSeek: thinking…\n");
    }
    if let Ok(mut t) = log_q.single_mut() {
        **t = s;
    }
    if let Ok(mut t) = input_q.single_mut() {
        **t = format!("> {}▏", console.input);
    }
}

// ---------------------------------------------------------------------------
// Open / close
// ---------------------------------------------------------------------------

/// `T` opens the console, `Escape` closes it.
pub fn toggle_console(
    keys: Res<ButtonInput<KeyCode>>,
    mut console: ResMut<HarnessConsole>,
    cfg: Res<HarnessConfig>,
    in_menu: Res<InMainMenu>,
    pause: Res<PauseState>,
    inventory_open: Res<InventoryOpen>,
    ct_open: Res<CraftingTableOpen>,
    furnace_open: Res<FurnaceOpen>,
    chest_open: Res<ChestOpen>,
    dead: Res<PlayerDead>,
    mut cursor_q: Query<&mut CursorOptions, With<PrimaryWindow>>,
) {
    if console.open {
        if keys.just_pressed(KeyCode::Escape) {
            console.open = false;
            console.just_closed = true;
            if let Ok(mut cursor) = cursor_q.single_mut() {
                cursor.grab_mode = CursorGrabMode::Locked;
                cursor.visible = false;
            }
        }
        return;
    }

    if !keys.just_pressed(KeyCode::KeyT) {
        return;
    }
    if in_menu.0
        || pause.0
        || inventory_open.0
        || ct_open.0
        || furnace_open.0.is_some()
        || chest_open.0.is_some()
        || dead.0
    {
        return;
    }

    console.open = true;
    console.input.clear();
    if console.log.is_empty() {
        console.push(ChatLine::system(
            "DeepSeek harness ready. Type a message and press Enter.",
        ));
        console.push(ChatLine::system(if cfg.api_key.is_some() {
            format!("API key found — model: {}", cfg.model)
        } else {
            "DEEPSEEK_API_KEY is not set: chatting will fail until you set it and restart."
                .to_string()
        }));
        console.push(ChatLine::system(
            "Examples: \"give me 64 oak planks\" · \"build a 5x5 stone wall\" · \"set time to midnight\"",
        ));
    }
    if let Ok(mut cursor) = cursor_q.single_mut() {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
}

/// Per-frame flag cleanup (runs in `Last`).
pub fn clear_frame_flags(mut console: ResMut<HarnessConsole>) {
    if console.just_closed {
        console.just_closed = false;
    }
}

// ---------------------------------------------------------------------------
// Text input
// ---------------------------------------------------------------------------

const KEYMAP: &[(KeyCode, char, char)] = &[
    (KeyCode::KeyA, 'a', 'A'),
    (KeyCode::KeyB, 'b', 'B'),
    (KeyCode::KeyC, 'c', 'C'),
    (KeyCode::KeyD, 'd', 'D'),
    (KeyCode::KeyE, 'e', 'E'),
    (KeyCode::KeyF, 'f', 'F'),
    (KeyCode::KeyG, 'g', 'G'),
    (KeyCode::KeyH, 'h', 'H'),
    (KeyCode::KeyI, 'i', 'I'),
    (KeyCode::KeyJ, 'j', 'J'),
    (KeyCode::KeyK, 'k', 'K'),
    (KeyCode::KeyL, 'l', 'L'),
    (KeyCode::KeyM, 'm', 'M'),
    (KeyCode::KeyN, 'n', 'N'),
    (KeyCode::KeyO, 'o', 'O'),
    (KeyCode::KeyP, 'p', 'P'),
    (KeyCode::KeyQ, 'q', 'Q'),
    (KeyCode::KeyR, 'r', 'R'),
    (KeyCode::KeyS, 's', 'S'),
    (KeyCode::KeyT, 't', 'T'),
    (KeyCode::KeyU, 'u', 'U'),
    (KeyCode::KeyV, 'v', 'V'),
    (KeyCode::KeyW, 'w', 'W'),
    (KeyCode::KeyX, 'x', 'X'),
    (KeyCode::KeyY, 'y', 'Y'),
    (KeyCode::KeyZ, 'z', 'Z'),
    (KeyCode::Digit0, '0', ')'),
    (KeyCode::Digit1, '1', '!'),
    (KeyCode::Digit2, '2', '@'),
    (KeyCode::Digit3, '3', '#'),
    (KeyCode::Digit4, '4', '$'),
    (KeyCode::Digit5, '5', '%'),
    (KeyCode::Digit6, '6', '^'),
    (KeyCode::Digit7, '7', '&'),
    (KeyCode::Digit8, '8', '*'),
    (KeyCode::Digit9, '9', '('),
    (KeyCode::Space, ' ', ' '),
    (KeyCode::Minus, '-', '_'),
    (KeyCode::Equal, '=', '+'),
    (KeyCode::BracketLeft, '[', '{'),
    (KeyCode::BracketRight, ']', '}'),
    (KeyCode::Backslash, '\\', '|'),
    (KeyCode::Semicolon, ';', ':'),
    (KeyCode::Quote, '\'', '"'),
    (KeyCode::Comma, ',', '<'),
    (KeyCode::Period, '.', '>'),
    (KeyCode::Slash, '/', '?'),
    (KeyCode::Backquote, '`', '~'),
];

/// Capture keyboard input while the console is open and submit on Enter.
pub fn console_keyboard_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut console: ResMut<HarnessConsole>,
    cfg: Res<HarnessConfig>,
    channel: Res<HarnessChannel>,
    mut history: ResMut<HarnessHistory>,
    mut mods: ResMut<crate::mods::ModRuntime>,
) {
    if !console.open {
        return;
    }

    if keys.just_pressed(KeyCode::Backspace) {
        console.input.pop();
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for (kc, ch, shift_ch) in KEYMAP {
        if keys.just_pressed(*kc) {
            console.input.push(if shift { *shift_ch } else { *ch });
        }
    }

    if !keys.just_pressed(KeyCode::Enter) {
        return;
    }

    let text = console.input.trim().to_string();
    console.input.clear();
    if text.is_empty() {
        return;
    }
    console.push(ChatLine::user(&text));

    // "/cmd args..." is a mod command, not a DeepSeek chat message.
    if let Some(cmdline) = text.strip_prefix('/') {
        let mut parts = cmdline.split_whitespace();
        let name = parts.next().unwrap_or("").to_string();
        let args: Vec<String> = parts.map(|s| s.to_string()).collect();
        if name.is_empty() {
            return;
        }
        if mods.loaded_count() == 0 {
            console.push(ChatLine::system(
                "no mods loaded — '/command' needs a mod with on_command (see mods/).",
            ));
            return;
        }
        mods.queue_command(name, args);
        return;
    }

    if console.busy {
        console.push(ChatLine::system("DeepSeek is still thinking, please wait…"));
        return;
    }
    if cfg.api_key.is_none() {
        console.push(ChatLine::system(
            "DEEPSEEK_API_KEY is not set. Set the environment variable and restart the game.",
        ));
        return;
    }

    if history.messages.is_empty() {
        history.messages.push(json!({
            "role": "system",
            "content": tools::system_prompt(),
        }));
    }
    history.messages.push(json!({"role": "user", "content": text}));
    history.rounds = 0;
    history.trim();

    console.busy = true;
    api::spawn_request(cfg.clone(), history.messages.clone(), channel.tx.clone());
}

// ---------------------------------------------------------------------------
// Networking & tool driving (main thread)
// ---------------------------------------------------------------------------

/// Bundled world access for tool execution (keeps poll_network under the
/// Bevy system parameter limit).
#[derive(bevy::ecs::system::SystemParam)]
pub struct WorldAccess<'w, 's> {
    store: ResMut<'w, ChunkDataStore>,
    inventory: ResMut<'w, Inventory>,
    cycle: ResMut<'w, DayNightCycle>,
    player_q: Query<'w, 's, &'static mut Transform, With<Player>>,
    manager: Res<'w, ChunkManager>,
    commands: Commands<'w, 's>,
}

/// Maximum consecutive tool-call rounds per user message.
const MAX_TOOL_ROUNDS: usize = 8;

pub fn poll_network(
    channel: Res<HarnessChannel>,
    mut console: ResMut<HarnessConsole>,
    mut history: ResMut<HarnessHistory>,
    cfg: Res<HarnessConfig>,
    mut access: WorldAccess,
) {
    let Ok(outcome) = channel.rx.try_recv() else {
        return;
    };
    console.busy = false;

    match outcome {
        ApiOutcome::Error(e) => {
            console.push(ChatLine::system(format!("Error: {e}")));
        }
        ApiOutcome::Final { content } => {
            history.messages.push(json!({"role": "assistant", "content": content}));
            history.rounds = 0;
            history.trim();
            console.push(ChatLine::assistant(&content));
        }
        ApiOutcome::ToolCalls { assistant, calls } => {
            history.messages.push(assistant);
            history.rounds += 1;

            let Ok(mut player_tf) = access.player_q.single_mut() else {
                console.push(ChatLine::system("Error: player not found"));
                return;
            };
            let mut remesh = Vec::new();

            for call in &calls {
                console.push(ChatLine::tool(format!(
                    "{}({})",
                    call.name, call.args
                )));
                let mut ctx = tools::ToolContext {
                    store: &mut access.store,
                    inventory: &mut access.inventory,
                    cycle: &mut access.cycle,
                    player_pos: player_tf.translation,
                    player_tf: &mut player_tf,
                    remesh: &mut remesh,
                };
                let result = tools::execute(call, &mut ctx);
                history.messages.push(json!({
                    "role": "tool",
                    "tool_call_id": call.id,
                    "content": result,
                }));
            }
            drop(player_tf);

            tools::apply_remesh(&access.manager, &mut access.commands, remesh);

            if history.rounds >= MAX_TOOL_ROUNDS {
                history.rounds = 0;
                console.push(ChatLine::system(
                    "Tool round limit reached — stopping here.",
                ));
            } else {
                console.busy = true;
                api::spawn_request(cfg.clone(), history.messages.clone(), channel.tx.clone());
            }
        }
    }
}
