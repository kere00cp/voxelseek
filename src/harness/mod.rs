//! DeepSeek harness plugin.
//!
//! An open, MIT-licensed integration layer that embeds a DeepSeek-powered
//! assistant into the game. The assistant chats with the player and can act
//! inside the local single-player world through a small, auditable tool set
//! (place/break blocks, teleport, give items, query the world, ...).
//!
//! - Press `T` in-game to open the console, `Esc` to close it.
//! - The API key is read from the `DEEPSEEK_API_KEY` environment variable.
//!   `DEEPSEEK_BASE_URL` and `DEEPSEEK_MODEL` can override the defaults
//!   (`https://api.deepseek.com`, `deepseek-chat`).
//!
//! DeepSeek is a third-party service by 深度求索 (DeepSeek); this plugin is
//! not affiliated with or endorsed by DeepSeek. API usage is subject to the
//! DeepSeek terms of service and your own API key.

pub mod api;
pub mod console;
pub mod tools;

use bevy::prelude::*;
use std::sync::mpsc;

/// Bevy plugin wiring the whole harness into the app.
pub struct HarnessPlugin;

impl Plugin for HarnessPlugin {
    fn build(&self, app: &mut App) {
        let (tx, rx) = mpsc::channel();
        app.insert_resource(HarnessChannel {
            tx,
            rx: std::sync::Mutex::new(rx),
        })
            .init_resource::<HarnessConsole>()
            .init_resource::<HarnessHistory>()
            .init_resource::<HarnessConfig>()
            .add_systems(Startup, console::setup_console_ui)
            .add_systems(
                Update,
                (
                    (console::toggle_console, console::console_keyboard_input).chain(),
                    console::update_console_ui,
                    console::poll_network,
                ),
            )
            .add_systems(Last, console::clear_frame_flags);
    }
}

/// One line in the on-screen console log.
#[derive(Clone)]
pub struct ChatLine {
    pub role: &'static str,
    pub text: String,
}

impl ChatLine {
    pub fn user(text: impl Into<String>) -> Self {
        Self { role: "You", text: text.into() }
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self { role: "DeepSeek", text: text.into() }
    }
    pub fn system(text: impl Into<String>) -> Self {
        Self { role: "System", text: text.into() }
    }
    pub fn tool(text: impl Into<String>) -> Self {
        Self { role: "Tool", text: text.into() }
    }
}

/// State of the in-game console.
#[derive(Resource, Default)]
pub struct HarnessConsole {
    pub open: bool,
    pub input: String,
    pub log: Vec<ChatLine>,
    /// True while an API request (or tool round-trip) is in flight.
    pub busy: bool,
    /// Set for the remainder of the frame in which the console was closed
    /// with Escape, so the pause menu does not also trigger on that key press.
    pub just_closed: bool,
    /// Set for the remainder of the frame in which the console was opened
    /// with T, so the same key press is not typed into the input field.
    pub just_opened: bool,
}

impl HarnessConsole {
    pub fn push(&mut self, line: ChatLine) {
        const MAX_LOG: usize = 100;
        self.log.push(line);
        if self.log.len() > MAX_LOG {
            let overflow = self.log.len() - MAX_LOG;
            self.log.drain(0..overflow);
        }
    }
}

/// Conversation history sent to the DeepSeek API (OpenAI-compatible format).
#[derive(Resource, Default)]
pub struct HarnessHistory {
    pub messages: Vec<serde_json::Value>,
    /// Number of consecutive tool-call rounds in the current request chain.
    pub rounds: usize,
}

impl HarnessHistory {
    /// Keep the system prompt plus the most recent messages, bounding tokens.
    pub fn trim(&mut self) {
        const KEEP: usize = 24;
        if self.messages.len() > KEEP + 1 {
            let system = self.messages.first().cloned();
            let tail = self.messages.split_off(self.messages.len() - KEEP);
            self.messages.clear();
            if let Some(sys) = system {
                if sys.get("role").and_then(|r| r.as_str()) == Some("system") {
                    self.messages.push(sys);
                }
            }
            self.messages.extend(tail);
        }
    }
}

/// Runtime configuration, sourced from environment variables.
#[derive(Resource, Clone)]
pub struct HarnessConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub model: String,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("DEEPSEEK_API_KEY")
                .ok()
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty()),
            base_url: std::env::var("DEEPSEEK_BASE_URL")
                .ok()
                .map(|u| u.trim_end_matches('/').to_string())
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| "https://api.deepseek.com".to_string()),
            model: std::env::var("DEEPSEEK_MODEL")
                .ok()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| "deepseek-chat".to_string()),
        }
    }
}

/// Channel carrying results from background HTTP threads back to Bevy.
/// The receiver is wrapped in a Mutex because `mpsc::Receiver` is not `Sync`,
/// and Bevy resources must be `Send + Sync`.
#[derive(Resource)]
pub struct HarnessChannel {
    pub tx: mpsc::Sender<api::ApiOutcome>,
    pub rx: std::sync::Mutex<mpsc::Receiver<api::ApiOutcome>>,
}
