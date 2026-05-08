use anyhow::{anyhow, Context, Result};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use std::thread;
use std::time::Duration;

/// Outcome of a paste attempt.
/// - `Pasted`: keystroke sent without error. Prior clipboard restored.
/// - `Failed`: enigo errored, or clipboard write failed. Cleaned text is left
///    on the clipboard so the user can recover by pressing Cmd+V manually
///    (or by clicking a history entry).
pub enum PasteOutcome {
    Pasted,
    Failed(String),
}

/// Save current clipboard, write `text`, send Cmd+V, restore previous clipboard after a short delay.
/// Returns `Failed` instead of erroring so callers can record the failure in history
/// and surface it to the user without unwinding the pipeline.
pub fn paste_text(text: &str) -> PasteOutcome {
    let mut clip = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => return PasteOutcome::Failed(format!("clipboard open: {e}")),
    };
    let prior = clip.get_text().ok();

    if let Err(e) = clip.set_text(text.to_owned()) {
        return PasteOutcome::Failed(format!("clipboard set: {e}"));
    }
    // brief delay for the OS to register the new clipboard contents
    thread::sleep(Duration::from_millis(40));

    let mut enigo = match Enigo::new(&Settings::default()) {
        Ok(e) => e,
        Err(e) => {
            // cleaned text is on clipboard; user can paste manually
            return PasteOutcome::Failed(format!("enigo init: {e}"));
        }
    };
    #[cfg(target_os = "macos")]
    let modifier = Key::Meta;
    #[cfg(not(target_os = "macos"))]
    let modifier = Key::Control;

    if let Err(e) = try_send_paste(&mut enigo, modifier) {
        return PasteOutcome::Failed(format!("paste keystroke: {e}"));
    }

    if let Some(prev) = prior {
        // Only restore prior clipboard on success — on failure we want the
        // cleaned text to stay on the clipboard for manual recovery.
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(900));
            if let Ok(mut c) = Clipboard::new() {
                let _ = c.set_text(prev);
            }
        });
    }
    PasteOutcome::Pasted
}

fn try_send_paste(enigo: &mut Enigo, modifier: Key) -> Result<()> {
    enigo
        .key(modifier, Direction::Press)
        .map_err(|e| anyhow!("press: {e}"))?;
    thread::sleep(Duration::from_millis(20));
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| anyhow!("click: {e}"))?;
    thread::sleep(Duration::from_millis(20));
    enigo
        .key(modifier, Direction::Release)
        .map_err(|e| anyhow!("release: {e}"))?;
    Ok(())
}

/// Set clipboard text WITHOUT sending Cmd+V — used by the history "Copy"
/// button. Doesn't restore prior clipboard.
pub fn set_clipboard(text: &str) -> Result<()> {
    let mut clip = Clipboard::new().context("clipboard open")?;
    clip.set_text(text.to_owned()).context("clipboard set")?;
    Ok(())
}
