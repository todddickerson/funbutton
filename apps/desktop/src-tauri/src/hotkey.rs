use crate::state::HotkeyKind;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down,
    Up,
}

/// Spawn the rdev listener on a dedicated thread.
/// Sends HotkeyEvent::Down on Right Option press, Up on release.
/// Note: rdev needs Accessibility permission on macOS.
///
/// The listener only emits events when `armed` matches `HotkeyKind::RightOption`
/// — both listeners run simultaneously so the user can hot-swap the active
/// hotkey from the Settings UI without restarting the app.
pub fn spawn_listener(tx: Sender<HotkeyEvent>, armed: Arc<AtomicU8>) {
    thread::spawn(move || {
        let mut down = false;
        let cb = move |event: Event| {
            // Only emit if Right Option is the armed hotkey right now.
            let armed_kind = HotkeyKind::from_u8(armed.load(Ordering::SeqCst));
            if !matches!(armed_kind, HotkeyKind::RightOption) {
                return;
            }
            match event.event_type {
                EventType::KeyPress(key) if is_right_option(key) => {
                    if !down {
                        down = true;
                        log::info!("hotkey: Right Option DOWN");
                        let _ = tx.send(HotkeyEvent::Down);
                    }
                }
                EventType::KeyRelease(key) if is_right_option(key) => {
                    if down {
                        down = false;
                        log::info!("hotkey: Right Option UP");
                        let _ = tx.send(HotkeyEvent::Up);
                    }
                }
                _ => {}
            }
        };
        if let Err(e) = listen(cb) {
            log::error!(
                "rdev listener stopped: {e:?} — Accessibility permission likely not granted. \
                 macOS Settings → Privacy & Security → Accessibility → enable FunButton."
            );
        }
    });
}

/// Right Option key. rdev maps macOS Right Option to `Key::AltGr`.
fn is_right_option(k: Key) -> bool {
    matches!(k, Key::AltGr)
}
