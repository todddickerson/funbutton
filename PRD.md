# FunButton.ai — PRD (MVP through V1)

**Created:** 2026-05-08
**Owner:** Todd Dickerson
**Build window:** Fri 5pm → Mon 9am (~64 hours, autonomous coding-agent build)
**Goal:** Working macOS dictation app Todd can use Monday morning. Wispr Flow competitor. Local-first, half the price, half the resource footprint.

---

## North Star
Todd presses Right Option, talks, releases, and his cleaned-up dictation appears at his cursor in <2 seconds. No cloud lock-in. No 800MB Electron tax. No subscription required.

## Positioning
- **vs Wispr Flow:** Local-first by default, Linux-friendly, lifetime pricing, open-source desktop core, half the price
- **vs SuperWhisper / MacWhisper:** Cross-platform, AI cleanup as first-class layer, dev-friendly (handles code/symbols), Tauri lightweight
- **vs native Dictation:** Actual AI cleanup, customizable, hotkey-driven, app-aware

## Tech Stack (locked in)
| Layer | Choice | Why |
|---|---|---|
| Desktop shell | **Tauri 2** (Rust core + React/TS UI) | 10MB bundle vs 100MB Electron; ~50MB RAM idle |
| Audio capture | `cpal` (Rust crate) | Cross-platform, low-level CoreAudio/WASAPI |
| Global hotkey | `rdev` or `tauri-plugin-global-shortcut` | Sequoia-compatible |
| ASR (MVP) | **Groq Whisper Large v3 Turbo** (cloud, $0.00667/min, ~300ms TTFT) | Fastest cloud, no local-model startup time |
| ASR (V1.1) | `whisper.cpp` local model option | Offline mode, privacy tier |
| AI cleanup | **Groq Llama 3.3 70B** (~200ms TTFT, $0.59/M tokens) | Fast, cheap, Wispr-quality |
| Text injection | Clipboard + Cmd+V paste (macOS), SendInput (Win) | Most reliable across all apps |
| Settings UI | React + TypeScript + Tailwind v4 + shadcn/ui | Standard stack |
| State | Tauri store + SQLite for history | Local first |

## API Keys (env)
- `GROQ_API_KEY` — already in ~/clawd/.env
- `ANTHROPIC_API_KEY` — fallback for higher-quality cleanup tier
- `OPENAI_API_KEY` — Whisper API fallback if Groq down

For shipped app: user enters their own Groq key in Settings. Defaults pull from env in dev.

---

## Scope: 3 Sprints

### Sprint 1 — MVP (Sat by EOD)
**Acceptance:** Todd can hold Right Option, talk for up to 60s, release, see clean text appear at cursor in macOS Notes/iMessage/Slack/Cursor.

- [ ] Tauri 2 project scaffolded, builds on macOS arm64
- [ ] Global hotkey: Right Option (configurable later) — push-to-talk
- [ ] Audio capture via `cpal` to in-memory PCM, encoded to wav/flac/mp3 in memory
- [ ] Visible recording indicator: menu bar icon + small floating pill UI showing waveform
- [ ] On release: POST audio to Groq Whisper Turbo, get transcript
- [ ] POST transcript to Groq Llama 3.3 with cleanup prompt → cleaned text
- [ ] Inject cleaned text via clipboard + paste shortcut (preserve original clipboard)
- [ ] Settings window: API key entry, hotkey display, basic stats (today's word count)
- [ ] Menu bar app: idle/recording/processing states, quit option
- [ ] Auto-launch on login (optional toggle)
- [ ] First-run flow: Accessibility permission prompt with clear instructions

### Sprint 2 — V1 (Sun by EOD)
**Acceptance:** Modes work, history exists, dictionary handles brand names, app feels polished.

- [ ] Modes: Auto / Email / Slack / Code / Raw
   - Auto picks based on frontmost app (Mail.app → Email, VS Code/Cursor → Code, Slack → Slack)
- [ ] Mode-specific cleanup prompts
- [ ] Code mode: handles spoken symbols ("camelCase", "snake_case", "open paren", "equals", "arrow", "open curly", etc.)
- [ ] Custom dictionary: user-added terms boost during cleanup ("Spontent", "ClickFunnels", etc.)
- [ ] Transcription history view (local SQLite)
- [ ] Cmd+Shift+V to re-paste last transcription
- [ ] Cmd+Shift+H to toggle history window
- [ ] Per-mode tone tuning sliders (formal ↔ casual)
- [ ] Polished onboarding: 30-second walkthrough on first launch
- [ ] Local Whisper fallback toggle (whisper.cpp + tiny.en model bundled)

### Sprint 3 — Ship (Mon AM)
**Acceptance:** Signed (or unsigned with bypass instructions), packaged .dmg/.app, GitHub release, landing page live, demo video, Todd actually using it.

- [ ] Build universal .app bundle (arm64 minimum, universal if time)
- [ ] DMG with drag-to-Applications layout
- [ ] First release on GitHub Releases (v0.1.0)
- [ ] Auto-update via Tauri updater (server: GitHub Releases JSON endpoint)
- [ ] Landing page at funbutton.ai (Vercel) — hero, demo gif, download button, pricing tease
- [ ] 30-second screen recording demo (Todd drives, agent records script)
- [ ] PROGRESS.md committed each sprint with what shipped + what slipped

### V1.1 / V2 (post-Monday — backlog)
- Windows build (Tauri 2 makes this 1-2 days)
- Linux build (X11 day-1, Wayland later)
- Streaming transcription (Deepgram or local Parakeet TDT)
- Stripe billing, license server, paid tiers
- Team features
- Telemetry (privacy-respecting, PostHog with opt-in)
- Code-signing with Apple Developer ID + notarization
- iOS keyboard companion
- "Open core" repo split: desktop OSS (MIT), cloud sync proprietary

---

## Pricing (decision for landing page)
- **Free:** Unlimited local Whisper (BYO compute), bring-your-own-API-key for cloud
- **Pro $7/mo annual / $10/mo monthly:** Includes cloud quota (10K words/month), all modes, sync
- **Lifetime $99 (first 1,000 customers, then $149):** Founder's pricing, kills subscription fatigue
- **Team $5/seat/mo (5+ seats):** Shared dictionary, admin

---

## Risk Register & Pre-Mitigations (so the agent doesn't get stuck)

| Risk | Mitigation |
|---|---|
| macOS Accessibility permission complex flow | Use `tauri-plugin-macos-permissions` + onboarding screen with screenshot |
| Code signing fails / Apple Dev ID missing | Ship unsigned .dmg, document `xattr -cr /Applications/FunButton.app` Gatekeeper bypass for Todd |
| Tauri 2 plugin missing for X | Drop to direct Rust crate, don't fight the plugin system |
| Audio glitches / low quality | Encode to 16kHz mono FLAC for Whisper API (recommended input) |
| Groq rate limits during dev | Local Whisper fallback (whisper.cpp tiny model, bundle weights) |
| Hotkey conflicts | Right Option default; settings UI lets user remap; document common conflicts |
| Cmd+V paste not working in some apps | Detect frontmost app, fall back to AppleScript / `enigo` keystroke simulation |
| Tauri build issues | Pin all versions in Cargo.toml + package.json; commit lockfiles |
| Universal binary fails | Ship arm64-only for MVP (Todd's Mac Studio is arm64) |
| Auto-update server complexity | GitHub Releases JSON endpoint — Tauri has native support |

---

## Communication Protocol (agent ↔ Todd)

The coding agent will:
1. Commit + push to `github.com/todddickerson/funbutton` after every working unit (~30-60 min cycles)
2. Update `PROGRESS.md` after each cycle with: what shipped, what's blocked, what's next
3. Post to Telegram **FunButton.ai group** (chat_id `-5180707304`) only when:
   - A sprint is complete (with screenshot/video proof)
   - Genuinely stuck and needs Todd's input (rare — try every alternative first)
   - Ready for Todd to test
4. Never mark "shipped" until binary actually built + installable + tested

If stuck on a hard blocker: try 3 alternative approaches before pinging Todd. Document each attempt in PROGRESS.md.

---

## Success Criteria for Monday Morning Demo
1. ✅ App installed on Todd's Mac Studio
2. ✅ Right Option hotkey works in Cursor, Slack, iMessage, Mail, Notes, Discord
3. ✅ Cleanup output is noticeably better than raw transcription
4. ✅ Latency feels comparable to or better than Wispr Flow
5. ✅ App uses <100MB RAM idle, <300MB while transcribing
6. ✅ Landing page live at funbutton.ai
7. ✅ GitHub repo public with 1+ release
8. ✅ Todd can show it to Russell and feel proud
