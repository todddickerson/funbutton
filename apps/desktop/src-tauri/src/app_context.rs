//! Deep-context capture via macOS Accessibility (AX).
//!
//! Reads the focused window title, the focused element's role, and any
//! selected text from the frontmost app, so the cleanup prompt can spell
//! names, identifiers, and jargon the way they appear on screen (a window
//! titled `auth_middleware.rs — funbutton` biases the model to that exact
//! identifier). This is the borrowed shape of freeflow's context awareness
//! (reference/freeflow-swift/AppContextService.swift) — the APPROACH, in
//! Rust/GPLv3, not the Swift.
//!
//! ## Hard constraints (see the ctxmodes brief)
//!
//! - **Never blocks the hotkey path.** Every read runs off the dictation
//!   thread inside `app_detect::DetectHandle`, and each AX round-trip is
//!   bounded by `AXUIElementSetMessagingTimeout` so a hung target app can't
//!   stall past a few hundred ms. If the read is slow it simply never arrives
//!   and no context is injected — dictation is unaffected.
//! - **Degrades to nothing.** Accessibility may be ungranted, or an app may
//!   refuse a given attribute; every path returns `None`, never panics, never
//!   errors up.
//! - **No layout / input-source APIs.** This module touches only the AX
//!   (HIServices) API surface. It never calls TIS/TSM/UCKeyTranslate — those
//!   are the macOS 26 `dispatch_assert_queue` SIGTRAP that killed a tester's
//!   install (see the memory note + `hotkey.rs`). AX calls are safe here.
//! - **Privacy.** Window titles and selected text are sensitive. This module
//!   never logs their content (callers log presence booleans only), and the
//!   captured context is used solely inside the local cleanup prompt — see
//!   `pipeline.rs` for the local-only-by-default routing.
//!
//! Zero new crates: the AX C functions are declared here against the
//! `ApplicationServices` framework, using `core-foundation` types that are
//! already in the dependency tree.

/// One dictation's worth of on-screen context. Every field is optional and
/// independently degradable; an all-`None` snapshot is the norm when
/// Accessibility is ungranted and is handled exactly like "no context".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FocusContext {
    /// Title of the focused window (e.g. `auth_middleware.rs — funbutton`).
    pub window_title: Option<String>,
    /// AX role of the focused element (e.g. `AXTextArea`) — a coarse hint at
    /// whether the user is in a text field vs elsewhere.
    pub focused_role: Option<String>,
    /// Currently selected text near the caret, if the app exposes it. Gives
    /// the cleanup model tone/vocabulary context.
    pub selected_text: Option<String>,
}

impl FocusContext {
    /// True when there is nothing worth injecting — same as no context.
    pub fn is_empty(&self) -> bool {
        self.window_title.is_none() && self.focused_role.is_none() && self.selected_text.is_none()
    }
}

/// Max chars kept for a window title / role before we truncate. Titles are
/// usually short; this only bounds pathological cases.
const MAX_TITLE: usize = 200;
/// Max chars kept for selected text — enough for tone/vocabulary context
/// without ballooning the prompt (or capturing a whole document).
const MAX_SELECTION: usize = 280;

/// Normalize a short attribute (window title / role): collapse newlines to
/// spaces, trim, cap length. Returns `None` when empty after cleaning.
pub(crate) fn clean_short(raw: &str) -> Option<String> {
    clean(raw, MAX_TITLE)
}

/// Normalize selected text: same collapse/trim, larger cap.
pub(crate) fn clean_selection(raw: &str) -> Option<String> {
    clean(raw, MAX_SELECTION)
}

fn clean(raw: &str, max: usize) -> Option<String> {
    let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Truncate on a char boundary so multibyte titles never split mid-glyph.
    let out: String = trimmed.chars().take(max).collect();
    Some(out)
}

/// Whether this process is trusted for Accessibility. Deep-context reads are
/// impossible without it (every AX Copy call errors), so this is the gate that
/// decides whether the feature can do anything at all. Non-prompting.
#[cfg(target_os = "macos")]
pub fn ax_trusted() -> bool {
    unsafe { ax::AXIsProcessTrusted() != 0 }
}

#[cfg(not(target_os = "macos"))]
pub fn ax_trusted() -> bool {
    false
}

/// Read the focused window title, focused element role, and selected text for
/// the app with process id `pid`. Never blocks unboundedly, never panics;
/// returns an empty `FocusContext` when Accessibility is ungranted or nothing
/// is readable.
#[cfg(target_os = "macos")]
pub fn read_focus_context(pid: i32) -> FocusContext {
    unsafe { read_focus_context_impl(pid) }
}

#[cfg(not(target_os = "macos"))]
pub fn read_focus_context(_pid: i32) -> FocusContext {
    FocusContext::default()
}

#[cfg(target_os = "macos")]
mod ax {
    #![allow(non_snake_case, non_upper_case_globals)]
    use core_foundation::base::{CFTypeID, CFTypeRef};
    use core_foundation::string::CFStringRef;

    pub type AXUIElementRef = CFTypeRef;
    pub type AXError = i32;
    pub const kAXErrorSuccess: AXError = 0;

    // AXUIElement* live in HIServices, part of the ApplicationServices
    // umbrella framework. Linking the umbrella resolves them.
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
        pub fn AXUIElementCopyAttributeValue(
            element: AXUIElementRef,
            attribute: CFStringRef,
            value: *mut CFTypeRef,
        ) -> AXError;
        pub fn AXUIElementSetMessagingTimeout(
            element: AXUIElementRef,
            timeoutInSeconds: f32,
        ) -> AXError;
        pub fn AXUIElementGetTypeID() -> CFTypeID;
        /// Non-prompting trust check (unlike `AXIsProcessTrustedWithOptions`).
        pub fn AXIsProcessTrusted() -> u8;
    }
}

#[cfg(target_os = "macos")]
unsafe fn read_focus_context_impl(pid: i32) -> FocusContext {
    use core_foundation::base::{CFGetTypeID, CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};

    // Ungranted Accessibility → every Copy call would error anyway. Short
    // circuit so we neither spin nor risk any first-call side effects.
    if ax::AXIsProcessTrusted() == 0 {
        return FocusContext::default();
    }

    let app = ax::AXUIElementCreateApplication(pid);
    if app.is_null() {
        return FocusContext::default();
    }
    // Bound every AX round-trip. The default messaging timeout is effectively
    // unbounded for a wedged target; 0.4s keeps us off the hotkey path's back.
    let _ = ax::AXUIElementSetMessagingTimeout(app, 0.4);

    // Read a string-valued attribute. `AXUIElementCopyAttributeValue` returns
    // a +1 reference (the CF "Copy" rule); `wrap_under_create_rule` adopts it,
    // and a non-CFString value is released explicitly.
    let copy_string = |element: ax::AXUIElementRef, attr: &str| -> Option<String> {
        let attr_cf = CFString::new(attr);
        let mut value: CFTypeRef = std::ptr::null();
        let err =
            ax::AXUIElementCopyAttributeValue(element, attr_cf.as_concrete_TypeRef(), &mut value);
        if err != ax::kAXErrorSuccess || value.is_null() {
            return None;
        }
        if CFGetTypeID(value) == CFString::type_id() {
            Some(CFString::wrap_under_create_rule(value as CFStringRef).to_string())
        } else {
            CFRelease(value);
            None
        }
    };

    // Read an element-valued attribute (focused window / focused element).
    // The caller owns the returned ref and must `CFRelease` it.
    let copy_element = |element: ax::AXUIElementRef, attr: &str| -> Option<ax::AXUIElementRef> {
        let attr_cf = CFString::new(attr);
        let mut value: CFTypeRef = std::ptr::null();
        let err =
            ax::AXUIElementCopyAttributeValue(element, attr_cf.as_concrete_TypeRef(), &mut value);
        if err != ax::kAXErrorSuccess || value.is_null() {
            return None;
        }
        if CFGetTypeID(value) == ax::AXUIElementGetTypeID() {
            Some(value)
        } else {
            CFRelease(value);
            None
        }
    };

    let mut out = FocusContext::default();

    // Focused window → title.
    if let Some(win) = copy_element(app, "AXFocusedWindow") {
        out.window_title = copy_string(win, "AXTitle").and_then(|s| clean_short(&s));
        CFRelease(win);
    }

    // Focused element → role + selected text.
    if let Some(el) = copy_element(app, "AXFocusedUIElement") {
        out.focused_role = copy_string(el, "AXRole").and_then(|s| clean_short(&s));
        out.selected_text = copy_string(el, "AXSelectedText").and_then(|s| clean_selection(&s));
        CFRelease(el);
    }

    // Some apps expose the selection on the app element rather than the
    // focused element — fall back to that.
    if out.selected_text.is_none() {
        out.selected_text = copy_string(app, "AXSelectedText").and_then(|s| clean_selection(&s));
    }

    CFRelease(app);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_collapses_whitespace_and_trims() {
        assert_eq!(
            clean_short("  auth_middleware.rs   —  funbutton \n"),
            Some("auth_middleware.rs — funbutton".to_string())
        );
        assert_eq!(clean_short("   \n  \t "), None);
        assert_eq!(clean_short(""), None);
    }

    #[test]
    fn clean_truncates_on_char_boundary() {
        let long = "x".repeat(500);
        let out = clean_short(&long).unwrap();
        assert_eq!(out.chars().count(), MAX_TITLE);
        // Multibyte must never split mid-glyph.
        let emoji = "🚀".repeat(500);
        let out = clean_selection(&emoji).unwrap();
        assert_eq!(out.chars().count(), MAX_SELECTION);
        assert!(out.chars().all(|c| c == '🚀'));
    }

    #[test]
    fn selection_cap_is_larger_than_title_cap() {
        const { assert!(MAX_SELECTION > MAX_TITLE) };
    }

    #[test]
    fn empty_context_is_empty() {
        assert!(FocusContext::default().is_empty());
        let c = FocusContext {
            window_title: Some("x".into()),
            ..Default::default()
        };
        assert!(!c.is_empty());
    }
}
