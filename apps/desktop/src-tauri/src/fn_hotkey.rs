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

#[cfg(target_os = "macos")]
use crate::hotkey::HotkeyEvent;

#[cfg(target_os = "macos")]
pub fn spawn_listener(tx: std::sync::mpsc::Sender<HotkeyEvent>) {
    use core_foundation::runloop::CFRunLoop;
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType, CallbackResult,
    };
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    std::thread::spawn(move || {
        let last = Arc::new(AtomicBool::new(false));
        let last_cb = Arc::clone(&last);
        let tx_cb = tx.clone();

        let result = CGEventTap::with_enabled(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::ListenOnly,
            vec![CGEventType::FlagsChanged],
            move |_proxy, _etype, event| {
                let flags = event.get_flags();
                let fn_now = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
                let fn_was = last_cb.swap(fn_now, Ordering::SeqCst);
                if fn_now && !fn_was {
                    let _ = tx_cb.send(HotkeyEvent::Down);
                } else if !fn_now && fn_was {
                    let _ = tx_cb.send(HotkeyEvent::Up);
                }
                CallbackResult::Keep
            },
            || {
                log::info!("Fn key tap installed; running CFRunLoop");
                CFRunLoop::run_current();
            },
        );

        match result {
            Ok(_) => log::info!("Fn key tap exited"),
            Err(_) => log::error!(
                "Fn key tap creation failed — Input Monitoring permission likely not granted yet. \
                 macOS Settings → Privacy & Security → Input Monitoring → enable FunButton."
            ),
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn spawn_listener(_tx: std::sync::mpsc::Sender<crate::hotkey::HotkeyEvent>) {
    log::warn!("Fn key listener is macOS-only; falling back at runtime");
}
