use crate::embedded_llm::EmbeddedServerHandle;
use crate::embedded_stt::{EmbeddedStt, EmbeddedSttHandle};
use crate::history::History;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU8};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum Backend {
    /// Try embedded → ollama-external → groq, in that order.
    #[default]
    Auto,
    /// Force Groq cloud (fast tier or premium via license).
    Groq,
    /// Force user-installed Ollama at `ollama_url`.
    Local,
    /// Force the app-bundled llama.cpp + Qwen 2.5 1.5B GGUF.
    Embedded,
}

/// Which engine transcribes speech. On-device is the default — a fresh
/// install dictates with zero API keys, fully offline. Groq is the optional
/// faster/cloud path (needs a key).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SttBackend {
    /// Bundled whisper base.en via transcribe-cpp (Metal). Default.
    #[default]
    Local,
    /// Groq Whisper Turbo (cloud, BYOK) — or the licensed cloud proxy.
    Groq,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ModeOverride {
    #[default]
    Auto,
    Code,
    Email,
    Slack,
    Raw,
}

/// Which key acts as push-to-talk.
///
/// Every variant except `Fn` is a normal modifier detected by a raw CGEventTap
/// on `FlagsChanged`, reading only the virtual keycode + the device-specific
/// modifier flag bit — never a layout/input-source API, so the whole set is
/// safe on macOS 26 (the off-main-thread TIS/TSM trap; see `hotkey.rs`).
///
/// - `Fn` — the Function key. The brand default *where it applies* (built-in
///   MacBook keyboards and compact Magic Keyboards, where Fn is the bottom-left
///   key). Detected via the `kCGEventFlagMaskSecondaryFn` bit because Fn is not
///   exposed as a normal modifier (see `fn_hotkey.rs`). Fn still exists — and
///   still works — on extended keyboards, it's just not in the bottom-left.
/// - `RightOption` / `LeftOption` — Option keys (kbd `0x3D` / `0x3A`).
/// - `RightControl` / `LeftControl` — Control keys (kbd `0x3E` / `0x3B`). On
///   the extended Magic Keyboard the bottom-left key is Left Control.
/// - `RightCommand` / `LeftCommand` — Command keys (kbd `0x36` / `0x37`). Left
///   Command intercepts every ⌘ shortcut, so the UI flags it.
/// - `CapsLock` — Caps Lock (kbd `0x39`). Toggles rather than reporting a hold,
///   so it works as tap-to-start / tap-to-stop.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum HotkeyKind {
    #[default]
    Fn,
    RightOption,
    LeftOption,
    RightControl,
    LeftControl,
    RightCommand,
    LeftCommand,
    CapsLock,
}

impl HotkeyKind {
    /// Runtime tag for the shared `armed` atomic. Fn=0 and RightOption=1 are
    /// pinned for back-compat with any value already stored at runtime; the
    /// atomic is never persisted (settings serialize the serde name), so the
    /// rest are free to append.
    pub fn as_u8(self) -> u8 {
        match self {
            HotkeyKind::Fn => 0,
            HotkeyKind::RightOption => 1,
            HotkeyKind::RightControl => 2,
            HotkeyKind::LeftControl => 3,
            HotkeyKind::RightCommand => 4,
            HotkeyKind::CapsLock => 5,
            HotkeyKind::LeftOption => 6,
            HotkeyKind::LeftCommand => 7,
        }
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => HotkeyKind::RightOption,
            2 => HotkeyKind::RightControl,
            3 => HotkeyKind::LeftControl,
            4 => HotkeyKind::RightCommand,
            5 => HotkeyKind::CapsLock,
            6 => HotkeyKind::LeftOption,
            7 => HotkeyKind::LeftCommand,
            _ => HotkeyKind::Fn,
        }
    }
    /// Virtual keycode this kind fires `FlagsChanged` on, or `None` for `Fn`
    /// (Fn has no stable modifier keycode — it's read off the SecondaryFn flag
    /// bit instead, in `fn_hotkey.rs`). Used by the generic keycode listener
    /// and by "press the key you want" capture.
    pub fn keycode(self) -> Option<i64> {
        Some(match self {
            HotkeyKind::Fn => return None,
            HotkeyKind::RightOption => 0x3D,
            HotkeyKind::LeftOption => 0x3A,
            HotkeyKind::RightControl => 0x3E,
            HotkeyKind::LeftControl => 0x3B,
            HotkeyKind::RightCommand => 0x36,
            HotkeyKind::LeftCommand => 0x37,
            HotkeyKind::CapsLock => 0x39,
        })
    }
    /// Human label shown in the Settings UI / tray. Single source of truth —
    /// never persisted; always derived. This is what fixed the "UI says Right
    /// Option but actual listener is Fn" bug in v0.1.0. Deliberately does NOT
    /// assert "bottom-left of your keyboard" — that claim is false on extended
    /// boards, which is the whole reason the picker exists.
    pub fn label(self) -> &'static str {
        match self {
            HotkeyKind::Fn => "Fn (the Fun Button)",
            HotkeyKind::RightOption => "Right Option (right of the spacebar)",
            HotkeyKind::LeftOption => "Left Option",
            HotkeyKind::RightControl => "Right Control",
            HotkeyKind::LeftControl => "Left Control",
            HotkeyKind::RightCommand => "Right Command",
            HotkeyKind::LeftCommand => "Left Command",
            HotkeyKind::CapsLock => "Caps Lock (tap to start, tap to stop)",
        }
    }
    /// Concise word form for inline prose ("hold X to dictate"). Same single
    /// source as `label`, minus the parenthetical — used by the ready
    /// notification and any place that names the armed key in a sentence.
    pub fn short_label(self) -> &'static str {
        match self {
            HotkeyKind::Fn => "Fn",
            HotkeyKind::RightOption => "Right Option",
            HotkeyKind::LeftOption => "Left Option",
            HotkeyKind::RightControl => "Right Control",
            HotkeyKind::LeftControl => "Left Control",
            HotkeyKind::RightCommand => "Right Command",
            HotkeyKind::LeftCommand => "Left Command",
            HotkeyKind::CapsLock => "Caps Lock",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub groq_api_key: String,
    #[serde(default)]
    pub backend: Backend,
    #[serde(default)]
    pub stt_backend: SttBackend,
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
fn default_premium_model() -> String {
    "premium-haiku".to_string()
}
fn default_retention_days() -> u32 {
    30
}

fn default_ollama_model() -> String {
    "qwen2.5:1.5b".to_string()
}
fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}
fn default_words_today() -> u64 {
    0
}
fn default_hotkey_label() -> String {
    "the Fun Button (Fn) — bottom-left of your keyboard".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            groq_api_key: std::env::var("GROQ_API_KEY").unwrap_or_default(),
            backend: Backend::default(),
            stt_backend: SttBackend::default(),
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
    /// Set when the bundled llama-server failed to start, so the UI can
    /// distinguish "still warming up" from "not going to happen".
    pub embedded_error: Mutex<Option<String>>,
    /// Bundled on-device whisper engine. Always present; its internal status
    /// tracks starting/ready/failed.
    pub stt: EmbeddedSttHandle,
    /// Which hotkey is "armed" right now — both listeners run, but only the
    /// one whose kind matches this atomic emits Down/Up events. Lets us
    /// hot-swap the active hotkey without restarting the app.
    pub armed_hotkey: Arc<AtomicU8>,
    /// "Press the key you want" capture. When active, both listeners report the
    /// first recognized modifier keydown here (regardless of what's armed) and
    /// suppress the normal push-to-talk path, so the user can bind a key by
    /// pressing it. See `hotkey::CaptureState`.
    pub capture: Arc<crate::hotkey::CaptureState>,
    /// Sender end of the hotkey-event channel, kept here so the
    /// `simulate_hotkey` Tauri command can push synthetic Down/Up events
    /// without going through the listener — useful for bisecting the
    /// pipeline when the listener itself is suspect.
    pub hotkey_tx: Mutex<Option<std::sync::mpsc::Sender<crate::hotkey::HotkeyEvent>>>,
    /// Set once quit teardown begins (see `crate::shutdown`). The hotkey loop
    /// checks it so a hold that lands mid-quit can't start a new recording /
    /// pipeline run and grab the whisper session while we're unloading it.
    pub shutting_down: AtomicBool,
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
            embedded_error: Mutex::new(None),
            stt: EmbeddedStt::new(),
            armed_hotkey: armed,
            capture: Arc::new(crate::hotkey::CaptureState::new()),
            hotkey_tx: Mutex::new(None),
            shutting_down: AtomicBool::new(false),
        })
    }
}
