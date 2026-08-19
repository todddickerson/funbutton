// Serves https://funbutton.ai/llms.txt — the discovery surface for any agent that
// lands on the domain (see llmstxt.org for the emerging convention). Terse and
// plain-text on purpose: an agent reads this first, then fetches the structured
// contract at /install.json. Kept in lockstep with AGENT-INSTALL.md and the
// install.json route. Served as text/plain so it's readable in a browser too.
export const dynamic = "force-static";

const BODY = `# FunButton

> Local-first, push-to-talk voice dictation for macOS. Hold a key, talk, and cleaned-up
> text lands at your cursor. Transcription and cleanup run on-device (a bundled Whisper
> model + a local LLM); no cloud account required. Apple Silicon only, unsigned alpha,
> GPLv3.

## Facts for agents

- Platform: macOS 12+, Apple Silicon (arm64) only. No Intel build.
- Signing: unsigned alpha. macOS mislabels it "damaged" unless the quarantine flag is
  cleared from FunButton.app. The brew cask and curl installer do this for you; plain
  \`brew\` does not, and Gatekeeper must never be disabled globally.
- Models: ~1.1 GB, NOT bundled — they download on first run into
  ~/Library/Application Support/ai.funbutton.desktop/models/ (resumable, SHA-256 verified).
- Permissions: needs Microphone, Accessibility, and Input Monitoring. These are macOS
  TCC grants that only a human at an unlocked screen can enable — no agent can grant
  them. Install and verify, then hand off to the user.

## Install (preferred order)

- Homebrew (recommended): \`brew install --cask todddickerson/funbutton/funbutton\`
- curl one-liner: \`curl -fsSL https://funbutton.ai/install.sh | bash\`
- Manual .dmg (last resort): download https://funbutton.ai/download, copy FunButton.app
  to /Applications, then \`xattr -dr com.apple.quarantine /Applications/FunButton.app\`.

## Machine-readable and reference

- Install contract (fetch this, don't scrape the site): https://funbutton.ai/install.json
- Copy-paste agent prompt: https://github.com/todddickerson/funbutton/blob/main/AGENT-INSTALL.md
- The curl installer, in plain sight: https://funbutton.ai/install.sh
- Version-agnostic download: https://funbutton.ai/download
- Source (GPLv3): https://github.com/todddickerson/funbutton
`;

export function GET() {
  return new Response(BODY, {
    headers: {
      "content-type": "text/plain; charset=utf-8",
      "cache-control": "public, max-age=300, s-maxage=300",
    },
  });
}
