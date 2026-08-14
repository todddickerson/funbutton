//! Modifier-key push-to-talk via CGEventTap (everything except Fn).
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
//! for `FlagsChanged`, inspecting only the raw virtual keycode and the modifier
//! flag bits — never a layout/input-source API. That makes it safe on every
//! macOS version.
//!
//! One tap serves every keycode-based hotkey (Right/Left Option, Right/Left
//! Control, Right/Left Command, Caps Lock). Which one is live is decided per
//! event from the shared `armed` atomic, so the UI can hot-swap the active key
//! with no restart. Left vs right is disambiguated by the device-specific
//! modifier flag bits (`NX_DEVICE*KEYMASK`), which the HID-layer tap reports;
//! when a keyboard doesn't report them we fall back to the device-independent
//! modifier flag so detection never silently dies.
//!
//! Requires **Input Monitoring** (the same permission as the Fn tap).

use crate::state::HotkeyKind;

use parking_lot::Mutex;
use std::sync::atomic::AtomicU8;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down,
    Up,
}

/// "Press the key you want" capture. While `active`, both the Fn tap and this
/// keycode tap report the first recognized modifier *keydown* through `tx`
/// (regardless of what's currently armed) and suppress the normal push-to-talk
/// path. One-shot: whoever reports first clears `active`.
pub struct CaptureState {
    pub active: std::sync::atomic::AtomicBool,
    pub tx: Mutex<Option<Sender<u8>>>,
}

impl CaptureState {
    pub fn new() -> Self {
        Self {
            active: std::sync::atomic::AtomicBool::new(false),
            tx: Mutex::new(None),
        }
    }
}

impl Default for CaptureState {
    fn default() -> Self {
        Self::new()
    }
}

// Device-dependent modifier flag masks (IOKit `IOLLEvent.h` `NX_DEVICE*`).
// Present in CGEvent flags at the HID/session tap layer — this is exactly what
// Karabiner-Elements / Hammerspoon / Hyperkey read to tell left from right.
const NX_DEVICE_LCTRL: u64 = 0x0000_0001;
const NX_DEVICE_RCTRL: u64 = 0x0000_2000;
const NX_DEVICE_LCMD: u64 = 0x0000_0008;
const NX_DEVICE_RCMD: u64 = 0x0000_0010;
const NX_DEVICE_LALT: u64 = 0x0000_0020;
const NX_DEVICE_RALT: u64 = 0x0000_0040;

// Device-independent modifier flags (used as the fallback when a keyboard
// doesn't surface the device-dependent bits, and directly for Caps Lock).
const CG_ALPHASHIFT: u64 = 0x0001_0000;
const CG_CONTROL: u64 = 0x0004_0000;
const CG_ALTERNATE: u64 = 0x0008_0000;
const CG_COMMAND: u64 = 0x0010_0000;

/// Map a `FlagsChanged` virtual keycode to the hotkey kind it represents, if
/// any. `Fn` is intentionally absent — it has no stable modifier keycode and
/// is read off the SecondaryFn flag bit in `fn_hotkey.rs`.
pub fn kind_for_keycode(keycode: i64) -> Option<HotkeyKind> {
    match keycode {
        0x3D => Some(HotkeyKind::RightOption),
        0x3A => Some(HotkeyKind::LeftOption),
        0x3E => Some(HotkeyKind::RightControl),
        0x3B => Some(HotkeyKind::LeftControl),
        0x36 => Some(HotkeyKind::RightCommand),
        0x37 => Some(HotkeyKind::LeftCommand),
        0x39 => Some(HotkeyKind::CapsLock),
        _ => None,
    }
}

/// Is the physical key for `kind` currently held, given the raw flag bits from
/// a `FlagsChanged` event? Prefers the device-specific bit (precise left/right,
/// correct even when both sides of a modifier are held); falls back to the
/// device-independent flag when no device bits for that family are reported.
///
/// Pure integer math — no CG types — so it is unit-testable on any platform.
pub fn is_active(kind: HotkeyKind, raw: u64) -> bool {
    match kind {
        HotkeyKind::RightControl => side_active(raw, NX_DEVICE_RCTRL, NX_DEVICE_LCTRL, CG_CONTROL),
        HotkeyKind::LeftControl => side_active(raw, NX_DEVICE_LCTRL, NX_DEVICE_RCTRL, CG_CONTROL),
        HotkeyKind::RightOption => side_active(raw, NX_DEVICE_RALT, NX_DEVICE_LALT, CG_ALTERNATE),
        HotkeyKind::LeftOption => side_active(raw, NX_DEVICE_LALT, NX_DEVICE_RALT, CG_ALTERNATE),
        HotkeyKind::RightCommand => side_active(raw, NX_DEVICE_RCMD, NX_DEVICE_LCMD, CG_COMMAND),
        HotkeyKind::LeftCommand => side_active(raw, NX_DEVICE_LCMD, NX_DEVICE_RCMD, CG_COMMAND),
        // Caps Lock is a toggle with no left/right; its device-independent flag
        // reflects the current lock state (on = "held" = start recording).
        HotkeyKind::CapsLock => (raw & CG_ALPHASHIFT) != 0,
        // Fn is handled by the SecondaryFn bit in fn_hotkey.rs, never here.
        HotkeyKind::Fn => false,
    }
}

/// One side of a two-sided modifier. If either the wanted side or its sibling
/// reports a device bit, the OS is giving us device-specific flags → trust the
/// precise bit. Otherwise fall back to the shared device-independent flag
/// (loses left/right precision only in the rare both-sides-held case, but never
/// silently fails to register the key).
fn side_active(raw: u64, my_mask: u64, sibling_mask: u64, general_flag: u64) -> bool {
    let mine = raw & my_mask != 0;
    let sibling = raw & sibling_mask != 0;
    if mine || sibling {
        mine
    } else {
        raw & general_flag != 0
    }
}

/// Spawn the keycode-modifier listener on a dedicated thread. Emits
/// `HotkeyEvent::Down`/`Up` for whichever keycode-based kind is armed; the Fn
/// tap (`fn_hotkey.rs`) handles the `Fn` case. Both share `armed` and
/// `capture` so hot-swap and "press to capture" work without a restart.
#[cfg(target_os = "macos")]
pub fn spawn_listener(tx: Sender<HotkeyEvent>, armed: Arc<AtomicU8>, capture: Arc<CaptureState>) {
    use core_foundation::runloop::CFRunLoop;
    use core_graphics::event::{
        CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType,
        CallbackResult, EventField,
    };
    use std::sync::atomic::{AtomicBool, Ordering};

    thread::spawn(move || {
        // Defensive isolation: if anything in the tap callback or runloop
        // panics (Rust unwind — the crate is built panic=unwind), log loudly
        // and let this thread die instead of taking the whole app down. A hard
        // trap like the macOS 26 HIToolbox assertion is NOT a Rust panic and
        // cannot be caught here — which is exactly why this tap touches no
        // layout API rather than relying on this net.
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            // Edge tracker for the currently-armed key, plus the armed kind we
            // last saw so we can reset the edge when it hot-swaps.
            let held = Arc::new(AtomicBool::new(false));
            let last_armed = Arc::new(AtomicU8::new(u8::MAX));
            let mut warned = false;
            loop {
                let held_cb = Arc::clone(&held);
                let last_armed_cb = Arc::clone(&last_armed);
                let tx_cb = tx.clone();
                let armed_cb = Arc::clone(&armed);
                let capture_cb = Arc::clone(&capture);

                let result = CGEventTap::with_enabled(
                    CGEventTapLocation::HID,
                    CGEventTapPlacement::HeadInsertEventTap,
                    CGEventTapOptions::ListenOnly,
                    vec![CGEventType::FlagsChanged],
                    move |_proxy, _etype, event| {
                        // Reading keycode + flags is plain integer field
                        // access — no TIS/TSM/UCKeyTranslate, so this is
                        // main-thread-agnostic and safe on macOS 26.
                        let keycode =
                            event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
                        let raw = event.get_flags().bits();

                        // Capture takes precedence and never drives the
                        // pipeline: bind the first recognized modifier keydown.
                        if capture_cb.active.load(Ordering::SeqCst) {
                            if let Some(kind) = kind_for_keycode(keycode) {
                                if is_active(kind, raw) {
                                    if let Some(ctx) = capture_cb.tx.lock().as_ref() {
                                        let _ = ctx.send(kind.as_u8());
                                    }
                                    capture_cb.active.store(false, Ordering::SeqCst);
                                    log::info!("hotkey capture: {kind:?} pressed");
                                }
                            }
                            return CallbackResult::Keep;
                        }

                        let armed_u8 = armed_cb.load(Ordering::SeqCst);
                        let armed_kind = HotkeyKind::from_u8(armed_u8);
                        // Fn (or nothing keycode-based) armed → not our job.
                        let Some(target_kc) = armed_kind.keycode() else {
                            return CallbackResult::Keep;
                        };
                        if keycode != target_kc {
                            return CallbackResult::Keep;
                        }
                        // Hot-swap safety: if the armed kind changed since the
                        // last event, drop any stale held edge so the next
                        // DOWN isn't swallowed.
                        if last_armed_cb.swap(armed_u8, Ordering::SeqCst) != armed_u8 {
                            held_cb.store(false, Ordering::SeqCst);
                        }
                        let now = is_active(armed_kind, raw);
                        let was = held_cb.swap(now, Ordering::SeqCst);
                        if now && !was {
                            log::info!("hotkey: {armed_kind:?} DOWN");
                            let _ = tx_cb.send(HotkeyEvent::Down);
                        } else if !now && was {
                            log::info!("hotkey: {armed_kind:?} UP");
                            let _ = tx_cb.send(HotkeyEvent::Up);
                        }
                        CallbackResult::Keep
                    },
                    || {
                        log::info!(
                            "modifier-key tap installed (Input Monitoring granted); running CFRunLoop"
                        );
                        CFRunLoop::run_current();
                    },
                );

                match result {
                    Ok(_) => {
                        log::warn!("modifier-key tap runloop exited; reinstalling in 3s");
                        warned = false;
                    }
                    Err(_) => {
                        if !warned {
                            warned = true;
                            log::error!(
                                "modifier-key tap creation FAILED — Input Monitoring permission not \
                                 granted. macOS Settings → Privacy & Security → Input Monitoring → \
                                 enable FunButton (takes effect within seconds; retrying every 3s)."
                            );
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
        }));

        if outcome.is_err() {
            log::error!(
                "modifier-key listener thread panicked and exited; keycode hotkeys unavailable \
                 until app restart (other listeners and the rest of the app keep running)."
            );
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn spawn_listener(_tx: Sender<HotkeyEvent>, _armed: Arc<AtomicU8>, _capture: Arc<CaptureState>) {
    log::warn!("modifier-key listener is macOS-only; falling back at runtime");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keycodes_round_trip_with_state() {
        // Every keycode-based kind maps back from its own keycode.
        for kind in [
            HotkeyKind::RightOption,
            HotkeyKind::LeftOption,
            HotkeyKind::RightControl,
            HotkeyKind::LeftControl,
            HotkeyKind::RightCommand,
            HotkeyKind::LeftCommand,
            HotkeyKind::CapsLock,
        ] {
            let kc = kind.keycode().expect("keycode-based kind has a keycode");
            assert_eq!(kind_for_keycode(kc), Some(kind), "{kind:?}");
        }
        assert_eq!(HotkeyKind::Fn.keycode(), None);
        assert_eq!(kind_for_keycode(0x00), None);
    }

    #[test]
    fn device_bits_disambiguate_left_and_right() {
        // Right Control device bit set, left clear → only Right Control active.
        assert!(is_active(HotkeyKind::RightControl, NX_DEVICE_RCTRL | CG_CONTROL));
        assert!(!is_active(HotkeyKind::LeftControl, NX_DEVICE_RCTRL | CG_CONTROL));
        // Left Option device bit set → only Left Option active.
        assert!(is_active(HotkeyKind::LeftOption, NX_DEVICE_LALT | CG_ALTERNATE));
        assert!(!is_active(HotkeyKind::RightOption, NX_DEVICE_LALT | CG_ALTERNATE));
    }

    #[test]
    fn both_sides_held_keeps_each_side_active() {
        let both = NX_DEVICE_LCTRL | NX_DEVICE_RCTRL | CG_CONTROL;
        assert!(is_active(HotkeyKind::LeftControl, both));
        assert!(is_active(HotkeyKind::RightControl, both));
    }

    #[test]
    fn falls_back_to_general_flag_without_device_bits() {
        // No device bits reported, only the general Command flag → the command
        // key still registers (degraded left/right, never a dead key).
        assert!(is_active(HotkeyKind::RightCommand, CG_COMMAND));
        assert!(is_active(HotkeyKind::LeftCommand, CG_COMMAND));
        // Nothing set → released.
        assert!(!is_active(HotkeyKind::RightCommand, 0));
    }

    #[test]
    fn caps_lock_tracks_alphashift() {
        assert!(is_active(HotkeyKind::CapsLock, CG_ALPHASHIFT));
        assert!(!is_active(HotkeyKind::CapsLock, 0));
    }

    #[test]
    fn release_of_one_side_with_other_held_stays_precise() {
        // Left Control still held, Right Control just released: the right
        // device bit is clear even though the general Control flag is still on.
        let raw = NX_DEVICE_LCTRL | CG_CONTROL;
        assert!(!is_active(HotkeyKind::RightControl, raw));
        assert!(is_active(HotkeyKind::LeftControl, raw));
    }

    /// Real-runtime proof: spawn the ACTUAL listeners and drive them with
    /// synthetic `FlagsChanged` CGEvents (an autonomous agent can't hold a
    /// physical key, but a posted event is byte-for-byte what the hardware
    /// produces at the HID tap — same keycode, same device flag bits — so the
    /// tap can't tell the difference). Proves that, per armed kind, the RIGHT
    /// key fires DOWN/UP and the others stay silent.
    ///
    /// Needs Input Monitoring (to tap) + Accessibility (to post) granted to the
    /// process running the test. If they're missing the tap never installs /
    /// events never land; the test then prints a skip notice and returns rather
    /// than false-failing. Run:
    ///   RUST_LOG=info cargo test --release listener_maps_injected_cgevents \
    ///     -- --ignored --nocapture
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore]
    fn listener_maps_injected_cgevents() {
        use core_graphics::event::{
            CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, EventField,
        };
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        use std::sync::atomic::Ordering;
        use std::time::Duration;

        const CG_SECONDARY_FN: u64 = 0x0080_0000;

        let _ = env_logger::builder().is_test(false).try_init();

        fn post(keycode: i64, bits: u64) {
            let src = CGEventSource::new(CGEventSourceStateID::HIDSystemState).expect("source");
            let ev = CGEvent::new_keyboard_event(src, keycode as u16, true).expect("event");
            ev.set_type(CGEventType::FlagsChanged);
            // from_bits_retain keeps the device-specific NX_DEVICE* bits, which
            // are not named CGEventFlags constants (from_bits_truncate drops them).
            ev.set_flags(CGEventFlags::from_bits_retain(bits));
            ev.set_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE, keycode);
            ev.post(CGEventTapLocation::HID);
        }

        let (tx, rx) = std::sync::mpsc::channel::<HotkeyEvent>();
        let armed = Arc::new(AtomicU8::new(HotkeyKind::RightControl.as_u8()));
        let capture = Arc::new(CaptureState::new());
        spawn_listener(tx.clone(), Arc::clone(&armed), Arc::clone(&capture));
        crate::fn_hotkey::spawn_listener(tx.clone(), Arc::clone(&armed), Arc::clone(&capture));

        // Let both taps install (retries every 3s if IM was just granted).
        std::thread::sleep(Duration::from_millis(1500));
        let drain = |rx: &std::sync::mpsc::Receiver<HotkeyEvent>| while rx.recv_timeout(Duration::from_millis(60)).is_ok() {};
        let next = |rx: &std::sync::mpsc::Receiver<HotkeyEvent>| rx.recv_timeout(Duration::from_millis(1500)).ok();

        // Probe: Right Control armed, inject a down.
        armed.store(HotkeyKind::RightControl.as_u8(), Ordering::SeqCst);
        drain(&rx);
        post(0x3E, NX_DEVICE_RCTRL | CG_CONTROL);
        let down = next(&rx);
        post(0x3E, 0);
        let up = next(&rx);
        println!("[armed=RightControl] inject RCtrl → down={down:?} up={up:?}");
        if down.is_none() {
            eprintln!(
                "SKIP: no events observed — Input Monitoring + Accessibility are not granted to \
                 this test process. Grant them to your terminal, or run scripts/verify-hotkeys.sh \
                 against the /Applications install for a physical-key check."
            );
            return;
        }
        assert_eq!(down, Some(HotkeyEvent::Down), "Right Control DOWN");
        assert_eq!(up, Some(HotkeyEvent::Up), "Right Control UP");
        // Right Option must NOT fire while Right Control is armed.
        post(0x3D, NX_DEVICE_RALT | CG_ALTERNATE);
        post(0x3D, 0);
        assert!(next(&rx).is_none(), "Right Option leaked while Right Control armed");
        println!("[armed=RightControl] inject ROpt → correctly silent");

        // Right Option armed.
        armed.store(HotkeyKind::RightOption.as_u8(), Ordering::SeqCst);
        drain(&rx);
        post(0x3D, NX_DEVICE_RALT | CG_ALTERNATE);
        let down = next(&rx);
        post(0x3D, 0);
        let up = next(&rx);
        println!("[armed=RightOption] inject ROpt → down={down:?} up={up:?}");
        assert_eq!(down, Some(HotkeyEvent::Down), "Right Option DOWN");
        assert_eq!(up, Some(HotkeyEvent::Up), "Right Option UP");
        post(0x3E, NX_DEVICE_RCTRL | CG_CONTROL);
        post(0x3E, 0);
        assert!(next(&rx).is_none(), "Right Control leaked while Right Option armed");
        println!("[armed=RightOption] inject RCtrl → correctly silent");

        // Fn armed (SecondaryFn bit; keycode irrelevant to the Fn tap).
        armed.store(HotkeyKind::Fn.as_u8(), Ordering::SeqCst);
        drain(&rx);
        post(0x3F, CG_SECONDARY_FN);
        let down = next(&rx);
        post(0x3F, 0);
        let up = next(&rx);
        println!("[armed=Fn] inject Fn bit → down={down:?} up={up:?}");
        assert_eq!(down, Some(HotkeyEvent::Down), "Fn DOWN");
        assert_eq!(up, Some(HotkeyEvent::Up), "Fn UP");
        // Right Control must NOT fire while Fn is armed.
        post(0x3E, NX_DEVICE_RCTRL | CG_CONTROL);
        post(0x3E, 0);
        assert!(next(&rx).is_none(), "Right Control leaked while Fn armed");
        println!("[armed=Fn] inject RCtrl → correctly silent");
        println!("ALL INJECTED-EVENT ASSERTIONS PASSED");
    }
}
