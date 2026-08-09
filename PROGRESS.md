# FunButton.ai — Build Progress Log

> Heartbeat for Todd. One entry per commit-cycle. Newest at top.

## 2026-08-08 23:55 — Gauntlet follow-up: injection guard + detection latency + first real runtime QA

**Branch `gauntlet-followup-guard-latency` (off gauntlet-v2-polish). Findings #5 and #14 fixed, and the runtime QA pass that had never actually been done — done, on the real release binary with the real bundled models. PR into main open; no release cut.**

**Done:**
- **Finding #5 (security) — instruction-execution guard.** Every mode prompt (auto/email/slack/raw/code/terminal) now opens with a hardened prime directive (`with_prime_directive!` in cleanup.rs), ported verbatim to the premium worker prompts so paying users aren't less protected. Behind it, a runtime guard (`guard.rs`): five signals detect that the cleanup model answered/obeyed the dictation instead of cleaning it (empty output, assistant-voice openers, vanished instruction markers + low substring overlap, length explosion, question-came-back-as-answer) → raw transcript is pasted, never the model's output; reason logged, shown on the pill, carried in `funbutton:result`. All backends (embedded/ollama/groq/cloud) funnel through one guarded finalization. 13 unit tests pin trip/no-trip both ways; thresholds tuned so self-corrections, casing merges ("get user by id"→getUserById), and symbol conversions never false-positive.
- **Proof on the real model:** new `#[ignore]`-gated `guard_stack_holds_on_live_model` drives the hardened prompts through a live llama-server running the shipped Qwen 2.5 1.5B. The bundled model obeyed 3 of 4 injections *despite* the hardened prompt (said literally `banana`; wrote an actual haiku; answered "what is two plus two") — guard caught all three, 4/4 paste the dictated sentence. The Q&A case was a guard gap found BY this probe and fixed (signal 5).
- **Finding #14 (latency) — detection can't block the pipeline.** `FrontApp::detect()` (osascript, observed ~105s under TCC stall) replaced by native `NSWorkspace.frontmostApplication` via objc2-app-kit (already in the tree via Tauri — zero new crates; permission-free, µs-level). Detection runs concurrently on its own thread behind `DetectHandle`: auto mode waits ≤400ms for the STT-bias decision, cleanup picks up late results for free, stalled detection degrades to Unknown without ever holding the pipeline. Tradeoff documented in code: a missed 400ms window costs only the STT vocabulary bias for that dictation. osascript kept solely as fallback when native yields nothing.
- **Runtime QA on a real install (RUNTIME-QA-2026-08-08.md).** Release `.app` built from this branch + 1.1GB DMG via the hdiutil workaround; release binary run headlessly twice (fresh HOME, selftest hotkey, `say`-synthesized WAVs): engines spawn (whisper on Metal 8.9s cold, llama-server 9.4s), whisper transcribes 395ms/69ms, pipeline end-to-end offline, **guard fired in the real app on the injection run** (Qwen output `"banana"` → raw sentence pasted), native detection live (history recorded `loginwindow` — screen was locked), no crash/panic in either run.
- **Fixed along the way:** release link failure `___isPlatformVersionAtLeast` (ggml-metal @available vs Rust -nodefaultlibs, new with Xcode 17 — build.rs now links clang_rt.osx); history `model_used` claiming `groq-…` for fully-offline embedded runs (now honest per-backend labels); crate-wide `cargo fmt` + clippy 1.97 lints.
- **Gates:** 43/43 lib tests green (+ 2 opt-in live probes) · cargo fmt + clippy clean · worker tsc clean · release build + bundle clean.

**Next:** Todd: 30-second human pass per RUNTIME-QA §Human (fresh-account TCC grants, real speech, paste into Cursor, tray/pill visuals). Then merge the PR. Residual guard gaps documented: auxiliary-led questions ("is the build green"→"yes") and short claimed-execution outputs ("File deleted.") ride on the hardened prompt only.

**Blocked:** none.

## 2026-08-08 09:45 — GAUNTLET LOOP v2: polish pass across all six user-facing surfaces

**Worker + blind-critic gauntlet (max 3 rounds/piece) on onboarding, settings, pill HUD, tray, dev-first engine, and the landing page. All six pieces converged to PASS; nothing passed round 1, so the bar did real work. Branch `gauntlet-v2-polish`, PR open for review — no release cut, no deploy.**

**Done:**
- Onboarding: grants animate as a circuit closing, live-poll made visible, engine-warmup staged as ignition, crafted keyboard hero, real end-to-end try-it step, denied/revoked recovery paths, Fn/emoji-picker collision fix surfaced.
- Settings: single ENGINES status board, per-app mode map shown (incl. new terminal mode), dictionary chip editor, day-grouped history with paste-failure triage, instant-save interaction model, light+dark verified at 540x680.
- Pill HUD: real-mic waveform (RMS via AnalyserNode), 340x96 bottom-center on the cursor's monitor, distinct state layers, "code mode · Cursor" receipt on paste, airtight mic/RAF teardown.
- Tray: ● recording glyph in the menu bar, live status + engine telemetry with click-to-fix, mode quick-switch, copy-last, start-at-login, replay onboarding; all AppKit mutations routed through run_on_main_thread.
- Dev-first engine (the wedge): curated two-tier DEV_DICTIONARY with whisper 224-token budget handling, CODE_PROMPT covering spoken flags/paths/versions/git conventions, new auto-detected TERMINAL mode (literal commands, never prose), Groq BYOK STT now gets vocabulary bias, 23 unit tests green.
- Landing: real project page — dev-first hero, animated terminal demo, honest comparison table (Wispr/Handy/VoiceInk), install + pricing preserved, OG image, all prior slop violations (glow shadows, 14px input) killed.
- Gates: cargo check/build --release clean · 23/23 lib tests · keyless offline STT proof passes ("Refactor the auth middleware and open a pull request.") · rdev/TIS grep clean · desktop tsc+vite + web next build+eslint clean.
- GAUNTLET-ROUNDS.md (auditable PASS/FAIL log) + GAUNTLET-FINDINGS.md (~200 deduped findings, top-14 curated) at repo root.

**Next:** Todd reviews PR; top findings queue: real product visual on landing, deep-context cleanup, editable per-app mode rules, premium-cloud dev parity.

**Blocked:** none.

## 2026-08-02 11:11 — v0.1.4: macOS 26 crash fix (SIGTRAP on keypress during setup)

**Tester on M4 / macOS 26.6 couldn't get through onboarding — app SIGTRAP'd ~15s in, the moment a key event arrived. Root cause + fix shipped.**

**Root cause:** macOS 26 now hard-enforces `dispatch_assert_queue(main)` inside HIToolbox's Text Services input-source APIs (`TSMGetInputSourceProperty` → `islGetInputSourceListWithAdditions`). Any off-main-thread call traps (`EXC_BREAKPOINT`/`SIGTRAP`). macOS 15 silently allowed it, which is why it never reproduced on the Mac Studio. Two paths hit it:
1. **`rdev`** mapped keycodes→chars via TIS/TSM *inside* the CGEventTap callback on our spawned Right Option listener thread → crash on the first key event (the onboarding crash).
2. **`enigo`'s `Key::Unicode('v')`** reverse-maps the char via `TISGetInputSourceProperty`/`UCKeyTranslate`, and paste runs on a worker thread → would have trapped on the *first dictation* even after fixing rdev.

**Done:**
- **Right Option listener rewritten** (`hotkey.rs`): `rdev` removed entirely (dep + its legacy transitive tree), replaced with a raw `CGEventTap` on `FlagsChanged` — same HID-tap pattern as the Fn key. Right Option = virtual keycode `0x3D` + `CGEventFlagAlternate`; Left Option (`0x3A`) ignored. State machine: DOWN doesn't re-fire when other modifiers change, UP fires even while other modifiers are held. **No layout/input-source APIs on the listener thread.**
- **Paste injection fixed** (`inject.rs`): `Key::Unicode('v')` → `Key::Other(0x09)` (raw V keycode, no layout API). `Key::Meta` already used a fixed keycode.
- **Crash isolation** (`hotkey.rs` + `fn_hotkey.rs`): each listener thread body wrapped in `catch_unwind` — a future listener panic logs loudly and dies alone instead of killing the app.
- Audited: `tauri-plugin-global-shortcut` is Carbon `RegisterEventHotKey` (no per-event TSM mapping, main-thread) — safe, not a crash path.
- v0.1.4 built (.app + DMG via `hdiutil`, the Sequoia bundler workaround — tauri's `bundle_dmg.sh` still fails on Sequoia), released, landing page bumped, funbutton.ai deployed (now serving v0.1.4-alpha; `latest/download` DMG link verified 200).

**Verified (this Mac, macOS 15.6.1 / M3 Ultra):**
- `cargo check` + `cargo build --release` clean.
- `grep -r rdev src/` and `grep -riE "TSMGetInputSource|TISCopy|UCKeyTranslate" src/` → nothing on the hotkey path.
- Launched the built binary with `armed=RightOption`, `RUST_LOG=info`: new raw Right Option tap installs ("Input Monitoring granted; running CFRunLoop"), whisper (MTL0) + llama-server load, **app ran 90s+ with zero panics/traps.**
- Release: https://github.com/todddickerson/funbutton/releases/tag/v0.1.4

**Next:**
- **Todd: 10-second physical confirmation** — hold Right Option in any app, confirm dictation fires. I could not automate the literal "hold key → DOWN/UP log" line: FunButton's tap is at `kCGHIDEventTap` (the HID layer, deliberately, matching the Fn tap), and HID taps by design only observe *real hardware* — synthetic `CGEventPost` events don't reach them. The state machine is verified by code review + exact parity with the shipping Fn tap; only a real keypress can exercise the final edge.
- Ideal: the M4 / macOS 26.6 tester re-runs onboarding on v0.1.4 to confirm the SIGTRAP is gone end-to-end (build a QWERTY-layout note: raw V keycode `0x09` is the V position on QWERTY; non-QWERTY layouts fall back to clipboard + manual ⌘V).

**Heads-up (not blocking):** `scripts/update-landing-version.sh` committed + pushed the page.tsx bump correctly, but its `vercel --prod` step aborted under `set -euo pipefail` (the git changes had already landed). I ran the prod deploy by hand — it built clean and aliased funbutton.ai. Worth hardening that step before the next release (or the periodic cron) so the deploy doesn't silently no-op.

**Blocked:** none.

---

## 2026-08-01 18:10 — Dev-first behavior forward

**Per the research, "dev-first / code-aware out of the box" is the open cell nobody owns. Made the desktop behavior match the claim:**

- **Editor/terminal coverage widened** (`app_detect.rs`): new `Editor` class → code mode for Zed, Windsurf, Sublime Text, Nova, TextMate, BBEdit, Emacs; JetBrains detection adds Android Studio, DataGrip, Fleet; terminal list adds WezTerm, Tabby, Hyper, Rio; VSCodium → VS Code. Auto mode now lands in code mode across effectively every dev surface.
- **Built-in `DEV_DICTIONARY` (~130 terms)** now does double duty: biases the on-device whisper via initial prompt (shipped in the STT commit) AND is injected into the code-mode cleanup prompt as a normalize-to-these-spellings block (git, GitHub, npm, kubectl, JSON, camelCase/snake_case, async/await, …). User dictionary explicitly outranks it.
- **Code prompt additions**: 'double colon' → `::`, 'spread'/'dot dot dot' → `...`, 'optional chain' → `?.`.
- **The code-aware behavior is now visible**: the pill shows "pasting — code mode · Cursor" (mode + frontmost app) on every dictation.

**Verified:** `cargo check` + `tsc --noEmit` clean.

**Next:** keyless E2E on a real install, then v0.1.3 build + DMG + release.

**Blocked:** none.

---

## 2026-08-01 17:55 — ON-DEVICE STT: "no API key, ever" is now literally true

**The category (Handy 28k★, unramble, VoiceInk) is on-device; needing a Groq key for STT was our biggest weakness per `OSS-LANDSCAPE-DEEP-RESEARCH-2026-08-01.md`. Killed it.**

**Done:**
- **`embedded_stt.rs`** — bundled whisper base.en (GGUF Q8_0, 81 MB, from Handy's HF mirrors, sha256-pinned in `fetch-vendor-deps.sh`) via **`transcribe-cpp` 0.1.3** — the same whisper.cpp/ggml wrapper Handy uses, statically linked with the Metal backend (zero extra dylibs). Chosen over raw `whisper-rs` after studying Handy's tree: one API covers whisper today AND Parakeet-GGUF tomorrow, `session.run(&audio).text` with no segment-loop/`[BLANK_AUDIO]` plumbing (we still strip artifacts defensively).
- **WAV → 16 kHz mono** decode path (hound + `rubato` FftFixedIn, Handy's resampler choice), sub-second inputs padded (whisper.cpp misbehaves under ~1 s).
- **`stt_backend` setting: `local` (DEFAULT) | `groq`.** Whichever is picked, the others are silent fallbacks (local → licensed-cloud → BYOK-groq, or cloud-first when groq selected). Existing installs flip to on-device by default on upgrade — intended.
- **Whisper initial-prompt bias**: user dictionary + (in code mode) the new built-in `DEV_DICTIONARY` (~130 dev terms) feed whisper's initial prompt, so "git/npm/kubectl/PR" transcribe correctly cased at the STT layer, before cleanup even runs. Mode detection moved BEFORE transcription to make this possible (also: frontmost app now detected once per dictation instead of twice).
- **Crash containment**: release profile switched from `panic=abort` to unwind; ggml inference wrapped in `catch_unwind` so a native panic degrades to the fallback chain instead of killing the app (Handy's pattern).
- **Quit-path resource teardown** in the tray handler — fixes TWO latent bugs: the orphaned `llama-server` child on quit (Drop never ran on `app.exit`), and the ggml-metal SIGABRT risk from a live Metal context at C++ static-destructor time.
- **UI**: new Settings "Transcription" section (on-device default / groq cloud pills + live model status); `embedded_check` now returns `{cleanup, stt}`; onboarding step 6 headline is now **"No API key. No account. Ever."** with a dual-engine status strip; Groq tile demoted to "optional cloud path"; welcome banner + pill copy updated.

**Verified:**
- `cargo check` + `tsc --noEmit` + unit tests clean.
- **Offline integration test with real speech** (`say`-generated 48 kHz WAV → resampler → bundled GGUF, Metal): transcript came back **verbatim-perfect including term casing**: `"Hold the Fun button and dictate a git commit message, then run npm install and push the PR."` — 2.4 s inference on M-series, no network, no key. Test is committed (`transcribes_real_wav_offline`, `--ignored`-gated).

**Next:** dev-first polish (app_detect coverage, dev vocab in cleanup prompt, pill mode display), then keyless E2E on a real install + v0.1.3 release.

**Blocked:** none.

---

## 2026-08-01 17:05 — v0.1.2 SHIPPED: install-just-works release

**What shipped (three commits this cycle + release):**

1. **Permissions UX — the #1 adoption stall, fixed at the root.** `fn_hotkey.rs` retries CGEventTap creation every 3s, so granting Input Monitoring mid-onboarding arms the Fn key within seconds — no relaunch. The onboarding wizard (already live-polling at 700ms with auto-advance) now composes with this: grant → card flips green → hotkey is genuinely live. Step 6 rebuilt around the bundled model: new `embedded_check` command (`starting|ready|failed`), 1.5s live polling shows "warming up → ready", the advance gate counts the embedded backend, and the copy is honest that STT still needs a free Groq key today. Stale "quit + relaunch" copy removed from Settings; Lucide icons replace emoji in system UI.
2. **Groq key in the macOS Keychain** (`keyring` 3.6.3 → Security framework). Idempotent sentinel-free migration off plaintext `settings.json`; graceful file fallback when the Keychain refuses; key edits/clears in the UI sync the Keychain item (`ai.funbutton.desktop` / `groq_api_key`).
3. **Sprint 2 audit:** all items verified genuinely shipped in code (5 modes + frontmost-app classifier, dictionary prompt injection, SQLite history + ⌘⇧H, ⌘⇧V re-paste, hotkey remap UI with hot-swap, embedded Qwen with vendor payload intact + `llama-server --version` runs).

**Release:** https://github.com/todddickerson/funbutton/releases/tag/v0.1.2
- `FunButton-v0.1.2-macos-arm64.dmg` — 1.1 GB (bundled Qwen 2.5 1.5B), sha256 `f13bafd42e722f4fbe5a156d8fd97baf318abe30fe70944d7d8af7fda240860a`

**Install steps (also in release notes):**
1. Download the DMG, drag FunButton → Applications.
2. `sudo xattr -cr /Applications/FunButton.app` (unsigned build — Gatekeeper bypass).
3. Launch; onboarding walks the three permissions with live auto-advance.
4. Hold **Fn**, talk, release.

**Verified on this machine (real install, not just compile):**
- `cargo check` + `cargo build --release` + `tsc --noEmit` clean throughout; bundle Info.plist has all three usage descriptions + `LSUIElement`, version 0.1.2.
- Installed to `/Applications` over v0.1.1 and launched: log shows `migrated Groq API key from settings.json into the macOS Keychain` → `settings.json` key field now empty, `security find-generic-password -s ai.funbutton.desktop -a groq_api_key` finds the item.
- `Fn key tap installed (Input Monitoring granted); running CFRunLoop` — hotkey armed.
- `llama-server ready at http://127.0.0.1:59557 (8846ms)` — embedded cleanup live from the installed bundle's resources.
- v0.1.2 is running in the tray on the Mac Studio right now, with the real Groq key in the Keychain.

**Known issues (carried in release notes):**
- STT still requires a Groq key (free) — bundled whisper.cpp (~75 MB tiny.en) is the next "no key, ever" step.
- arm64-only; Intel Macs can't run the embedded backend.
- Unsigned — xattr step required until we buy a signing cert.
- Real-Fn-keypress E2E still benefits from a human tap (CGEvent synthesis can't fake the SecondaryFn flag without Accessibility on the synthesizer); every layer beneath it is verified, and the Test Hotkey button bisects it in-app.

**Next (v0.2 candidates, per PRD Sprint 3 + carried items):** bundled Whisper for zero-key STT, Tauri auto-updater on GH Releases, smart dictionary/snippets/command mode (read QUALITY-MATCH-SPEC.md first), 30s demo video, universal binary.

**Blocked:** none for the desktop track. (Stripe keys + CF zone permission still block monetization — unchanged, documented in the 2026-05-12 entry.)

---

## 2026-08-01 16:50 — Groq key moved into the macOS Keychain

**Done:**
- **New `keychain.rs`** (keyring crate `=3.6.3`, `apple-native` → Security framework). Login-Keychain item: service `ai.funbutton.desktop`, account `groq_api_key`.
- **Idempotent, sentinel-free migration** (lesson from freeflow's `keychain_migration_done` bug — their global flag orphaned every secret except the first): on load, a non-empty key found in `settings.json` is written to the Keychain and the file field is blanked; once blanked the branch never fires again. No version flag to get wrong.
- **Graceful degradation everywhere:** if the Keychain refuses (denied ACL prompt, locked keychain, ad-hoc-signed rebuild changing code identity), a `KEY_IN_KEYCHAIN` flag stays false and `persist()` keeps writing the key to `settings.json` as before — worse at-rest protection, but the key is never lost and the app never hard-fails. Keychain read failure at launch falls back file → env var.
- **Key edits in Settings/onboarding sync the Keychain** via `save_settings` (write on change, delete on clear). Clearing the key deletes the Keychain item.
- **Fresh-install recovery:** wiping `~/.funbutton` but keeping the login keychain restores the key on next launch.
- UI copy updated: Settings key hint + onboarding tile now say the key lives in the macOS Keychain.

**Verified:** `cargo check` + `tsc --noEmit` clean. Runtime migration check (settings.json key blanked + `security find-generic-password` finds the item) happens at the v0.1.2 build/launch step below.

**Next:** Sprint 2 feature audit, then v0.1.2 build + DMG + release.

**Blocked:** none.

---

## 2026-08-01 16:35 — Permissions UX: live-heal Fn tap + onboarding step 6 rework

**Done:**
- **`fn_hotkey.rs` now retries CGEventTap creation every 3s.** Root fix for the #1 install-stall: TCC checks happen at `CGEventTapCreate` time, so on a fresh install (Input Monitoring not yet granted) the tap failed once and the listener thread died forever — granting the permission mid-onboarding did nothing until an app relaunch. Now the listener keeps retrying, so the Fn key arms itself within seconds of the grant. Failure logged once (not every 3s); the tap also reinstalls if its runloop ever exits. This is the same "live-polling makes install just work" behavior as freeflow's SetupView, applied at the Rust layer where our actual failure was.
- **Onboarding step 6 rewritten around the bundled model.** New `embedded_check` Tauri command returns `starting | ready | failed` (backed by a new `embedded_error` field in AppState). Step 6 polls it (plus `ollama_check`) every 1.5s — the "Bundled model warming up… → ready" transition shows live, and the advance gate now counts the embedded backend, so a zero-key user is no longer forced to paste a Groq key or install Ollama to finish onboarding. Honest copy added: speech-to-text still uses Groq Whisper today (free key), bundled Whisper is the next step.
- **Settings amber warning updated** — the old copy told users to quit + relaunch after granting Input Monitoring; now correctly says the listener picks it up within seconds.
- **Lucide icons replace emoji in system UI** (`lucide-react` added): step-6 tile tags (Zap/Lock), bundled-model strip (Cpu), Settings warning (AlertTriangle).

**Verified:** `cargo check` clean, `tsc --noEmit` clean.

**Next:** Groq key → macOS Keychain (with migration off settings.json), then v0.1.2 build + DMG + GitHub release.

**Blocked:** none.

---

## 2026-05-15 12:40 — v0.1.1: HOTKEY BUG FIXED

**Done — root cause was the installed v0.1.0 `.app` missing `NSInputMonitoringUsageDescription`.** Without that Info.plist key macOS silently denies Input Monitoring at the kernel level, `CGEventTapCreate` returns NULL, the Fn-key tap fails to install, the listener thread runs forever receiving zero events. No log surfaces — Settings just sits at "IDLE" and nothing happens when you press Fn. To make matters worse, the UI rendered a hardcoded "Right Option (hold) · Cmd+Shift+V re-paste" string from the persisted `settings.json` even though the actual armed listener was Fn — so anyone trying to debug pressed the wrong key.

**Repaired everything that contributed:**

- **`Info.plist` already had the key** in the repo (line 9-10) — it just hadn't been in the v0.1.0 bundle. New v0.1.1 bundle ships it. Verified via `plutil -p`.
- **Both listeners now run simultaneously** — Fn (CGEventTap) and Right Option (rdev) — and each only emits events when the shared `armed_hotkey: Arc<AtomicU8>` matches its kind. Changing the hotkey in Settings flips the atomic; the previously-passive listener becomes active **without an app restart**. If Fn fails to install because Input Monitoring was denied, Right Option still works without any user action beyond a click.
- **`HotkeyKind::label()`** is now the single source of truth for the hotkey name shown in Settings. The persisted `hotkey_label` field is overwritten on load to match `hotkey_kind`. The Settings UI never displays a stale label again.
- **Permissions panel in Settings.** Three rows (Microphone / Accessibility / Input Monitoring) showing live grant state via `tauri-plugin-macos-permissions`. Each has a "Grant in System Settings" button that calls the matching `request_*_permission` command. Auto-refreshes when the window regains focus (so the round-trip to System Settings shows up immediately). When the Fn hotkey is armed but Input Monitoring is denied, an amber warning recommends switching to Right Option.
- **Test Hotkey button** in the Settings → Fun Button section. Calls the new `simulate_hotkey` Tauri command, which fires `HotkeyEvent::Down` then `HotkeyEvent::Up` 1.5 s later directly into the channel — bypassing the listener entirely. This is the **bisection tool**: if Test Hotkey makes audio → transcribe → cleanup → paste happen but holding the real key doesn't, you know it's the listener (and the permissions panel above will say which TCC bit is missing).
- **Hot-swap on hotkey change.** Clicking Fn / Right Option in Settings now persists immediately and flips the atomic. No "changes apply on next launch" caveat.
- **Hardcoded "Hold Right Option" empty-history message** replaced with the dynamic "Hold {Fn / Right Option}".
- **Better logging.** Every Down/Up event from either listener now logs `hotkey: Fn DOWN` / `hotkey: Right Option UP`. If `CGEventTapCreate` fails, the error message explicitly tells the user to enable Input Monitoring **or switch to Right Option**.

**Versions bumped to 0.1.1.** Cargo.toml + tauri.conf.json + Settings footer all consistent.

**Verification done autonomously:**
- `cargo check` + `cargo build --release` both clean.
- `tsc --noEmit` on frontend clean.
- New `.app` bundle's `Info.plist` contains `NSInputMonitoringUsageDescription` (verified with `plutil -p`).
- Killed the old running pid 915, removed the v0.1.0 install, installed v0.1.1 to `/Applications/FunButton.app`, `xattr -cr` to strip quarantine.
- `tccutil reset {ListenEvent,Accessibility,Microphone} ai.funbutton.desktop` to force fresh TCC prompts on first key event.
- Launched from terminal with `RUST_LOG=info` stderr capture: confirmed `[INFO] spawning both hotkey listeners; armed = Fn` and `[INFO] Fn key tap installed (Input Monitoring granted); running CFRunLoop`. The tap is alive and listening.

**Final-mile verification (real Fn keypress → Down/Up log line)** needs a human keypress because synthesizing CGEvents with the SecondaryFn flag from a test process requires Accessibility on the synthesizer, which terminal lacks. Todd has been notified via Telegram to test.

**Files changed:**
- `apps/desktop/src-tauri/src/state.rs` — `HotkeyKind::label()`, `as_u8/from_u8`, `armed_hotkey: Arc<AtomicU8>`, `hotkey_tx: Mutex<Option<Sender>>` so `simulate_hotkey` can reach the channel.
- `apps/desktop/src-tauri/src/fn_hotkey.rs` — accepts `armed` filter, logs Down/Up, clearer error message.
- `apps/desktop/src-tauri/src/hotkey.rs` — same shape (`armed` filter, Down/Up logs, error message).
- `apps/desktop/src-tauri/src/lib.rs` — spawns both listeners with shared channel, `save_settings` flips the atomic, new commands `get_hotkey_label` + `simulate_hotkey`, `load_settings` overwrites stale `hotkey_label`.
- `apps/desktop/src-tauri/Cargo.toml` + `tauri.conf.json` — version 0.1.1.
- `apps/desktop/src/App.tsx` — Permissions panel + `PermRow` component, Test Hotkey button, hot-swap on hotkey-kind click, dynamic label/empty-history copy, refreshPerms on mount + window focus.
- `apps/desktop/src/App.css` — `.fb-perms` + `.fb-perm-row` styles in light + dark.

**Next (Sprint 2.6 / 3 carried forward):**
- Smart dictionary (per-user terms injected into cleanup prompt as a boost list).
- Snippets (named shortcuts → expansion text).
- Command mode ("new line", "delete that", "select all").
- Per-user learning loop (track corrections, build user-specific cleanup prompt) — read QUALITY-MATCH-SPEC.md first.
- Tauri auto-updater pointing at GH Releases JSON.
- 30-second demo gif/video for landing page.

**Blocked:** Real-keypress confirmation depends on Todd (autonomous CGEvent synthesis hits the Accessibility wall on the synthesizer). All other layers verified.

---

## 2026-05-14 17:35 — Bundled local inference (zero-key cleanup)

**Reversing the 2026-05-08 PRD lock.** The "no API key, ever" headline isn't literally true until cleanup runs on first install without an Ollama detour. Shipping the bundle. Trade-off (200× bloat, 5 MB → ~1 GB) accepted in writing in `PRD.md` Sprint 2 list.

**Shipped:**
- **`apps/desktop/src-tauri/vendor/llama/`** (gitignored) — `llama-server` binary from llama.cpp release `b9151` for macOS arm64 (~8.9 MB binary + ~5 MB dylibs, `@loader_path` rpath) plus `Qwen2.5-1.5B-Instruct-Q4_K_M.gguf` (~1.0 GB) from HuggingFace.
- **`scripts/fetch-vendor-deps.sh`** — idempotent fetcher that runs once before any build. CI workflow should call this before `cargo build`. Repo stays ~5 MB; build artifacts get the model.
- **`src-tauri/src/embedded_llm.rs`** — new module. Locates the vendored binary (resource-dir in bundled mode, `CARGO_MANIFEST_DIR/vendor` in dev), picks a free ephemeral port, spawns `llama-server` with the GGUF, polls `/health` until 200 (timeout 90 s), exposes OpenAI-compatible `/v1/chat/completions`. Child process killed on `Drop`.
- **`Backend::Embedded`** added to `state.rs`. `AppState` gains `Mutex<Option<EmbeddedServerHandle>>` so the pipeline can read whether the server is up.
- **Spawn at app startup** — `lib.rs` setup block launches the server in a tokio task. Emits `funbutton:embedded-ready` (success) or `funbutton:embedded-failed` (with error string) when settled.
- **Pipeline fallback chain rewritten** (`pipeline.rs`):
  - `Auto` → embedded → ollama-external (if running) → groq (if key set)
  - `Embedded` / `Local` / `Groq` → just that backend
  - Final last-resort if every backend fails: return raw transcript verbatim with `backend_used = "raw-passthrough"` (better than a hard error for a free user).
- **React Settings UI** — `embedded` added to the backend chip row; new `embeddedReady` state listens to `funbutton:embedded-ready` / `-failed` and surfaces a "Bundled local model ready" toast + status pill in the backend hint.
- **`tauri.conf.json` resources** — `vendor/llama/llama-server`, all `lib*.dylib`s, the GGUF, and the LICENSE are declared as bundle resources. The `.app` ships them in `Contents/Resources/_up_/vendor/llama/`.
- **README** rewritten — headline now "Zero API key, zero install — cleanup runs on a bundled local model out of the box." Bundle size moved from "~10 MB" → "~1 GB (incl. bundled LLM)" honestly; install instructions add the `fetch-vendor-deps.sh` one-time step.

**Smoke tests passed locally:**
- `llama-server --version` runs, `@loader_path` resolves all 9 dylibs cleanly.
- Cold spawn → `/health` 200 in ~1 s (warm OS page cache; cold disk is ~5-10 s on M1/M2).
- `POST /v1/chat/completions` with the AUTO cleanup prompt returned a sensible cleaned string. Quality is below Llama 3.3 70B (expected for 1.5B Q4) but usable for the free-tier promise.
- `cargo check` + `tsc --noEmit` both clean.

**Sprint 2 backlog audit — items 2–6 were already shipped in Sprint 1:**
- ✅ Email / Slack / Raw / Code / Auto modes (`cleanup.rs` AUTO/EMAIL/SLACK/CODE/RAW prompts since the initial Tauri scaffold)
- ✅ Custom dictionary UI + prompt injection (Settings `dictionary: Vec<String>` + `USER DICTIONARY` block in `pipeline.rs`)
- ✅ Transcription history view (local SQLite `history.rs` + Cmd+Shift+H toggle, filter UI in `App.tsx`)
- ✅ Cmd+Shift+V re-paste last cleaned (`lib.rs` global shortcut, `last_cleaned` mutex)
- ✅ Hotkey remap UI in Settings (`hotkey_kind: Fn | RightOption` pill in Settings)

So bundled LLM was the only genuinely-new Sprint 2 item left.

**Known limitations / V1.2 follow-ups:**
- **Transcription still needs a key.** Whisper isn't bundled yet — free users need a Groq key OR a license for speech-to-text. Adding `whisper.cpp` + `ggml-tiny.en.bin` (~75 MB) is the next "no key, ever" step.
- **Universal binary not shipped** — vendor binary is arm64-only. Intel Mac users currently can't run the embedded backend (fall back to Groq / Ollama). Adding x86_64 vendor + `lipo` universal binary is a separate task.
- Cleanup quality on 1.5B Q4 is noticeably below Llama 3.3 70B. Power users will still pick premium models via license.

**Blockers (unchanged):**
- Stripe live keys + the 9 `STRIPE_PRICE_*` IDs.
- CF token with `Account:Zones:Edit` (or a 1-click "Add a Site" in the dashboard) to migrate `funbutton.ai` and attach `api.funbutton.ai`.

---

## 2026-05-12 11:30 — Day 7 polish: License UI + deep-link + landing pricing

**Settings UI (License panel)** — `apps/desktop/src/App.tsx`:
- New "License" tab (third alongside Settings + History). Paste-to-activate JWT flow → calls `validate_license` Tauri command → live tier/expiry/included-words/cap surfaced.
- Premium model selector (fast / Haiku / Sonnet / Opus / GPT-4.1) with per-model pricing hint.
- Monthly cap slider $0–$100, persists via `set_cap_cents` → Worker `POST /v1/usage/cap`.
- **ROSCA-compliant activation disclosure modal** — gates moving cap above $0 with explicit "Enable $X/mo cap" button (no dark patterns). `$0 = OFF` clearly labelled.
- "Manage subscription" button → opens Stripe Customer Portal (Worker `POST /v1/portal/portal`).
- "Sign out (BYOK)" button — clears `license_jwt`, returns to free tier.
- **Cap-hit toast** — listens for `funbutton:result` events with `backend === "cloud-fallback"` (set by `pipeline.rs` when 402 cap-exceeded + fast retry succeeds). Toast: "Monthly cap hit. Switched to fast tier. Adjust in Settings → License." Auto-dismisses after 5s.
- All new surfaces have dark-mode counterparts. Typecheck + cargo check both clean.

**Deep-link handler** — `funbutton://activate?jwt=<token>`:
- Added `tauri-plugin-deep-link@2.4.9` + capability `deep-link:default`.
- Registered `funbutton` scheme in `tauri.conf.json`.
- On URL receipt: parse JWT, persist to settings.json, show Settings window, switch to License tab, re-verify against Worker, fire success toast + macOS native notification.
- `parse_activation_jwt()` handles both `activate?jwt=...` and `activate/?jwt=...`. JWT is base64url-safe, no decoding needed.

**Landing page pricing** — `apps/web/app/page.tsx`:
- New `<PricingSection />` below the hero email capture with three tier cards: Free / Pro ($79/yr or $9/mo, marked "most popular") / Lifetime ($149 founder rung).
- Buttons POST to `/api/checkout` (Edge runtime route) → proxies to Worker `/v1/checkout/create-session` with tier name → redirects browser to Stripe Checkout.
- Worker checkout endpoint extended to accept `tier` *or* `price_id`; resolves tier to env var. Lifetime tier auto-resolves to the lowest active ladder rung.
- Graceful degrade: when Stripe is unconfigured (current state), the proxy returns 503 and the UI shows "Checkout opens soon — join the waitlist".
- Next.js production build passes (`next build` clean).

**DNS setup helper** — `apps/worker/scripts/setup-dns.sh`:
- One-shot migration script. Given a CF token with `Account:Zones:Edit + Zone:Zones:Edit + Zone:DNS:Edit` scopes (the existing tokens in `~/clawd/.env` lack `Zones:Edit`), the script:
  1. Adds `funbutton.ai` zone to Spontent CF account
  2. Pre-creates the existing Vercel A records (DNS-only / gray cloud, preserves landing page TLS)
  3. Updates Spaceship NS via their API → CF NS
  4. Polls for zone-active status (max 10 min)
  5. Attaches `api.funbutton.ai` as a Worker custom domain on `funbutton-api`
  6. Smoke tests `https://api.funbutton.ai/health`
- Idempotent — safe to re-run after partial failure.
- Desktop default `cloud_api_base` is currently `https://funbutton-api.todd-e03.workers.dev`; flip to `https://api.funbutton.ai` once DNS is live.

**Deploys:**
- Production Worker redeployed (`Current Version ID: 1f59bccc-0813-4a2a-9908-e38613a4f252`) with tier-aware checkout.
- Smoke tested: `POST /v1/checkout/create-session {"tier":"pro_annual"}` → 503 `stripe_not_configured` (expected; Stripe not wired yet).

**Blocked (unchanged from last entry):**
- **Stripe live keys** — need `STRIPE_SECRET_KEY` + 9 `STRIPE_PRICE_*` IDs in `~/clawd/.env` so the Worker can call Stripe. Until then: landing page CTAs show "Checkout opens soon", desktop License tab still works for any externally-minted JWT, and Worker webhook signature verification is alive (just no events flowing).
- **CF zone create permission** — `CF_API_TOKEN` and `CLOUDFLARE_WORKERS_API_TOKEN` lack `Account:Zones:Edit`. Either:
  - Mint a new CF token at dash.cloudflare.com → My Profile → API Tokens with the "Edit zone" template + add `Account → Zones → Edit`, then run `bash apps/worker/scripts/setup-dns.sh`, or
  - One-click "Add a Site" in the CF dashboard for `funbutton.ai`, then re-enable the route in `wrangler.toml` and `wrangler deploy --env production`.

---

## 2026-05-10 21:45 — Worker deployed (staging + production) + desktop wired

**Deployed:**
- **Staging:** https://funbutton-api-staging.todd-e03.workers.dev (Spontent LLC CF account)
- **Production:** https://funbutton-api.todd-e03.workers.dev (workers.dev URL — `api.funbutton.ai` route pending DNS)
- All 4 KV namespaces, D1 database `funbutton-prod`, both Durable Objects, monthly reset cron are live in production.
- All secrets pushed to both environments (JWT_SECRET, GROQ/Anthropic/OpenAI/Resend API keys, STRIPE_WEBHOOK_SECRET).

**Smoke tests passed (against staging worker, pro_annual JWT):**
- `GET /health` → `{"ok":true}` ✅
- `POST /v1/license/verify` → tier, expiry, included words, cap surfaced ✅
- `POST /v1/license/refresh` → fresh 30-day JWT returned ✅
- `POST /v1/cleanup` fast → `"we should probably ship the worker today"` (input had 8 fillers stripped) ✅
- `POST /v1/cleanup` premium-haiku → 1¢ recorded ✅
- `POST /v1/cleanup` w/ cap=0 on Lifetime tier → **HTTP 402 `{"fallback":"fast","reason":"cap_exceeded",...}`** exactly per spec ✅
- 50 req/min sliding-window rate limit → 429s fire under burst ✅
- D1 audit log → 36 rows recorded fire-and-forget ✅
- Bad JWT → 401 ✅
- Production worker passed identical end-to-end test (`"we shipped the worker today 👍"`) ✅

**Desktop client wired (`apps/desktop`):**
- New module `src-tauri/src/cloud.rs` — HTTP client for `/v1/transcribe`, `/v1/cleanup`, `/v1/license/verify`. Handles HTTP 402 cap-exceeded by exposing `CleanupOutcome::CapExceeded` so the caller can silently retry on fast tier.
- `src-tauri/src/pipeline.rs` rewritten: when `settings.license_jwt` is set, transcribe + cleanup route through the Worker. On any cloud failure (network, 5xx, cap exceeded → fast retry also fails) it transparently falls back to the existing BYOK Groq direct path. **Free-tier BYOK behavior is byte-for-byte unchanged when `license_jwt` is empty.**
- `state::Settings` gains `license_jwt`, `cloud_api_base` (default `https://api.funbutton.ai`), `premium_model` (default `premium-haiku`).
- New Tauri commands: `validate_license` (verifies a JWT against the Worker, returns tier/usage/cap), `set_cap_cents` (writes the monthly cap via `POST /v1/usage/cap`).
- `cargo check` clean. The existing `save_settings` command already accepts the full Settings struct so the React frontend can persist a pasted JWT today without further Rust changes.

**Remaining work (Day 7 polish — not blocking):**
- Settings UI: License panel (tier, expiry, included words remaining, spend, "Manage subscription"), cap slider ($0–$100), auto top-up activation disclosure modal, cap-hit toast.
- `funbutton://activate?jwt=...` URL scheme handler — currently the user pastes JWT into Settings manually.
- Stripe Customer Portal integration end-to-end (Worker has `/v1/portal/portal`, frontend needs the button).
- Landing page pricing buttons → `/v1/checkout/create-session`.
- DNS: Todd needs to either move `funbutton.ai` zone to Spontent LLC's Cloudflare account, or proxy api.funbutton.ai via a CNAME after adding the zone. Worker is live on `*.workers.dev` in the meantime.

**Blocked:**
- **Stripe live keys still needed.** Webhook signature verification works (whsec_ is set), but `STRIPE_SECRET_KEY` and the 9 `STRIPE_PRICE_*` IDs are not in `~/clawd/.env`. Stripe code paths return 503 `stripe_not_configured` gracefully. Todd needs to create FunButton products in Stripe per `WORKER-SPEC.md` §7 and run `wrangler secret put STRIPE_SECRET_KEY --env production` (+ the 9 price IDs).

---

## 2026-05-10 21:30 — Worker scaffold + full endpoint implementation (Week 2 Day 1–6 in one push)

**Done (all in `apps/worker/`):**
- TypeScript Worker project with Hono router. Typechecks clean.
- `wrangler.toml` — bindings for 4 KVs (`LICENSE_KV`, `USAGE_KV`, `CAP_KV`, `RATE_LIMIT_KV`), 2 Durable Objects (`UsageCounter`, `LifetimeCounter`), D1 (`D1_DATABASE`), cron `0 0 1 * *` for monthly reset, production route `api.funbutton.ai/*`, staging env on `*.workers.dev`. CF account ID locked to `e03523c149209369c46ebc10b8a30b43`. IDs left as `REPLACE_*` placeholders pending provisioning.
- D1 migration `migrations/0001_init.sql` — `usage_log` + `licenses` tables matching WORKER-SPEC §8.
- **JWT (HS256)** — `src/lib/jwt.ts`, hand-rolled on Web Crypto, no npm dep. `newClaims`, `refreshClaims`, `newLicenseId`.
- **Auth middleware** — `src/lib/auth.ts` checks signature, expiry, and `${license_id}:revoked` flag in KV.
- **Endpoints (all under `/v1`):**
  - `POST /license/verify` — returns tier, expiry, included words remaining, current spend, cap.
  - `POST /license/refresh` — issues fresh 30d / 90d JWT.
  - `POST /transcribe` — Groq Whisper Turbo proxy (≤25 MB, audio/* content types).
  - `POST /cleanup` — full hot-path: rate limit → Pro included-words check → cap pre-check → provider call → DO counter + D1 audit log + Stripe metered usage (all fire-and-forget via `waitUntil`). Returns HTTP 402 with `{ fallback: "fast", ... }` on cap hit.
  - `GET\|POST /usage` — dashboard data (fast words, premium words by model, spend, cap, history).
  - `POST /usage/cap` — set monthly cap ($0–$100, clamped).
  - `POST /checkout/create-session` — Stripe Checkout for Pro/Lifetime (subscription auto-attaches the 4 metered items).
  - `POST /portal/portal` — Stripe Customer Portal URL.
  - `POST /stripe/webhook` — HMAC-SHA256 signature verify, handles `checkout.session.completed` (mints JWT, persists to KV+D1, sends Resend activation email with `funbutton://activate?jwt=...`), `customer.subscription.updated`, `customer.subscription.deleted` (sets revoked flag), `invoice.paid` (extends expiry). Lifetime ladder ($149 → $199 at 1K, → $249 at 5K) wired via `LifetimeCounter` DO with atomic increment + auto `archivePrice`/`activatePrice` Stripe calls.
- **Cap enforcement** (the Stripe gotcha) — `src/routes/cleanup.ts` checks `USAGE_DO` counter + `CAP_KV` *before* the premium provider call. Conservative `projectedCost = calcCost(input_words × 2)`. $0 cap → hard stop. Pro included quota (default 50K words/mo, counted across all premium models) checked first.
- **Rate limit** — 50 req/min sliding window via `RATE_LIMIT_KV`, weighted-bucket approximation.
- **Provider routing** — Groq Llama 3.3 70B for `fast`, Anthropic `claude-haiku-4-5` / `claude-sonnet-4-7` / `claude-opus-4-7` for `premium-*`, OpenAI `gpt-4.1` for `premium-gpt41`. Cleanup prompts ported verbatim from `apps/desktop/src-tauri/src/cleanup.rs` (Auto/Email/Slack/Code/Raw) + identical dictionary boost.
- **Cron** — monthly reset iterates `LICENSE_KV.list({ limit: 1000 })`, resets each `UsageCounter` DO at 00:00 UTC on the 1st.
- **Test license minting** — `scripts/mint-test-license.ts` (tsx-runnable), reads `JWT_SECRET` from `.dev.vars` or env.
- **Docs** — `apps/worker/README.md` covers local dev, provisioning, deploy, endpoint table, Stripe model.

**Next:**
- Provision real CF resources (KV namespaces × 4, D1 database, secrets) — Wrangler API token is valid for `Spontent LLC` account.
- Deploy staging worker, smoke test all endpoints with minted JWT.
- Wire desktop app: send `Authorization: Bearer <jwt>` to `api.funbutton.ai` when a JWT is in Keychain; keep BYOK Groq direct path as fallback. Add Settings panel (license, cap slider, "Manage subscription").
- Configure DNS `api.funbutton.ai` → workers (Spaceship → Cloudflare zone).

**Blocked:**
- **Stripe live keys not in `~/clawd/.env`.** Only `STRIPE_WEBHOOK_SECRET` is set; no `STRIPE_SECRET_KEY` / price IDs. Stripe CLI is logged into a different sandbox account ("Overskill") and the test key there has expired. **Need from Todd:** (a) create a FunButton Stripe account (or reuse Spontent), (b) create the 9 products/prices from `WORKER-SPEC.md` §7, (c) drop the resulting `sk_test_*` + 9 `STRIPE_PRICE_*` IDs into `~/clawd/.env`. Code paths return HTTP 503 `stripe_not_configured` gracefully until then; everything else (license verify/refresh, transcribe, cleanup, cap enforcement, usage) is independent of Stripe and ready to ship.

---

## 2026-05-09 11:54 — Pricing locked + Worker spec'd

**Done:**
- Validated pricing via deep research (`PRICING-RESEARCH.md`, 40 sources, commit b35428d)
- Locked Pro $9/mo or $79/yr, Lifetime $149 → $199 → $249 ladder, Haiku 4.5 as premium default ($0.40/10K words, ~78 % margin)
- Sonnet 4.7 ($0.60), Opus 4.7 ($0.99), GPT-4.1 ($0.50) as premium options
- 50K included premium words/mo on Pro; Lifetime is pay-as-you-go above Groq fast tier
- Auto top-up: $20 default cap, $0–$100 slider, opt-in OFF by default (ROSCA + California ARL safe)
- `PRD.md` updated with validated tiers + compliance UI requirements + monetization architecture pointer
- `WORKER-SPEC.md` written for Week 2 monetization sprint — endpoints, KV/DO/D1 schema, Stripe products, hot-path TS pseudocode, day-by-day 7-day ship plan
- Critical implementation note documented: Stripe `billing_thresholds` does NOT hard-stop — Worker MUST enforce caps app-side via Durable Object counters before every premium API call

**Next:** Sprint 2 desktop features continue this weekend (modes, dictionary, history). Week 2 (V1.1) coding agent will build the Cloudflare Worker from `WORKER-SPEC.md`.

**Blocked:** none.


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
