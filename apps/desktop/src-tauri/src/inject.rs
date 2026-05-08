use anyhow::{Context, Result};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Save current clipboard, write `text`, send Cmd+V, restore previous clipboard after a short delay.
pub fn paste_text(text: &str) -> Result<()> {
    let mut clip = Clipboard::new().context("opening clipboard")?;
    let prior = clip.get_text().ok();

    clip.set_text(text.to_owned()).context("set clipboard")?;
    // brief delay for the OS to register the new clipboard contents
    thread::sleep(Duration::from_millis(40));

    let mut enigo = Enigo::new(&Settings::default()).context("init enigo")?;
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta; // Cmd
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    enigo.key(modifier, Direction::Press).ok();
    thread::sleep(Duration::from_millis(20));
    enigo.key(Key::Unicode('v'), Direction::Click).ok();
    thread::sleep(Duration::from_millis(20));
    enigo.key(modifier, Direction::Release).ok();

    if let Some(prev) = prior {
        // restore prior clipboard a moment later so the paste lands first
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(900));
            if let Ok(mut c) = Clipboard::new() {
                let _ = c.set_text(prev);
            }
        });
    }
    Ok(())
}
