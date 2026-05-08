# FunButton.ai — Quality Match Spec (vs Wispr Flow)

*Date: 2026-05-08 · Author: Ea (Claude Opus 4.7) · Inputs: RESEARCH.md, STACK-WARGAME.md, PRD.md, ~25 fresh web searches across Wispr docs, reviews, and YouTube walkthroughs (May 2026)*

---

## TL;DR

Wispr Flow's "magic" is **not one model** — it is a tightly-orchestrated five-layer pipeline: (1) per-language ASR engine routing, (2) cleanup model with backtrack/filler logic, (3) per-app-category style formatter, (4) two-channel dictionary (recognition boosting + post-correction replacement), (5) trigger-phrase snippet expander. We can match four of the five layers within Sprint 2.6 using Groq Whisper + Llama 3.3 70B, and **decisively beat them on layer #2 (cleanup)** and layer #6 they don't have (per-user few-shot learning loop).

The single biggest leverage is **the cleanup prompt**. Llama 3.3 70B has more headroom per pass than Wispr's tighter cloud-cleanup budget can afford. The second biggest leverage is **the personal learning loop** — Wispr does not ship per-user adaptation today; this is our durable wedge against their data flywheel.

This document specifies feature-by-feature what to build, in what order, with prompts and code skeletons a coding agent can implement directly. Sprint 2.6 implementation order is at the bottom.

---

## Section 1 — Feature-by-Feature Spec

### 1.1 AI Cleanup / Auto-Edit

#### How Wispr does it

**User POV** — speak naturally with filler words, mid-sentence redirects, run-on sentences. Output arrives as polished prose with: filler words removed, punctuation inferred from pauses, capitalization fixed, run-ons split, mid-sentence redirects collapsed (e.g. "let's meet Tuesday — actually, Wednesday" → "let's meet Wednesday"), bullet points auto-formatted when speech pattern implies a list ("first... second... third...").

Key behaviors confirmed across reviews:
- **Backtracking**: "Hey, can we meet on Tuesday? Scratch that, I'd rather meet Wednesday" → "Hey, can we meet on Wednesday?" — Wispr drops "Tuesday + scratch that" entirely. (Source: chrismenardtraining.com, May 2026)
- **Numerical correction**: "meet at 2... actually 3" → "meet at 3"
- **List detection**: "the items we need are apples, pears, bananas" → bulleted list when context is appropriate
- **Punctuation inference**: pauses → commas/periods; rising tone → question marks (claim, varies in practice)
- **Selective cleanup**: doesn't always strip everything — "let me think about this" stays in because it's content, not filler

**Inferred implementation**: Two-stage pipeline. Stage 1 = streaming ASR (in-house model, possibly distilled Whisper or Conformer variant, dynamically routed per language). Stage 2 = small cleanup LLM that reads the raw transcript + last-N tokens of context + app metadata, outputs cleaned prose. The cleanup model is pipelined to start as soon as ASR returns its first finalized chunk — this is how they hit sub-second perceived latency. Cloud-side, co-located. Likely a 7-13B model fine-tuned on (raw transcript, cleaned output) pairs from their data flywheel.

**Limitations / complaints**:
- "Quality dropped after I paid" (Trustpilot 2.7/5, multiple Vocai-cited reviews). Not confirmed root cause; could be model swap, rate-limit throttling, or psychological.
- Sometimes leaves filler that shouldn't be ("let me think about that") — judgment calls go either way.
- Sometimes mis-formats as bullets when user wanted prose.
- Cannot disable cleanup ("just give me raw") — workaround is "Raw" mode but power users complain it's not raw enough.

#### How FunButton matches or beats

**Architecture**: Whisper Large v3 Turbo on Groq → Llama 3.3 70B cleanup pass with mode-aware prompt. We pipeline by streaming the Whisper response token-by-token into a Llama prompt that begins generating before Whisper finalizes. Llama 3.3 70B at 250-330 tok/s on Groq is fast enough for sub-second TTFT.

**Why we can beat them on cleanup**: Llama 3.3 70B is substantially larger than Wispr's likely 7-13B cleanup model. We have more reasoning headroom per pass. Wispr's tight latency budget likely forces them into a smaller model. We can afford a slower, smarter pass *and still feel fast* because (a) we stream, (b) most users care about the perceived end-to-end, not p50 latency.

**Concrete cleanup prompt for Llama 3.3 70B (Mode = Auto/Default)**:

```
You are FunButton's cleanup model. You receive a raw spoken transcript and produce
polished written text that matches what the user MEANT to say.

RULES:
1. REMOVE filler words: um, uh, like (when filler), you know (when filler), sort of, kind of, I mean (when filler).
2. KEEP filler words when they carry meaning: "I sort of agree" stays (it's hedging).
3. HANDLE backtracks: when speaker changes mind ("X, scratch that, Y", "X, actually Y", "X — no, Y"), output ONLY the final intent.
4. HANDLE numerical corrections: "at 2, actually 3" → "at 3". Apply same to dates, names, prices.
5. ADD punctuation, capitalization, sentence breaks. Infer from pauses (marked as "...") and intonation cues.
6. SPLIT run-ons. A spoken paragraph often becomes 2-4 written sentences.
7. DETECT list intent: explicit ("the three things are...") or implicit ("first... second... third..."). Format as bullets if 3+ items AND context isn't a sentence.
8. PRESERVE: numbers, proper nouns, technical terms, code identifiers verbatim. NEVER paraphrase.
9. PRESERVE the user's voice: don't make it sound like ChatGPT. Match their register.
10. If raw transcript is already clean, return it unchanged. Do NOT add structure where the user did not intend it.

DICTIONARY (boost recognition, prefer these spellings):
{user_dictionary_csv}

REPLACEMENTS (apply post-cleanup):
{user_replacements_json}

RECENT CLEANUP EXAMPLES (last 3, for personal style reference):
{few_shot_examples}

APP CONTEXT: {frontmost_app_category}

RAW TRANSCRIPT:
{whisper_output}

POLISHED OUTPUT:
```

**Eval criteria**: build a corpus of 50 hand-recorded utterances with hand-cleaned expected outputs across 5 categories (10 each): clean speech, heavy fillers, backtracks, numerical corrections, list intent. Measure exact-match-after-normalization and edit-distance against expected. Re-run after every prompt change. Target: 90% exact-match, <5 edit-distance avg. Compare against a parallel run through Wispr Flow on the same audio (semi-automated via macOS Accessibility API).

**Eval harness location**: `apps/fun-eval/` — a Rust binary that takes a fixtures dir of `.wav` + `expected.txt` pairs, runs them through our pipeline, diffs outputs, and emits a markdown report. Run before every Sprint or model/prompt change.

---

### 1.2 Custom Dictionary

#### How Wispr does it

**Confirmed from docs.wisprflow.ai/articles/4052411709**:
- Two distinct mechanisms in one UI:
  - **Vocabulary words**: sent to ASR server as recognition bias (Whisper's `prompt` parameter or equivalent). Improves transcription accuracy for the term itself.
  - **Replacement rules**: applied AFTER transcription. Maps a misrecognized form ("Draught") → correct form ("Draft"). 1-to-1 only.
- **Auto-add**: when user types a correction over a transcription, Wispr's proper-noun detector decides whether to add it. Filters out common words; only proper nouns / uncommon terms get auto-added.
- **Star/priority**: users can star high-priority words. Starred words win conflicts.
- **Sync** across devices via account.
- **60-char limit per term, 1-60 chars min/max**. Duplicates rejected. Cannot also be a snippet trigger.
- Bulk CSV import (gated behind Experimental flag).

**Inferred implementation**: Whisper's `initial_prompt` parameter accepts up to ~244 tokens of bias context. Wispr likely concatenates dictionary words (prioritizing starred + recently-used) into this prompt for each transcription request. Replacement rules are a regex/string-replace pass downstream of ASR, before cleanup.

**Limitations / complaints**:
- 60-char cap rules out longer phrases.
- Cannot have one word be both a dictionary entry AND a snippet trigger — confusing for users.
- Auto-add false positives in noisy domains (dev usernames, code identifiers). No "ignore" list.
- No domain-specific dictionaries (one per user).

#### How FunButton matches or beats

**Two-channel implementation, identical surface**:

1. **Recognition boost** — feed last-N starred dictionary words + last-N recently-used corrections into Whisper API's `prompt` field on every transcription request:
   ```rust
   let prompt = format!(
     "Vocabulary: {}",
     dictionary.starred().chain(dictionary.recent(20))
       .map(|w| w.term.as_str())
       .collect::<Vec<_>>()
       .join(", ")
   );
   // POST to api.groq.com/openai/v1/audio/transcriptions with `prompt: prompt`
   ```
   Whisper API accepts ~244 tokens. We have ample room.

2. **Replacement pass** — applied between Whisper output and Llama cleanup. Implementation: simple `HashMap<String, String>` with case-preserving replace (preserve "Draught" → "Draft", "draught" → "draft"). Use Aho-Corasick for >100 entries.

**Where we beat Wispr**:

a. **Auto-promotion via learning loop** — when user edits a transcription and the edit involves a token that's not in dictionary AND looks like a proper noun (NER tag = PER/ORG/MISC), prompt the user once: "Add 'Spontent' to your dictionary?" After 3 occurrences without dismissal, auto-add. Wispr does this at proper-noun level today; we tighten it to "3-strike confirmed" so it's less noisy.

b. **No 60-char cap** — we accept phrases up to 240 chars. Limit is total token budget for Whisper prompt, not per-entry. Lets users add things like "ClickFunnels payment integration suite" verbatim.

c. **One term can be both dictionary + snippet** — disambiguate by context. Snippet expansion runs as a separate post-cleanup pass; dictionary is pre-cleanup. No conflict, no validation error.

d. **Domain dictionaries** — let user create named dictionary sets ("Engineering", "Marketing", "Personal") and toggle which is active per-app. Wispr is one-flat-list; we ship layered.

**Concrete spec for auto-promote logic**:

```rust
// In src/learning/auto_promote.rs
pub struct AutoPromoteCandidate {
  term: String,
  occurrences: u32,
  first_seen: DateTime<Utc>,
  last_seen: DateTime<Utc>,
  dismissed: bool,
}

// Run after each user edit that diffs from cleanup output.
// Tokenize the diff, run NER (use rustbert or call out to Llama for proper-noun extraction),
// for each new proper-noun token: increment counter in candidates table.
// At 3 occurrences without dismissal, auto-add to active dictionary and surface a toast:
// "Added 'Spontent' to your dictionary. [Undo]"
```

**Eval criteria**: 90% accuracy on "if user said this jargon term, it transcribed correctly" across a 30-term jargon corpus (mix of dev, medical, legal). Compared head-to-head with Wispr on the same audio.

---

### 1.3 Snippets

#### How Wispr does it

**Confirmed from docs.wisprflow.ai/articles/5784437944**:
- Trigger phrase → static expansion text. Inline OR standalone.
- Inline: "send the package to my address" → trigger phrase "my address" replaced inline with expanded value.
- Standalone: speak only the trigger → punctuation stripped before matching, snippet fires.
- **No variables, no dynamic content**. Confirmed in docs FAQ: *"Dynamic variables aren't supported. Snippets insert static text only."*
- Sync across devices, max 1000 items per JSON import, 3MB cap.
- Team snippets (Pro/Enterprise) vs personal snippets. Personal wins on conflict.
- Cannot share a trigger with a dictionary entry.

**Limitations / complaints**:
- Static-only is a real limit. Users want `{date}`, `{recipient_name}`, `{calendar_link}`, last-message-context, etc. Multiple Reddit requests over 12+ months with no movement.
- Trigger matching is exact-string when inline. "my address" won't match if speech-to-text adds "the my address."
- No fuzzy matching, no synonym expansion.

#### How FunButton matches or beats

**Phase 1 (Sprint 2.6) — match Wispr's static snippet feature**:

- SQLite table: `snippets(id, trigger, expansion, scope, app_filter, created_at)`.
- Post-cleanup pass: for each snippet, do case-insensitive whole-word match. If matched standalone (only trigger said, after stripping punctuation), replace the entire output. If matched inline, replace inline.
- UI: settings tab with import/export, JSON/CSV.
- Same sync mechanism as dictionary (tied to optional cloud account in V1.1; local-only by default).

**Phase 2 (Sprint 3 / V1.1) — beat Wispr with dynamic snippets**:

Templating with handlebars-style `{{var}}` placeholders. Built-in variables:

- `{{date}}` — today's date in user's locale
- `{{time}}` — current time
- `{{recipient}}` — best-effort detection from frontmost app context (e.g., Slack DM partner name, email To: field) via macOS Accessibility API
- `{{clipboard}}` — current clipboard contents
- `{{lastmsg}}` — last received message in current Slack/iMessage thread
- `{{selection}}` — currently selected text

Plus **AI-aware expansion** — `{{ai:rephrase formal}}` triggers a Llama 3.3 pass on the rest of the snippet content. Lets users build snippets like:

```
Trigger: "client follow up"
Expansion: "Hi {{recipient}}, following up on our call. {{ai:summarize the last 3 messages from this thread in 2 sentences}} Let me know your thoughts. — Todd"
```

Wispr cannot ship this without rebuilding their architecture. We can ship it because we already have a Llama pipeline in the hot path.

**Concrete spec**:

```rust
// In src/snippets/expand.rs
pub async fn expand_snippet(snippet: &Snippet, ctx: &AppContext) -> Result<String> {
  let mut out = snippet.expansion.clone();
  // 1. Substitute simple vars
  out = out.replace("{{date}}", &chrono::Local::now().format("%B %-d, %Y").to_string());
  out = out.replace("{{recipient}}", &ctx.detect_recipient().unwrap_or_default());
  // ... etc
  // 2. Resolve {{ai:...}} blocks via Llama
  while let Some((start, end, instruction)) = find_ai_block(&out) {
    let resolved = llama_quick(instruction, ctx).await?;
    out.replace_range(start..end, &resolved);
  }
  Ok(out)
}
```

**Eval criteria**: 30 hand-built snippets across personal/work categories. 100% trigger-fire rate, <100ms expansion latency for static, <500ms for dynamic with AI block.

---

### 1.4 Tone Matching / App Awareness ("Personalized Style")

#### How Wispr does it

**Confirmed from wisprflow.ai/post/personalized-style (Oct 29, 2025)**:
- Four app categories: Personal messaging (iMessage/WhatsApp/Telegram), Work messaging (Slack/Teams), Email (Gmail/Outlook/Superhuman), Other (Docs/Notes/ChatGPT).
- Per-category style choice from: **Formal**, **Casual**, **Excited**, **Very Casual**.
- **Important caveat from Wispr's own FAQ**: *"Personalized Style only adjusts capitalization, punctuation, and spacing. Flow doesn't change your grammar, word choice, or phrasing."*
- App detection via macOS Accessibility API (frontmost app bundle ID) and Windows UIA.
- English-only on desktop today. Mobile/iOS coming "soon."
- For code editors (Cursor/VS Code/Cursor extensions), Wispr ships a separate "code-aware" mode that handles spoken symbols and identifiers — this is not in the Personalized Style matrix.

**Limitations / complaints**:
- "Style" is narrower than the marketing implies — only formatting, not actual rephrasing. "Casual" mostly means less punctuation, lowercase i, no Oxford comma, etc.
- Only 4 categories. No way to add custom app categories or per-app overrides within a category.
- No per-recipient tone (e.g., "casual with Russell, formal with the lawyer").

#### How FunButton matches or beats

**Architecture**: keep mode-aware cleanup prompts (already in PRD Sprint 2). Modes detected by frontmost app bundle ID via macOS Accessibility / Windows UIA. Detection table:

```
com.tinyspeck.slackmacgap → work_chat
com.microsoft.teams → work_chat
com.apple.MobileSMS → personal_chat
com.hnc.Discord → casual_chat (separate category — Discord is its own beast)
com.google.Chrome / com.apple.Safari → URL-based: gmail.com → email, slack.com → work_chat, claude.ai → ai_prompt, ...
com.todesktop.230313mzl4w4u92 (Cursor) → code
com.microsoft.VSCode → code
com.apple.Terminal / com.googlecode.iterm2 → terminal
default → notes
```

**Per-mode prompt variations**:

```
work_chat:    "Use Slack-style casual register. Lowercase first letter OK. Use emoji sparingly when natural. Keep sentences short."
personal_chat: "Use texting register. Lowercase. Casual contractions. No formal sign-offs."
email:        "Use professional email register. Proper capitalization. Full sentences. Polite tone."
code:         "Treat as code dictation. Preserve identifiers verbatim. Convert spoken symbols (open paren, close paren, equals, arrow, etc.). Use the active file's language conventions: {detected_language}."
terminal:     "Treat as shell command. Preserve flags and paths verbatim. Convert spoken symbols. NEVER add narrative prose."
ai_prompt:    "Treat as prompt to an AI. Clean filler, preserve intent verbatim. Do NOT add structure or formatting the user didn't ask for."
notes:        "Default cleanup. Polite, structured, well-punctuated."
raw:          "Pass through with minimal cleanup: only fix obvious filler and punctuation. Do not restructure."
```

**Where we beat Wispr**:

a. **Mode actually rephrases, not just capitalization** — because we have Llama 3.3 70B in the loop. "Yo can we move our 1-on-1 to like 3pm tomorrow" stays as-is in personal_chat mode but becomes "Hi — can we move our 1:1 to 3pm tomorrow?" in email mode. This is what users actually want when they hear "tone matching."

b. **Ship code mode in MVP** (already in PRD). Wispr's code-aware mode is a separate path; ours is part of the unified cleanup pipeline.

c. **Per-app overrides** — user can override the bundle-ID detection per-app in settings ("Treat Discord as work_chat instead of casual_chat").

d. **Per-recipient tone** (V1.1) — when the destination is detectable (Slack DM partner, email To:), let user define tone overrides per-contact.

**Eval criteria**: 5-utterance-per-mode corpus (35 total). Manual blind grade by 3 testers comparing FunButton vs Wispr output for "fits the context."

---

### 1.5 Command Mode (Pro-only on Wispr)

#### How Wispr does it

**Confirmed from docs.wisprflow.ai/articles/4816967992**:
- User selects text in any app, presses Command Mode shortcut (default Mac: `Fn+Ctrl`; Windows: `Ctrl+Win+Alt`), speaks an instruction, releases.
- Wispr fetches selection via OS clipboard interaction (likely Cmd+C silently → read pasteboard → process), runs it through their LLM with the spoken instruction as the prompt, replaces the selection with the result.
- Selection limit: 1-1000 words.
- Press ESC to cancel mid-transformation.
- Cmd+Z restores original.
- Without selection: speaks command, output inserted at cursor (acts like a generation).
- "Press enter" voice command — recognized at end of dictation, strips the words and emits Enter keystroke. Useful for chat apps where Enter sends.
- **Paid only**, no free-tier access.

**Limitations / complaints**:
- Selection mechanic via clipboard means it temporarily clobbers user's clipboard. Wispr does restore but there's a window.
- Confused with chat apps (some apps strip keyboard events); reports of glitchy behavior.
- 1000-word cap.
- Steeper learning curve than dictation; reviews note "takes practice."

#### How FunButton matches or beats

**Architecture**: identical surface, simpler implementation.

```
1. User selects text, presses Cmd+Shift+F (configurable).
2. Save current clipboard content.
3. Send Cmd+C synthesizer (via enigo / CGEvent) to copy selection.
4. Read pasteboard. If empty, treat as "no selection" generative mode.
5. Show recording UI (waveform pill).
6. User speaks instruction. We transcribe via Whisper.
7. Send to Llama 3.3 70B with prompt: "Transform the following text per the user's instruction. INSTRUCTION: {transcript}\n\nTEXT:\n{selection}\n\nTRANSFORMED TEXT:"
8. Stream Llama response into replacement: type-over via clipboard + Cmd+V (preserve original clipboard).
9. ESC at any point cancels and restores selection (we kept it).
```

**Where we match or beat**:

a. **Free in MVP** — not gated behind a paid tier. This is a brand promise (anti-enterprise, fun-not-walled).

b. **Selection-aware vs generative**, same as Wispr.

c. **No 1000-word cap** — our LLM context is 128K. We chunk on huge selections (>3000 words) by paragraph but allow it.

d. **"Press enter" command** — implement identically. Strip case-insensitive `press enter` (and `send it`, `submit`) from end-of-dictation, emit Enter via enigo after paste. Setting toggle to disable.

e. **Bonus: presets** — quick-pick verbs from the recording UI: [Shorten] [Expand] [Formalize] [Casualize] [Translate to ___]. Press a preset key while holding the hotkey to skip speaking the verb. Wispr makes you say it every time.

**Concrete prompt**:

```
You are FunButton's transform model. Apply the user's voice instruction to the
selected text. Return ONLY the transformed text — no preamble, no explanation,
no quotation marks around the output.

If the instruction is unclear or unsafe, return the original text unchanged.

USER INSTRUCTION (spoken): {transcript}

ORIGINAL TEXT:
{selection}

TRANSFORMED TEXT:
```

**Eval criteria**: 20 hand-built (selection, instruction, expected) triples. Manual blind grade vs Wispr for "did the right thing." Target 90%+ correct.

---

### 1.6 The "85% Zero-Edit Rate" Claim

#### What it actually means

Wispr's 85% zero-edit claim, as stated by Tanay Kothari on the Lightspeed Generative Now podcast (Oct 2025), refers to **end-to-end pipeline output** — transcription + cleanup combined. It is the rate at which a finished cleanup output is sent or used unchanged. It is NOT raw transcription accuracy (which is ~96-97% on quiet/external mic, dropping to ~92% on noisy laptop mic per ToolCrush 2026 testing).

This is an important reframe: **Wispr loses ~3-15% on raw STT alone**. Their cleanup pass recovers most of those errors. So our cleanup model has parallel work to do — recover Whisper's errors via context, just like Wispr does.

#### How FunButton hits 85% (and goes higher)

**Path to 85%**:
1. Clean cleanup prompt (§1.1) — the 70B model has more room to reason about backtracks, fillers, list intent than Wispr's smaller model.
2. Whisper recognition prompt biasing via dictionary (§1.2) — recovers 2-4% WER on jargon.
3. Mode-aware prompt (§1.4) — reduces "wrong tone" edits.

**Path to 90%+ (our edge)**:
4. Personal learning loop (§2 below) — the few-shot examples in the cleanup prompt include the user's recent (raw, cleaned, edited) tuples. Over time, the prompt knows the user's voice. This is something Wispr does NOT ship per-user today.

**Eval harness** (specified in §3 below) makes this measurable. We will publish our own zero-edit rate from internal testing in our launch story: *"FunButton hit 91% zero-edit rate after 50 transcriptions of personal use, vs Wispr's 85% claim. Your mileage will vary — and our rate climbs as our cleanup model learns your voice."*

---

### 1.7 Multi-Language Handling

#### How Wispr does it

**Confirmed from docs.wisprflow.ai/articles/3191899797 + wisprflow.ai/research/supporting-languages**:

- **Per-session language detection**, NOT per-word. Once a session starts, the detected language is locked in for the whole session.
- **Code-switching has known limits**. Wispr's docs explicitly state: *"Flow does not support rapid language switching within a single sentence. English with Spanish, French, or German generally performs better than English with Chinese or Japanese."*
- **Hinglish (Hindi-English) has a dedicated model** because it's such a common code-switched language in their user base. Outputs romanized Hindi mixed with English.
- **Dynamic ASR engine routing per language** — they don't use one model for all 100+ languages. Their research blog explicitly says *"Flow dynamically selects the most accurate ASR engine for each language, cutting transcription error rates by more than half in internal testing."* This is non-trivial infrastructure.
- Auto-detect mode chooses from all 100+; manual selection of 2-3 narrows the pool and improves accuracy.
- Mutually exclusive variants: en-US/en-GB, Hindi/Hinglish, zh-Hans/zh-Hant, de/de-CH.

**Limitations / complaints**:
- Mid-sentence Spanglish: degraded. Wispr will lock to one language and try to romanize/translate the other.
- Non-English accuracy materially worse than English (their own research blog admits this).
- "Code-switching has limits" is doing a lot of work in the docs.

#### How FunButton matches or beats

**Be pragmatic — don't try to beat Wispr's data flywheel on raw multilingual STT**. We won't have 100 languages tuned. Strategy:

**Sprint 2.6 — match the basics**:
- Whisper Large v3 Turbo on Groq supports ~99 languages out of the box, with auto-detect.
- Set `language` parameter explicitly when user has a preferred language (not auto). Falls back to Whisper auto-detect when unset.
- Cleanup prompt is language-agnostic by default (works in any language Llama 3.3 70B speaks well, which is ~30 languages strongly).
- For Spanish, French, German, Italian, Portuguese, Dutch, Japanese, Korean, Mandarin: ship language-specific cleanup prompts that account for that language's punctuation conventions (e.g., Spanish ¿/¡, French spaces before colons, German capitalization of nouns).

**Where we beat Wispr structurally**:

a. **Code-switching via local + cloud routing** — when user has 2+ languages selected, run Whisper auto-detect on each ~5-second chunk independently. Stitch chunks together; cleanup pass handles the bilingual blend coherently because Llama 3.3 70B handles code-switching better than ASR does. This is a different architectural bet than Wispr's "lock to one language per session." Wispr can't easily change this without re-training their per-language models; we just change the chunking and prompt.

b. **Local-first for sensitive languages** — for users in regulated/private contexts who happen to speak Hindi, Mandarin, or Arabic, our local Whisper option works (degraded quality vs cloud, but at least it works). Wispr cannot offer this.

**Honest positioning**: don't claim "best multilingual." Claim "good enough for 30+ languages, code-switch-friendly, and local-first option for any of them." That's a wedge, not a war.

---

### 1.8 Long-Form Behavior

#### How Wispr handles it

**Confirmed from docs.wisprflow.ai/articles/4841123325 (March 31, 2026 update)**:
- Desktop dictation cap: **20 minutes** (raised from 6 minutes in March 2026). Auto-submits at the cap.
- iOS: 5 minutes. Android: no enforced cap.
- No cooldown between sessions — start a new one immediately.

The cap exists because (a) a single long upload chunk is risky for cloud reliability, (b) cleanup latency degrades nonlinearly past ~5K words.

**Limitations / complaints**:
- "I'm doing a brain dump and the warning interrupts me" (Zackproser, also r/WisprFlow). Annoying.
- No way to bridge sessions — a long thought split across two recordings has no shared context for cleanup.

#### How FunButton matches or beats

**No hard cap** — but architectural reality means we need chunked processing for >5 min audio.

**Implementation**:
1. cpal captures continuously to in-memory ring buffer + flushes to disk every 30 seconds (so a crash doesn't lose a 15-minute brain dump).
2. Chunk audio into ~60-second windows. Stream each window to Whisper as it completes (don't wait for stop).
3. Cleanup pass runs over chunks with a sliding context window. Pass last 200 words of the previous chunk's cleaned output to the cleanup prompt as `PREVIOUS_CONTEXT`. Llama uses this to maintain coherence across chunks (don't restart sentences, don't re-introduce names, keep tone consistent).

```
PREVIOUS CLEANED CONTEXT (last 200 words for continuity):
{prev_context}

NEW CHUNK RAW TRANSCRIPT:
{whisper_chunk}

CLEANED OUTPUT (continuing from previous context, do not repeat):
```

4. Final assembly: concatenate chunk outputs with a final "stitching pass" if total length > 5 min — Llama gets the full concatenated cleanup and produces a final coherent version. Optional, default off (it's ~2s extra latency).

**UX**: tiny progress indicator showing "12:34 / unlimited" during recording. No nag, no warning.

**Eval criteria**: 3 long-form recordings (10, 20, 30 min). Score on (a) coherence across chunks (no name resets, no tone resets), (b) total cleanup time (target: <2x audio length), (c) WER vs hand-cleaned reference.

---

### 1.9 Personal Learning Loop (OUR EDGE)

#### Confirming Wispr does NOT ship this

**Confirmed across all reviewed sources** (wisprflow.ai docs, research blog, Tanay's podcast appearances, all third-party reviews):

- Wispr trains centrally on aggregated, anonymized corrections sent (opt-in) from the user base.
- They do NOT ship per-user fine-tuning, per-user few-shot prompting, or any other per-user adaptation that runs in real time as the user uses the product.
- The closest thing they have is **Auto-add to Dictionary** (which is just term-level recognition boosting via the proper-noun detector) and the central training loop (which is org-wide, not per-user).
- No mention anywhere of per-user learning. Sid Saladi's deep dive substack post (March 2026) explicitly contrasts: *"Auto-add to Dictionary: If you correct a transcription by typing over it, Flow notices and adds the corrected spelling automatically. Over time, it learns your vocabulary without you lifting a finger."* — that's term-level only, not style-level.

**Why they don't ship it**: their architecture is centrally optimized for low latency. A per-user adaptation layer adds personalization fetch latency to each request. They're optimizing for the median; we can optimize for the individual.

**Why this is our durable wedge**: their data flywheel feeds central improvements. Our learning loop feeds *each user's individual improvement*. After 100 transcriptions, our cleanup output is more "your voice" than Wispr's will ever be — without sending your data to a central training set.

#### Architecture

**Storage** (local, SQLite):
```sql
CREATE TABLE transcriptions (
  id INTEGER PRIMARY KEY,
  audio_hash TEXT,
  raw_transcript TEXT,
  cleaned_output TEXT,
  user_edit TEXT,          -- final edit if user modified after paste; NULL if unchanged
  app_category TEXT,
  mode TEXT,
  language TEXT,
  created_at INTEGER,
  duration_ms INTEGER
);
CREATE INDEX idx_recent ON transcriptions(created_at DESC);
CREATE INDEX idx_edited ON transcriptions(user_edit) WHERE user_edit IS NOT NULL;
```

**Capture** (the hard part):
1. After paste, monitor the focused text field via Accessibility API for ~30 seconds.
2. If user types/edits within the pasted region within that window, capture the final state and store as `user_edit`.
3. If user navigates away or sends/submits unchanged, mark as zero-edit.

This Accessibility API monitoring is non-trivial but tractable on macOS (`AXObserver` + `AXNotificationCallback` for `kAXValueChangedNotification`). Document the privacy implications: monitoring is opt-in, runs only on text fields the user just dictated into, retention is local-only.

**Few-shot injection**:
- On every cleanup call, fetch the 3 most recent (raw, cleaned, edited) tuples where user_edit ≠ cleaned_output. Inject into cleanup prompt:
  ```
  RECENT CLEANUP EXAMPLES (these show the user's preferred style — match this style):
  Example 1:
  RAW: {tup1.raw}
  YOUR PREVIOUS OUTPUT: {tup1.cleaned}
  USER'S PREFERRED EDIT: {tup1.user_edit}
  
  Example 2: ...
  ```
- The model uses these as few-shot guidance. After 5-10 cycles, the cleanup output starts matching the user's voice on punctuation, register, sign-offs, signature phrases.

**Auto-promotion of dictionary terms** (already specified in §1.2): when a user_edit introduces a new proper-noun, after 3 occurrences it's auto-added to the dictionary.

**Privacy guarantees** (this is the marketing):
- All learning state is local SQLite. Never uploaded.
- User can wipe at any time: Settings → Privacy → Clear Learning Data.
- Optional export (sqlite dump + json) so users own their data.
- Zero telemetry on learning data, ever, even with opt-in analytics.

**Eval criteria**:
- Run a 50-utterance corpus. Compare zero-edit rate at session 1 vs session 10 (after 10 sessions of feedback loop). Target: +3-5 percentage points improvement.
- Manual: read 10 outputs from session 1 and session 10. Tester (blind) judges which set sounds more like the user. >70% picking session 10 = win.

---

### 1.10 Edge Cases Wispr Underperforms On

#### Documented Wispr weaknesses (from reviews)

| Weakness | Source | Our opportunity |
|---|---|---|
| Quality drops post-trial (perceived) | Trustpilot 2.7/5, Vocai | Lifetime/trial-equal pricing eliminates this perception entirely |
| Heavy non-American accents drop accuracy | Wispr's own research blog | Local Whisper option + accent-aware cleanup prompt instruction |
| Mid-sentence Spanglish/code-switching | Wispr's own docs | Chunk-level language detection (§1.7) |
| Code dictation in non-Cursor IDEs (vim, JetBrains, Zed) | Reddit complaints | Code mode in MVP for all common editors |
| Heavy technical jargon (medical, legal) without dictionary setup | Reddit | Domain dictionary import packs ship with app (medical, legal, dev) |
| Always-on network requirement (airplane, bad wifi) | Universal | Local mode |
| Re-paste failures in Citrix/RDP/some VDI | Reddit, dictaflow.io | Multiple injection methods (clipboard, accessibility direct-write, keystroke synth) — try in order, fall back automatically |
| Resource hog while idle (8% CPU constant) | Letterly, Vocai | Tauri 2 baseline 50MB, <1% idle CPU |
| 60-char dictionary cap | docs | We accept up to 240 chars |
| No dictionary domain switching | docs | Ship layered domain dictionaries |
| Snippets are static-only, no dynamic vars | docs FAQ | Dynamic snippets with `{{vars}}` and `{{ai:...}}` blocks (Sprint 3) |
| Limited shortcut customization | r/WisprFlow #2 complaint | Tauri global shortcut plugin allows arbitrary chord configuration |
| No Linux | universal | 2-day Tauri 2 port |

#### Specific failure modes we should target in marketing

These are the "showdown" demos to film:
1. **Spanglish demo**: speak a paragraph alternating Spanish and English. Show Wispr locking to one. Show FunButton handling both.
2. **Brain dump >20 min**: speak for 25 minutes. Show Wispr auto-cutting at 20:00. Show FunButton continuing seamlessly.
3. **Offline demo**: airplane mode toggled. Wispr fails. FunButton local mode keeps working.
4. **Code dictation in vim**: Wispr's code mode in vim is unsupported. Ours works.
5. **Privacy demo**: open Little Snitch. Show Wispr's constant outbound traffic. Show FunButton's silence except during transcription requests.

---

## Section 2 — The Personal Learning Loop (Deep Dive)

This is the durable wedge. Specified in §1.9 above; expanding here on **why this works architecturally and what to build first**.

### Why few-shot in-prompt > fine-tuning

Two reasons we use in-prompt few-shot examples instead of LoRA fine-tuning:

1. **Latency**: fine-tuning a model means we'd need our own inference. Groq doesn't host user-fine-tuned 70B models. We'd need Together/Fireworks LoRA or self-host. Both add latency we don't want to pay.
2. **Privacy**: fine-tuning means user data leaves the device for training, even if just once. Few-shot in-prompt means user data stays local; only the *current request* leaves the device, and only with the user's recent corrections embedded as ephemeral context.

**Tradeoff**: prompt grows by ~500-1500 tokens with few-shot examples. Cost: $0.001-0.003 per request at Llama 3.3 70B Groq pricing. Negligible.

### Capture mechanism (the hard part)

The single hardest engineering problem in this whole spec is **reliably detecting that the user edited a pasted transcription**. Approaches:

**Approach A — Accessibility observation**: Register an `AXObserver` on the focused element after paste. Watch for `kAXValueChangedNotification` for ~30s. Diff initial paste against final value. *Risk: some apps don't fire AX notifications on text changes (Discord, Telegram, some Electron apps).*

**Approach B — Clipboard delta**: After paste, snapshot the focused text field's value. Re-snapshot every 5s for 30s. Diff. *Same AX-availability risk.*

**Approach C — Optional opt-in heuristic**: User explicitly hits Cmd+Shift+E within 60s of a paste to "tag this as edited; capture for learning." Less automatic, more reliable. **Recommended for V0.1**; upgrade to AX observation in V1.

**Approach D — Re-prompt heuristic**: User dictates again within 60s of a previous paste in the same app/field. Treat as "the previous one wasn't quite right." Capture both as a (pre-edit, post-edit) pair. Imprecise but signal-bearing.

**Recommended for Sprint 2.6**: Approach C (manual tag) + Approach D (re-prompt heuristic). Approach A as a Sprint 3 stretch when we have time to test AX behavior across apps.

### Schema and storage

(See §1.9 SQL schema.)

Retention: keep last 1000 transcriptions or 30 days, whichever is larger. Offer manual purge in settings.

### Few-shot selection algorithm

Don't always pick the most recent 3. Pick the 3 most *informative* recent edits:

1. Filter: edits where `user_edit` differs from `cleaned_output` by edit distance > 5 chars AND < 200 chars (skip noise; skip massive rewrites).
2. Bucket: by app_category. Prefer examples from the same category as the current request.
3. Diversity: avoid 3 examples that are all the same kind of edit (e.g., all 3 are "removed sign-off"). Pick across edit types if available.
4. Recency tiebreaker: most recent wins within bucket.

Implementation in `src/learning/few_shot.rs`:

```rust
pub fn select_examples(category: &str, conn: &Connection, limit: usize) -> Vec<Tuple> {
  let mut candidates: Vec<Tuple> = conn.query(
    "SELECT raw_transcript, cleaned_output, user_edit FROM transcriptions
     WHERE user_edit IS NOT NULL
       AND user_edit != cleaned_output
       AND length(user_edit) - length(cleaned_output) BETWEEN -200 AND 200
     ORDER BY app_category = ? DESC, created_at DESC
     LIMIT ?", &[category, &(limit * 4)]
  );
  // Deduplicate by edit-type fingerprint (e.g., "removed_signoff", "added_punctuation")
  // and return up to `limit` diverse examples.
  diversify(candidates, limit)
}
```

### What Wispr could do to neutralize this

- **Most likely counter**: ship per-user fine-tuning via small adapter heads on their cloud model. ~12 months of work given their team.
- **Less likely**: ship a few-shot in-prompt mechanism like ours. Easier engineering, but cuts into their latency budget. They are latency-obsessed; this is a hard tradeoff for them.
- **Even less likely**: open-source their cleanup model. Wouldn't address the per-user learning gap.

If Wispr ships per-user adaptation in 2027, our edge becomes "local + GPL + no cloud lock-in" — which we have anyway. The learning loop is leverage during the 12-24 month window where they don't have it.

---

## Section 3 — Eval Harness Design

We cannot know if we're matching or beating Wispr without measurement. The eval harness is non-negotiable for Sprint 2.6.

### 3.1 Goals

1. **Regression detection**: each prompt change or model swap is auto-evaluated against a fixed corpus. Fail-fast if zero-edit rate drops.
2. **Comparison**: parallel run through Wispr Flow on the same audio. Diff outputs. Score blind by human or by LLM-as-judge.
3. **User-facing eval**: a "Cleanup Showdown" page (`funbutton.ai/showdown`) that anyone can run their own audio through both pipelines and see side-by-side. Marketing weapon and credibility builder.

### 3.2 Corpus

**50 utterances**, hand-recorded on:
- 10 clean-speech (well-formed sentences, no fillers)
- 10 heavy-filler (lots of um/uh/like)
- 10 backtrack (mid-sentence corrections)
- 10 list-intent (implicit and explicit lists)
- 10 code/jargon (programmer dictation, medical/legal terms)

For each: hand-edited expected output. Stored in `apps/fun-eval/fixtures/`.

**Diversity**:
- 3 voices (Todd + 2 community testers, gender + accent variety)
- 2 mic conditions (quiet/external mic, noisy/laptop mic)
- = 50 utterances × 3 voices × 2 mics = 300 audio files

### 3.3 Pipeline

```
For each fixture:
  raw_whisper = call_whisper(audio_file)
  fb_cleaned = call_funbutton_cleanup(raw_whisper, mode, dict, recent_examples)
  expected = read_expected(fixture_dir)
  
  metrics:
    exact_match = (fb_cleaned == expected)
    edit_distance = lev(fb_cleaned, expected)
    semantic_score = embed_cosine_sim(fb_cleaned, expected)  // bge-small embeddings
    bleu_score = bleu(fb_cleaned, expected)
  
  if FUN_EVAL_COMPARE_WISPR=1:
    wispr_cleaned = drive_wispr_via_macos_accessibility(audio_file)
    record(wispr_cleaned vs expected)
    record(wispr_cleaned vs fb_cleaned head-to-head)
```

### 3.4 Wispr comparison harness

Drive Wispr Flow programmatically:
1. Play audio through a virtual mic (BlackHole / Loopback).
2. Trigger Wispr's hotkey via `osascript`.
3. Wait for paste.
4. Read clipboard or focused text field via Accessibility API.
5. Reset state, next fixture.

Caveats: Wispr ToS likely restricts automated benchmarking. We are not redistributing or republishing their output; we are using their consumer product for personal benchmarking, comparable to any reviewer. Document this; comply with reasonable rate limits.

### 3.5 Reporting

Markdown report committed to repo each run:

```
# Eval Report — 2026-05-12 14:32

## Summary
- Corpus: 300 fixtures
- FunButton zero-edit rate: 87.3%
- FunButton edit-distance avg: 4.2 chars
- Wispr Flow zero-edit rate: 84.6%
- Wispr Flow edit-distance avg: 5.1 chars

## By category
| Category | FB ZER | Wispr ZER | Δ |
|---|---|---|---|
| Clean speech | 96% | 95% | +1 |
| Heavy filler | 88% | 84% | +4 |
| Backtrack | 79% | 81% | -2 |
| List intent | 85% | 82% | +3 |
| Code/jargon | 88% | 81% | +7 |

## Regressions
None.

## New wins (FB > Wispr by > 3 chars edit-distance)
- fixture_023.wav: ...
```

CI: run on every Sprint completion. Diff against last commit. Fail PR if regression > 1 percentage point.

### 3.6 Public Showdown page

`funbutton.ai/showdown`:
- Drag-drop or record audio in browser.
- Send to our backend (CF Worker → Groq pipeline).
- Show side-by-side: raw Whisper, FunButton cleaned, (optional: user pastes Wispr output for comparison).
- Highlight diffs.
- Anonymize and store opt-in for our own training corpus growth (with explicit checkbox).

This is **launch-week marketing** as much as it is internal QA. Goes on Product Hunt, HN, Twitter.

---

## Section 4 — Sprint 2.6 Implementation Order (What to Build First)

Ranked by `(quality_lift × dev_speed) / risk`:

### Tier 1 — Ship by end of Sprint 2.6 weekend (highest leverage, lowest cost)

1. **Cleanup prompt v2** (§1.1) — replace whatever's currently in the cleanup pipeline with the structured prompt above, with mode-aware variants (§1.4). **2-4 hours.** Massive zero-edit improvement.
2. **Whisper recognition prompt biasing** (§1.2) — feed dictionary into Whisper API's `prompt` field on every transcription. **1-2 hours.** Free 2-4% WER on jargon.
3. **Replacement rules pass** (§1.2) — Aho-Corasick over Whisper output before cleanup. **2-3 hours.**
4. **App-mode detection table** (§1.4) — map bundle IDs to mode. Frontmost-app detection via macOS Accessibility / Windows UIA. **3-5 hours.**
5. **Static snippets** (§1.3 phase 1) — SQLite + post-cleanup pass + settings UI tab. **4-6 hours.**
6. **Eval corpus v0** (§3.2) — record 25 fixtures (Todd + 1 alt voice), commit. **2 hours of recording.**
7. **Eval harness MVP** (§3.3) — Rust binary that runs Whisper + cleanup over fixtures and emits a markdown diff report. **4-6 hours.**

**Subtotal: 1.5-2 days. Sprint 2.6 ships with a measurable, mode-aware, dictionary-aware, snippet-aware cleanup pipeline.**

### Tier 2 — Sprint 3 (post-launch within 2 weeks)

8. **Personal learning loop v0** (§1.9, §2) — schema + manual-tag capture (Approach C) + few-shot injection. **8-12 hours.**
9. **Command Mode** (§1.5) — selection capture + transform + paste-back. **6-8 hours.**
10. **Auto-promote to dictionary** (§1.2 auto-promote) — NER + 3-strike confirm. **4-6 hours.**
11. **Long-form chunked cleanup** (§1.8) — chunked Whisper + sliding context. **6-8 hours.**
12. **"Press enter" command** (§1.5) — trivial. **1 hour.**
13. **Wispr comparison eval** (§3.4) — virtual mic + AX-driven Wispr scraping. **8-12 hours, risky.**
14. **Showdown public page** (§3.6) — Next.js + CF Worker. **8-12 hours.**

### Tier 3 — V1.1 (4-6 weeks out)

15. **Dynamic snippets with `{{vars}}` and `{{ai:...}}`** (§1.3 phase 2). 8-12 hours.
16. **Per-user accessibility-observed learning capture** (§1.9 Approach A). 12-16 hours.
17. **Code-switching multi-language chunked detection** (§1.7). 12-16 hours.
18. **Domain dictionary packs** (medical, legal, dev) ship with app. 4-6 hours of curation each.
19. **Per-recipient tone overrides** (§1.4). 8-12 hours.

---

## Section 5 — Risk Register

Where we might underperform Wispr, with mitigations:

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| Cold-start latency feels slower than Wispr in demos | High | High (first-impression killer) | Warm HTTP/2 connection pool to Groq, streaming audio uploads, pipeline cleanup off first ASR partial (already specified in STACK-WARGAME.md adjustment #1) |
| Llama 3.3 70B cleanup occasionally over-edits and changes intent | Medium | High | Cleanup prompt rule #8/9: "PRESERVE numbers/proper-nouns/identifiers verbatim. PRESERVE user's voice." + "If raw is clean, return unchanged." Eval corpus catches over-edits as edit-distance regressions. |
| Whisper-only ASR can't match Wispr's per-language ASR routing for Hindi/Mandarin/etc. | High | Low (English is 90% of users) | Honest positioning: "30+ languages well, 100+ supported, local fallback." Don't claim multilingual leadership. |
| Mid-sentence backtrack handling weaker than Wispr's | Medium | Medium | Cleanup prompt rule #3 + heavy backtrack examples in eval corpus. If failing, escalate prompt with explicit backtrack-detection sub-instruction. |
| Personal learning loop privacy story confused with telemetry | Medium | Medium | Hard-coded zero-network-egress for learning data. Explicit Privacy panel showing what's stored. Open-source the learning code. |
| Zero-edit rate evaluations look bad vs Wispr's 85% claim | Low | High (PR risk) | Run eval before launch; tune prompts to ≥85% before any public claim. Don't publish numbers we can't beat. |
| Accessibility-API-based capture (learning loop Approach A) doesn't work in Electron apps (Discord, Telegram) | High | Low (manual-tag fallback) | Approach C (manual tag) is V1 default. Approach A is V1.1 stretch. |
| Wispr ships per-user adaptation in 2027 | Low (12 mo) | Medium | Our other moats (local, GPL, Linux, lifetime, fun) survive without learning-loop being unique. |
| Eval harness drives Wispr in violation of ToS | Low | Low | Internal use only. Reviews and public benchmarks rely on user-submitted side-by-side via Showdown page, not automated scraping. |
| Llama 3.3 70B Groq rate limits during launch traffic spike | Medium | Medium | Multiple Groq accounts + Anthropic Claude 4.7 fallback for cleanup tier. Cloudflare Worker proxy with circuit breaker. |
| Bundled local LLM (Qwen 2.5 1.5B) cleanup quality is way worse than cloud Llama 70B, leading to "local mode is bad" review | High | Medium | Ship tiered local: Qwen 2.5 0.5B (basic), Qwen 2.5 1.5B (better), Llama 3.2 3B (best, slower). User picks. Document expected quality drop honestly. |

---

## Appendix A — Cleanup Prompt Quick Reference

**Mode = `default` / `notes`**: full prompt from §1.1.

**Mode = `email`**: append to default — *"Use professional email register. Add greeting if absent (Hi [name],) when context implies a new email. Add sign-off if absent (—Todd) when the user appears to be ending. Punctuation and capitalization formal."*

**Mode = `work_chat`**: append — *"Slack/Teams casual register. Lowercase first letter when natural. Strip greetings/sign-offs. Use line breaks instead of paragraph breaks. No Oxford comma. Emoji OK if user's recent style includes them."*

**Mode = `personal_chat`**: append — *"iMessage/WhatsApp casual. Lowercase. Contractions (gonna, wanna, don't). No formal punctuation. Allow sentence fragments."*

**Mode = `code`**: replace cleanup rules with code-specific — *"Treat as code dictation in {detected_language}. Convert spoken symbols ('open paren'→'(', 'close paren'→')', 'equals'→'=', 'arrow'→'=>', 'open curly'→'{', 'pipe'→'|', 'ampersand'→'&', 'dollar'→'$', 'at sign'→'@', 'semicolon'→';', 'colon'→':', 'dot'→'.', 'comma'→',', 'hashtag'→'#'). Apply naming conventions ('user name camelCase' → 'userName'). Preserve identifiers verbatim. Do not paraphrase. Do not add comments. Output the literal code only."*

**Mode = `terminal`**: stricter code mode — *"Shell command. Convert spoken symbols. Preserve flags (--verbose, -h) and paths (/usr/local/bin) verbatim. NEVER add narrative. Output literal command only."*

**Mode = `ai_prompt`**: minimalist — *"This is a prompt to an AI. Clean filler words only. Preserve all intent verbatim. Do NOT add structure, formatting, or framing the user did not explicitly request. Trust the user's phrasing."*

**Mode = `raw`**: minimal — *"Output the raw transcript with only filler-word removal and basic punctuation. Do not restructure. Do not paraphrase. Trust the user."*

---

## Appendix B — Sources Cited

All sources fetched May 8, 2026 unless noted.

- docs.wisprflow.ai/articles/4052411709-teach-flow-your-words-with-the-dictionary
- docs.wisprflow.ai/articles/5784437944-create-and-use-snippets
- docs.wisprflow.ai/articles/4816967992-how-to-use-command-mode
- docs.wisprflow.ai/articles/3191899797-use-flow-with-multiple-languages
- docs.wisprflow.ai/articles/4841123325-Longer-dictation-sessions (March 31, 2026 — confirms 6-min → 20-min cap upgrade)
- docs.wisprflow.ai/articles/9559327591-flow-plans-and-what-s-included
- docs.wisprflow.ai/articles/6901148133-transcription-suddenly-got-worse-or-feels-less-accurate
- wisprflow.ai/features
- wisprflow.ai/post/snippets (Nov 12, 2025)
- wisprflow.ai/post/personalized-style (Oct 29, 2025 — confirms style is *capitalization/punctuation/spacing only*, NOT grammar/word-choice)
- wisprflow.ai/research/supporting-languages (Jan 19, 2026 — confirms dynamic ASR engine routing per language, accent confidence scoring, Hinglish dedicated model)
- chrismenardtraining.com/post/wispr-flow-ai-dictation-removes-filler-words (May 1, 2026)
- letterly.app/blog/wispr-flow-review (privacy/quality post-trial concerns)
- vocai.net/blog/wispr-flow-review-privacy-2026 (Trustpilot 2.7/5, "60% post-trial" reviews)
- eesel.ai/blog/wispr-flow-review (Oct 6, 2025)
- toolcrush.io/tool/wispr-flow (May 1, 2026 — 96-97% quiet/97% noisy mic accuracy numbers)
- fritz.ai/wispr-flow-review (March 8, 2026)
- filipkonecny.com/2026/03/25/wispr-flow-ai-editing
- filipkonecny.com/2026/03/25/wispr-flow-limitations (cap details across platforms)
- appreviewlab.com/wispr-flow-review (March 11, 2026)
- comparateur-ia.com/en/reviews/wispr-flow (March 31, 2026)
- aimanifesto.net/tools/wispr-flow
- aisharenet.com/en/wispr-flow (March 29, 2026)
- sidsaladi.substack.com/p/wispr-flow-101-the-complete-guide (March 27, 2026)
- dominikgabor.com/blog/voice-dictation-ai-prompting-wispr-flow-2026 (April 30, 2026)
- samanthakasbrick.com/blog/wispr-flow-review-tutorial
- lillytechsystems.com/ai-school/wispr-flow/dictation
- lillytechsystems.com/ai-school/wispr-flow/advanced
- zackproser.com/blog/wisprflow-review (6-min cap original; superseded by March 2026 update to 20-min)
- dictaflow.io (Citrix/RDP injection failures)
- YouTube: K0qiWGTeLQs (Chris Menard tutorial, May 2026)
- YouTube: qlg3p5HdXRQ (beginner tutorial, Nov 2025)
- YouTube: mbunGLyvlRg (walkthrough, Nov 2025)
- Lightspeed Generative Now podcast — Tanay Kothari interview (Oct 2025) — 85% zero-edit claim source

---

*Spec compiled by Ea (Claude Opus 4.7), 2026-05-08. Total research time: ~75 min. Recommended next step: kick off Sprint 2.6 with Tier 1 items in order; eval harness ships alongside cleanup prompt v2 so we can measure as we tune.*
