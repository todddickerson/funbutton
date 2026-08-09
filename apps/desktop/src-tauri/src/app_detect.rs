use std::process::Command;

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

impl FrontApp {
    /// One osascript round-trip, then pure string matching — keep it that
    /// way. Latency here is paid on every single dictation.
    pub fn detect() -> Self {
        #[cfg(target_os = "macos")]
        {
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
}
