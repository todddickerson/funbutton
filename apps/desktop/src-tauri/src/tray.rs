//! State-aware menu-bar tray.
//!
//! The tray is the whole face of a menu-bar app, so it earns real state:
//! a recording dot in the menu bar itself, live engine health, a mode
//! quick-switch, and one-click access to the last dictation. Peg: Handy's
//! power-user depth + Wispr's polish; today's Settings+Quit menu was an
//! afterthought.
//!
//! Threading discipline (macOS 26): every AppKit mutation — title, tooltip,
//! menu construction/swap — is funneled through [`sync`], which hops to the
//! main thread via `run_on_main_thread`. Off-main-thread AppKit/TSM calls are
//! exactly the class of bug that SIGTRAP-crashed v0.1.3. This module uses no
//! layout APIs (no TIS/TSM/UCKeyTranslate) — status glyphs are plain
//! typographic characters (● ○ ◌ ✓ ✕ →), not emoji.

use crate::inject;
use crate::state::{AppStateHandle, HotkeyKind, ModeOverride, Status};
use parking_lot::Mutex;
use std::time::Duration;
use tauri::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, Wry,
};
use tauri_plugin_autostart::ManagerExt as _;

/// Stable id so any thread can look the tray up via `app.tray_by_id`.
pub const TRAY_ID: &str = "main";

/// Everything the tray renders, captured in one comparable value. `sync`
/// skips all AppKit work when nothing changed, so callers can invoke it on
/// every state transition (and the warm-up watcher every second) for free.
#[derive(Clone, PartialEq)]
struct Snapshot {
    status: Status,
    stt: &'static str,
    cleanup: &'static str,
    mode: ModeOverride,
    hotkey: HotkeyKind,
    has_last: bool,
    last_words: usize,
    words_today: u64,
    has_cloud: bool,
    autostart: bool,
}

/// Last snapshot the tray rendered. Managed app state; written only on the
/// main thread inside `sync`.
#[derive(Default)]
struct TrayMenuCache {
    last: Mutex<Option<Snapshot>>,
}

/// Build the tray. Called once from `setup` (main thread).
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(TrayMenuCache::default());
    let snap = snapshot(app);
    let menu = build_menu(app, &snap)?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(app.default_window_icon().expect("window icon").clone())
        .icon_as_template(true)
        .tooltip(tooltip_text(&snap))
        .menu(&menu)
        // Registered once; Tauri routes menu events globally by item id, so
        // menus swapped in later via `set_menu` keep hitting this handler.
        .on_menu_event(on_menu_event)
        .build(app)?;
    *app.state::<TrayMenuCache>().last.lock() = Some(snap);
    spawn_warmup_watcher(app.clone());
    Ok(())
}

/// Refresh title + tooltip + menu from current app state. Safe to call from
/// any thread — all AppKit work happens in a main-thread closure, and the
/// snapshot dedupe makes redundant calls near-free.
pub fn sync(app: &AppHandle) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = app2.tray_by_id(TRAY_ID) else {
            return;
        };
        let snap = snapshot(&app2);
        {
            let cache = app2.state::<TrayMenuCache>();
            let mut last = cache.last.lock();
            if last.as_ref() == Some(&snap) {
                return;
            }
            *last = Some(snap.clone());
        }
        // The menu bar itself signals live state: ● while the mic is hot,
        // … while the pipeline runs, ! until the next successful run.
        let title: Option<&str> = match snap.status {
            Status::Recording => Some("●"),
            Status::Transcribing | Status::Cleaning | Status::Pasting => Some("…"),
            Status::Error => Some("!"),
            Status::Idle => None,
        };
        let _ = tray.set_title(title);
        let _ = tray.set_tooltip(Some(tooltip_text(&snap)));
        match build_menu(&app2, &snap) {
            Ok(menu) => {
                let _ = tray.set_menu(Some(menu));
            }
            Err(e) => log::warn!("tray menu rebuild failed: {e:#}"),
        }
    });
}

/// Engine warm-up takes a few seconds (GGUF load into page cache). Poll until
/// both on-device engines reach a terminal state so the health lines go live,
/// then stop burning cycles.
fn spawn_warmup_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        for _ in 0..240 {
            std::thread::sleep(Duration::from_secs(1));
            sync(&app);
            let st = app.state::<AppStateHandle>();
            let stt_done = st.stt.status().label() != "starting";
            let llm_done = st.embedded.lock().is_some() || st.embedded_error.lock().is_some();
            if stt_done && llm_done {
                break;
            }
        }
        sync(&app);
    });
}

fn snapshot(app: &AppHandle) -> Snapshot {
    let st = app.state::<AppStateHandle>();
    let (mode, hotkey, words_today, has_cloud) = {
        let s = st.settings.lock();
        (
            s.mode_override,
            s.hotkey_kind,
            s.words_today,
            !s.groq_api_key.trim().is_empty() || !s.license_jwt.trim().is_empty(),
        )
    };
    let status = *st.status.lock();
    let stt = st.stt.status().label();
    let cleanup = if st.embedded.lock().is_some() {
        "ready"
    } else if st.embedded_error.lock().is_some() {
        "failed"
    } else {
        "starting"
    };
    let (has_last, last_words) = {
        let l = st.last_cleaned.lock();
        (!l.is_empty(), l.split_whitespace().count())
    };
    let autostart = app.autolaunch().is_enabled().unwrap_or(false);
    Snapshot {
        status,
        stt,
        cleanup,
        mode,
        hotkey,
        has_last,
        last_words,
        words_today,
        has_cloud,
        autostart,
    }
}

fn build_menu(app: &AppHandle, s: &Snapshot) -> tauri::Result<Menu<Wry>> {
    // Telemetry block — deliberately lowercase, reads like log output.
    // Healthy/warming lines are read-only; a failed engine line becomes a
    // live affordance (click → Settings) instead of a read-only lament.
    let status_line = MenuItem::with_id(app, "st_status", status_text(s), false, None::<&str>)?;
    let stt_line = MenuItem::with_id(app, "st_stt", stt_text(s), s.stt == "failed", None::<&str>)?;
    let llm_line = MenuItem::with_id(
        app,
        "st_llm",
        cleanup_text(s),
        s.cleanup == "failed",
        None::<&str>,
    )?;

    // Mode quick-switch — radio via CheckMenuItems; persists through the
    // same settings save path the Settings window uses.
    let modes: [(&str, ModeOverride, &str); 5] = [
        (
            "mode_auto",
            ModeOverride::Auto,
            "Auto (match the front app)",
        ),
        ("mode_code", ModeOverride::Code, "Code"),
        ("mode_email", ModeOverride::Email, "Email"),
        ("mode_slack", ModeOverride::Slack, "Slack"),
        ("mode_raw", ModeOverride::Raw, "Raw (verbatim, no cleanup)"),
    ];
    let mut mode_items: Vec<CheckMenuItem<Wry>> = Vec::with_capacity(modes.len());
    for (id, m, label) in modes {
        mode_items.push(CheckMenuItem::with_id(
            app,
            id,
            label,
            true,
            s.mode == m,
            None::<&str>,
        )?);
    }
    let mode_refs: Vec<&dyn IsMenuItem<Wry>> = mode_items
        .iter()
        .map(|i| i as &dyn IsMenuItem<Wry>)
        .collect();
    let mode_menu = Submenu::with_id_and_items(
        app,
        "mode_menu",
        format!("Mode: {}", mode_label(s.mode)),
        true,
        &mode_refs,
    )?;

    let copy_text = if s.has_last {
        format!(
            "Copy Last Dictation ({} {})",
            s.last_words,
            if s.last_words == 1 { "word" } else { "words" }
        )
    } else {
        "Copy Last Dictation".to_string()
    };
    let copy_item = MenuItem::with_id(app, "copy_last", copy_text, s.has_last, None::<&str>)?;
    let history_item = MenuItem::with_id(
        app,
        "history",
        "Open History",
        true,
        Some("CmdOrCtrl+Shift+H"),
    )?;
    let settings_item = MenuItem::with_id(app, "open", "Settings…", true, None::<&str>)?;
    let onboarding_item = MenuItem::with_id(
        app,
        "replay_onboarding",
        "Replay Onboarding",
        true,
        None::<&str>,
    )?;
    let autostart_item = CheckMenuItem::with_id(
        app,
        "autostart",
        "Start at Login",
        true,
        s.autostart,
        None::<&str>,
    )?;
    let footer = MenuItem::with_id(app, "st_footer", footer_text(app, s), false, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit FunButton", true, None::<&str>)?;

    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;
    let sep3 = PredefinedMenuItem::separator(app)?;
    let sep4 = PredefinedMenuItem::separator(app)?;

    Menu::with_items(
        app,
        &[
            &status_line,
            &stt_line,
            &llm_line,
            &sep1,
            &mode_menu,
            &sep2,
            &copy_item,
            &history_item,
            &sep3,
            &settings_item,
            &onboarding_item,
            &autostart_item,
            &sep4,
            &footer,
            &quit_item,
        ],
    )
}

fn on_menu_event(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "open" => show_window(app, "settings"),
        // Engine health lines are only enabled while that engine is in the
        // failed state; clicking one jumps straight to Settings to fix it.
        "st_stt" | "st_llm" => show_window(app, "settings"),
        "history" => {
            show_window(app, "settings");
            let _ = app.emit("funbutton:open-history", ());
        }
        "replay_onboarding" => {
            // The onboarding window is hidden-not-destroyed on close, so the
            // webview still sits on whatever step the user last saw. Reload
            // it before showing so the wizard genuinely restarts at step 1.
            if let Some(w) = app.get_webview_window("onboarding") {
                let _ = w.eval("window.location.reload()");
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "copy_last" => {
            let st = app.state::<AppStateHandle>();
            let last = st.last_cleaned.lock().clone();
            if !last.is_empty() {
                if let Err(e) = inject::set_clipboard(&last) {
                    log::warn!("copy last dictation failed: {e:#}");
                }
            }
        }
        "autostart" => {
            let al = app.autolaunch();
            let enabled = al.is_enabled().unwrap_or(false);
            let res = if enabled { al.disable() } else { al.enable() };
            if let Err(e) = res {
                log::warn!("autostart toggle failed: {e}");
            }
            sync(app);
        }
        id @ ("mode_auto" | "mode_code" | "mode_email" | "mode_slack" | "mode_raw") => {
            let mode = match id {
                "mode_code" => ModeOverride::Code,
                "mode_email" => ModeOverride::Email,
                "mode_slack" => ModeOverride::Slack,
                "mode_raw" => ModeOverride::Raw,
                _ => ModeOverride::Auto,
            };
            let st = app.state::<AppStateHandle>();
            {
                let mut s = st.settings.lock();
                s.mode_override = mode;
                if let Err(e) = crate::persist(&s) {
                    log::warn!("persist after tray mode switch failed: {e:#}");
                }
            }
            // Let an open Settings window pick up the change.
            let _ = app.emit("funbutton:settings-changed", ());
            sync(app);
        }
        "quit" => {
            // Same ordered teardown as the AppleEvent-quit path. `app.exit(0)`
            // then unwinds the event loop and fires RunEvent::Exit, which calls
            // `shutdown` again — a no-op the second time (it is idempotent).
            // Doing it here too means the tray path is safe even if a future
            // runtime change stops emitting RunEvent::Exit for `app.exit`.
            crate::shutdown(app);
            app.exit(0);
        }
        _ => {}
    }
}

fn show_window(app: &AppHandle, label: &str) {
    if let Some(w) = app.get_webview_window(label) {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

fn status_text(s: &Snapshot) -> String {
    match s.status {
        Status::Idle => format!("○ idle — hold {} to talk", hotkey_short(s.hotkey)),
        Status::Recording => "● recording — release to ship it".to_string(),
        Status::Transcribing => "◌ transcribing…".to_string(),
        Status::Cleaning => "◌ cleaning up…".to_string(),
        Status::Pasting => "→ pasting…".to_string(),
        Status::Error => "✕ last run failed — try again".to_string(),
    }
}

fn stt_text(s: &Snapshot) -> &'static str {
    match s.stt {
        "ready" => "✓ whisper — on-device, ready",
        // Failed lines are enabled (click → Settings), so the label says so.
        "failed" => {
            if s.has_cloud {
                "✕ whisper — down, using cloud · click to fix"
            } else {
                "✕ whisper — down, no fallback · click to fix"
            }
        }
        _ => "◌ whisper — warming up…",
    }
}

fn cleanup_text(s: &Snapshot) -> &'static str {
    match s.cleanup {
        "ready" => "✓ cleanup — qwen 1.5B, local",
        "failed" => "✕ cleanup — down, raw fallback · click to fix",
        _ => "◌ cleanup — warming up…",
    }
}

fn tooltip_text(s: &Snapshot) -> String {
    match s.status {
        Status::Recording => "FunButton — recording (release to paste)".to_string(),
        Status::Transcribing | Status::Cleaning | Status::Pasting => {
            "FunButton — working…".to_string()
        }
        Status::Error => "FunButton — last dictation failed".to_string(),
        Status::Idle => format!(
            "FunButton — hold {} to dictate · ⌘⇧V re-paste · ⌘⇧H history",
            hotkey_short(s.hotkey)
        ),
    }
}

fn footer_text(app: &AppHandle, s: &Snapshot) -> String {
    format!(
        "v{} · GPLv3 · {} words today",
        app.package_info().version,
        fmt_thousands(s.words_today)
    )
}

fn hotkey_short(k: HotkeyKind) -> &'static str {
    match k {
        HotkeyKind::Fn => "Fn",
        HotkeyKind::RightOption => "Right ⌥",
        HotkeyKind::LeftOption => "Left ⌥",
        HotkeyKind::RightControl => "Right ⌃",
        HotkeyKind::LeftControl => "Left ⌃",
        HotkeyKind::RightCommand => "Right ⌘",
        HotkeyKind::LeftCommand => "Left ⌘",
        HotkeyKind::CapsLock => "Caps Lock",
    }
}

fn mode_label(m: ModeOverride) -> &'static str {
    match m {
        ModeOverride::Auto => "Auto",
        ModeOverride::Code => "Code",
        ModeOverride::Email => "Email",
        ModeOverride::Slack => "Slack",
        ModeOverride::Raw => "Raw",
    }
}

fn fmt_thousands(n: u64) -> String {
    let raw = n.to_string();
    let mut out = String::with_capacity(raw.len() + raw.len() / 3);
    for (i, c) in raw.chars().enumerate() {
        if i > 0 && (raw.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}
