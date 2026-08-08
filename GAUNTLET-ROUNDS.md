# GAUNTLET LOOP v2 — Round Log

Run date: 2026-08-08 · branch `gauntlet-v2-polish` · 6 pieces × (worker + separate blind critic), max 3 rounds per piece.
Critics judged the current file state side-by-side against the reference pegs (Handy, VoiceInk, zachlatta/freeflow, unramble, Wispr Flow) plus the AI-slop ban list; any ban hit = automatic FAIL.

**Calibration note:** no piece passed round 1 — every surface failed at least once before converging, so the bar was doing real work. All six pieces ended PASS with the critic answering YES to "would a developer with Handy or VoiceInk installed switch on looks and feel alone?"

| Piece | R1 | R2 | R3 | Final |
|---|---|---|---|---|
| A — Onboarding wizard | FAIL | FAIL | PASS | **PASS** |
| B — Settings window | FAIL | PASS | — | **PASS** |
| C — Recording pill / HUD | FAIL | FAIL | PASS | **PASS** |
| D — Tray menu + app-level UX | FAIL | PASS | — | **PASS** |
| E — Dev-first behavior depth | FAIL | PASS | — | **PASS** |
| F — Landing page | FAIL | FAIL | PASS | **PASS** |

## Piece A — Onboarding wizard — first-run permission flow that feels like the app healing itself

### Round 1: **FAIL** (would_switch: False)
Critic must-fix:
- Remove the step-2 dead-end (onboarding.tsx:371, :137): when !allGranted the primary CTA must stay enabled and advance to step 3 (e.g.
- Make step 6 fit 720x520 with the primary CTA visible without scrolling (currently 261-289px of vertical overflow, measured): cut the 'Want it faster or fancier?' sub-line, compress the two option tiles into compact single-column rows or a collapsed 'Optional: faster/cloud paths' disclosure, and shri…
- Add min-width: 0 to .ob-tile (or the .ob-tiles grid children, onboarding.css:559) so the 1fr/1fr grid stays balanced — the Ollama tile's nowrap CopyBlock currently squeezes the Groq tile to ~1/3 width and clips its copy mid-sentence.
- Fix step 2's 37px overflow so the Right-Option skip link is fully visible below the CTA (reduce .ob-perm-stack card padding or .ob-slide.compact gaps); verify at 720x520 with all three cards in 'waiting' state.
- Fix the stale-closure permission poll (onboarding.tsx:70-72): use functional updates — setMicPerm(prev => m ? 'granted' : prevDeny(prev)) — so revocation/denial actually reflects; and make finish() (tsx:186-192) build one merged settings object and call save_settings once instead of two racing persi…

### Round 2: **FAIL** (would_switch: False)
Critic must-fix:
- Step 7 overflow: with the 'Fn opening the emoji picker instead?' details open at 720x520, .ob-stage scrollHeight exceeds clientHeight by ~61px and the finish CTA is pushed fully out of view (onboarding.tsx:643-655, onboarding.css:687-725). Reclaim the space — e.g. shrink .
- Step 7 must prove the loop, not just the mic: add a visible landing target — an autofocused 'dictate right here' textarea (styled like .

### Round 3: **PASS** (would_switch: True)
Why it passed (critic's evidence, condensed):
- VERIFIED LIVE (mocked Tauri IPC in Chrome at exactly 720x520): flipping a permission heals the screen with no clicks — card scale-pop + drawn check ring (onboarding.css:226-249, 289-301), one-shot light sweep (onboarding.css:303-315), inter-card wire lighting top-to-bottom (onboarding.
- Denied recovery exceeds the peg: revocation is detected by downgrading a previously-granted switch to denied (onboarding.tsx:83-89) and surfaces 'macOS yanked that switch back off' with numbered rescue steps including the off-and-on-again reality (onboarding.
- The live poll is made visible and honest — 'watching System Settings live' / 're-checking every 0.7s · flip it and we jump' pulsing pill (onboarding.tsx:343-350, onboarding.css:252-270) — which is what makes the wizard feel like the app healing itself rather than a form.
- Keyboard hero is crafted, not clip-art: cropped aluminum plate bleeding off-frame, home-row sliver selling the zoom (onboarding.tsx:829-832), hand-drawn SVG globe glyph explicitly instead of emoji (onboarding.tsx:849-853), fn-cap press nudge + staggered double ripple halo (onboarding.css:889-911).
- Fn/Globe emoji-picker collision surfaced sensibly in two places: Right Option escape hatch on the Input Monitoring slide (onboarding.tsx:279-282) and a collapsed aside on the try-it step with copyable 'defaults write com.apple.

## Piece B — Settings window — engine status, mode intelligence, dictionary, history, hotkey

### Round 1: **FAIL** (would_switch: False)
Critic must-fix:
- App.tsx:453-465 — stop gating the welcome card on !hasGroqKey. Key it to first-run instead: add a dismiss (×) button that persists a flag (reuse settings.onboarded or a localStorage key) so the default zero-key local user doesn't see intro copy at the top of Settings forever.
- App.tsx:503-534 — when all three permissions are granted, collapse the perms block to a single compact row ('permissions · 3/3 granted ●', click/chevron to expand the three rows).
- App.tsx:733-738 + App.css — move the words-today counter out of the pre-footer stats strip into the always-visible shell (e.g. right side of the fb-header status area, mono style: '1,284 words today'), keeping last-dictation details where they are.

### Round 2: **PASS** (would_switch: True)
Why it passed (critic's evidence, condensed):
- Engines is now ONE coherent status board (App.tsx:609-634): four rows (whisper / qwen 2.5 1.5b / ollama / groq cloud) each with state glyph, detail, and a tokenized status chip, plus a live route label from autoRouteLabel() (App.tsx:421-431) that honestly mirrors pipeline.
- Per-app mode intelligence is SHOWN, not buried, and it is truthful: MODE_MAP (App.tsx:80-103) was verified line-by-line against app_detect.rs classify() and cleanup.rs Mode::from_front_app — including the term mode that is auto-detect-only and correctly absent from the override pills.
- Dictionary is a proper chip editor (App.tsx:949-993): enter/comma commit, multi-term paste split on commas/newlines, dedupe, backspace-deletes-last-chip, commit on blur, term count in the section header. Clearly better than the old raw textarea (diff confirms the textarea was replaced).
- History got real product care: day-grouped with sticky 'today/yesterday' labels (App.tsx:405-415, App.css:665-677), app/mode/duration chips per row, a paste-failed triage banner with one-click copy (App.
- Interaction model matured to the Wispr bar: the global Save button is dead — pills save instantly via setAndSave (App.tsx:296-299), text fields save on blur, with a 'saved ✓' tick in the header. Permissions collapse to a single green 3/3 row when granted (App.

## Piece C — Recording pill / HUD — the floating indicator while dictating

### Round 1: **FAIL** (would_switch: False)
Critic must-fix:
- Fix the too-short hint clipping (pill.tsx:314, pill.css:168-173): shorten the copy to fit 212px inner width (e.g. 'too quick — keep holding') OR give .t-hint min-width:0 + overflow:hidden + text-overflow:ellipsis on a min-width:0 layer. Verify at 11px mono the final string measures under 212px.
- Fix the clipped drop shadow: either tighten pill.css:24 to a shadow that fits 14px of bottom clearance (e.g. 0 4px 12px rgba(0,0,0,0.45)) or keep the shadow and bias the pill upward inside #root (align-items with bottom padding) / bump the pill window height in tauri.conf.
- Close the getUserMedia race in startMic (pill.tsx:124-146): make the post-await guard also bail when a stream is already installed — `if (disposed || current !== "recording" || audio) { stream.getTracks().forEach((tr) => tr.
- Resolve the dead drag affordance: either implement it (data-tauri-drag-region attribute on the pill root + add core:window:allow-start-dragging to capabilities/pill.json) or delete pill.css:28 and the 'stays draggable' claim in the pill.tsx header comment.
- Swap the '✓' text glyph (pill.tsx:301) for lucide-react's <Check size={13}/> per the repo owner's global icon rules — lucide-react is already in package.json.

### Round 2: **FAIL** (would_switch: False)
Critic must-fix:
- tauri.conf.json (pill window object only): add "visibleOnAllWorkspaces": true so the HUD appears over fullscreen apps and on every Space — the fullscreen-IDE dictation flow currently shows no pill at all.
- pill.tsx: extract the bottom-center placement block (lines 60-76) into a reusable async function and call it on every 'recording' status event (before/alongside startMic), targeting the monitor the user is actually on via cursorPosition() + monitorFromPoint() (both already granted through core:windo…
- pill.tsx onStatus (lines 228-259): guard the 'pasting', 'error', and 'idle' cases so they do NOT stopMic/stopPoll/replace the UI when current === 'recording' (an overlapped previous dictation finishing mid-hold).

### Round 3: **PASS** (would_switch: True)
Why it passed (critic's evidence, condensed):
- Waveform is driven by real mic data, not fake randomness: getUserMedia + AnalyserNode, RMS with gain/gamma (pill.tsx:118-137), 26-bar scrolling ~1.2s history (pill.tsx:45-47, 128-134), direct DOM transforms with zero per-frame React renders (pill.tsx:110-116).
- Mic and animation teardown is airtight: stopMic cancels the RAF, stops all tracks, closes the AudioContext, and zeroes+repaints the history (pill.tsx:139-152); a generation counter self-destructs stale in-flight getUserMedia resolutions on quick taps and rapid re-press (pill.
- The dev-first receipt is real and wired end-to-end: lib.rs:819-823 emits '{mode} mode · {frontmost}' with the pasting status, rendered as green Lucide check + monospace 'code mode · Cursor' + 'pasted' tag (pill.tsx:362-368).
- State transitions are distinct and smooth: springy pill-in entry (pill.css:43, 53-56), per-state layer-in (pill.css:74-78), the capsule width glides 244→284px for receipt/error states (pill.css:46, 65), a proper leaving fade (pill.css:48-52), per-state border tints (pill.
- Overlapped-dictation handling is genuinely thought through: late pasting/error/idle events from a previous pipeline can never kill a live waveform (holdIsHot guard, pill.tsx:244, 262-264, 274-281, 299-301), matching Rust's unconditional post-pipeline hide (lib.

## Piece D — Tray menu + app-level UX — state-aware, useful, not an afterthought

### Round 1: **FAIL** (would_switch: False)
Critic must-fix:
- tray.rs replay_onboarding handler (tray.rs:269 via show_window at tray.rs:324-329): reload the onboarding webview before showing it so the flow restarts at step 1 — e.g. in the replay_onboarding branch, before show/focus, call `let _ = w.eval("window.location.reload()");` on the onboarding window.
- tray.rs:170-172 + build_menu/on_menu_event: when the whisper or cleanup engine is in the 'failed' state, make that telemetry line an enabled, actionable item (click -> show_window(app, "settings")) instead of a permanently disabled label; keep ready/warming states disabled.

### Round 2: **PASS** (would_switch: True)
Why it passed (critic's evidence, condensed):
- Every promised item exists and is wired: live status line (tray.rs:351-359), armed-hotkey display (tray.rs:353,393), engine health with click-to-fix-when-failed (tray.rs:172-180,275,362-383), mode quick-switch persisting via the same crate::persist path Settings uses (tray.rs:308-327, lib.
- Main-thread discipline is exemplary and fixes a pre-existing hazard: the old code called tray.
- The menu-bar title glyph (● recording / … working / ! error, tray.rs:93-99) makes recording visible with zero windows — a genuinely Wispr-grade touch neither Handy nor VoiceInk has, and error state persists visibly until the next attempt instead of failing silently.
- Factual claims in menu copy verified against the code: 'qwen 1.5B, local' matches the bundled qwen2.5-1.5b-instruct-q4_k_m.gguf (embedded_llm.rs:18); 'down, using cloud' matches the pipeline's actual STT fallback order gated on the same groq_api_key/license_jwt predicate (pipeline.rs:84-115, tray.
- Side-by-side with the pegs: Handy's tray is thin (its power-user depth lives in CLI flags) and Wispr shows state but no engine transparency — this tray's lowercase log-style telemetry, click-to-fix engine lines, and 'v0.1.

## Piece E — Dev-first behavior depth — code mode, dev vocabulary, editor/terminal detection

### Round 1: **FAIL** (would_switch: True)
Critic must-fix:
- Thread the STT vocabulary bias into the Groq BYOK transcription path: change groq::transcribe (groq.rs:51) to accept prompt: Option<&str> and add .text("prompt", p) to the multipart form when present; pass the existing stt_prompt at the single call site (pipeline.rs:113).
- Add an explicit strict self-correction rule with one dev-flavored example to CODE_PROMPT (e.g.

### Round 2: **PASS** (would_switch: True)
Why it passed (critic's evidence, condensed):
- WEDGE HOLDS vs freeflow: CODE_PROMPT (cleanup.rs:154-214) covers a ~25-mapping spoken-symbol table, long/short/capital-letter CLI flags, file paths/URLs/localhost ports, dotted versions, git commit conventions (imperative mood, no trailing period, conventional-commit types), six identifier-casing co…
- TERMINAL-VS-EDITOR SPLIT is the standout nobody in the peg set has: TERMINAL_PROMPT (cleanup.
- TOKEN-BUDGET HANDLING VERIFIED END-TO-END: build_stt_prompt (pipeline.rs:258-281) loads user dictionary terms first (they outrank built-ins under truncation), appends the curated DEV_DICTIONARY_STT, caps at 600 chars under whisper's ~224-token limit; embedded_stt.
- DICTIONARY DESIGN IS CURATED, NOT PADDED: two-tier split (cleanup.rs:47-86 full cleanup vocabulary; cleanup.rs:94-105 STT subset chosen for phonetic confusability) with four real invariant tests — STT-subset relation (cleanup.
- APP DETECTION BREADTH WITHOUT ADDED LATENCY: app_detect.rs keeps the single pre-existing osascript round-trip and adds only pure string matching — 15 terminal emulators, the VS Code family, 13 JetBrains products, AI-first IDEs (Kiro/Trae/Void/PearAI/Positron/Antigravity), and the smart git-clients-a…

## Piece F — Landing page — funbutton.ai, from retrofitted coming-soon to real project page

### Round 1: **FAIL** (would_switch: False)
Critic must-fix:
- Add a real OG image: create apps/web/app/opengraph-image.tsx (next/og ImageResponse, dark bg + red Fn keycap + 'The dictation button for developers' + 'No API key. Ever.') so the declared summary_large_image card (apps/web/app/layout.
- Surface GitHub social proof pre-conversion: move/duplicate the shields star badge currently hidden in SuccessState (apps/web/app/page.tsx:516-520) into the Nav next to 'source →' (page.tsx:48-55) or beside the hero download CTA, so repo liveness is visible without submitting an email.
- Fix the mobile eyebrow wrap (apps/web/app/page.tsx:67-69): render the '▌' marker as a separate span in a flex container (or use padding-left with a positioned marker) so the wrapped second line ('developers') aligns with the first line's text at 390px.
- Trigger the terminal demo stagger on visibility instead of mount: add a small IntersectionObserver (or animation-timeline: view() with a mount fallback) that applies the fb-line animation class when #demo enters the viewport (apps/web/app/page.tsx:147, apps/web/app/globals.

### Round 2: **FAIL** (would_switch: False)
Critic must-fix:
- Remove the live shields.io star-count badge from the nav (apps/web/app/page.tsx:55-59) and from the email success state (page.tsx:568-573). Replace with a plain 'source' link or an inline GitHub SVG mark — reintroduce a count only when it is five digits.
- Fix the '(Apple Silicon)' annotation contrast on the primary download CTA (apps/web/app/page.tsx:123): replace text-red-900/70 with text-black/70 (≥4.5:1 on bg-red-500), or move it out of the button into the existing font-mono sub-caption below (page.tsx:135).
- Stop 'bottom-left' hyphen-splitting in the H1 at desktop widths (apps/web/app/page.tsx:98-99): wrap it in <span className="whitespace-nowrap">bottom-left</span> or move the <br /> so the line breaks after 'your Mac' cleanly; verify at 1440 and 390 that no dangling hyphen remains.

### Round 3: **PASS** (would_switch: True)
Why it passed (critic's evidence, condensed):
- LEADS with the wedge: first content line is the eyebrow 'the dictation button for developers' (page.tsx:103) and the hero paragraph lands 'terminal, your editor, your coding agent' + 'No API key. No account. No cloud.' (page.
- The animated terminal demo (page.tsx:188-282) is the best section on any page in the peg set: it shows the actual product value (rambling speech -> 'git commit -m "fix race condition in backoff logic"', and a Claude Code prompt) instead of describing it.
- The 'honest table' (page.tsx:366-439) is factually consistent with OSS-LANDSCAPE-DEEP-RESEARCH-2026-08-01.md row by row: Handy MIT/$0/offline/optional cleanup/Tauri+Rust; VoiceInk GPLv3/$25-49 lifetime/optional cloud-text cleanup/Swift; Wispr $12-15/mo, account required, cloud-only, ~$334M raised.
- Voice is punk without cringe: 'GPLv3 — read every line before it reads you' (page.tsx:144-145), 'raised ~$334M and still can't work on a plane' (page.tsx:433-436), 'we don't have a marketing team' (page.
- All prior slop violations fixed in this diff: the glow pulse on the brand glyph was removed (globals.css:41, now static inset with comment 'brand mark — static'), keycaps use inset edge depth explicitly 'not glow' (globals.css:91-103), and the email input is now 16px at mobile (page.
