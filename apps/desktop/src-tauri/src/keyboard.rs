//! Keyboard detection — figure out which physical keyboard is connected so the
//! picker can draw an accurate bottom row and pick a sensible default hotkey.
//!
//! The brand pillar is "the Fn key at the bottom-left IS the Fun Button." That
//! is true on built-in MacBook keyboards and compact Magic Keyboards — and
//! FALSE on the Magic Keyboard with Numeric Keypad and most full-size / third
//! party boards, where the bottom-left key is Control. A tester (Todd, on a Mac
//! Studio with the numeric-keypad board) held the bottom-left key, got Control
//! instead of Fn, and the product looked broken. Detection lets the UI stop
//! asserting a key that isn't there.
//!
//! Primary source is the IOKit HID registry via `ioreg` — fast (~tens of ms),
//! always present, and it reports the product name + vendor/product id for
//! built-in, USB, and Bluetooth keyboards alike. `system_profiler
//! SPUSBDataType` is a slower secondary source. Detection NEVER hard-fails:
//! nothing found → `Generic`, a safe layout that doesn't claim an Fn key.
//!
//! No layout/input-source APIs anywhere here (this runs on a Tauri command
//! thread, and TIS/TSM SIGTRAP off the main thread on macOS 26) — just child
//! processes and string parsing.

use serde::Serialize;
use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

/// Physical bottom-row shape, which is all the picker needs to draw the right
/// diagram and know where (if anywhere) Fn lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyboardLayout {
    /// Built-in MacBook keyboard — Fn/🌐 is the bottom-left key.
    BuiltinMacbook,
    /// Compact external Apple keyboard (Magic Keyboard, no numpad) — Fn/🌐 is
    /// the bottom-left key.
    MagicCompact,
    /// Full-size / numeric-keypad Apple keyboard — bottom-left is Left Control;
    /// Fn sits in the navigation cluster, not the bottom-left.
    MagicExtended,
    /// Unknown / third-party — generic fallback; no Fn assumed in bottom-left.
    Generic,
}

impl KeyboardLayout {
    /// Whether the physical bottom-left key is Fn (true) or Control (false).
    pub fn fn_bottom_left(self) -> bool {
        matches!(
            self,
            KeyboardLayout::BuiltinMacbook | KeyboardLayout::MagicCompact
        )
    }

    /// The sensible default hotkey for this layout, as a `HotkeyKind` serde
    /// name. Fn where Fn is the bottom-left key and the brand story holds;
    /// Right Option (present, safe, easy to find) on boards where it doesn't.
    pub fn default_hotkey(self) -> &'static str {
        match self {
            KeyboardLayout::BuiltinMacbook | KeyboardLayout::MagicCompact => "fn",
            KeyboardLayout::MagicExtended | KeyboardLayout::Generic => "right_option",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct KeyboardInfo {
    /// Product name as reported by the OS, or "Unknown keyboard".
    pub model: String,
    pub layout: KeyboardLayout,
    pub is_builtin: bool,
    /// Convenience mirror of `layout.fn_bottom_left()` for the frontend.
    pub fn_bottom_left: bool,
    /// `HotkeyKind` serde name of the sensible default for this layout.
    pub default_hotkey: String,
    pub vendor_id: Option<u32>,
    pub product_id: Option<u32>,
    /// Where the answer came from: "ioreg" | "system_profiler" | "none".
    pub source: String,
}

impl KeyboardInfo {
    fn from_candidate(c: &Candidate, source: &str) -> Self {
        let (layout, is_builtin) = classify_name(&c.name);
        KeyboardInfo {
            model: c.name.clone(),
            layout,
            is_builtin,
            fn_bottom_left: layout.fn_bottom_left(),
            default_hotkey: layout.default_hotkey().to_string(),
            vendor_id: c.vendor_id,
            product_id: c.product_id,
            source: source.to_string(),
        }
    }

    /// The always-safe fallback: a generic keyboard with no Fn asserted.
    pub fn generic() -> Self {
        KeyboardInfo {
            model: "Unknown keyboard".to_string(),
            layout: KeyboardLayout::Generic,
            is_builtin: false,
            fn_bottom_left: false,
            default_hotkey: KeyboardLayout::Generic.default_hotkey().to_string(),
            vendor_id: None,
            product_id: None,
            source: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    name: String,
    vendor_id: Option<u32>,
    product_id: Option<u32>,
}

/// Classify a keyboard product name into a layout + whether it's built-in.
/// Substring matching only — resilient to the many name variants Apple ships
/// ("Magic Keyboard with Touch ID and Numeric Keypad", "Apple Internal
/// Keyboard / Trackpad", …).
fn classify_name(name: &str) -> (KeyboardLayout, bool) {
    let n = name.to_lowercase();
    // Built-in laptop keyboard.
    if n.contains("internal") || n.contains("built-in") {
        return (KeyboardLayout::BuiltinMacbook, true);
    }
    let extended_marker = n.contains("numeric")
        || n.contains("keypad")
        || n.contains("full")
        || n.contains("extended");
    // Apple full-size / numeric-keypad boards → Left Control is bottom-left.
    if (n.contains("magic keyboard") || n.contains("apple keyboard")) && extended_marker {
        return (KeyboardLayout::MagicExtended, false);
    }
    // Compact Magic Keyboard (Touch ID or plain) → Fn is bottom-left.
    if n.contains("magic keyboard") {
        return (KeyboardLayout::MagicCompact, false);
    }
    // Everything else (third-party, unknown): generic, no Fn claim.
    (KeyboardLayout::Generic, false)
}

/// Rank so the picker shows the keyboard most likely surprising the user.
/// External-with-a-Control-bottom-left ranks highest (that's the confusing
/// case); a built-in laptop keyboard is the natural floor when nothing external
/// is attached.
fn score(layout: KeyboardLayout, is_builtin: bool) -> u8 {
    match (layout, is_builtin) {
        (KeyboardLayout::MagicExtended, _) => 5,
        (KeyboardLayout::MagicCompact, _) => 4,
        (KeyboardLayout::Generic, false) => 3,
        (KeyboardLayout::BuiltinMacbook, _) => 2,
        _ => 1,
    }
}

fn pick_best(candidates: &[Candidate]) -> Option<&Candidate> {
    candidates.iter().max_by_key(|c| {
        let (layout, builtin) = classify_name(&c.name);
        score(layout, builtin)
    })
}

/// Detect the connected keyboard. Never errors — returns `KeyboardInfo::generic`
/// if nothing recognizable is found.
pub fn detect() -> KeyboardInfo {
    #[cfg(target_os = "macos")]
    {
        // Primary: HID registry. Query a couple of driver classes so both
        // Apple (AppleHIDKeyboardEventDriverV2) and generic third-party
        // (IOHIDKeyboard) boards are covered.
        for class in ["AppleHIDKeyboardEventDriverV2", "IOHIDKeyboard"] {
            if let Some(out) = output_with_timeout(
                "ioreg",
                &["-r", "-c", class, "-d", "1"],
                Duration::from_secs(2),
            ) {
                let candidates = parse_ioreg(&out);
                if let Some(best) = pick_best(&candidates) {
                    let info = KeyboardInfo::from_candidate(best, "ioreg");
                    log::info!(
                        "keyboard detected via ioreg({class}): {:?} → {:?} (fn_bottom_left={})",
                        info.model,
                        info.layout,
                        info.fn_bottom_left
                    );
                    return info;
                }
            }
        }
        // Secondary: system_profiler USB tree.
        if let Some(out) = output_with_timeout(
            "system_profiler",
            &["SPUSBDataType"],
            Duration::from_secs(6),
        ) {
            let candidates = parse_system_profiler(&out);
            if let Some(best) = pick_best(&candidates) {
                let info = KeyboardInfo::from_candidate(best, "system_profiler");
                log::info!(
                    "keyboard detected via system_profiler: {:?} → {:?}",
                    info.model,
                    info.layout
                );
                return info;
            }
        }
        log::info!("no keyboard recognized; falling back to generic layout");
    }
    KeyboardInfo::generic()
}

/// Run a child process, capturing stdout, but never block longer than
/// `timeout`. On timeout the child is abandoned (it exits on its own) and we
/// return `None` — mirrors the app-detection pattern that stops a TCC-stalled
/// helper from hanging a command thread.
#[cfg(target_os = "macos")]
fn output_with_timeout(program: &str, args: &[&str], timeout: Duration) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    std::thread::spawn(move || {
        let out = Command::new(&program).args(&args).output();
        let text = out.ok().and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).into_owned())
            } else {
                None
            }
        });
        let _ = tx.send(text);
    });
    rx.recv_timeout(timeout).ok().flatten()
}

/// Parse `ioreg -r -c <class> -d 1` output into keyboard candidates. Each HID
/// node begins with a `+-o` header; within a node we read `"Product"`,
/// `"VendorID"`, `"ProductID"`.
fn parse_ioreg(text: &str) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    // Split into per-node chunks on the tree-branch marker.
    for chunk in split_nodes(text) {
        let name = extract_quoted(chunk, "\"Product\"");
        let Some(name) = name else { continue };
        if name.trim().is_empty() {
            continue;
        }
        candidates.push(Candidate {
            name,
            vendor_id: extract_int(chunk, "\"VendorID\""),
            product_id: extract_int(chunk, "\"ProductID\""),
        });
    }
    candidates
}

fn split_nodes(text: &str) -> Vec<&str> {
    // `ioreg` uses "+-o " to open each object; keep behaviour sane if it's
    // absent (treat the whole blob as one node).
    if !text.contains("+-o") {
        return vec![text];
    }
    let mut nodes = Vec::new();
    let mut last = 0usize;
    let bytes = text.as_bytes();
    let marker = b"+-o";
    let mut i = 0;
    while i + marker.len() <= bytes.len() {
        if &bytes[i..i + marker.len()] == marker {
            if i > last {
                nodes.push(&text[last..i]);
            }
            last = i;
            i += marker.len();
        } else {
            i += 1;
        }
    }
    if last < text.len() {
        nodes.push(&text[last..]);
    }
    nodes
}

/// Extract the string value after `key = "..."` (ioreg style).
fn extract_quoted(chunk: &str, key: &str) -> Option<String> {
    let idx = chunk.find(key)?;
    let after = &chunk[idx + key.len()..];
    let eq = after.find('=')?;
    let after = &after[eq + 1..];
    let start = after.find('"')? + 1;
    let rest = &after[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// Extract the first integer value after `key = N` (ioreg style).
fn extract_int(chunk: &str, key: &str) -> Option<u32> {
    let idx = chunk.find(key)?;
    let after = &chunk[idx + key.len()..];
    let eq = after.find('=')?;
    let after = after[eq + 1..].trim_start();
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Parse `system_profiler SPUSBDataType` for device entries whose name looks
/// like a keyboard. Names are indented and end with a colon; vendor/product ids
/// (if present nearby) are best-effort and optional.
fn parse_system_profiler(text: &str) -> Vec<Candidate> {
    let mut candidates = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    for (i, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if !line.ends_with(':') {
            continue;
        }
        let name = line.trim_end_matches(':').trim();
        if name.is_empty() || !name.to_lowercase().contains("keyboard") {
            continue;
        }
        // Scan the next few lines for Product/Vendor IDs of this device block.
        let mut vendor_id = None;
        let mut product_id = None;
        for probe in lines.iter().skip(i + 1).take(12) {
            let p = probe.trim();
            if let Some(rest) = p.strip_prefix("Product ID:") {
                product_id = parse_hex_or_dec(rest);
            } else if let Some(rest) = p.strip_prefix("Vendor ID:") {
                vendor_id = parse_hex_or_dec(rest);
            }
            // A blank-ish boundary or a new device header ends this block.
            if p.ends_with(':') && !p.starts_with("Product") && !p.starts_with("Vendor") {
                break;
            }
        }
        candidates.push(Candidate {
            name: name.to_string(),
            vendor_id,
            product_id,
        });
    }
    candidates
}

fn parse_hex_or_dec(s: &str) -> Option<u32> {
    let tok = s.split_whitespace().next()?;
    if let Some(hex) = tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else {
        tok.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_magic_keyboard_is_control_bottom_left() {
        // The exact product string on the tester's Mac Studio.
        let (layout, builtin) = classify_name("Magic Keyboard with Touch ID and Numeric Keypad");
        assert_eq!(layout, KeyboardLayout::MagicExtended);
        assert!(!builtin);
        assert!(
            !layout.fn_bottom_left(),
            "extended board: bottom-left is Control, not Fn"
        );
        assert_eq!(layout.default_hotkey(), "right_option");
    }

    #[test]
    fn compact_magic_keyboard_keeps_fn_bottom_left() {
        for name in ["Magic Keyboard", "Magic Keyboard with Touch ID"] {
            let (layout, builtin) = classify_name(name);
            assert_eq!(layout, KeyboardLayout::MagicCompact, "{name}");
            assert!(!builtin);
            assert!(
                layout.fn_bottom_left(),
                "{name}: compact board keeps Fn bottom-left"
            );
            assert_eq!(layout.default_hotkey(), "fn");
        }
    }

    #[test]
    fn builtin_macbook_keeps_fn_and_is_builtin() {
        let (layout, builtin) = classify_name("Apple Internal Keyboard / Trackpad");
        assert_eq!(layout, KeyboardLayout::BuiltinMacbook);
        assert!(builtin);
        assert!(layout.fn_bottom_left());
        assert_eq!(layout.default_hotkey(), "fn");
    }

    #[test]
    fn unknown_third_party_is_generic_without_fn_claim() {
        for name in [
            "Keychron K2",
            "HHKB Professional",
            "USB Keyboard",
            "Logitech MX Keys",
        ] {
            let (layout, builtin) = classify_name(name);
            assert_eq!(layout, KeyboardLayout::Generic, "{name}");
            assert!(!builtin);
            assert!(
                !layout.fn_bottom_left(),
                "{name}: never assert Fn on unknown boards"
            );
            assert_eq!(layout.default_hotkey(), "right_option");
        }
    }

    #[test]
    fn parses_real_ioreg_block() {
        // Trimmed from actual `ioreg -r -c AppleHIDKeyboardEventDriverV2 -d 1`
        // on the tester's machine.
        let sample = r#"
+-o AppleHIDKeyboardEventDriverV2  <class AppleHIDKeyboardEventDriverV2, id 0x100000abc>
    {
      "ProductIDArray" = (671)
      "Product" = "Magic Keyboard with Touch ID and Numeric Keypad"
      "VendorID" = 1452
      "ProductID" = 671
      "VendorIDSource" = 0
    }
"#;
        let candidates = parse_ioreg(sample);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].name,
            "Magic Keyboard with Touch ID and Numeric Keypad"
        );
        assert_eq!(candidates[0].vendor_id, Some(1452));
        assert_eq!(candidates[0].product_id, Some(671));

        let info = KeyboardInfo::from_candidate(&candidates[0], "ioreg");
        assert_eq!(info.layout, KeyboardLayout::MagicExtended);
        assert!(!info.fn_bottom_left);
        assert_eq!(info.default_hotkey, "right_option");
    }

    #[test]
    fn picks_external_over_builtin() {
        let candidates = vec![
            Candidate {
                name: "Apple Internal Keyboard / Trackpad".into(),
                vendor_id: Some(1452),
                product_id: Some(1),
            },
            Candidate {
                name: "Magic Keyboard with Numeric Keypad".into(),
                vendor_id: Some(1452),
                product_id: Some(671),
            },
        ];
        let best = pick_best(&candidates).unwrap();
        assert!(
            best.name.contains("Numeric"),
            "external extended board should win"
        );
    }

    #[test]
    fn parses_system_profiler_block() {
        let sample = "\
    USB 3.1 Bus:

      Host Controller Driver: AppleT8103USBXHCI

        Magic Keyboard with Touch ID and Numeric Keypad:

          Product ID: 0x029f
          Vendor ID: 0x05ac (Apple Inc.)
          Version: 4.20
";
        let candidates = parse_system_profiler(sample);
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].name,
            "Magic Keyboard with Touch ID and Numeric Keypad"
        );
        assert_eq!(candidates[0].product_id, Some(0x029f));
        assert_eq!(candidates[0].vendor_id, Some(0x05ac));
    }

    #[test]
    fn generic_fallback_never_claims_fn() {
        let g = KeyboardInfo::generic();
        assert_eq!(g.layout, KeyboardLayout::Generic);
        assert!(!g.fn_bottom_left);
        assert_eq!(g.model, "Unknown keyboard");
        assert_eq!(g.source, "none");
    }

    /// Real-runtime detection on the machine running the test. Machine-specific
    /// (asserts the tester's extended board), so it's `#[ignore]`d — run with:
    ///   cargo test --release detect_on_this_machine -- --ignored --nocapture
    #[test]
    #[ignore]
    fn detect_on_this_machine() {
        let info = detect();
        println!("DETECTED KEYBOARD: {info:#?}");
        // On the tester's Mac Studio this is the Magic Keyboard with Numeric
        // Keypad → extended layout, Control (not Fn) in the bottom-left.
        assert_eq!(
            info.layout,
            KeyboardLayout::MagicExtended,
            "expected extended board"
        );
        assert!(!info.fn_bottom_left, "bottom-left must be Control, not Fn");
        assert!(
            info.model.to_lowercase().contains("numeric"),
            "model should be the Numeric Keypad board, got {:?}",
            info.model
        );
        // And the drawn bottom row's first key on an extended board is Control.
        println!("bottom-left key for magic_extended diagram = Control (Left Control)");
    }
}
