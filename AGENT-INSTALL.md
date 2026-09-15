# Install FunButton with a coding agent

FunButton is a local-first, push-to-talk dictation app for macOS. This file is for
people who live in a coding agent — Claude Code, OpenClaw, Hermes, Cursor, Codex,
whatever. Paste the prompt below into your agent and it will install and verify
FunButton for you, then hand the last-mile permission grants back to you (because
macOS will not let any agent grant them — more on that below).

The machine-readable version of everything here lives at
[`https://funbutton.ai/install.json`](https://funbutton.ai/install.json) — an agent
should fetch that instead of scraping the marketing page. There is also an
[`https://funbutton.ai/llms.txt`](https://funbutton.ai/llms.txt) for discovery.

Ground truth you should know before you start:

- Apple Silicon (arm64) only. No Intel build exists.
- The alpha is **unsigned** (no Apple Developer ID yet). Left alone, macOS mislabels
  it as "damaged." The brew cask and the curl installer both clear that for you by
  stripping the quarantine flag from `FunButton.app` — and nothing else. `brew` does
  **not** do this on its own; our cask does it explicitly in a postflight.
- The on-device models (~1.1 GB) are **not** bundled. They download on first run.
- Three macOS permissions must be granted by a human. No agent can do it. Plan for a
  handoff, not a fully unattended finish.

## The prompt

Copy everything in the block and paste it into your agent.

```text
Install FunButton for me, a local-first push-to-talk dictation app for macOS.
It is Apple Silicon only, unsigned (alpha), GPLv3. Work through the steps in order
and STOP the moment a success check fails — do not improvise around a failure.

PREFLIGHT
- Run: uname -sm
  It must print "Darwin arm64". If it prints anything else, stop and tell me there
  is no build for this machine.

INSTALL — try these in order, stop at the first that succeeds:

1) Homebrew cask (preferred)
   Command: brew install --cask todddickerson/funbutton/funbutton
   Success: brew prints "was successfully installed" AND /Applications/FunButton.app
   exists. The cask's postflight strips the quarantine flag from FunButton.app
   (brew does NOT do this by default; the cask does it explicitly).
   If `brew` is not installed or the tap step errors, go to method 2.

2) curl one-liner (no Homebrew needed)
   Command: curl -fsSL https://funbutton.ai/install.sh | bash
   Success: it prints "FunButton is installed". The script uses no sudo and makes no
   global Gatekeeper change; you can read it at https://funbutton.ai/install.sh
   before running it. If it errors, go to method 3.

3) Manual DMG (last resort)
   Commands:
     curl -fL -o /tmp/FunButton.dmg https://funbutton.ai/download
     mkdir -p /tmp/fb-mnt
     hdiutil attach /tmp/FunButton.dmg -nobrowse -noautoopen -mountpoint /tmp/fb-mnt
     rm -rf /Applications/FunButton.app
     ditto /tmp/fb-mnt/FunButton.app /Applications/FunButton.app
     hdiutil detach /tmp/fb-mnt
     xattr -dr com.apple.quarantine /Applications/FunButton.app
   The final xattr line is REQUIRED here — the manual path is the only one that does
   not clear quarantine for you. NEVER run `spctl --master-disable` or otherwise
   disable Gatekeeper system-wide. If /Applications is not writable, stop and tell me.

VERIFY — all four must pass. Report each result:
a) App present:
     test -d /Applications/FunButton.app && echo OK
   Expect: OK
b) No quarantine:
     xattr /Applications/FunButton.app
   Expect: output does NOT contain "com.apple.quarantine".
c) Version is the current release:
     defaults read /Applications/FunButton.app/Contents/Info CFBundleShortVersionString
   Compare it to the "latest_version" field from:
     curl -fsSL https://funbutton.ai/install.json
   Expect: they match.
d) It launches with both engines up:
     /Applications/FunButton.app/Contents/MacOS/funbutton >/tmp/fb-launch.log 2>&1 &
     FB=$!; sleep 25
     grep -iE 'embedded STT model loaded|llama-server ready|cleanup engine reloaded' /tmp/fb-launch.log
     kill "$FB" 2>/dev/null
   Expect: the log shows "embedded STT model loaded" (the Whisper transcriber) AND
   "llama-server ready" or "cleanup engine reloaded" (the local cleanup LLM).
   FIRST-RUN NOTE: if the models are not present yet, the app downloads ~1.1 GB into
   ~/Library/Application Support/ai.funbutton.desktop/models/ before those lines
   appear. Tell me this download is happening, wait for it (it is resumable and
   SHA-256 verified), then re-check the log.

HAND OFF TO ME — you cannot finish this yourself:
FunButton needs three macOS permissions that only a human at an unlocked screen can
grant. No agent, script, MDM, or CLI can grant them — do not claim you did. Tell me
to open FunButton (menu-bar icon) and grant, in System Settings:
  - Microphone       -> System Settings > Privacy & Security > Microphone
  - Accessibility    -> System Settings > Privacy & Security > Accessibility
  - Input Monitoring -> System Settings > Privacy & Security > Input Monitoring
Then tell me to open FunButton's Settings (menu-bar icon) and pick a push-to-talk
hotkey (Fn, Right/Left Control, Right/Left Command, Right Option, or Caps Lock — the
app detects my keyboard; on extended Apple keyboards the bottom-left key is Control,
not Fn).

FINAL REPORT: which install method worked, the four verification results, and the
exact permission + hotkey steps I still need to do by hand.
```

## Why the handoff is non-negotiable

Microphone, Accessibility, and Input Monitoring are TCC permissions. macOS gates them
behind a human clicking a real toggle at an unlocked screen — by design, so malware
(and, yes, an over-eager agent) can't self-authorize a keylogger. There is no flag,
no `tccutil` grant, no MDM profile that grants them silently for a normal install. An
agent that says it "enabled Accessibility for you" is wrong. The honest move is to
install, verify, and then tell the human exactly which three toggles to flip.

## What each permission is for

| Permission | Why FunButton needs it |
|---|---|
| Microphone | Hear you while you hold the button. |
| Accessibility | Paste the cleaned text at your cursor. |
| Input Monitoring | Detect your push-to-talk key press. |

## The model download, so nobody is surprised

On first run FunButton fetches ~1.1 GB of on-device models (a Qwen 2.5 1.5B cleanup
model plus a small Whisper model) into
`~/Library/Application Support/ai.funbutton.desktop/models/`. It is resumable and
each file is SHA-256 verified. After that, transcription and cleanup run fully
offline. An agent should surface this before it happens — a silent 1.1 GB pull on a
metered connection is a bad surprise.

## For agents: the machine-readable contract

Fetch [`https://funbutton.ai/install.json`](https://funbutton.ai/install.json)
instead of parsing this page. It carries the latest version, the DMG URL and its
SHA-256, min macOS, arch, the install methods with their commands, the required
permissions, the model download size, and a `verify` block listing the commands and
expected results. It is schema-versioned (`schema_version`) so you can depend on its
shape.

## Links

- Machine-readable install contract: <https://funbutton.ai/install.json>
- Discovery / summary for agents: <https://funbutton.ai/llms.txt>
- The curl installer, in plain sight: <https://funbutton.ai/install.sh>
- Version-agnostic DMG download: <https://funbutton.ai/download>
- Source (GPLv3): <https://github.com/todddickerson/funbutton>
- Why it's unsigned, and the signing plan: [`SIGNING.md`](SIGNING.md)
