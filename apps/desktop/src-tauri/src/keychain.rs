//! Groq API key storage in the macOS Keychain.
//!
//! The key used to live in plaintext in `~/.funbutton/settings.json`. It now
//! lives in the login Keychain (service `ai.funbutton.desktop`, account
//! `groq_api_key`); the settings file keeps an empty string in that field.
//!
//! Migration is idempotent and needs no sentinel flag: on every load, a
//! non-empty key found in the settings file is written to the Keychain and
//! blanked in the file. If the Keychain is unavailable (denied ACL prompt,
//! locked keychain, changed code signature on an unsigned dev build), every
//! call degrades gracefully and the key stays in the settings file — worse
//! at-rest protection, but the app never loses the key or hard-fails.

#[cfg(target_os = "macos")]
mod imp {
    use keyring::Entry;

    const SERVICE: &str = "ai.funbutton.desktop";
    const ACCOUNT: &str = "groq_api_key";

    fn entry() -> keyring::Result<Entry> {
        Entry::new(SERVICE, ACCOUNT)
    }

    /// Read the key. `None` means "no usable key" — absent, empty, or the
    /// Keychain refused us; the caller falls back to file/env.
    pub fn get_groq_key() -> Option<String> {
        match entry().and_then(|e| e.get_password()) {
            Ok(k) if !k.trim().is_empty() => Some(k),
            Ok(_) => None,
            Err(keyring::Error::NoEntry) => None,
            Err(e) => {
                log::warn!("keychain read failed (falling back to settings file): {e}");
                None
            }
        }
    }

    pub fn set_groq_key(key: &str) -> anyhow::Result<()> {
        entry()?.set_password(key)?;
        Ok(())
    }

    /// Best-effort delete; a missing entry is success.
    pub fn delete_groq_key() {
        match entry().and_then(|e| e.delete_credential()) {
            Ok(()) | Err(keyring::Error::NoEntry) => {}
            Err(e) => log::warn!("keychain delete failed: {e}"),
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod imp {
    pub fn get_groq_key() -> Option<String> {
        None
    }
    pub fn set_groq_key(_key: &str) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("keychain storage is macOS-only"))
    }
    pub fn delete_groq_key() {}
}

pub use imp::{delete_groq_key, get_groq_key, set_groq_key};
