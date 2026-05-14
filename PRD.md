# FunButton.ai — PRD (MVP through V1)

**Created:** 2026-05-08
**Owner:** Todd Dickerson
**Build window:** Fri 5pm → Mon 9am (~64 hours, autonomous coding-agent build)
**Goal:** Working macOS dictation app Todd can use Monday morning. Wispr Flow competitor. Local-first, half the price, half the resource footprint.

---

## Brand Pillar — locked 2026-05-08

**FunButton = Fn (Function) Button.** The brand is a literal rename of the
dead key at the bottom-left of every Mac keyboard. Most users never bound
anything to it. We just gave it a job.

**Default hotkey: Fn.** Detected via `CGEventTap` on `flagsChanged` events
checking the `kCGEventFlagMaskSecondaryFn` (0x00800000) bit — the same path
Karabiner-Elements, Hyperkey, and Raycast Hotkey use, because macOS does not
expose Fn as a normal modifier. Requires the **Input Monitoring** permission
(separate from Accessibility — both are prompted on first run).

**Fallback hotkey: Right Option.** Settings remap option for users who already
have something bound to Fn (Karabiner / Hyperkey crowd — rare).

**Brand copy** must lean on this everywhere it appears:
- Settings: "The Fun Button (Fn) — bottom-left of your keyboard."
- Tray tooltip: "FunButton — hold Fn to dictate"
- Recording pill subtitle: "release Fn to send"
- Onboarding: visual of the keyboard with Fn glowing red, label "FUN", caption "nobody used it. we just gave it a job."
- Landing hero: "The key at the bottom-left of your keyboard finally has a job." (handled by web team in apps/web.)
- Showdown: row showing "Wispr Flow: Right Option (or any custom modifier)" vs "FunButton: the Fun Button" — emphasize the conceptual elegance of using the dead key.

This is a **design pillar, not a feature.** Treat it as locked. If `CGEventTap`
cannot be made to work in pure Rust within ~2 hours of attempts, fall back to
Right Option for v0.1.0 only — but Fn detection MUST land in v0.2.0 or the
brand collapses.

---

## North Star
Todd presses the Fun Button (Fn — bottom-left of his keyboard), talks, releases, and his cleaned-up dictation appears at his cursor in <2 seconds. No cloud lock-in. No 800MB Electron tax. No subscription required.

## Positioning (sharpened 2026-05-08 after COMPETITIVE-LANDSCAPE.md)
The "Tauri + local + cheap" cell is **occupied** — Handy (14k★ MIT) and MumbleFlow ($5) already ship the same stack. So FunButton stacks three claims no single competitor owns simultaneously:

1. **Developer-grade out of the box** — code mode in MVP, not V2: spoken `camelCase` / `snake_case` / `kebab-case` / braces / operators / common symbols handled by default. IDE-aware (Cursor / VS Code / Vim / JetBrains). *Whisperer is the only commercial comp; nobody else.*
2. **Local AI cleanup, no API key required, ever** — bundle a quantized small LLM (Qwen 2.5 0.5B/1.5B GGUF) so cleanup runs on-device. Groq stays the FAST default; local is the toggle that ships in MVP. *MumbleFlow does this at $5 with no brand; we do it with brand.*
3. **Lifetime + GPLv3 desktop core** — VoiceInk's model ($25-49 paid binary, GPLv3 source). Hostile to enterprise lock-in. *Wispr has zero lifetime ever despite $94M raised — lifetime is the structural weapon.*

**Brand voice:** punk, anti-enterprise, fun. The anti-Wispr — "press the fun button, talk to your computer like it's a friend." Wispr is humorless productivity software. FunButton is the assistant you'd have a beer with.

- **vs Wispr Flow:** Local-first by default, Linux-friendly, lifetime pricing, open-source desktop core (GPLv3), half the price
- **vs Handy / MumbleFlow:** Dev-grade code mode out of the box, AI cleanup as headline (not transcription), polished brand
- **vs SuperWhisper / VoiceInk / Voibe:** Cross-platform, on-device LLM cleanup default, code-aware in MVP, fun
- **vs Whisperer (closest dev comp):** Lifetime + GPLv3 + Linux + on-device LLM

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
**Acceptance:** Todd can hold Right Option, talk for up to 60s, release, see clean text appear at cursor in macOS Notes/iMessage/Slack/Cursor. **Code mode works** — saying "open paren camelCase user name close paren" yields `(userName)` in Cursor. **Local cleanup toggle works** — when Ollama is detected at localhost:11434, cleanup runs on-device with no API key.

- [ ] Tauri 2 project scaffolded, builds on macOS arm64
- [ ] Global hotkey: Right Option (configurable later) — push-to-talk
- [ ] Audio capture via `cpal` to in-memory PCM, encoded to wav/flac/mp3 in memory
- [ ] Visible recording indicator: menu bar icon + small floating pill UI showing waveform
- [ ] On release: POST audio to Groq Whisper Turbo, get transcript
- [ ] **Cleanup pipeline (pluggable):**
  - [ ] Default: Groq Llama 3.3 70B (fast, cloud)
  - [ ] **Local toggle (ships in MVP, not V1.1):** Ollama at `http://localhost:11434` if available — auto-detect, fall back to Groq if unavailable. Recommended model: `qwen2.5:1.5b` (Qwen 2.5 1.5B Q4, ~1GB). Settings UI shows current backend + status.
- [ ] **Code mode (Sprint 1, was Sprint 2):** When frontmost app is Cursor / VS Code / Vim / JetBrains / Terminal, switch to code-aware cleanup prompt that handles spoken symbols (`camelCase`, `snake_case`, `kebab-case`, `open paren`, `close paren`, `equals`, `arrow`, `open curly`, `dot`, `semicolon`, `dollar`, `at sign`, `pipe`, `ampersand`, etc.) and preserves identifiers verbatim.
- [ ] Inject cleaned text via clipboard + paste shortcut (preserve original clipboard)
- [ ] Settings window: API key entry (BYOK), hotkey display, basic stats (today's word count), backend toggle (Groq / Local Ollama / Auto)
- [ ] Menu bar app: idle/recording/processing states, quit option
- [ ] Auto-launch on login (optional toggle)
- [ ] First-run flow: Accessibility permission prompt with clear instructions
- [ ] LICENSE file: GPLv3 (desktop core)

### Sprint 2 — V1 (Sun by EOD)
**Acceptance:** Modes work, history exists, dictionary handles brand names, app feels polished, dev-first identity is visible.

- [ ] Modes: Auto / Email / Slack / Code / Raw (Code already shipped in Sprint 1; Sprint 2 adds Email / Slack / Raw)
   - Auto picks based on frontmost app (Mail.app → Email, VS Code/Cursor → Code, Slack → Slack, default → Raw cleanup)
- [ ] Mode-specific cleanup prompts
- [ ] Custom dictionary: user-added terms boost during cleanup ("Spontent", "ClickFunnels", etc.)
- [ ] Transcription history view (local SQLite)
- [ ] Cmd+Shift+V to re-paste last transcription
- [ ] Cmd+Shift+H to toggle history window
- [ ] Per-mode tone tuning sliders (formal ↔ casual)
- [ ] Polished onboarding: 30-second walkthrough on first launch — leans punk/fun brand
- [x] **Bundled local LLM — DECISION: SHIP (re-locked 2026-05-14, reversing 2026-05-08 skip).** The "no API key, ever" headline isn't literally true until cleanup works on first install without an Ollama detour. Now bundles `llama-server` (llama.cpp b9151, ~9 MB binary + ~5 MB dylibs) + `Qwen2.5-1.5B-Instruct Q4_K_M` GGUF (~1.0 GB). Trade-off accepted: bundle grew from ~5 MB → ~1 GB (200× bloat as flagged in the prior lock). Justification: free-tier UX win is significant — first dictation works with zero account, zero key, zero shell commands. Ollama-external path stays as a power-user option. Vendor artifacts fetched via `src-tauri/scripts/fetch-vendor-deps.sh`, not committed to git.
- [ ] Local Whisper fallback toggle (whisper.cpp + small.en model bundled)
- [ ] Per-language code profiles (Python: `def`/`self`; JS/TS: `const`/`=>`; Rust: `let`/`mut`; etc.)

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
- **Week 2 (V1.1) — Monetization sprint (7 days, see WORKER-SPEC.md):**
  - Cloudflare Worker at api.funbutton.ai (license verify, transcribe/cleanup proxy, usage logging)
  - Stripe Checkout: Pro $9/mo, Pro $79/yr, Lifetime $149/$199/$249 ladder
  - Premium metered usage (Haiku/Sonnet/Opus/GPT-4.1) with app-side cap enforcement
  - Desktop Settings UI: cap slider, license activation, fallback toast, monthly receipt link
  - ROSCA + California ARL-compliant auto top-up flow (opt-in OFF by default, plain-English disclosure, one-click cancel)
- Windows build (Tauri 2 makes this 1-2 days)
- Linux build (X11 day-1, Wayland later)
- Streaming transcription (Deepgram or local Parakeet TDT)
- Team features (V2 — $7/seat/mo annual, 3-seat min)
- Telemetry (privacy-respecting, PostHog with opt-in)
- Code-signing with Apple Developer ID + notarization
- iOS keyboard companion
- "Open core" repo split: desktop OSS (MIT), cloud sync proprietary

---

## Pricing (LOCKED — validated 2026-05-09 via deep research, see PRICING-RESEARCH.md for citations)

### Tiers

**Free — forever, no card.**
- BYOK Groq key OR local Ollama, GPLv3 desktop core
- Unlimited use; user pays Groq directly (their key) or zero (local)
- This is the punk-rock pillar. We don't ransom your voice.

**Pro — $9/mo or $79/yr (~26 % discount on annual).**
- Groq Whisper Turbo + Llama 3.3 70B unlimited (soft ceiling 500K words/mo — well above 95th percentile of real users at ~187K)
- 50K words/mo of premium cleanup included (Claude Haiku 4.5 default)
- All modes, dictionary, history, sync
- Above included quota → metered pay-as-you-go (see meter below) gated by user-set monthly cap

**Lifetime — founder ladder, no recurring charges on the base license.**
- **$149** for first 1,000 customers
- **$199** for next 1K → 5K
- **$249** thereafter
- Includes Groq fast tier unlimited forever
- Premium cleanup is pay-as-you-go (no included quota); same metered pricing as Pro overage, same user-set cap
- Tier auto-bumps via Stripe webhook on sales count crossings

**Team — $7/seat/mo annual, 3-seat minimum.**
- Defer to V2 (post-launch). Same engine, shared dictionary, admin console.

### Premium model meter (Pro overage + Lifetime usage)

| Model | Price per 10K words | Margin | Notes |
|---|---|---|---|
| **Claude Haiku 4.5** | **$0.40** | ~78 % | Default premium, best quality/$ for cleanup |
| Claude Sonnet 4.7 | $0.60 | ~50 % | Pro+ tier (long-form, code) |
| Claude Opus 4.7 | $0.99 | ~40 % | Reasoning tier (only when user explicitly picks) |
| GPT-4.1 | $0.50 | ~55 % | Alternative provider; replaces deprecating GPT-4o |

### Auto top-up & user cap (compliance-first)

- **Default cap $20/mo**, user-configurable slider $0–$100 in Settings
- **$0 = hard stop** — silently falls back to Groq fast tier with toast notification ("You hit your monthly cap. Premium cleanup paused. Adjust in Settings.")
- **Opt-in OFF by default** to satisfy ROSCA + California ARL — user must affirmatively enable auto top-up
- **Plain-English disclosure** at activation, monthly receipt email, one-click disable in Settings
- **CRITICAL ENFORCEMENT NOTE:** Stripe `billing_thresholds` does NOT hard-stop — it only triggers an invoice. The Cloudflare Worker MUST enforce the cap app-side **before every premium API call** via Durable Object counters. Architecture spec'd in `WORKER-SPEC.md`.

### Why this pricing (one-liner)

Fastr-than-Wispr at half the price, with a free tier that actually works and a lifetime option that respects you. Not because we're charitable — because the math at Groq + Anthropic Haiku 4.5 makes it work.

## Monetization Architecture

All paid-tier traffic flows through a Cloudflare Worker at `api.funbutton.ai`. The desktop app holds a license JWT in macOS Keychain and authenticates every premium request. The Worker enforces caps app-side, routes to Groq/Anthropic/OpenAI, logs usage to D1, and posts metered usage records to Stripe.

**Full implementation spec — endpoints, KV/D1 schema, Stripe products, hot-path pseudocode, 7-day ship plan — lives in `WORKER-SPEC.md`. The Week 2 coding agent works from that file.**

Desktop app (open source, GPLv3) talks to Worker (closed source, our infra). This is the open-core split previously called out in the V2 backlog — it ships in V1.1 because pricing requires it.

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
