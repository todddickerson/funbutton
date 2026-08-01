use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrontApp {
    Cursor,
    VSCode,
    JetBrains,
    Vim,
    Terminal,
    Xcode,
    /// Any other code editor (Zed, Windsurf, Sublime, Emacs, …) — code mode.
    Editor(String),
    Mail,
    Slack,
    Discord,
    Messages,
    Other(String),
    Unknown,
}

impl FrontApp {
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
        } else if n.contains("code") && n.contains("visual") || n == "code" || n == "code - insiders" || n == "vscodium" {
            FrontApp::VSCode
        } else if n.contains("intellij") || n.contains("pycharm") || n.contains("webstorm") || n.contains("rubymine") || n.contains("rustrover") || n.contains("goland") || n.contains("phpstorm") || n.contains("clion") || n.contains("android studio") || n.contains("datagrip") || n == "fleet" {
            FrontApp::JetBrains
        } else if n == "vim" || n == "neovim" || n == "nvim" || n == "macvim" {
            FrontApp::Vim
        } else if n == "terminal" || n == "iterm2" || n == "iterm" || n == "warp" || n == "alacritty" || n == "kitty" || n == "ghostty" || n == "wezterm" || n == "wezterm-gui" || n == "tabby" || n == "hyper" || n == "rio" {
            FrontApp::Terminal
        } else if n == "zed" || n == "windsurf" || n.contains("sublime text") || n == "nova" || n == "textmate" || n == "bbedit" || n == "emacs" || n == "aquamacs" {
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
