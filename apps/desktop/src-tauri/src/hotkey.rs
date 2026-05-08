use rdev::{listen, Event, EventType, Key};
use std::sync::mpsc::Sender;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyEvent {
    Down,
    Up,
}

/// Spawn the rdev listener on a dedicated thread.
/// Sends HotkeyEvent::Down on Right Option press, Up on release.
/// Note: rdev needs Accessibility permission on macOS.
pub fn spawn_listener(tx: Sender<HotkeyEvent>) {
    thread::spawn(move || {
        let mut down = false;
        let cb = move |event: Event| match event.event_type {
            EventType::KeyPress(key) if is_right_option(key) => {
                if !down {
                    down = true;
                    let _ = tx.send(HotkeyEvent::Down);
                }
            }
            EventType::KeyRelease(key) if is_right_option(key) => {
                if down {
                    down = false;
                    let _ = tx.send(HotkeyEvent::Up);
                }
            }
            _ => {}
        };
        if let Err(e) = listen(cb) {
            log::error!("rdev listener stopped: {e:?}");
        }
    });
}

/// Right Option key. rdev maps macOS Right Option to `Key::AltGr`.
/// We accept both AltGr and Alt to be tolerant of different keyboard layouts.
fn is_right_option(k: Key) -> bool {
    matches!(k, Key::AltGr)
}
