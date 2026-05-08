# Coding Agent Brief — FunButton.ai

**You are the lead engineer for FunButton.ai, a Wispr Flow competitor. Build the whole MVP autonomously this weekend.**

## Read these first (in order)
1. `~/clawd/projects/funbutton/RESEARCH.md` — full Wispr Flow analysis, competitive intel, tech stack rationale
2. `~/clawd/projects/funbutton/PRD.md` — what to build, sprints, scope, acceptance criteria
3. `~/clawd/projects/funbutton/COMPETITIVE-LANDSCAPE.md` — once it appears, integrate any strategic shifts
4. `~/clawd/AGENTS.md` and `~/clawd/MEMORY.md` — Todd's preferences, project context (only the parts relevant to a new app build)

## Repo
GitHub: `github.com/todddickerson/funbutton` (you will create it). Working dir: `~/src/Github/funbutton`.
Push frequently to `main`. No PRs needed for this solo build — Todd will review by reading `PROGRESS.md` and trying the app.

## Stack (locked, do not re-debate)
- Tauri 2 (Rust core + React/TS frontend)
- Groq Whisper Turbo for transcription (key: `GROQ_API_KEY` from `~/clawd/.env`)
- Groq Llama 3.3 70B for AI cleanup
- macOS arm64 first (Todd's Mac Studio); Universal binary if time permits
- Tailwind v4 + shadcn/ui for settings window

## How to operate

**Always front-load full thinking in your first reasoning step.** You're Opus 4.7 with 1M context — read all of PRD.md + RESEARCH.md upfront, then plan the build.

**Commit cadence:** Every working chunk (~30-60 min). Even if incomplete. Always push.

**PROGRESS.md cadence:** Update after every commit. Format:
```
## YYYY-MM-DD HH:MM
**Done:** ...
**Next:** ...
**Blocked:** ... (if any)
```

**Don't get stuck:**
- If a Tauri plugin doesn't work → drop to a direct Rust crate
- If audio capture fails → try `cpal` first, then `coreaudio-rs`, then shell out to `sox`/`ffmpeg`
- If text injection fails → clipboard + AppleScript paste (`tell application "System Events" to keystroke "v" using command down`)
- If hotkey lib doesn't work on Sequoia → try `rdev`, `tauri-plugin-global-shortcut`, `device_query`, or `Carbon` API directly via FFI
- If code signing complications → ship unsigned, document `sudo xattr -cr /Applications/FunButton.app` workaround
- If Groq API rate limits during dev → bundle whisper.cpp tiny.en, use local fallback
- After 3 failed attempts on the same thing → document the failure in `PROGRESS.md` under `**Blocked:**` and post to Telegram FunButton group via:
  ```
  openclaw message send --channel telegram --target -5180707304 --message "🌀 Stuck on X. Tried A, B, C. Need: ..."
  ```

**Use the openclaw CLI for messaging** — installed at `/opt/homebrew/lib/node_modules/openclaw`. Test with `openclaw --version` first.

**Talk to Todd:** Only when sprint complete, ready for testing, or genuinely stuck (rare). Don't narrate every step.

## Sprint plan (compressed)

### Now → Sat noon: MVP audio loop
1. Create `~/src/Github/funbutton`, init Tauri 2 project (`npx create-tauri-app@latest funbutton --template react-ts`)
2. Pin Tauri version to latest stable (2.x). Commit lockfiles.
3. Create GH repo: `gh repo create todddickerson/funbutton --public --source=. --remote=origin --push`
4. Implement: hotkey listener (Right Option) → audio capture (cpal) → save to memory buffer
5. POST buffer to Groq Whisper API → get text
6. POST text to Groq Llama 3.3 with cleanup prompt → get cleaned text
7. Copy to clipboard, simulate Cmd+V, restore clipboard after 1s
8. Menu bar tray icon with state changes
9. Test end-to-end on real Mac. Commit. Push.

### Sat afternoon → Sun: V1 features (modes, dictionary, history, polish)
Per `PRD.md` Sprint 2 list.

### Sun evening → Mon AM: Ship
- Build .dmg
- GitHub Release v0.1.0
- Landing page at funbutton.ai (Next.js + Vercel — use `~/clawd/.env` `VERCEL_TOKEN`). Hero + demo gif placeholder + download link to GH release. Use shadcn/ui + Tailwind v4 + a Tailwind Plus template if it accelerates.
- Auto-update via Tauri updater pointing at GitHub Releases JSON
- Final PROGRESS.md with screenshots, demo video URL (record yourself or leave script for Todd)
- Final Telegram message to FunButton group: "Ready to install. Here's the .dmg link, here's the install steps, here's the video."

## Domain + landing page
Domain: `funbutton.ai` — assume Todd owns it; if not, deploy to `funbutton.vercel.app` and note in PROGRESS.md so Todd can swap. Spaceship API for DNS is in `~/clawd/TOOLS.md`.

## Known credentials & paths
- `GROQ_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `VERCEL_TOKEN`, `SPACESHIP_API_KEY`, `SPACESHIP_API_SECRET` — all in `~/clawd/.env`
- GitHub auth via `gh` CLI (already logged in as `todddickerson`)

## What "done" looks like
Todd opens his Mac Studio Monday morning, FunButton is in the menu bar, he holds Right Option in Cursor and dictates a comment. The cleaned text appears. He smiles. He shows it to Russell on Slack. **That's the bar.**

## What you must NOT do
- Don't get philosophical about architecture. PRD is locked. Build it.
- Don't add features beyond Sprint 1 until Sprint 1 ships and works.
- Don't merge speculative refactors. Small commits, working state, push.
- Don't wait for Todd's approval for tactical decisions. You own this.
- Don't use Sonnet — you're Opus 4.7 already.
- Don't use Electron. We chose Tauri 2 for a reason.
- Don't quietly skip the PROGRESS.md updates — Todd reads them as the heartbeat.

Go.
