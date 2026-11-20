//! DeepSeek API client (OpenAI-compatible chat completions with tool calling).
//!
//! All network I/O happens on background threads; the Bevy world is only
//! touched from the main thread via [`crate::harness::console::poll_network`].

use serde_json::{Value, json};
use std::sync::mpsc::Sender;
use std::time::Duration;

use super::HarnessConfig;
use super::tools::tool_schemas;

/// Result of one HTTP round-trip.
pub enum ApiOutcome {
    /// The model produced a final text answer.
    Final {
        content: String,
    },
    /// The model requested tool calls. `assistant` is the raw assistant
    /// message (with `tool_calls`) that must be appended to the history.
    ToolCalls {
        assistant: Value,
        calls: Vec<PendingCall>,
    },
    Error(String),
}

/// A single tool call requested by the model.
pub struct PendingCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

/// Spawn a background thread performing one chat-completions request.
pub fn spawn_request(cfg: HarnessConfig, history: Vec<Value>, tx: Sender<ApiOutcome>) {
    std::thread::spawn(move || {
        let _ = tx.send(run(&cfg, history));
    });
}

fn run(cfg: &HarnessConfig, history: Vec<Value>) -> ApiOutcome {
    let Some(key) = cfg.api_key.clone() else {
        return ApiOutcome::Error(
            "DEEPSEEK_API_KEY is not set. Set it and restart the game to enable the assistant."
                .to_string(),
        );
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => return ApiOutcome::Error(format!("failed to build HTTP client: {e}")),
    };

    let url = format!("{}/chat/completions", cfg.base_url);
    let body = json!({
        "model": &cfg.model,
        "messages": history,
        "tools": tool_schemas(),
        "tool_choice": "auto",
        "stream": false,
    });

    let resp = match client.post(&url).bearer_auth(key).json(&body).send() {
        Ok(r) => r,
        Err(e) => return ApiOutcome::Error(format!("request failed: {e}")),
    };

    let status = resp.status();
    let text = match resp.text() {
        Ok(t) => t,
        Err(e) => return ApiOutcome::Error(format!("failed to read response: {e}")),
    };

    if !status.is_success() {
        let snippet: String = text.chars().take(300).collect();
        return ApiOutcome::Error(format!("API returned {status}: {snippet}"));
    }

    let v: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => return ApiOutcome::Error(format!("invalid JSON from API: {e}")),
    };

    if let Some(err) = v.get("error") {
        return ApiOutcome::Error(format!("API error: {err}"));
    }

    let message = v
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .cloned()
        .unwrap_or(Value::Null);

    let tool_calls = message
        .get("tool_calls")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    if !tool_calls.is_empty() {
        let mut calls = Vec::new();
        for tc in tool_calls {
            let id = tc
                .get("id")
                .and_then(|i| i.as_str())
                .unwrap_or_default()
                .to_string();
            let name = tc
                .get("function")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or_default()
                .to_string();
            // `arguments` is a JSON-encoded string per the OpenAI spec.
            let args = tc
                .get("function")
                .and_then(|f| f.get("arguments"))
                .map(|a| {
                    a.as_str()
                        .and_then(|s| serde_json::from_str::<Value>(s).ok())
                        .unwrap_or_else(|| a.clone())
                })
                .unwrap_or(json!({}));
            if !name.is_empty() {
                calls.push(PendingCall { id, name, args });
            }
        }
        if !calls.is_empty() {
            return ApiOutcome::ToolCalls {
                assistant: message,
                calls,
            };
        }
    }

    let content = message
        .get("content")
        .and_then(|c| c.as_str())
        .unwrap_or("(empty response)")
        .to_string();

    ApiOutcome::Final { content }
}
