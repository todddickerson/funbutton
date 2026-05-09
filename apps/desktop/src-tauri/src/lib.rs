mod app_detect;
mod audio;
mod cleanup;
mod fn_hotkey;
mod groq;
mod history;
mod hotkey;
mod inject;
mod ollama;
mod pipeline;
mod state;

use crate::audio::Recorder;
use crate::hotkey::HotkeyEvent;
use crate::state::{AppState, AppStateHandle, HotkeyKind, Settings, Status};

use parking_lot::Mutex as PMutex;
use serde::Serialize;
use std::sync::mpsc;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Emitter, Manager,
};

#[derive(Serialize, Clone)]
struct StatusEvent {
    status: &'static str,
    message: Option<String>,
}

fn emit_status(app: &AppHandle, status: Status, message: Option<String>) {
    let _ = app.emit(
        "funbutton:status",
        StatusEvent { status: status.label(), message },
    );
}

#[derive(Serialize, Clone)]
struct PipelinePayload {
    raw: String,
    cleaned: String,
    mode: String,
    backend: String,
    word_count: usize,
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, AppStateHandle>) -> Settings {
    state.settings.lock().clone()
}

#[tauri::command]
fn save_settings(
    state: tauri::State<'_, AppStateHandle>,
    settings: Settings,
) -> Result<(), String> {
    persist(&settings).map_err(|e| e.to_string())?;
    *state.settings.lock() = settings;
    Ok(())
}

#[tauri::command]
fn get_status(state: tauri::State<'_, AppStateHandle>) -> String {
    state.status.lock().label().to_string()
}

#[tauri::command]
fn get_last_transcript(state: tauri::State<'_, AppStateHandle>) -> String {
    state.last_transcript.lock().clone()
}

#[tauri::command]
async fn ollama_check(state: tauri::State<'_, AppStateHandle>) -> Result<bool, String> {
    let url = state.settings.lock().ollama_url.clone();
    Ok(ollama::is_available(&url).await)
}

#[tauri::command]
fn history_list(
    state: tauri::State<'_, AppStateHandle>,
    limit: Option<i64>,
    search: Option<String>,
    mode: Option<String>,
) -> Result<Vec<history::HistoryEntry>, String> {
    state
        .history
        .list(limit.unwrap_or(200), search.as_deref(), mode.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn history_copy(
    state: tauri::State<'_, AppStateHandle>,
    id: i64,
) -> Result<(), String> {
    let entries = state
        .history
        .list(1000, None, None)
        .map_err(|e| e.to_string())?;
    let entry = entries
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("history id {} not found", id))?;
    inject::set_clipboard(&entry.cleaned_text).map_err(|e| e.to_string())
}

#[tauri::command]
fn history_purge_now(state: tauri::State<'_, AppStateHandle>) -> Result<u64, String> {
    let days = state.settings.lock().history_retention_days;
    state.history.purge_older_than(days).map_err(|e| e.to_string())
}

#[tauri::command]
fn history_last_failed(
    state: tauri::State<'_, AppStateHandle>,
) -> Result<Option<history::HistoryEntry>, String> {
    state.history.last_failed().map_err(|e| e.to_string())
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn open_onboarding(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("onboarding") {
        let _ = win.show();
        let _ = win.set_focus();
    }
    Ok(())
}

#[tauri::command]
fn close_onboarding(app: AppHandle, state: tauri::State<'_, AppStateHandle>) -> Result<(), String> {
    {
        let mut s = state.settings.lock();
        s.onboarded = true;
        let _ = persist(&s);
    }
    if let Some(win) = app.get_webview_window("onboarding") {
        let _ = win.hide();
    }
    // Show the quick-ref card via the settings webview as a transient overlay,
    // and trigger a tray-icon flash.
    let _ = app.emit("funbutton:onboarding-complete", ());
    Ok(())
}

/// Open a Privacy & Security pane in System Settings via macOS URL scheme.
/// `panel` is one of: "microphone" | "accessibility" | "input_monitoring".
#[tauri::command]
fn open_system_settings_panel(panel: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url = match panel.as_str() {
            "microphone" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
            "accessibility" => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            "input_monitoring" => "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
            other => return Err(format!("unknown panel: {other}")),
        };
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| format!("open failed: {e}"))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = panel;
    }
    Ok(())
}

/// Validate a Groq API key by hitting GET /v1/models. Returns (ok, message).
#[tauri::command]
async fn validate_groq_key(key: String) -> Result<bool, String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("empty key".into());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.groq.com/openai/v1/models")
        .bearer_auth(trimmed)
        .send()
        .await
        .map_err(|e| format!("network: {e}"))?;
    if resp.status().is_success() {
        Ok(true)
    } else if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        Err("invalid key".into())
    } else {
        Err(format!("unexpected status: {}", resp.status()))
    }
}

fn settings_path() -> std::path::PathBuf {
    let mut p = dirs_home();
    p.push(".funbutton");
    let _ = std::fs::create_dir_all(&p);
    p.push("settings.json");
    p
}

fn dirs_home() -> std::path::PathBuf {
    if let Ok(h) = std::env::var("HOME") {
        return std::path::PathBuf::from(h);
    }
    std::path::PathBuf::from(".")
}

fn load_settings() -> Settings {
    let p = settings_path();
    if let Ok(bytes) = std::fs::read(&p) {
        if let Ok(s) = serde_json::from_slice::<Settings>(&bytes) {
            // Refresh API key from env if file's is empty
            if s.groq_api_key.is_empty() {
                let mut s = s;
                if let Ok(env_key) = std::env::var("GROQ_API_KEY") {
                    s.groq_api_key = env_key;
                }
                return s;
            }
            return s;
        }
    }
    Settings::default()
}

fn persist(s: &Settings) -> anyhow::Result<()> {
    let p = settings_path();
    let json = serde_json::to_vec_pretty(s)?;
    std::fs::write(p, json)?;
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let first_run = !settings_path().exists();
    let initial = load_settings();
    let history_db_path = {
        let mut p = dirs_home();
        p.push(".funbutton");
        p.push("history.db");
        p
    };
    let history_arc = match history::History::open(history_db_path) {
        Ok(h) => Arc::new(h),
        Err(e) => {
            log::error!("history db unavailable: {e:#}");
            // Fall back to an in-memory db so the rest of the app keeps running.
            Arc::new(history::History::open(std::path::PathBuf::from(":memory:"))
                .expect("in-memory sqlite"))
        }
    };
    // Apply retention purge on startup.
    let _ = history_arc.purge_older_than(initial.history_retention_days);
    let app_state: AppStateHandle = AppState::new(initial, Arc::clone(&history_arc));

    // Hotkey channel — pick the listener based on user setting (default: Fn).
    let (tx, rx) = mpsc::channel::<HotkeyEvent>();
    match app_state.settings.lock().hotkey_kind {
        HotkeyKind::Fn => {
            log::info!("hotkey: Fn (CGEventTap)");
            fn_hotkey::spawn_listener(tx);
        }
        HotkeyKind::RightOption => {
            log::info!("hotkey: Right Option (rdev)");
            hotkey::spawn_listener(tx);
        }
    }

    // Shared recorder (one at a time)
    let recorder: Arc<PMutex<Recorder>> = Arc::new(PMutex::new(Recorder::new()));

    let app_state_clone = Arc::clone(&app_state);
    let recorder_clone = Arc::clone(&recorder);

    let state_for_shortcut = Arc::clone(&app_state);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_macos_permissions::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    use tauri_plugin_global_shortcut::{Code, ShortcutState};
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }
                    match shortcut.key {
                        Code::KeyV => {
                            let last = state_for_shortcut.last_cleaned.lock().clone();
                            if !last.is_empty() {
                                std::thread::spawn(move || {
                                    let _ = inject::paste_text(&last);
                                });
                            }
                        }
                        Code::KeyH => {
                            if let Some(w) = app.get_webview_window("settings") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                            let _ = app.emit("funbutton:open-history", ());
                        }
                        _ => {}
                    }
                })
                .build(),
        )
        .manage(Arc::clone(&app_state))
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            get_status,
            get_last_transcript,
            ollama_check,
            open_settings,
            open_onboarding,
            close_onboarding,
            open_system_settings_panel,
            validate_groq_key,
            history_list,
            history_copy,
            history_purge_now,
            history_last_failed,
        ])
        .setup(move |app| {
            // Menu bar app. Pill always hidden until recording. Settings hidden
            // by default. Onboarding shown if user hasn't completed it.
            if let Some(w) = app.get_webview_window("pill") {
                let _ = w.hide();
            }
            if let Some(w) = app.get_webview_window("settings") {
                let _ = w.hide();
            }
            let needs_onboarding = first_run || !app_state.settings.lock().onboarded;
            if let Some(w) = app.get_webview_window("onboarding") {
                if needs_onboarding {
                    let _ = w.show();
                    let _ = w.set_focus();
                } else {
                    let _ = w.hide();
                }
            }

            // Register global shortcuts: Cmd+Shift+V (re-paste) and Cmd+Shift+H (open history).
            {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
                let mods = Some(Modifiers::SUPER | Modifiers::SHIFT);
                let v_shortcut = Shortcut::new(mods, Code::KeyV);
                let h_shortcut = Shortcut::new(mods, Code::KeyH);
                if let Err(e) = app.global_shortcut().register(v_shortcut) {
                    log::warn!("Cmd+Shift+V registration failed: {e:#}");
                }
                if let Err(e) = app.global_shortcut().register(h_shortcut) {
                    log::warn!("Cmd+Shift+H registration failed: {e:#}");
                }
            }

            // System tray
            let handle = app.handle().clone();
            let open_item = MenuItem::with_id(&handle, "open", "Settings", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(&handle, "quit", "Quit FunButton", true, None::<&str>)?;
            let menu = Menu::with_items(&handle, &[&open_item, &quit_item])?;
            let tray: TrayIcon = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .icon_as_template(true)
                .tooltip("FunButton — hold Fn to dictate · ⌘⇧V re-paste · ⌘⇧H history")
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "open" => {
                        if let Some(w) = app.get_webview_window("settings") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;

            // Spawn the hotkey-event handler thread.
            let app_handle = app.handle().clone();
            std::thread::spawn(move || {
                handle_hotkey_loop(app_handle, app_state_clone, recorder_clone, rx, tray);
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Hide instead of closing — we live in the tray.
                let label = window.label();
                if label == "settings" || label == "onboarding" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Touch the unused state ref so the compiler doesn't complain.
    let _ = app_state;
}

fn handle_hotkey_loop(
    app: AppHandle,
    state: AppStateHandle,
    recorder: Arc<PMutex<Recorder>>,
    rx: mpsc::Receiver<HotkeyEvent>,
    tray: TrayIcon,
) {
    // Tokio runtime for async pipeline calls.
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    // Pre-warm the Groq HTTP/2 connection so the first dictation pays no
    // TCP+TLS handshake cost. Best-effort — failures are fine.
    {
        let key = state.settings.lock().groq_api_key.clone();
        rt.spawn(async move { groq::prewarm(&key).await });
    }

    while let Ok(ev) = rx.recv() {
        match ev {
            HotkeyEvent::Down => {
                let mut rec = recorder.lock();
                match rec.start() {
                    Ok(()) => {
                        *state.status.lock() = Status::Recording;
                        let _ = tray.set_tooltip(Some("FunButton — recording"));
                        emit_status(&app, Status::Recording, None);
                        if let Some(p) = app.get_webview_window("pill") {
                            let _ = p.show();
                        }
                    }
                    Err(e) => {
                        log::error!("recorder start failed: {e:#}");
                        *state.status.lock() = Status::Error;
                        emit_status(&app, Status::Error, Some(format!("audio: {e}")));
                    }
                }
            }
            HotkeyEvent::Up => {
                let (wav, audio_duration_ms) = {
                    let mut rec = recorder.lock();
                    if rec.sample_count() < 4_000 {
                        // Less than ~50ms at 48kHz stereo — likely an accidental tap.
                        let _ = rec.stop_and_encode_wav();
                        *state.status.lock() = Status::Idle;
                        let _ = tray.set_tooltip(Some("FunButton — hold Fn to dictate"));
                        if let Some(p) = app.get_webview_window("pill") {
                            let _ = p.hide();
                        }
                        emit_status(&app, Status::Idle, Some("too short".into()));
                        continue;
                    }
                    let dur = rec.duration_ms() as i64;
                    let bytes = match rec.stop_and_encode_wav() {
                        Ok(b) => b,
                        Err(e) => {
                            log::error!("recorder stop failed: {e:#}");
                            *state.status.lock() = Status::Error;
                            emit_status(&app, Status::Error, Some(format!("encode: {e}")));
                            continue;
                        }
                    };
                    (bytes, dur)
                };

                let app_h = app.clone();
                let state_h = Arc::clone(&state);
                let tray_h = tray.clone();
                rt.spawn(async move {
                    let _ = tray_h.set_tooltip(Some("FunButton — transcribing"));
                    let result = pipeline::run(Arc::clone(&state_h), wav).await;
                    if let Some(p) = app_h.get_webview_window("pill") {
                        let _ = p.hide();
                    }
                    match result {
                        Ok(r) => {
                            *state_h.status.lock() = Status::Pasting;
                            emit_status(&app_h, Status::Pasting, None);
                            let to_paste = r.cleaned.clone();
                            *state_h.last_cleaned.lock() = to_paste.clone();
                            let words = to_paste.split_whitespace().count();

                            // Insert history row BEFORE paste, so even if paste fails the
                            // cleaned text is preserved.
                            let frontmost = crate::app_detect::FrontApp::detect().label();
                            let model_used = match r.backend_used {
                                "local" => format!("ollama-{}", state_h.settings.lock().ollama_model),
                                _ => format!("groq-{}", crate::groq::LLAMA_MODEL),
                            };
                            let history_id = match state_h.history.insert_pre_paste(
                                &r.raw,
                                &r.cleaned,
                                r.mode,
                                Some(&frontmost),
                                Some(audio_duration_ms),
                                &model_used,
                            ) {
                                Ok(id) => Some(id),
                                Err(e) => {
                                    log::error!("history insert failed: {e:#}");
                                    None
                                }
                            };

                            // bump words_today
                            {
                                let today = chrono_today();
                                let mut s = state_h.settings.lock();
                                if s.words_today_date != today {
                                    s.words_today = 0;
                                    s.words_today_date = today;
                                }
                                s.words_today += words as u64;
                                let _ = persist(&s);
                            }

                            // Inject on a non-async thread, capture the outcome,
                            // then update history + notify on failure.
                            let app_inject = app_h.clone();
                            let state_inject = Arc::clone(&state_h);
                            let cleaned_for_paste = to_paste.clone();
                            let history_id_paste = history_id;
                            std::thread::spawn(move || {
                                use inject::PasteOutcome;
                                let outcome = inject::paste_text(&cleaned_for_paste);
                                let success = matches!(outcome, PasteOutcome::Pasted);
                                if let Some(id) = history_id_paste {
                                    let _ = state_inject.history.mark_paste_result(id, success);
                                }
                                if let PasteOutcome::Failed(reason) = outcome {
                                    log::warn!("paste failed: {reason}");
                                    use tauri_plugin_notification::NotificationExt;
                                    let _ = app_inject
                                        .notification()
                                        .builder()
                                        .title("FunButton — paste blocked")
                                        .body("Cleaned text is on your clipboard. Press ⌘V to paste manually, or open History to copy it.")
                                        .show();
                                    let _ = app_inject.emit("funbutton:paste-failed", ());
                                }
                            });

                            let _ = app_h.emit(
                                "funbutton:result",
                                PipelinePayload {
                                    raw: r.raw,
                                    cleaned: r.cleaned,
                                    mode: r.mode.to_string(),
                                    backend: r.backend_used.to_string(),
                                    word_count: words,
                                },
                            );
                            *state_h.status.lock() = Status::Idle;
                            let _ = tray_h.set_tooltip(Some("FunButton — hold Fn to dictate"));
                            emit_status(&app_h, Status::Idle, None);
                        }
                        Err(e) => {
                            log::error!("pipeline failed: {e:#}");
                            *state_h.status.lock() = Status::Error;
                            let _ = tray_h.set_tooltip(Some("FunButton — error"));
                            emit_status(&app_h, Status::Error, Some(format!("{e:#}")));
                        }
                    }
                });
            }
        }
    }
}

fn chrono_today() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Rough YYYY-MM-DD using days since epoch — good enough for daily counter rollover.
    let days = (secs / 86400) as i64;
    let (y, m, d) = days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}")
}

fn days_to_ymd(mut days: i64) -> (i64, u32, u32) {
    days += 719468;
    let era = if days >= 0 { days } else { days - 146096 } / 146097;
    let doe = (days - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
