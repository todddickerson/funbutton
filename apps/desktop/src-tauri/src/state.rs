use crate::history::History;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Auto,
    Groq,
    Local,
}

impl Default for Backend {
    fn default() -> Self {
        Backend::Auto
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModeOverride {
    Auto,
    Code,
    Email,
    Slack,
    Raw,
}

impl Default for ModeOverride {
    fn default() -> Self {
        ModeOverride::Auto
    }
}

/// Which key acts as push-to-talk.
///
/// - `Fn` — the Function key (bottom-left of every Mac keyboard). Default. The
///   brand. Detected via CGEventTap on the `kCGEventFlagMaskSecondaryFn` bit
///   because Fn is not exposed as a normal modifier.
/// - `RightOption` — fallback for users who already mapped Fn (rare). Detected
///   via rdev `Key::AltGr`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HotkeyKind {
    Fn,
    RightOption,
}

impl Default for HotkeyKind {
    fn default() -> Self {
        HotkeyKind::Fn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default)]
    pub backend: Backend,
    #[serde(default = "default_ollama_model")]
    pub ollama_model: String,
    #[serde(default = "default_ollama_url")]
    pub ollama_url: String,
    #[serde(default = "default_words_today")]
    pub words_today: u64,
    #[serde(default)]
    pub words_today_date: String,
    #[serde(default = "default_hotkey_label")]
    pub hotkey_label: String,
    #[serde(default)]
    pub hotkey_kind: HotkeyKind,
    #[serde(default)]
    pub mode_override: ModeOverride,
    #[serde(default)]
    pub dictionary: Vec<String>,
    #[serde(default = "default_retention_days")]
    pub history_retention_days: u32,
    #[serde(default)]
    pub onboarded: bool,
}

fn default_retention_days() -> u32 { 30 }

fn default_ollama_model() -> String { "qwen2.5:1.5b".to_string() }
fn default_ollama_url() -> String { "http://localhost:11434".to_string() }
fn default_words_today() -> u64 { 0 }
fn default_hotkey_label() -> String {
    "the Fun Button (Fn) — bottom-left of your keyboard".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
            backend: Backend::default(),
            ollama_model: default_ollama_model(),
            ollama_url: default_ollama_url(),
            words_today: 0,
            words_today_date: String::new(),
            hotkey_label: default_hotkey_label(),
            hotkey_kind: HotkeyKind::default(),
            mode_override: ModeOverride::default(),
            dictionary: Vec::new(),
            history_retention_days: default_retention_days(),
            onboarded: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Recording,
    Transcribing,
    Cleaning,
    Pasting,
    Error,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Idle => "idle",
            Status::Recording => "recording",
            Status::Transcribing => "transcribing",
            Status::Cleaning => "cleaning",
            Status::Pasting => "pasting",
            Status::Error => "error",
        }
    }
}

pub struct AppState {
    pub settings: Mutex<Settings>,
    pub status: Mutex<Status>,
    pub last_transcript: Mutex<String>,
    pub last_cleaned: Mutex<String>,
    pub history: Arc<History>,
}

pub type AppStateHandle = Arc<AppState>;

impl AppState {
    pub fn new(settings: Settings, history: Arc<History>) -> AppStateHandle {
        Arc::new(AppState {
            settings: Mutex::new(settings),
            status: Mutex::new(Status::Idle),
            last_transcript: Mutex::new(String::new()),
            last_cleaned: Mutex::new(String::new()),
            history,
        })
    }
}
