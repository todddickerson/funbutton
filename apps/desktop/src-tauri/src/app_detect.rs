use std::process::Command;
use std::sync::mpsc;
use std::time::Duration;

/// Hard ceiling on how long the pipeline will wait for frontmost-app
/// detection before proceeding with `FrontApp::Unknown`. The native
/// NSWorkspace path answers in microseconds; this budget only matters if
/// the osascript fallback runs and TCC stalls it (observed ~105 s during
/// release-binary smoke QA — finding #14).
pub const DETECT_TIMEOUT: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontApp {
    Cursor,
    VSCode,
    JetBrains,
    Vim,
    /// Terminal emulator — gets the literal-command TERMINAL prompt, never
    /// the prose-leaning CODE prompt.
    Terminal,
    Xcode,
    /// Any other code-mode surface: editors (Zed, Windsurf, Sublime, Emacs),
    /// AI IDEs (Kiro, Trae, Void, PearAI, Antigravity), and git clients
    /// (commit-message fields want conventional-commit cleanup too).
    Editor(String),
    Mail,
    Slack,
    Discord,
    Messages,
    Other(String),
    Unknown,
}

/// In-flight frontmost-app detection. Detection runs on its own thread so
/// the pipeline is never blocked by it: `wait_up_to` bounds the wait for
/// the STT-bias decision, and `now_or_unknown` picks up a late arrival at
/// cleanup time for free. If detection never returns (TCC-stalled
/// osascript), both degrade to `FrontApp::Unknown` and the thread is
/// abandoned — it exits on its own when the stalled call finally resolves.
pub struct DetectHandle {
    rx: mpsc::Receiver<FrontApp>,
    cached: Option<FrontApp>,
}

impl DetectHandle {
    pub fn spawn() -> Self {
        Self::spawn_with(FrontApp::detect_blocking)
    }

    /// Test seam: run an arbitrary detector on the background thread.
    fn spawn_with<F>(detect: F) -> Self
    where
        F: FnOnce() -> FrontApp + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(detect());
        });
        DetectHandle { rx, cached: None }
    }

    /// Wait for the detection result, but never longer than `timeout`.
    /// Returns `FrontApp::Unknown` on timeout (the result may still be
    /// picked up later via `now_or_unknown`).
    pub fn wait_up_to(&mut self, timeout: Duration) -> FrontApp {
        if self.cached.is_none() {
            self.cached = self.rx.recv_timeout(timeout).ok();
        }
        self.cached.clone().unwrap_or(FrontApp::Unknown)
    }

    /// Whatever is known right now, without waiting: the cached result, a
    /// late arrival, or `FrontApp::Unknown`.
    pub fn now_or_unknown(&mut self) -> FrontApp {
        if self.cached.is_none() {
            self.cached = self.rx.try_recv().ok();
        }
        self.cached.clone().unwrap_or(FrontApp::Unknown)
    }
}

impl FrontApp {
    /// Preferred path: `NSWorkspace.frontmostApplication` — synchronous,
    /// permission-free (no Automation prompt), and near-zero cost vs the
    /// 50-150 ms osascript round-trip. NSWorkspace/NSRunningApplication are
    /// documented thread-safe, and no TIS/TSM layout APIs are involved
    /// (those SIGTRAP off the main thread on macOS 26).
    #[cfg(target_os = "macos")]
    fn frontmost_name_native() -> Option<String> {
        let ws = objc2_app_kit::NSWorkspace::sharedWorkspace();
        let app = ws.frontmostApplication()?;
        app.localizedName().map(|n| n.to_string())
    }

    /// Blocking detection: native NSWorkspace first, osascript as the
    /// fallback for the unlikely case the native call yields nothing.
    /// Callers on the dictation path must go through `DetectHandle` — the
    /// osascript fallback can stall for minutes under a pending TCC prompt,
    /// and only the handle's timeout protects the pipeline from that.
    fn detect_blocking() -> Self {
        #[cfg(target_os = "macos")]
        {
            if let Some(name) = Self::frontmost_name_native() {
                return Self::classify(&name);
            }
            log::warn!("NSWorkspace returned no frontmost app; falling back to osascript");
            let out = Command::new("osascript")
                .args([
                    "-e",
                    "tell application \"System Events\" to get name of first application process whose frontmost is true",
                ])
                .output();
            match out {
                Ok(o) if o.status.success() => {
                    let name = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    Self::classify(&name)
                }
                _ => FrontApp::Unknown,
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            FrontApp::Unknown
        }
    }

    fn classify(name: &str) -> Self {
        let n = name.to_lowercase();
        if n.contains("cursor") {
            FrontApp::Cursor
        } else if n.contains("code") && n.contains("visual")
            || n == "code"
            || n == "code - insiders"
            || n == "vscodium"
        {
            FrontApp::VSCode
        } else if n.contains("intellij")
            || n.contains("pycharm")
            || n.contains("webstorm")
            || n.contains("rubymine")
            || n.contains("rustrover")
            || n.contains("goland")
            || n.contains("phpstorm")
            || n.contains("clion")
            || n.contains("android studio")
            || n.contains("datagrip")
            || n.contains("dataspell")
            || n == "rider"
            || n == "appcode"
            || n == "fleet"
        {
            FrontApp::JetBrains
        } else if n == "vim" || n == "neovim" || n == "nvim" || n == "macvim" {
            FrontApp::Vim
        } else if n == "terminal"
            || n == "iterm2"
            || n == "iterm"
            || n == "warp"
            || n == "alacritty"
            || n == "kitty"
            || n == "ghostty"
            || n == "wezterm"
            || n == "wezterm-gui"
            || n == "tabby"
            || n == "hyper"
            || n == "rio"
            || n == "wave"
            || n == "waveterm"
            || n == "termius"
        {
            FrontApp::Terminal
        } else if n == "zed" || n.starts_with("zed ") || n == "windsurf" || n.contains("sublime text") || n == "nova" || n == "textmate" || n == "bbedit" || n == "emacs" || n == "aquamacs" || n == "coteditor"
            // AI-first IDEs — the surfaces a dev-first dictation app lives in.
            || n == "kiro" || n == "trae" || n == "void" || n == "pearai" || n == "positron" || n == "antigravity"
            // Git clients: the focused field is almost always a commit message.
            || n == "github desktop" || n == "fork" || n == "tower" || n == "gitkraken" || n.contains("sublime merge") || n == "sourcetree" || n == "gitup"
        {
            FrontApp::Editor(name.to_string())
        } else if n == "xcode" {
            FrontApp::Xcode
        } else if n == "mail" {
            FrontApp::Mail
        } else if n == "slack" {
            FrontApp::Slack
        } else if n == "discord" {
            FrontApp::Discord
        } else if n == "messages" || n == "imessage" {
            FrontApp::Messages
        } else {
            FrontApp::Other(name.to_string())
        }
    }

    pub fn label(&self) -> String {
        match self {
            FrontApp::Cursor => "Cursor".into(),
            FrontApp::VSCode => "VS Code".into(),
            FrontApp::JetBrains => "JetBrains IDE".into(),
            FrontApp::Vim => "Vim".into(),
            FrontApp::Terminal => "Terminal".into(),
            FrontApp::Xcode => "Xcode".into(),
            FrontApp::Editor(s) => s.clone(),
            FrontApp::Mail => "Mail".into(),
            FrontApp::Slack => "Slack".into(),
            FrontApp::Discord => "Discord".into(),
            FrontApp::Messages => "Messages".into(),
            FrontApp::Other(s) => s.clone(),
            FrontApp::Unknown => "Unknown".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(name: &str) -> FrontApp {
        FrontApp::classify(name)
    }

    #[test]
    fn classifies_terminals() {
        for name in [
            "Terminal",
            "iTerm2",
            "Warp",
            "Alacritty",
            "kitty",
            "Ghostty",
            "WezTerm",
            "wezterm-gui",
            "Tabby",
            "Hyper",
            "Rio",
            "Wave",
            "Termius",
        ] {
            assert_eq!(c(name), FrontApp::Terminal, "{name} should be a terminal");
        }
    }

    #[test]
    fn classifies_editors_and_ai_ides() {
        for name in [
            "Zed",
            "Zed Preview",
            "Windsurf",
            "Sublime Text",
            "Nova",
            "TextMate",
            "BBEdit",
            "Emacs",
            "CotEditor",
            "Kiro",
            "Trae",
            "Void",
            "PearAI",
            "Positron",
            "Antigravity",
        ] {
            assert!(
                matches!(c(name), FrontApp::Editor(_)),
                "{name} should be an editor"
            );
        }
    }

    #[test]
    fn classifies_git_clients_as_code_surfaces() {
        for name in [
            "GitHub Desktop",
            "Fork",
            "Tower",
            "GitKraken",
            "Sublime Merge",
            "Sourcetree",
            "GitUp",
        ] {
            assert!(
                matches!(c(name), FrontApp::Editor(_)),
                "{name} should be a code surface"
            );
        }
    }

    #[test]
    fn classifies_vscode_family_and_jetbrains() {
        for name in ["Code", "Code - Insiders", "VSCodium", "Visual Studio Code"] {
            assert_eq!(c(name), FrontApp::VSCode, "{name}");
        }
        for name in [
            "IntelliJ IDEA",
            "PyCharm",
            "WebStorm",
            "RustRover",
            "GoLand",
            "CLion",
            "Android Studio",
            "DataGrip",
            "DataSpell",
            "Rider",
            "Fleet",
        ] {
            assert_eq!(c(name), FrontApp::JetBrains, "{name}");
        }
    }

    #[test]
    fn classifies_the_rest() {
        assert_eq!(c("Cursor"), FrontApp::Cursor);
        assert_eq!(c("Xcode"), FrontApp::Xcode);
        assert_eq!(c("MacVim"), FrontApp::Vim);
        assert_eq!(c("Mail"), FrontApp::Mail);
        assert_eq!(c("Slack"), FrontApp::Slack);
        assert_eq!(c("Discord"), FrontApp::Discord);
        assert_eq!(c("Messages"), FrontApp::Messages);
        assert_eq!(c("Safari"), FrontApp::Other("Safari".into()));
        assert_eq!(c("Google Chrome"), FrontApp::Other("Google Chrome".into()));
    }

    #[test]
    fn labels_are_stable() {
        assert_eq!(FrontApp::Terminal.label(), "Terminal");
        assert_eq!(c("Ghostty").label(), "Terminal");
        assert_eq!(c("Zed").label(), "Zed");
        assert_eq!(c("Safari").label(), "Safari");
    }

    // ---- DetectHandle: the pipeline must never block on detection --------

    #[test]
    fn stalled_detection_times_out_to_unknown_quickly() {
        // Simulates the TCC-stalled osascript that blocked the pipeline
        // ~105s in release-binary smoke QA: the handle must give up at the
        // timeout, not wait for the detector.
        let mut h = DetectHandle::spawn_with(|| {
            std::thread::sleep(Duration::from_secs(10));
            FrontApp::Terminal
        });
        let started = std::time::Instant::now();
        assert_eq!(h.wait_up_to(Duration::from_millis(50)), FrontApp::Unknown);
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "timeout path took {:?} — must return well under a second",
            started.elapsed()
        );
        // Still stalled at cleanup time → still Unknown, still instant.
        assert_eq!(h.now_or_unknown(), FrontApp::Unknown);
    }

    #[test]
    fn late_detection_is_picked_up_at_cleanup_time() {
        // Detector misses the STT-bias window but finishes during
        // transcription — the cleanup-time read must see the real result.
        let mut h = DetectHandle::spawn_with(|| {
            std::thread::sleep(Duration::from_millis(150));
            FrontApp::Cursor
        });
        assert_eq!(h.wait_up_to(Duration::from_millis(10)), FrontApp::Unknown);
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(h.now_or_unknown(), FrontApp::Cursor);
    }

    #[test]
    fn fast_detection_is_cached_across_both_reads() {
        let mut h = DetectHandle::spawn_with(|| FrontApp::Slack);
        assert_eq!(h.wait_up_to(Duration::from_secs(2)), FrontApp::Slack);
        assert_eq!(h.now_or_unknown(), FrontApp::Slack);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn native_detection_answers_within_budget() {
        // On a headless runner there may be no frontmost app (None is
        // fine); what must hold is that the native path never approaches
        // the timeout budget.
        let started = std::time::Instant::now();
        let _ = FrontApp::frontmost_name_native();
        assert!(
            started.elapsed() < DETECT_TIMEOUT,
            "native NSWorkspace lookup took {:?}",
            started.elapsed()
        );
    }
}
