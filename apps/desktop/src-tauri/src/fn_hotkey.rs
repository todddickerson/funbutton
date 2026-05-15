//! Fn-key push-to-talk via CGEventTap.
//!
//! macOS does not expose the Fn key as a normal modifier — `rdev` and
//! `tauri-plugin-global-shortcut` cannot bind it. The standard route used by
//! Hyperkey, Karabiner-Elements, Raycast Hotkey, and others is a CGEventTap
//! at the HID layer, listening for `flagsChanged` events and inspecting the
//! `CGEventFlagSecondaryFn` (0x00800000) bit.
//!
//! This requires the **Input Monitoring** permission (separate from
//! Accessibility). On first run macOS will prompt; if the user denies, the
//! tap is created but no events arrive. We surface that case with a log line
//! at install time. Users can re-grant in System Settings → Privacy &
//! Security → Input Monitoring.
//!
//! The runloop blocks the spawning thread, so we run on a dedicated thread.
//!
//! The listener takes an `armed: Arc<AtomicU8>` filter — both Fn and Right
//! Option listeners run simultaneously, but only the one whose kind matches
//! the atomic emits events. This lets the UI hot-swap the hotkey without an
//! app restart.

#[cfg(target_os = "macos")]
use crate::hotkey::HotkeyEvent;
#[cfg(target_os = "macos")]
use crate::state::HotkeyKind;

#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicU8;
#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
pub fn spawn_listener(tx: std::sync::mpsc::Sender<HotkeyEvent>, armed: Arc<AtomicU8>) {
    use core_foundation::runloop::CFRunLoop;
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType, CallbackResult,
    };
    use std::sync::atomic::{AtomicBool, Ordering};

    std::thread::spawn(move || {
        let last = Arc::new(AtomicBool::new(false));
        let last_cb = Arc::clone(&last);
        let tx_cb = tx.clone();
        let armed_cb = Arc::clone(&armed);

        let result = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::FlagsChanged],
            move |_proxy, _etype, event| {
                let flags = event.get_flags();
                let fn_now = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
                let fn_was = last_cb.swap(fn_now, Ordering::SeqCst);
                // Only emit if the Fn hotkey is the currently armed one.
                let armed_kind = HotkeyKind::from_u8(armed_cb.load(Ordering::SeqCst));
                if !matches!(armed_kind, HotkeyKind::Fn) {
                    return CallbackResult::Keep;
                }
                if fn_now && !fn_was {
                    log::info!("hotkey: Fn DOWN");
                    let _ = tx_cb.send(HotkeyEvent::Down);
                } else if !fn_now && fn_was {
                    log::info!("hotkey: Fn UP");
                    let _ = tx_cb.send(HotkeyEvent::Up);
                }
                CallbackResult::Keep
            },
            || {
                log::info!("Fn key tap installed (Input Monitoring granted); running CFRunLoop");
                CFRunLoop::run_current();
            },
        );

        match result {
            Ok(_) => log::info!("Fn key tap exited"),
            Err(_) => log::error!(
                "Fn key tap creation FAILED — Input Monitoring permission likely not granted. \
                 macOS Settings → Privacy & Security → Input Monitoring → enable FunButton, \
                 OR switch to Right Option in Settings."
            ),
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn spawn_listener(
    _tx: std::sync::mpsc::Sender<crate::hotkey::HotkeyEvent>,
    _armed: std::sync::Arc<std::sync::atomic::AtomicU8>,
) {
    log::warn!("Fn key listener is macOS-only; falling back at runtime");
}
