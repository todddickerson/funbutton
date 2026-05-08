# FunButton.ai — Build Progress Log

> Heartbeat for Todd. One entry per commit-cycle. Newest at top.

---

## 2026-05-08 18:10 — Sprint 2 features + funbutton.ai live

**Done:**
- **Cmd+Shift+V re-paste:** `tauri-plugin-global-shortcut` registers the chord. Stores last cleaned in `AppState.last_cleaned`. Press the chord and the last clean text re-injects at the cursor. Falls back silently if the shortcut is busy.
- **Mode override** in settings: Auto / Code / Email / Slack / Raw radios. Overrides the auto-detected mode from the frontmost-app classifier.
- **Custom dictionary:** textarea, one term per line. Cleanup prompt is augmented with `USER DICTIONARY` block so brand names ("ClickFunnels", "Spontent") land verbatim even when Whisper hears them slightly off.
- **funbutton.ai DNS:** Spaceship API call set apex + www to A 76.76.21.21 (Vercel's IP). Vercel project linked. `funbutton.ai` resolves and returns HTTP 200 already. HTTPS coming online as Vercel provisions Let's Encrypt.
- New release Tauri build in progress.

**What's still queued for Sprint 2:**
- Bundled local LLM (Qwen 2.5 1.5B GGUF + embedded llama.cpp) — high effort, deferred for end-of-Sunday push if time permits.
- Transcription history + SQLite (the re-paste hotkey covers ~80% of the value).
- Hotkey remap UI (settings shows the label but doesn't let user rebind yet — Right Option default is fine for Todd's keyboard).
- Polished onboarding wizard.

**Blocked:** none.

---

## 2026-05-08 17:50 — Landing page live at funbutton.vercel.app

**Done:**
- Deployed `apps/web` to Vercel via `bootstrapped-cf259a39` team.
- **Live:** https://funbutton.vercel.app — punk hero, code-mode demos, vs-Wispr table, pricing tiers.
- HTTP 200, 1.3s build, all 4 routes prerender static.

**Domain hookup deferred:** `funbutton.ai` not yet wired through Spaceship → Vercel. If Todd already owns the domain, run `vercel domains add funbutton.ai` against the `bootstrapped-cf259a39` team to link. If not, register first, then alias.

**Tried Telegram update via openclaw:** `directory groups list --channel telegram` returns "No groups found" — the FunButton group hasn't been seen by the bot yet. Updates will land here in PROGRESS.md and on the GH release until that's fixed.

---

## 2026-05-08 17:40 — Sprint 1 MVP shipped: v0.1.0-alpha released

**Done:**
- Tauri 2.11.1 release build for aarch64-apple-darwin compiles clean (1m 58s).
- `.app` bundle: 36 MB, idles at expected RAM, launches without crash, stays alive in tray.
- DMG built via `hdiutil` (Tauri's `bundle_dmg.sh` hits AppleScript timeout on Sequoia — known issue, worked around).
- Both `.dmg` (5.1 MB compressed) and `.app.zip` (4.6 MB) attached.
- **GitHub Release v0.1.0-alpha live:** https://github.com/todddickerson/funbutton/releases/tag/v0.1.0-alpha
- Verified Groq pipeline directly with curl: Whisper Turbo + Llama 3.3 70B both respond as expected. Code mode prompt produces sensible output for spoken-symbol input.
- **Landing page** at `apps/web/` (Next.js 16 + Tailwind v4): hero, how-it-works with code-mode demos, vs-Wispr table, pricing tiers, GPLv3 footer. Builds clean (`next build` succeeds, all routes static-prerender). Ready to deploy to Vercel.
- Info.plist auto-merged into `.app`: `NSMicrophoneUsageDescription`, `NSAppleEventsUsageDescription`, `LSUIElement` (menu bar app, no Dock icon).
- Three commits + push, two artifact uploads, one tag (`v0.1.0-alpha`).

**What landed in MVP:**
- Right Option push-to-talk (rdev modifier-only listener)
- cpal audio capture, in-memory WAV PCM-16
- Groq Whisper Turbo transcription
- Groq Llama 3.3 70B cleanup (FAST default)
- **Ollama auto-detect at localhost:11434** with auto/groq/local toggle (Sprint 2 will bundle GGUF)
- **Code mode** with full spoken-symbol vocab + casing taxonomy, auto-activates on Cursor/VS Code/JetBrains/Vim/Terminal/Xcode
- Frontmost-app classifier (osascript shell-out, classifies 10+ apps)
- Clipboard + Cmd+V injection via enigo, prior-clipboard restore after 900ms
- Settings window: API key, backend toggle with live Ollama health check, today's word counter
- Floating recording pill (transparent, always-on-top)
- Tray with Settings + Quit, state-aware tooltip
- GPLv3 license, ~36 MB unsigned `.app`

**Next (Sprint 2 — Sun EOD):**
- Bundle Qwen 2.5 1.5B GGUF via embedded llama.cpp so "no API key, ever" is literally true post-install
- Email / Slack / Raw modes alongside Auto / Code
- Custom dictionary (boost user's brand names / jargon during cleanup)
- Transcription history (local SQLite)
- Cmd+Shift+V re-paste, Cmd+Shift+H toggle history
- Polished punk-flavored onboarding wizard
- Hotkey remap UI in settings

**Sprint 3 (Mon AM):**
- Vercel deploy of landing page to `funbutton.ai`
- Tauri auto-updater pointing at GitHub Releases JSON
- 30-second demo gif/video for landing
- Post-final Telegram with .dmg link + install steps

**Blocked:** none.

**Risk notes:**
- rdev maps Right Option to `Key::AltGr` on macOS; if Todd's keyboard layout differs, hotkey may not fire. Sprint 2 settings UI lets him remap.
- App is unsigned — Gatekeeper bypass (`sudo xattr -cr ...`) is documented in release notes.
- Tauri DMG bundler is broken on Sequoia; using `hdiutil` directly. Plain DMG, no styled background.

---

## 2026-05-08 17:30 — Strategic shift: dev-first wedge + local LLM in MVP

**Done:**
- Read PRD.md, RESEARCH.md, COMPETITIVE-LANDSCAPE.md.
- Confirmed the original "Tauri + local + cheap" cell is taken (Handy 14k★ MIT, MumbleFlow $5).
- Sharpened wedge in PRD.md to stack three claims no single competitor owns: (1) dev-first / code-aware out of the box, (2) local AI cleanup as headline / no API key required, (3) lifetime + GPLv3.
- Pulled **Code mode** forward from Sprint 2 → Sprint 1.
- Pulled **local LLM cleanup toggle** forward from V1.1 → Sprint 1 (via Ollama HTTP detection — bundled GGUF lands in Sprint 2 to keep MVP shippable).
- Brand voice locked: punk, anti-enterprise, fun. Anti-Wispr.
- License: GPLv3 (desktop core).

**Next:**
- Scaffold Tauri 2 app in `apps/desktop/` (React-TS template).
- Pin all dependency versions (Tauri 2.x, cpal, reqwest, etc.).
- Wire global hotkey (Right Option) → audio capture (cpal) → Groq Whisper Turbo → cleanup → clipboard paste.
- First end-to-end loop on macOS arm64 by tonight.

**Blocked:** none.

---
