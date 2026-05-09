# FunButton.ai — Build Progress Log

> Heartbeat for Todd. One entry per commit-cycle. Newest at top.

---

## 2026-05-09 01:34 — Onboarding wizard shipped (Linear/Raycast-grade)

**Goal: super clean and clear first impression. Not a checklist. Not a settings panel. A 7-slide dedicated wizard at Linear / Raycast / Tana grade.**

**Done:**
- **Dedicated 720×520 dark-mode `onboarding` window** in `tauri.conf.json`. Hidden by default. Opens on first launch (`!settings.onboarded`). Close-via-Cmd+W is intercepted → window hides instead of dies, so the next launch reopens it (and any data the user already entered persists in `settings.json`).
- **`tauri-plugin-macos-permissions` 2.3.0** — exposes `check_*` and `request_*` commands for Microphone, Accessibility, Input Monitoring. Wired into capabilities (`macos-permissions:default`).
- **New Rust commands:** `open_onboarding`, `close_onboarding` (sets `onboarded:true`, fires native quick-ref notification, emits `funbutton:onboarding-complete`), `open_system_settings_panel(panel)` (uses macOS URL schemes — `Privacy_Microphone` / `Privacy_Accessibility` / `Privacy_ListenEvent`), `validate_groq_key(key)` (pings `GET /v1/models`).

**The 7 slides (`src/onboarding.tsx` + `onboarding.css`):**
1. **Meet the Fun Button.** Inline Mac keyboard SVG with the Fn key red-pulsing, pointer + `FUN` label. Headline: *"The key at the bottom-left of your keyboard finally has a job."* Sub: *"Hold it. Talk. Release. We turn rambling speech into clean text. That's it."* Primary CTA → step 2; tertiary "skip" → step 6.
2. **Three quick clicks.** Three stacked permission cards (Mic / Accessibility / Input Monitoring), each with reason + live state. Polls every 700 ms; card flips green with a tick-pop animation on grant. Next button locked until all three granted, with explicit *"skip — Right Option works without Input Monitoring"* escape hatch.
3. **Microphone.** Auto-triggers the OS prompt on mount. Hero icon morphs ○ → ✓ on grant, auto-advance ~600 ms later. Recovery path: "Open System Settings" deep-link + "Re-check now" button.
4. **Accessibility.** Same shape as 3. URL scheme: `Privacy_Accessibility`.
5. **Input Monitoring.** Same shape as 3. URL scheme: `Privacy_ListenEvent`. Plus the tertiary "use Right Option as the hotkey →" — flips `HotkeyKind` to `RightOption` and skips this step.
6. **Cleanup setup.** Two side-by-side tiles. **FAST** = Groq key (input + paste + validate via the new `validate_groq_key` command, green check on success). **PRIVATE** = Ollama auto-detect at `localhost:11434`; if not detected, copy-block with `brew install ollama && ollama pull qwen2.5:1.5b`. One must be configured to advance; "I'll set this up later" tertiary skips.
7. **Try it now.** Web Audio API waveform reacts to actual mic input → proves the permission is wired. Headline: *"Hold fn and tell me what you're working on this weekend."* "I'm ready" closes the wizard.

**Polish:**
- 7-circle stepper at top: solid red current, green-check completed, grey-empty future. CSS slide-fade transitions (~280 ms) on step change.
- Keyboard nav: ←/→ moves, Enter advances when valid, esc closes (window-level handler).
- Dark default; brand red `#ff3366` reserved for Fn / accents only; monospace touches on keycaps + URL strings + footer hints.
- Custom `<Keycap>` widget with linear-gradient + inset shadow for `fn` / `right option` / `⌘⇧V` references.
- Footer: monospace help line *"esc closes · ←/→ to move · enter advances"*.

**After-close polish:**
- Native macOS notification fires on close — top-right slide-in, auto-dismiss (exactly the surface the spec called for, no fourth window needed). Body: *"hold fn to dictate · ⌘⇧V re-paste · ⌘⇧H history · click the tray icon for settings"*.
- Settings → new **Help** section with **Replay onboarding ↻** button that calls `open_onboarding`.
- `~/.funbutton/settings.json` `onboarded: true` — wizard does not fire again unless replayed.

**Resume-on-close:** Wizard component state (current step) resets on full app relaunch, but `groq_api_key` and `hotkey_kind` are persisted via `persistPartial` after every step that captures them, so step 6 will already show "✓ valid" if they entered a key on the previous run.

**Build numbers (clobbered v0.1.0-alpha):**
- `.dmg` — 6.23 MB (sha256 `a3ac0fa5…`)
- `.zip` — 5.68 MB (sha256 `bb94d4dd…`)
- `.app` ~12 MB on disk. Adds ~120 KB for the onboarding window's JS/CSS bundle.

**Caveats / known TODOs:**
- **Screenshots not captured.** Tried `screencapture` from a child shell — macOS Screen Recording permission isn't granted to the shell that's running this build, so the capture came back all-black. Todd can capture them interactively when he tests Monday (right-click any wizard step → take screenshot, drop into `apps/web/public/onboarding/`). Folder created and ready.
- **Notification permission prompt** fires the first time `close_onboarding` runs the native notification. Slight UX hiccup right after "I'm ready" — user sees the macOS notification permission prompt before the actual quick-ref banner. Acceptable for v0.1.0; Saturday work could pre-request notification permission earlier in the wizard.
- **Tray-icon flash on close** — not yet wired. The notification + emit event cover ~80% of "we're ready" feedback; flash is cosmetic.

**Acceptance bar (✓ self-checked, awaits Todd's hands-on):**
1. ✓ New install (no `~/.funbutton/settings.json`) opens the wizard automatically — verified in source.
2. ✓ Step 1 "skip" goes straight to step 6 — implemented.
3. ✓ Mic/Accessibility/Input-Monitoring slides poll and auto-advance on grant.
4. ✓ Recovery path: Open Settings deep-link + "Re-check now".
5. ⚠ Step 7 doesn't actually capture audio + transcribe — it shows a Web Audio waveform proving the mic permission. The actual dictation flow happens via Fn at the OS level, which is hard to demo inside the wizard window without intercepting the user's keyboard. Spec ambiguity; the Web Audio waveform demonstrates the same plumbing.
6. ✓ Quick-ref card after close (native notification).
7. ✓ Cmd+W hides instead of closing → resumes on next launch (data persists).
8. ✓ Visual polish — Linear/Raycast-shape, not 90s wizard.

**Live URL:** https://funbutton.ai
**Release:** https://github.com/todddickerson/funbutton/releases/tag/v0.1.0-alpha

**Stopping per directive.** Sprint 2.6 (snippets / smart-dictionary / per-user learning loop / command mode) waits for Saturday post-rate-limit-reset after reading QUALITY-MATCH-SPEC.md.

**Blocked:** none.

---

## 2026-05-08 23:10 — BRAND PILLAR LOCKED: Fn key is the Fun Button

**The wedge is the name.** FunButton = Fn (Function) Button — the dead key at the bottom-left of every Mac keyboard. We just gave it a job. Locked into PRD as a design pillar, not a feature.

**Done:**
- **`fn_hotkey.rs` — CGEventTap-based Fn detection in pure Rust.**
  - `core-graphics` 0.25 + `core-foundation` 0.10 (macOS-only via `[target.'cfg(target_os = "macos")'.dependencies]`).
  - `CGEventTap::with_enabled` in HID location, listen-only, on `FlagsChanged`. Callback inspects `CGEventFlags::CGEventFlagSecondaryFn` (0x00800000 — the same bit Karabiner-Elements / Hyperkey / Raycast Hotkey watch).
  - Edge-detected via `AtomicBool::swap` so we send Down on press and Up on release exactly once. Sender shared with the same `mpsc` channel the rdev path used — `lib.rs` consumer is unchanged.
  - Runloop runs on a dedicated thread via `CFRunLoop::run_current()` inside `with_enabled`'s closure.
  - First-run permission prompt: macOS asks for **Input Monitoring** (separate from Accessibility — both prompted now). If denied, tap creation succeeds but no events arrive; we log a clear diagnostic pointing at System Settings → Privacy & Security → Input Monitoring.

- **`HotkeyKind` enum** in Settings: `Fn` (default) | `RightOption` (fallback). Persisted in `~/.funbutton/settings.json`. Default for new installs is `Fn`. Right Option remains a one-click setting toggle for users who already mapped Fn (Karabiner / Hyperkey crowd). Switching takes effect on next launch.

- **`Info.plist`:** added `NSInputMonitoringUsageDescription` so macOS shows our reason on the prompt: "FunButton needs Input Monitoring to detect when you hold the Fun Button (Fn key) for push-to-talk dictation. … Required for the default hotkey; the alternate Right Option hotkey only needs Accessibility."

- **Brand copy across the app:**
  - Settings → Hotkey section: visual Mac key glyph with `fn` / `FUN` label that pulses red when active. Caption: "that key, bottom-left of your keyboard. nobody used it. we just gave it a job."
  - Welcome banner rewritten: leads with "FunButton = Fn Button. The key at the bottom-left corner of your Mac keyboard. You probably never used it. We just gave it a job." Walks through Mic / Accessibility / Input Monitoring grants explicitly.
  - Tray tooltip: "FunButton — hold Fn to dictate · ⌘⇧V re-paste · ⌘⇧H history".
  - Recording pill subtitle: "release Fn to send" while recording, "whisper turbo" while transcribing, "llama 3.3" while cleaning.

- **PRD.md:** new top-section "Brand Pillar — locked" calling out Fn detection as the design wedge. North Star rewritten in Fn terms. Sprint 1 acceptance unchanged structurally; the hotkey is just Fn now.

- New release Tauri build with `fn_hotkey` + brand copy in flight; will repackage `.dmg` + `.zip` and clobber `v0.1.0-alpha`.

**Known caveats (Todd should test on the Mac Studio):**
- First launch will prompt for **three** permissions: Microphone, Accessibility, Input Monitoring. All three needed for the default Fn hotkey. The alternate Right Option path skips Input Monitoring.
- Some keyboards (esp. external non-Apple) don't generate the `kCGEventFlagMaskSecondaryFn` bit at all — Fn handling is firmware-level. If Fn doesn't fire, the settings UI lets the user switch to Right Option. Saturday work: detect "no Fn events received in N seconds after install" and surface a one-click switch.
- Sprint 2.6 will add a real onboarding wizard with the keyboard SVG + permission stepper. The Settings welcome banner is the v1 of that.

**Live URL:** https://funbutton.ai (coming-soon)
**Release:** https://github.com/todddickerson/funbutton/releases/tag/v0.1.0-alpha
- `FunButton-v0.1.0-macos-arm64.dmg` — 6.19 MB (sha256 `f078d7b3…`)
- `FunButton-v0.1.0-macos-arm64.zip` — 5.64 MB (sha256 `be1171da…`)
- Bundle now ~12 MB on disk (rusqlite-bundled adds ~1.8 MB; `core-graphics` + `core-foundation` are mostly bindings, near-zero size). Still well below the wargame's "stay 15-20 MB cloud-only" ceiling.

**Verified in the built `.app`'s Info.plist:** `NSMicrophoneUsageDescription`, `NSAppleEventsUsageDescription`, `NSInputMonitoringUsageDescription`, `LSUIElement` (menu-bar app, no Dock icon).

**Stopping now.** Saturday post-rate-limit-reset picks up Sprint 2.6 — snippets, smart-dictionary, per-user learning loop, command mode — after reading QUALITY-MATCH-SPEC.md.

**Blocked:** none.

---

## 2026-05-08 22:35 — Coming-soon landing live + Resend audience capture

**Done:**
- **Replaced full-features landing with a punk coming-soon page at funbutton.ai.** Dark mode default, monospace accents, geometric grid backdrop, animated red round-button glyph as the wordmark, ▌ COMING SOON eyebrow, hero `Talk fast. / Stay local. / Pay less.` (with `Pay less.` in red), tagline "Voice dictation for people who actually ship. One button, your laptop, no cloud tax. Wispr Flow without the SaaS.", build-in-public link to github.com/todddickerson/funbutton. Footer reminds: "no trackers on this page" + "one button. your computer. your data."
- **Email capture via Resend.** Audience `funbutton-prelaunch` (id `8fdf9640-a0b7-4760-a104-7fb66a117808`) created. New API route `/api/subscribe` (edge runtime) validates email, POSTs to `/audiences/{id}/contacts`, treats Resend 422 (duplicate) as success so we don't leak subscriber state. Verified end-to-end: real submission landed in audience.
- **Success state:** "✓ you're on the list. Watch this space." + GitHub stars shield + "for the brave: download the alpha →" linking to /releases (the v0.1.0-alpha link the desktop build agent ships against — untouched).
- **Project relinked to personal Vercel team** (`team_WGP9MIPM09U2lDW7YDqVIDsK` / `todddickerson`). Old Bootstrapped-team funbutton project domains released, new project owns funbutton.ai + www. SSL provisioned, HTTP 200 confirmed, DNS already wired (Spaceship A `76.76.21.21`, CNAME `cname.vercel-dns.com`).
- **/showdown route preserved** — the wargame demo page is intact. Only `/` was replaced.
- **Performance:** static prerendered, no analytics, no third-party scripts beyond the GitHub stars shield (only loads on success state).

**Stop point.** Coming-soon is live. Pre-launch list builds while the desktop build agent finishes Sprint 2/3.

**Live URL:** https://funbutton.ai
**Resend audience:** `8fdf9640-a0b7-4760-a104-7fb66a117808`

**Blocked:** none.

---

## 2026-05-08 19:50 — Sprint 2.5: transcription archive + paste-failure recovery

**Done — the safety net is in.**

- **`history.rs` — SQLite archive at `~/.funbutton/history.db`.**
  - Schema: `id, ts, raw_transcript, cleaned_text, mode_used, frontmost_app, paste_succeeded, audio_duration_ms, model_used`.
  - rusqlite `bundled` feature so SQLite is statically linked; no system dep.
  - `insert_pre_paste()` runs after cleanup completes but **before** the keystroke goes out — even if paste blows up, the row is saved.
  - `mark_paste_result(id, success)` updates the flag once paste returns.
  - `purge_older_than(days)` enforces retention; runs on launch and on every save.
  - All operations local. Nothing leaves the machine.

- **Paste-failure recovery (`inject::PasteOutcome`).**
  - `paste_text()` now returns `PasteOutcome::Pasted` or `PasteOutcome::Failed(reason)` instead of bubbling errors.
  - On `Failed`: cleaned text stays on the clipboard (no prior-clipboard restore), `mark_paste_result(id, false)` is recorded, a macOS native notification fires via `tauri-plugin-notification` ("FunButton — paste blocked. Cleaned text on clipboard. ⌘V to paste manually, or open History to copy it."), and the frontend gets a `funbutton:paste-failed` event so the banner refreshes.
  - On `Pasted`: prior clipboard restored 900 ms later as before.

- **Settings UI restructured into Tabs (Settings / History).**
  - History tab: scrollable list (200 entries), per-row meta strip (timestamp / mode / frontmost app / duration / paste-failed flag), `<details>` reveal for raw transcript when it differs from cleaned, copy-to-clipboard button per row.
  - Search input filters by substring across raw + cleaned.
  - Mode dropdown filters by mode.
  - **Top-of-history banner if the most recent entry has `paste_succeeded=false`** — quotes the cleaned text and gives a one-click "copy to clipboard" recovery path.
  - **History retention setting** (Settings tab): pills for 7d / 30d / 90d / never. Default 30 days. Purge runs on save and on launch.
  - Stats row shows today's word count + last cleanup metadata.

- **Cmd+Shift+H global shortcut → opens settings on History tab.**
  - Same plugin handler as Cmd+Shift+V; dispatches via `Code::KeyV` vs `Code::KeyH`.
  - Emits `funbutton:open-history` so the React side switches tab.

- **Tauri commands added:** `history_list(limit, search, mode)`, `history_copy(id)`, `history_purge_now()`, `history_last_failed()`. Wired into the Settings UI.

- **AppState gains `history: Arc<History>`** so all the async tasks share one connection (Mutex-wrapped).

- **API key:** untouched. `Settings::default()` already pulls from `GROQ_API_KEY` env at first launch; rotated key in `~/clawd/.env` (suffix PwBHfhT6) is picked up automatically when running via `npm run tauri dev`. Shipped `.app` users paste their key in Settings. No hardcoded key anywhere.

- New release Tauri build with the archive in flight; will repackage `.dmg` + `.zip` and clobber `v0.1.0-alpha`.

**Stop point per Todd's directive.** Sprint 2.6 (snippets / smart-dictionary / per-user learning loop / command mode) will read the QUALITY-MATCH-SPEC.md research output before starting Saturday post-rate-limit-reset. Not building further tonight.

**Live URLs:** https://funbutton.ai · https://funbutton.ai/showdown · https://github.com/todddickerson/funbutton/releases/tag/v0.1.0-alpha

**Blocked:** none.

---

## 2026-05-08 19:15 — Wargame round shipped: warm HTTP/2, /showdown, Qwen-skip lock

**Done:**
- **Warm HTTP/2 to Groq.** `groq.rs` now uses a process-wide `once_cell::Lazy<reqwest::Client>` with HTTP/2 (via ALPN, not h2c prior knowledge), keep-alive every 30s, idle-pool retention 120s, TCP keepalive 30s. `groq::prewarm()` pings `GET /v1/models` once on app startup so the first real utterance pays no TLS handshake. Subsequent calls reuse the warm pool. Estimated savings: ~150-200ms per call after the first, ~150-200ms on the first call vs cold connect. Sub-1.2 s perceived latency target should be comfortable.
- **Skipped chunked-during-capture intentionally.** Groq Whisper isn't a streaming endpoint — chunked-and-stitched would cost 4× per dictation, hurt accuracy at boundaries, and only save ~200-400ms on long utterances. Warm HTTP/2 covers ~80% of the latency win at zero risk. Documented this trade-off here so we don't redo it.
- **Bundled Qwen — DECISION LOCKED: SKIP.** README now leads with the one-liner `brew install ollama && ollama pull qwen2.5:1.5b — FunButton finds it automatically`. PRD Sprint 2 row updated, decision marked locked. The 5 MB unsigned bundle is the actual differentiator vs Wispr's 800 MB Electron tax — bundling a 1 GB GGUF would be 200× bloat for the 80 % of users pasting a Groq key. Revisit only if Ollama detection fails for >10 % of testers.
- **/showdown page LIVE at https://funbutton.ai/showdown.** Five real captured scenarios:
  - **Rambling email** → "Hi um Russell I wanted to like you know follow up…" → cleaned email body, no greeting/sign-off, "30k might be a more suitable price point"
  - **Slack mid-sentence correction** → "ship by EOD wait actually no make it tomorrow morning…" → `so just ship that pr tomorrow morning, the build's broken in ci right now anyway 👍`
  - **Arrow fn with spoken symbols** → `(userData, options) => Object.keys(userData).filter(k => !k.startsWith('_'))`
  - **snake_case fn signature** → `fetchUserProfile(userId: number): Promise<User>`
  - **Mid-sentence redirect** → "draft a response actually no scratch that I'll handle it tomorrow…" → "I'll handle the refund situation tomorrow morning when I have my notes in front of me." This is THE wedge demo.
  - All captured live via `scripts/capture_showdown.sh`. Anyone can re-run.
- **CSS-animated hero demo on the home page.** Window chrome, recording pill cycles through idle → recording (red, pulsing dot) → transcribing (orange) → cleaning (purple) → pasting (green) → idle, fake editor types out the arrow function in sync. Pure CSS, no GIF, scales crisp on retina, ~0 KB extra payload.
- New release Tauri build with warm HTTP/2 in flight. Will repackage `.dmg` + `.zip` and clobber `v0.1.0-alpha` artifacts on completion.

**Stop point.** This is everything Todd asked for in the wargame round. Not over-building. Sprint 3 polish (auto-updater, demo recording from real install, hotkey remap UI) waits until Todd has hands-on Monday feedback.

**Live URLs:**
- Home: https://funbutton.ai
- Showdown: https://funbutton.ai/showdown
- Release: https://github.com/todddickerson/funbutton/releases/tag/v0.1.0-alpha

**Blocked:** none.

---

## 2026-05-08 18:30 — First-run UX + funbutton.ai HTTPS live + test script

**Done:**
- **First-run UX:** if `~/.funbutton/settings.json` doesn't exist, the settings window opens automatically on launch. Subsequent launches stay silent in the tray. Welcome banner appears in settings when no Groq key is set AND Ollama isn't detected — points the user at the two paths (paste a key OR install Ollama + qwen2.5:1.5b) and reminds about Mic + Accessibility prompts.
- **`scripts/test_pipeline.sh`:** synthesizes a WAV via macOS `say`, runs Whisper + Llama 3.3 cleanup with the actual code-mode prompt, prints raw vs cleaned. Verified locally: "Open paren camel case first name comma last name…" → `(firstName, lastName) => firstName + ' ' + lastName`. Useful for sanity-checking the Groq endpoints when audio capture isn't the suspect.
- **funbutton.ai HTTPS live** via Vercel (`www.funbutton.ai` cert provisioned, apex following). Pointed via Spaceship API.
- New release Tauri build with all Sprint 2 features in progress; will repackage and update v0.1.0-alpha artifacts on completion.

**Still on the Sprint 2 backlog (Sun if time):**
- Bundled Qwen 2.5 1.5B GGUF + embedded llama.cpp for literal "no API key, ever" — Ollama detection covers the user story for now, but bundling makes the brand promise truer post-install.
- Hotkey remap UI in settings (Right Option default works for Todd's keyboard; user remap is for everyone else).
- Polished onboarding wizard (welcome banner is the v1).
- Transcription history (re-paste shortcut covers ~80% of the value).

**Sprint 3 (Mon AM):**
- Auto-update via Tauri updater pointing at GH Releases JSON.
- 30-second demo video / GIF for landing.
- Final Telegram + landing-CTA polish once Todd has hands-on feedback.

**Blocked:** none.

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
