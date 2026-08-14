//! Fn-key push-to-talk via CGEventTap.
//!
//! macOS does not expose the Fn key as a normal modifier — general keycode
//! crates and `tauri-plugin-global-shortcut` cannot bind it. The standard
//! route used by
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
use crate::hotkey::{CaptureState, HotkeyEvent};
#[cfg(target_os = "macos")]
use crate::state::HotkeyKind;

#[cfg(target_os = "macos")]
use std::sync::atomic::AtomicU8;
#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
pub fn spawn_listener(
    tx: std::sync::mpsc::Sender<HotkeyEvent>,
    armed: Arc<AtomicU8>,
    capture: Arc<CaptureState>,
) {
    use core_foundation::runloop::CFRunLoop;
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType, CallbackResult,
    };
    use std::sync::atomic::{AtomicBool, Ordering};

    std::thread::spawn(move || {
        // Defensive isolation: a Rust-level panic (crate is built panic=unwind)
        // in the tap callback or runloop logs loudly and lets this thread die
        // instead of killing the whole app. A hard trap (e.g. an off-main-thread
        // HIToolbox assertion on macOS 26) is NOT a Rust panic and can't be
        // caught here — this tap deliberately touches no layout APIs, so there is
        // nothing to trap.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let last = Arc::new(AtomicBool::new(false));
            // Tap creation fails while Input Monitoring is denied (TCC checks
            // happen at CGEventTapCreate time). Retrying means a grant made
            // mid-onboarding arms the Fn key within seconds — no relaunch. The
            // failure is logged once; retries are silent.
            let mut warned = false;
            loop {
                let last_cb = Arc::clone(&last);
                let tx_cb = tx.clone();
                let armed_cb = Arc::clone(&armed);
                let capture_cb = Arc::clone(&capture);

                let result = CGEventTap::with_enabled(
                    CGEventTapLocation::HID,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::ListenOnly,
                    vec![CGEventType::FlagsChanged],
                    move |_proxy, _etype, event| {
                        let flags = event.get_flags();
                        let fn_now = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
                        let fn_was = last_cb.swap(fn_now, Ordering::SeqCst);
                        // "Press the key you want" capture: report the Fn
                        // keydown and stand down. Never drives the pipeline.
                        if capture_cb.active.load(Ordering::SeqCst) {
                            if fn_now && !fn_was {
                                if let Some(ctx) = capture_cb.tx.lock().as_ref() {
                                    let _ = ctx.send(HotkeyKind::Fn.as_u8());
                                }
                                capture_cb.active.store(false, Ordering::SeqCst);
                                log::info!("hotkey capture: Fn pressed");
                            }
                            return CallbackResult::Keep;
                        }
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
                        log::info!(
                            "Fn key tap installed (Input Monitoring granted); running CFRunLoop"
                        );
                        CFRunLoop::run_current();
                    },
                );

                match result {
                    Ok(_) => {
                        // Runloop stopped (shouldn't happen in normal operation) —
                        // reinstall after a pause rather than losing the hotkey.
                        log::warn!("Fn key tap runloop exited; reinstalling in 3s");
                        warned = false;
                    }
                    Err(_) => {
                        if !warned {
                            warned = true;
                            log::error!(
                            "Fn key tap creation FAILED — Input Monitoring permission not granted. \
                             macOS Settings → Privacy & Security → Input Monitoring → enable FunButton \
                             (takes effect within seconds; retrying every 3s), \
                             OR switch to Right Option in Settings."
                        );
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }));

        // The loop above never returns normally, so reaching here means a panic
        // unwound out of it.
        if outcome.is_err() {
            log::error!(
                "Fn listener thread panicked and exited; Fn hotkey unavailable until app restart \
               (other listeners and the rest of the app keep running)."
            );
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn spawn_listener(
    _tx: std::sync::mpsc::Sender<crate::hotkey::HotkeyEvent>,
    _armed: std::sync::Arc<std::sync::atomic::AtomicU8>,
    _capture: std::sync::Arc<crate::hotkey::CaptureState>,
) {
    log::warn!("Fn key listener is macOS-only; falling back at runtime");
}
