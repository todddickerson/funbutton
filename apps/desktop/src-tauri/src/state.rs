use crate::embedded_llm::EmbeddedServerHandle;
use crate::history::History;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// Try embedded → ollama-external → groq, in that order.
    Auto,
    /// Force Groq cloud (fast tier or premium via license).
    Groq,
    /// Force user-installed Ollama at `ollama_url`.
    Local,
    /// Force the app-bundled llama.cpp + Qwen 2.5 1.5B GGUF.
    Embedded,
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

impl HotkeyKind {
    pub fn as_u8(self) -> u8 {
        match self {
            HotkeyKind::Fn => 0,
            HotkeyKind::RightOption => 1,
        }
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => HotkeyKind::RightOption,
            _ => HotkeyKind::Fn,
        }
    }
    /// Human label shown in the Settings UI. Single source of truth — never
    /// persisted; always derived. This is what fixed the "UI says Right Option
    /// but actual listener is Fn" bug in v0.1.0.
    pub fn label(self) -> &'static str {
        match self {
            HotkeyKind::Fn => "Fn (the Fun Button — bottom-left of your keyboard)",
            HotkeyKind::RightOption => "Right Option (right side of spacebar)",
        }
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
    /// License JWT for the paid cloud tier. When set, transcribe + cleanup
    /// route through `cloud_api_base` instead of BYOK Groq direct.
    /// Empty string = BYOK mode (default).
    #[serde(default)]
    pub license_jwt: String,
    #[serde(default = "default_cloud_api_base")]
    pub cloud_api_base: String,
    /// Preferred premium model when license is active. Persisted across runs.
    /// Values: "fast" | "premium-haiku" | "premium-sonnet" | "premium-opus" | "premium-gpt41"
    #[serde(default = "default_premium_model")]
    pub premium_model: String,
}

fn default_cloud_api_base() -> String {
    // Will move to https://api.funbutton.ai once the funbutton.ai zone is
    // added to the Spontent CF account and a custom domain is attached.
    "https://funbutton-api.todd-e03.workers.dev".to_string()
}
fn default_premium_model() -> String { "premium-haiku".to_string() }
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
            license_jwt: String::new(),
            cloud_api_base: default_cloud_api_base(),
            premium_model: default_premium_model(),
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
    /// `Some` once the bundled llama-server has finished starting up. `None`
    /// during startup or if the user's machine couldn't launch it.
    pub embedded: Mutex<Option<EmbeddedServerHandle>>,
    /// Which hotkey is "armed" right now — both listeners run, but only the
    /// one whose kind matches this atomic emits Down/Up events. Lets us
    /// hot-swap the active hotkey without restarting the app.
    pub armed_hotkey: Arc<AtomicU8>,
    /// Sender end of the hotkey-event channel, kept here so the
    /// `simulate_hotkey` Tauri command can push synthetic Down/Up events
    /// without going through the listener — useful for bisecting the
    /// pipeline when the listener itself is suspect.
    pub hotkey_tx: Mutex<Option<std::sync::mpsc::Sender<crate::hotkey::HotkeyEvent>>>,
}

pub type AppStateHandle = Arc<AppState>;

impl AppState {
    pub fn new(settings: Settings, history: Arc<History>) -> AppStateHandle {
        let armed = Arc::new(AtomicU8::new(settings.hotkey_kind.as_u8()));
        Arc::new(AppState {
            settings: Mutex::new(settings),
            status: Mutex::new(Status::Idle),
            last_transcript: Mutex::new(String::new()),
            last_cleaned: Mutex::new(String::new()),
            history,
            embedded: Mutex::new(None),
            armed_hotkey: armed,
            hotkey_tx: Mutex::new(None),
        })
    }
}
