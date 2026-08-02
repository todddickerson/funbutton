//! Right Option push-to-talk via CGEventTap.
//!
//! This listener used to be built on a general keycode-listening crate whose
//! macOS backend mapped each key event's keycode to a character *inside the
//! tap callback* using the Text Services input-source APIs (TIS/TSM). macOS 26
//! hard-enforces `dispatch_assert_queue(main)` inside
//! `TSMGetInputSourceProperty` → `islGetInputSourceListWithAdditions`, so
//! calling it from our spawned listener thread traps
//! (EXC_BREAKPOINT / SIGTRAP) the instant the first key event arrives. The app
//! crashed ~15s into onboarding on M4 / macOS 26.6; macOS 15 silently allowed
//! the off-main-thread call, so it never reproduced here.
//!
//! The fix mirrors `fn_hotkey.rs`: a raw CGEventTap at the HID layer listening
//! for `FlagsChanged`, inspecting only the raw virtual keycode and modifier
//! flags — never a layout/input-source API. Right Option is virtual keycode
//! `0x3D`; the `CGEventFlagAlternate` bit says whether an Option key is
//! currently held. Reading keycode + flags needs no HIToolbox call and is
//! main-thread-agnostic, so it is safe on every macOS version.
//!
//! Unlike the old listener (which needed Accessibility), this needs **Input
//! Monitoring**, the same permission as the Fn tap. Both listeners share the
//! `armed` atomic so the UI can hot-swap the active hotkey without a restart.

#[cfg(target_os = "macos")]
use crate::state::HotkeyKind;

use std::sync::atomic::AtomicU8;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down,
    Up,
}

/// Right Option virtual keycode (`kVK_RightOption`). Left Option is `0x3A` and
/// is deliberately ignored — only the right key drives push-to-talk.
#[cfg(target_os = "macos")]
const RIGHT_OPTION_KEYCODE: i64 = 0x3D;

/// Spawn the Right Option listener on a dedicated thread.
/// Sends `HotkeyEvent::Down` on Right Option press, `Up` on release.
///
/// The listener only emits events when `armed` matches `HotkeyKind::RightOption`
/// — both listeners run simultaneously so the user can hot-swap the active
/// hotkey from the Settings UI without restarting the app.
#[cfg(target_os = "macos")]
pub fn spawn_listener(tx: Sender<HotkeyEvent>, armed: Arc<AtomicU8>) {
    use core_foundation::runloop::CFRunLoop;
    use core_graphics::event::{
        CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement,
        CGEventType, CallbackResult, EventField,
    };
    use std::sync::atomic::{AtomicBool, Ordering};

    thread::spawn(move || {
        // Defensive isolation: if anything in the tap callback or runloop
        // panics (Rust unwind — the crate is built panic=unwind), log loudly
        // and let this thread die instead of taking the whole app down. A hard
        // trap like the macOS 26 HIToolbox assertion is NOT a Rust panic and
        // cannot be caught here — which is exactly why we removed the layout
        // API that caused it rather than relying on this net.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            // Tracks whether Right Option is currently held, derived from the
            // Alternate flag observed at each Right-Option keycode edge.
            let held = Arc::new(AtomicBool::new(false));
            // Tap creation fails while Input Monitoring is denied (TCC checks
            // happen at CGEventTapCreate time). Retrying means a grant made
            // mid-onboarding arms the key within seconds — no relaunch. The
            // failure is logged once; retries are silent.
            let mut warned = false;
            loop {
                let held_cb = Arc::clone(&held);
                let tx_cb = tx.clone();
                let armed_cb = Arc::clone(&armed);

                let result = CGEventTap::with_enabled(
                    CGEventTapLocation::HID,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::ListenOnly,
                    vec![CGEventType::FlagsChanged],
                    move |_proxy, _etype, event| {
                        // Only the Right Option keycode drives the state
                        // machine. Every other modifier (Left Option, Shift,
                        // Cmd, Ctrl, Fn…) fires FlagsChanged with a *different*
                        // keycode, so pressing/releasing them while Right
                        // Option is held never re-fires DOWN and never fires a
                        // spurious UP. Reading the keycode field is a plain
                        // integer lookup — no TIS/TSM layout call.
                        let keycode =
                            event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                        if keycode != RIGHT_OPTION_KEYCODE {
                            return CallbackResult::Keep;
                        }
                        // On the Right-Option keycode edge, the Alternate bit
                        // disambiguates DOWN (bit now set) from UP (bit now
                        // clear). We track state unconditionally (like the Fn
                        // tap) so a hot-swap mid-press can't strand a stale
                        // edge; we only *emit* when Right Option is armed.
                        let alt_now =
                            event.get_flags().contains(CGEventFlags::CGEventFlagAlternate);
                        let alt_was = held_cb.swap(alt_now, Ordering::SeqCst);

                        let armed_kind = HotkeyKind::from_u8(armed_cb.load(Ordering::SeqCst));
                        if !matches!(armed_kind, HotkeyKind::RightOption) {
                            return CallbackResult::Keep;
                        }
                        if alt_now && !alt_was {
                            log::info!("hotkey: Right Option DOWN");
                            let _ = tx_cb.send(HotkeyEvent::Down);
                        } else if !alt_now && alt_was {
                            log::info!("hotkey: Right Option UP");
                            let _ = tx_cb.send(HotkeyEvent::Up);
                        }
                        CallbackResult::Keep
                    },
                    || {
                        log::info!(
                            "Right Option tap installed (Input Monitoring granted); running CFRunLoop"
                        );
                        CFRunLoop::run_current();
                    },
                );

                match result {
                    Ok(_) => {
                        // Runloop stopped (shouldn't happen in normal
                        // operation) — reinstall after a pause rather than
                        // losing the hotkey.
                        log::warn!("Right Option tap runloop exited; reinstalling in 3s");
                        warned = false;
                    }
                    Err(_) => {
                        if !warned {
                            warned = true;
                            log::error!(
                                "Right Option tap creation FAILED — Input Monitoring permission not \
                                 granted. macOS Settings → Privacy & Security → Input Monitoring → \
                                 enable FunButton (takes effect within seconds; retrying every 3s), \
                                 OR switch to Fn in Settings."
                            );
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }));

        // The loop above never returns normally, so reaching here means a
        // panic unwound out of it.
        if outcome.is_err() {
            log::error!(
                "Right Option listener thread panicked and exited; Right Option hotkey unavailable \
                 until app restart (other listeners and the rest of the app keep running)."
            );
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn spawn_listener(_tx: Sender<HotkeyEvent>, _armed: Arc<AtomicU8>) {
    log::warn!("Right Option listener is macOS-only; falling back at runtime");
}
