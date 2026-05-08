# FunButton.ai — Wispr Flow Competitor Research & Product Spec

*Research date: 2026-05-08 · Sources cited inline*

---

## TL;DR

Wispr Flow is the AI dictation category leader in 2026 — built on **Electron + native Swift modules**, powered by **in-house transcription models** (with OpenAI Whisper / Meta as cloud fallbacks), running fully cloud-only. They charge **$15/mo monthly or $12/mo annual**, hit **85% zero-edit rate** with sub-second latency, claim **220 WPM vs 45 WPM keyboard** (4x speedup), and raised **$56M Series A** (originally announced as $30M).

**The cracks in the moat are clear:**
1. **Cloud-only** — no offline mode, no Linux, screen-context capture freaks out devs/lawyers/medical
2. **Resource hog** — 800MB RAM, ~8% CPU idle, battery drain complaints (Electron overhead)
3. **Quality-after-trial syndrome** — multiple "works 60% post-payment" reviews on Trustpilot (2.7/5)
4. **Tight free tier** — 2,000 words/week burns in <1 day for power users
5. **Limited shortcut customization** — top reddit complaint, no Raycast extension yet
6. **6-min transcription cap** — annoying for brain-dump / coding sessions

**FunButton.ai's wedge:** *Local-first, fast, fun, cross-platform (incl. Linux), one button. Privacy is the product — no cloud unless you ask. Built on Tauri 2 + Parakeet TDT (sub-50ms streaming) + whisper.cpp + Groq for cloud burst. Half the price of Wispr at $7/mo or $99 lifetime.*

---

## Part 1 — Wispr Flow Deep Dive

### 1.1 Core Features (exhaustive, as of May 2026)

**Voice → Text Engine**
- Press-to-talk dictation system-wide on Mac, Windows, iOS, Android (keyboard replacement on mobile) [wisprflow.ai]
- Real-time transcription with sub-second perceived latency
- 100+ languages, including thick accents handled well [Lightspeed Generative Now podcast, Oct 2025]
- 4× faster than typing claim (45 WPM → 220 WPM benchmarked) [wisprflow.ai homepage]
- 85% zero-edit rate (rarely need to fix output) [Tanay Kothari, Lightspeed podcast]
- 6-minute hard cap per single transcription [Zackproser review]

**AI Cleanup / Auto-Edit**
- Removes filler words ("um", "uh", "like")
- Fixes grammar, punctuation, capitalization automatically
- Reformats run-on speech into clean structure
- Handles "ramble + change your mind mid-sentence" gracefully — rewrites coherently
- Tone matching by app context (casual for Slack, formal for email) [Zackproser, Tanay's interviews]

**Custom Dictionary & Snippets**
- Per-user dictionary for names, jargon, code symbols
- Snippets: voice shortcuts that expand to longer text in any app
- Synced across devices on same account

**Command Mode (Pro only)**
- After dictating, voice-edit selected text: "make this shorter", "translate to Spanish", "make it more formal"
- Released ~2025, gated behind Pro plan

**Whispers / Voice Commands**
- Auto-trigger commands by saying specific phrases
- Less prominent in 2026 marketing — appears subsumed into Command Mode

**App Awareness / Context Capture**
- Uses macOS Accessibility API + Windows UIA to detect what app/field cursor is in
- Adapts tone/format to context (e.g. code block in Cursor vs casual message in Slack)
- **Captures screen content** for context — became major privacy controversy [Vocai 2026 review]

**Push-to-Talk Shortcuts**
- Default: function key, configurable
- Limited keybinding customization (top user complaint — can't bind to >3-key chords, hyper-key issues) [r/WisprFlow stats]

**Platforms**
- macOS 12.0+ (Universal: arm64 + x86_64)
- Windows 10/11
- iOS (keyboard replacement)
- Android (limited time unlimited; keyboard replacement)
- **No Linux**, no web

**Privacy / Compliance Surface**
- Privacy Mode (Zero Data Retention) on all tiers
- HIPAA-ready on all plans
- SOC 2 Type II + ISO 27001 (Enterprise only)
- SSO/SAML Enterprise

**Collaboration**
- Shared team dictionary / snippets
- Centralized billing, admin dashboards
- Usage dashboards (basic on Pro, advanced on Enterprise)

### 1.2 Pricing (May 2026)

| Tier | Price | Limits |
|------|-------|--------|
| Free / Basic | $0 | 2,000 words/wk Mac+Win, 1,000 wk iPhone, unlimited Android (limited time) |
| Pro | **$12/mo annual** ($144/yr) or **$15/mo monthly** | Unlimited words all platforms, Command Mode, early features |
| Enterprise | Contact sales | + SOC 2, SSO/SAML, dedicated IT seats, bulk discount |

- 14-day free Pro trial, no credit card required
- Student discount: 3 mo free + 50% off afterward
- HIPAA-ready and Privacy Mode included even on free tier (unusually generous)

### 1.3 Tech Stack — Confirmed

**Confidence: HIGH** (based on official MDM docs + multiple job postings)

**Desktop framework: Electron + native Swift modules**
- macOS bundle ID: `com.electron.wispr-flow` [docs.wisprflow.ai/articles/1444083812-mdm-enterprise-deployment-guide]
- Hardened runtime entitlements: JIT, unsigned executable memory, DYLD env, **disabled library validation (standard for Electron)**
- Job postings recruit "Senior Electron + macOS Developer" with Swift native module bridging to Node/Electron
- Older job postings list: **Electron, Redux, Webpack, NextJS, Python, FastAPI, AWS** for full-stack
- iOS/Android: native Swift + Kotlin (separate codebases)

**ASR / Transcription Engine**
- Initially used **OpenAI Whisper API + Meta cloud models** [confirmed in Letterly review, citing Wispr's privacy policy]
- **Now training in-house models** — Tanay on Lightspeed podcast: *"The models are built in-house... my co-founder Sahaj is one of the inventors of diffusion models"*
- Sahaj Garg = ex-Luminous Computing photonic AI hardware → diffusion model researcher
- Cloud-only — **zero offline mode** [universal across reviews]

**LLM cleanup layer**
- Custom-trained or wrapped frontier models — Tanay refuses to even use the word "LLM" in product
- Sub-second latency suggests proprietary model + Groq-style inference, not raw GPT-5

**Latency claims**
- Sub-second perceived end-to-end on consumer broadband
- Achieved through: streaming audio upload, in-house ASR fine-tuned for low TTFT, optimized inference infra

**Funding & founders**
- Tanay Kothari (CEO) — Stanford CS+AI MS, ex-FeatherX founder (acquired by Cerebra), Forbes 30u30 2023, taught Stanford Deep Learning with Andrew Ng
- Sahaj Garg (CTO) — ex-Luminous Computing, diffusion model co-inventor
- Series A: $30M originally → $56M total announced, Lightspeed + 8VC + Menlo + NEA backing
- Founded 2021, originally building **silent-speech wearable** ("voice from thoughts"), pivoted to dictation

### 1.4 Customer Feedback — What Users Love

**Synthesized from r/WisprFlow, Product Hunt, Trustpilot, App Store, X, HN, Zackproser blog, eesel/Letterly reviews**

✅ **It just works** — one button, no setup, dictate in any app
✅ **Auto-cleanup is magic** — rambling speech → clean prose, especially for Slack/email/code comments
✅ **Tone matching by app** — adjusts formality automatically
✅ **Cross-platform** — only major AI dictation tool on Mac + Win + iOS + Android
✅ **HIPAA-ready on free tier** — unusually generous baseline
✅ **Speed** — sub-second feels instant; doctors, devs, founders rave about output quality
✅ **Custom dictionary** — handles jargon/names well after a few corrections
✅ **Built for power users** — coders, lawyers, founders all over testimonials

### 1.5 Customer Feedback — What Users HATE

**The biggest pain points (with sources):**

❌ **Privacy / surveillance vibes** [Vocai April 2026, Letterly, eesel]
- Cloud-only audio processing — sent to OpenAI / Meta / Wispr servers
- Screen-context capture (the product, not a bug) — devs with API keys, lawyers with client files, therapists with patient names visible all object
- Reddit thread went viral; user who flagged it was **banned**, then CTO apologized
- Original privacy policy allowed training on user content (now opt-in, but trust damaged)

❌ **Resource hog (Electron overhead)** [Letterly, Vocai]
- ~800 MB RAM idle, ~8% CPU constantly
- Re-adds itself to login items repeatedly (called a "bug")
- Constant outbound network traffic even when idle ("performance analytics")
- Battery drain on MacBooks — top r/WisprFlow concern

❌ **Quality degrades post-trial** [Vocai, Trustpilot 2.7/5]
- Multiple users: "works 60% of the time after I paid"
- Trustpilot 2.7/5 vs G2 4.5/5 — strong divergence

❌ **Tight free tier** [universal]
- 2,000 words/week = roughly 15-20 short emails, blows in <1 day for any pro user
- Forces upgrade pretty aggressively

❌ **Latency on long sessions** [r/WisprFlow "extremely slow today" — top pain post]
- Cloud latency varies, especially on slow networks or international users
- 6-minute per-transcription hard cap [Zackproser, Wispr docs]

❌ **No offline mode** [universal]
- Airplane = no dictation
- Sensitive workflows blocked
- Privacy Mode ≠ local mode

❌ **Limited shortcut customization** [r/WisprFlow #2 most-requested]
- Can't bind to >3-key chords
- Hyper-key (Karabiner) workflow broken
- No Raycast extension despite repeated requests

❌ **Windows is rough** [theplanettools, eesel]
- Windows build noticeably less polished, more bug reports early 2026

❌ **No Linux support** [zackproser, dev complaints]

❌ **Subscription-only pricing** [universal preference for lifetime / one-time]
- Compared to Voibe ($149 lifetime), VoiceInk (free/$39), BetterDictation ($24 lifetime), MacWhisper (~$59), Hearsy (one-time)

❌ **Bug list (from r/WisprFlow):**
- ClickUp typing bugs
- App freezes during voice + text editing
- Battery drain when always-on
- Random retries on transcription

---

## Part 2 — Competitive Landscape (May 2026)

| Tool | Platforms | Engine | Privacy | Pricing | Position |
|------|-----------|--------|---------|---------|----------|
| **Wispr Flow** | Mac, Win, iOS, Android | In-house cloud + Whisper/Meta | Cloud (HIPAA) | $12-15/mo | Category leader, polished AI cleanup |
| **SuperWhisper** | Mac, iOS, Win (new) | Local Whisper + cloud option | Local-first | $84.99/yr or $249 lifetime | Privacy + customization, premium price |
| **MacWhisper** | Mac only | Local Whisper | Local-first | ~$59 one-time / sub | File-transcription king, GUI |
| **BetterDictation** | Mac (Win soon) | Local Whisper + cloud Pro | Local | $24 lifetime / $2/mo Pro | Cheapest Whisper dictation |
| **Voibe** | Mac (Apple Silicon) | Local Whisper | Local | $149 lifetime / $59/yr | Superwhisper-killer, simpler |
| **VoiceInk** | Mac | Local Whisper | Local | Free / $39 | Open source |
| **Hearsy** | Mac | **Parakeet TDT** + Whisper | Local | One-time | <50ms latency, fastest |
| **Aqua Voice** | Mac, Win | Cloud | Cloud | $8/mo | Cheap cloud alternative |
| **Aiko** | Mac | Local Whisper | Local | Free | Free file transcription |
| **macOS native Dictation** | Mac, iOS | Apple on-device | Local | Free | Built-in but mediocre AI |
| **Dragon NaturallySpeaking** | Win | Nuance proprietary | Cloud/local | $200+ one-time / sub | Legacy enterprise |

**Positioning gaps FunButton can attack:**
1. **Local-first + great AI cleanup** (Voibe is closest but no Linux, weak cleanup)
2. **Cross-platform local-first incl. Linux** (nobody serves devs well today)
3. **Price-disruptive** ($7/mo or $99 lifetime undercuts Wispr 50%+)
4. **Personality / fun** (every competitor is humorless utility software)
5. **Developer-first features** (code mode, terminal injection, IDE awareness)
6. **Hybrid local+cloud burst** (Parakeet local for short, Groq cloud for long sessions)
7. **No screen capture by default** (privacy-as-product positioning)

---

## Part 3 — FunButton.ai Product Spec

### 3.1 Core Differentiators vs Wispr Flow

| Wispr Flow | FunButton.ai |
|------------|--------------|
| Cloud-only, no offline | **Local-first**, cloud burst optional |
| Electron, 800MB RAM idle | **Tauri 2**, ~50-80MB idle |
| Captures screen for context | **Zero screen capture by default** |
| 6-min transcription cap | **Unlimited duration** (chunked) |
| Mac, Win, iOS, Android | **+ Linux** (X11 + Wayland) |
| $12-15/mo subscription only | **$7/mo or $99 lifetime** |
| Limited shortcut customization | **Full hyper-key, Karabiner-friendly** |
| Closed source | **Open-source core (MIT) + closed cloud features** |
| Generic auto-cleanup | **Personas / modes** (code, email, Slack, doc, journal) |
| Polished but humorless | **Fun, opinionated, slightly weird brand** |
| No Raycast | **First-class Raycast extension** |

### 3.2 Desktop Architecture — RECOMMENDED: **Tauri 2.0 + Rust core + Swift native module (mac) + C++ native (win) + native Linux module**

**Trade-off table:**

| Option | Bundle | Idle RAM | Cross-platform | Latency | Velocity | Verdict |
|--------|--------|----------|----------------|---------|----------|---------|
| **Tauri 2** ⭐ | ~10MB | ~50MB | Mac+Win+Linux | Native Rust speed | Medium (Rust) | **Pick this** |
| Electron | 150MB+ | 400-800MB | Mac+Win+Linux | OK | High (JS) | Wispr's choice; loses on perf |
| Native Swift+SwiftUI | 5-15MB | ~30MB | Mac only | Best UX | Low | Locks out Win/Linux |
| .NET MAUI | 50MB | 150MB | Win-first | OK | Medium | Wrong starting platform |
| Flutter Desktop | 30-40MB | 80-150MB | All 3 | OK | Medium | Weak native integration |

**Why Tauri 2 wins for FunButton:**
- **Native Rust backend** = no IPC bridge tax for the audio pipeline (Wispr's idle RAM problem comes from Electron's V8+Chromium baseline)
- **~8-10MB shell** vs Electron's 150MB+
- Rust + `cpal` for audio capture = professional low-latency audio handling
- `tauri-plugin-global-shortcut` works cross-platform (limited on Wayland)
- For mac-specific F5 dictation override or hardware-level interception → `tauri-plugin-macos-input-monitor` (CGEventTap FFI)
- Allowlist-based IPC = privacy story credible
- Single Rust codebase for the heavy lifting → easier hiring & velocity vs full-native triple-stack
- Real-world precedent: **MumbleFlow** (Tauri 2 + whisper.cpp + llama.cpp, 45MB idle) proves the pattern works

**The frontend:** React + Vite + shadcn/ui + Tailwind in the Tauri webview. Simple menubar UI + settings.

### 3.3 Transcription Engine — RECOMMENDED: **Hybrid Parakeet TDT (default local) + Groq Whisper Turbo (cloud burst, opt-in) + whisper.cpp (offline fallback)**

| Engine | Latency | Accuracy | Cost | Use for |
|--------|---------|----------|------|---------|
| **Parakeet TDT** (NVIDIA, Apple Silicon optimized) | **<50ms streaming** on M-series | 1.69% WER LibriSpeech (beats Whisper) | $0 (local) | **Default for short dictations <60s** |
| **whisper.cpp small/medium** (Metal/CUDA) | ~400ms for 10s clip | ~2.7% WER | $0 (local) | Offline fallback, multilingual |
| **Groq Whisper Large v3 Turbo** | ~1-3s for 1min file (cloud) | ~95%+ EN | **$0.00067/min** (~$0.04/hr) | Long sessions, low-resource devices, cloud burst |
| **Deepgram Nova-3** (alt) | ~150ms streaming | Best EN noisy audio | $0.0077/min stream | Real-time alt if needed |
| **Distil-Whisper EN-only (Groq)** | Even faster | Same as Whisper EN | **$0.00033/min** | English-only cheap path |

**Routing logic:**
```
if user_in_offline_mode || privacy_strict:
    use whisper.cpp local
elif clip < 60s && parakeet_supported(device):
    use Parakeet local (Apple Silicon, modern NVIDIA)
elif clip > 5min || low-end CPU:
    use Groq Whisper Turbo (cloud, opt-in)
else:
    use whisper.cpp local
```

**Pitch:** *"Your audio stays on your machine by default. Cloud is opt-in for long sessions or weak hardware."*

### 3.4 AI Cleanup Layer — RECOMMENDED: **Groq Llama 3.3 70B (default) + local Ollama Phi-3.5 (offline mode) + Claude Haiku (premium tier)**

| Model | Latency | Cost/1M tok | Quality | Use |
|-------|---------|-------------|---------|-----|
| **Groq Llama 3.3 70B** ⭐ | ~200-400ms TTFT | ~$0.59/$0.79 | Excellent | Default cloud cleanup |
| Local Ollama Phi-3.5 / Llama 3.2 3B | 200-800ms M-series | $0 | Good for cleanup | Offline mode |
| Claude Haiku 4.5 | ~300ms | $1/$5 | Best polish | Premium "Magic Mode" |
| GPT-5.1-mini | ~400ms | $0.40/$1.60 | Very good | Backup |
| Gemini Flash 2.5 | ~300ms | $0.075/$0.30 | Good, cheap | Backup |

**Cleanup prompts by mode:**
- **Email** — formal, structured, signature-aware
- **Slack** — casual, emoji-okay, no salutation
- **Code** — preserve symbols, technical terms, no auto-format prose
- **Doc** — paragraphs, headings, formal grammar
- **Journal** — preserve voice, minimal cleanup, raw vibe
- **Raw** — disable cleanup entirely (a top r/WisprFlow request)

### 3.5 Global Shortcut + Audio Capture Stack

| Platform | Hotkey | Audio Capture | Text Injection |
|----------|--------|---------------|----------------|
| **macOS** | `tauri-plugin-global-shortcut` (default), `tauri-plugin-macos-input-monitor` (CGEventTap for F5/system-key override) | `cpal` (Rust, Core Audio) | `CGEventCreateKeyboardEvent` + Cmd+V fallback. Requires Accessibility permission. |
| **Windows** | `tauri-plugin-global-shortcut` | `cpal` (WASAPI) | `SendInput` Win32 API w/ `KEYEVENTF_UNICODE` |
| **Linux** | `tauri-plugin-global-shortcut` (X11), custom for Wayland | `cpal` (ALSA/PulseAudio) | X11: `XTest`. Wayland: portal-based, GNOME/KDE specific paths |

**Audio pipeline:**
- 16kHz mono PCM float32
- VAD (voice activity detection) via Silero VAD (small ML model, runs on CPU)
- `rubato` for high-quality sample rate conversion if device sample rate differs
- Streaming chunks of 1-3s windows for Parakeet, full clip for Whisper

### 3.6 MVP Feature List (4-week build)

**Week 1: Core pipeline**
- [ ] Tauri 2 shell, system tray, menubar UI (mac), tray (win), AppIndicator (linux)
- [ ] Global hotkey registration (Fn key default, configurable)
- [ ] Audio capture via `cpal`, push-to-talk + toggle modes
- [ ] whisper.cpp integration via `whisper-rs` FFI
- [ ] Text injection on macOS + Windows

**Week 2: Quality + UX**
- [ ] AI cleanup via Groq Llama 3.3 (cloud, with API key entry on free / FunButton key on paid)
- [ ] Mode picker (Email / Slack / Code / Doc / Journal / Raw)
- [ ] Custom dictionary
- [ ] Visual feedback overlay (mic indicator, transcribing spinner)
- [ ] Onboarding wizard (perm grants, hotkey setup, mic test)

**Week 3: Polish + platforms**
- [ ] Linux X11 support (Wayland deferred to V2)
- [ ] Settings: hotkey customization (full chord, Karabiner-friendly), model picker, privacy toggles
- [ ] Auto-update via Tauri updater
- [ ] Crash reporting (Sentry, opt-in)
- [ ] Code signing + notarization (mac), EV cert (win)

**Week 4: Launch readiness**
- [ ] License key system (Stripe + license server)
- [ ] Free tier: 1,500 words/wk on cloud cleanup; unlimited local + raw transcription
- [ ] Pro tier: Unlimited cloud, all modes, priority Groq queue
- [ ] Landing page (funbutton.ai) — playful brand, comparison table vs Wispr
- [ ] Docs site
- [ ] Telemetry (PostHog, opt-in)

### 3.7 V2 Features (Months 2-3)

- **Parakeet TDT integration** — sub-50ms streaming on Apple Silicon (huge "fastest in the world" headline)
- **Raycast extension** (top r/WisprFlow request, FunButton-ships-it differentiator)
- **iOS keyboard** (React Native or native Swift)
- **Snippets** (voice macros that expand)
- **Command Mode** — voice-edit selected text ("make this shorter", "translate")
- **Wayland support** — full Linux story
- **Team plan** — shared dictionary, snippets, billing
- **HIPAA-mode** — enforced local-only routing
- **Tone presets** — "professional", "friendly", "savage", "Shakespeare", "pirate" (lean into fun branding)
- **Memory** — remembers your jargon, names, code style automatically
- **Multi-language hot-switching** (no mode change needed)
- **Whisper file transcription** — drag-drop audio/video files
- **Open source the desktop core** under MIT (closed-source cloud only)

### 3.8 Pricing Model Proposal

| Tier | Price | Features |
|------|-------|----------|
| **Free / Local** | $0 | Unlimited local transcription (whisper.cpp + Parakeet), basic raw cleanup, BYO API key for cloud cleanup |
| **Pro** | **$7/mo annual** ($84/yr) or **$10/mo monthly** | Unlimited Groq cleanup, all modes, Raycast, snippets, Command Mode, all platforms |
| **Lifetime** | **$99 one-time** | Everything in Pro, forever, locked-in. Limit to first 1,000 customers as launch promo. |
| **Team** | $5/seat/mo (5+ seats) | Shared dictionary, snippets, billing, admin |
| **Enterprise** | Contact us | SOC 2, SAML, on-prem cloud option, HIPAA enforcement |

**Why this works:**
- Free tier converts: local works great, but cloud cleanup is faster + no API key fuss → upsell natural
- $7/mo annual undercuts Wispr ($12) by ~42%, undercuts Voibe ($59/yr ≈ $5/mo) only slightly
- $99 lifetime kills the "subscription fatigue" objection (top universal complaint)
- Team plan at $5/seat undercuts Wispr by 60%+

### 3.9 Build Pipeline & Operations

**Monorepo structure (Turborepo):**
```
funbutton/
├── apps/
│   ├── desktop/          # Tauri 2 app (Rust + React)
│   ├── web/              # Marketing + docs (Next.js on Vercel)
│   ├── license-server/   # Stripe + license keys (Hono on CF Workers)
│   └── raycast/          # Raycast extension
├── packages/
│   ├── core/             # Rust crate: pipeline orchestration
│   ├── asr/              # Rust: Parakeet + whisper.cpp wrappers
│   ├── cleanup/          # Rust: prompt templates, mode logic
│   ├── injection/        # Rust: cross-platform text injection
│   └── ui/               # Shared React components (shadcn)
└── tooling/              # CI scripts, signing configs
```

**CI/CD:**
- GitHub Actions: build matrix (mac arm64, mac x86, win x86, linux x86)
- macOS: signing with Apple Developer ID + notarization (`notarytool`)
- Windows: EV code-signing cert (DigiCert, ~$400/yr)
- Linux: AppImage + .deb + Flatpak
- Auto-update: Tauri updater (signed manifests, S3-hosted)
- Release channels: `stable`, `beta`, `nightly`

**Telemetry & Observability:**
- PostHog (privacy-respecting, opt-in)
- Sentry for crash reporting (DSN already in stack)
- Trigger.dev for any backend background jobs (license validation, billing webhooks)

**Cloud infra:**
- License server: Hono on Cloudflare Workers + D1 + Stripe
- Cloud cleanup proxy: CF Workers → Groq API (we eat the cost difference)
- Webhook handlers: Trigger.dev (durable jobs)
- Marketing: Next.js on Vercel
- Auto-update artifacts: S3 + CloudFront

### 3.10 Estimated Cost Per Active User at Scale

**Assumptions:** Pro user dictates ~30 min/day = 15 hrs/mo, 80% local / 20% cloud cleanup

**Cloud costs (per user/mo):**
- Cloud cleanup: ~3 hrs/mo of Groq Llama 3.3 70B
  - Avg dictation = 60s audio → ~150 words → ~200 input tokens, ~250 output tokens
  - 3 hrs / 60s = ~180 cleanups × 450 tokens = ~80k tokens/mo
  - Groq Llama 3.3 70B: ~$0.59/$0.79 per 1M → **~$0.05/user/mo**
- Cloud transcription burst (rare): ~30 min Groq Whisper Turbo at $0.00067/min = **~$0.02/user/mo**
- License server / CDN / observability: **~$0.10/user/mo**

**Total cloud cost:** **~$0.17/user/mo**

**Pro pricing $7/mo annual** → **~97% gross margin**. Even at $5/mo team pricing, ~96% margin.

Wispr's cost is much higher because they're 100% cloud — every dictation goes through their stack. FunButton's local-first design gives massive margin headroom.

---

## Part 4 — Brand & Positioning Notes

**Name vibe:** "FunButton" is intentionally silly — leans into the "one button, just press it" simplicity Tanay himself talks about, but with personality. Wispr is humorless productivity software. FunButton is the assistant you'd actually have a beer with.

**Tagline candidates:**
- "Your voice. Your computer. One button."
- "Press the button. Speak. Done."
- "The fun way to never type again."
- "Talk fast. Stay local. Pay less."

**Founders' market wedge messaging:**
> "Wispr Flow charges $15/mo to send your voice to OpenAI's servers. We charge $7 — and we don't have to phone home. Your audio. Your machine. Your typing replaced."

**Launch channels:**
- Product Hunt (1-week prep, target #1 Product of the Day)
- HackerNews Show HN ("I built an open-source local-first Wispr Flow alternative")
- r/MacApps, r/productivity, r/Apple, r/programming
- X/Twitter — short demo videos showing 220 WPM + privacy story
- Indie hackers / Lenny's Newsletter for SaaS audience
- Coverage pitch to TechCrunch / The Verge with privacy angle

---

## Sources Cited

- wisprflow.ai (homepage, /pricing, /tanay, /media-kit)
- docs.wisprflow.ai/articles/1444083812-mdm-enterprise-deployment-guide
- jobs.lever.co/WisprAI (multiple postings, Swift + Electron)
- jobs.ashbyhq.com/wispr-flow (Mobile Eng Lead 2025)
- jobs.8vc.com (iOS Engineer)
- Lightspeed "Generative Now" podcast — Tanay Kothari interview, Oct 2025
- "Voice AI Deep Dive" YouTube interview, Dec 2025 (50% MoM growth, $30M Series A confirmed; later raised to $56M)
- letterly.app/blog/wispr-flow-review
- eesel.ai/blog/wispr-flow-review
- vocai.net/blog/wispr-flow-review-privacy-2026 (Trustpilot 2.7/5, "works 60% post-trial")
- tldv.io/blog/wisprflow
- gummysearch.com/r/WisprFlow (subreddit pain stats)
- zackproser.com/blog/wisprflow-review (6-min cap)
- betterdictation.com/blog/betterdictation-vs-superwhisper
- getvoibe.com/blog/superwhisper-alternatives
- hearsy.app (Parakeet TDT <50ms latency reference)
- dev.to/auratech (MumbleFlow Tauri 2 + whisper.cpp build log, Feb 2026)
- introl.com/blog/voice-ai-infrastructure-real-time-speech-agents-asr-tts-guide-2025
- novascribe.ai (Groq vs Deepgram vs OpenAI pricing comparison, April 2026)
- opentypeless.com/en/blog/deepgram-vs-whisper
- littlewhisper.app (Groq Whisper Turbo $0.00067/min)
- docs.rs/crate/tauri-plugin-macos-input-monitor (CGEventTap FFI)

---

*Report compiled by Ea (Claude Opus 4.7). Total research time: ~30 min. Recommended next step: prototype Tauri 2 + whisper.cpp + Groq cleanup pipeline in 1 week to validate latency claims and feel the UX delta vs Wispr.*
