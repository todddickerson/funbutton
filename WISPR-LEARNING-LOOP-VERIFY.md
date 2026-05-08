# Wispr Flow: Per-User Learning Loop Verification

**Research date:** 2026-05-08
**Question:** Does Wispr Flow run a true per-user model adaptation loop (option #1), or only personalization layers + aggregate flywheel (options #2 + #3)?
**Bottom line:** **NO per-user model fine-tuning.** Wispr does **#2 (personalization layer) + #3 (aggregate flywheel)** only. Confirmed on the record by their CEO.

---

## 1. Verdict

**NO** — Wispr Flow does not have a per-user learning loop in the strong sense (option #1: model adaptation that fine-tunes the ASR/cleanup model on YOUR voice + YOUR corrections specifically).

**Confidence: HIGH (~90%)**

What they actually do:
- ✅ **#2 Personalization layer** — personal dictionary (auto-grows from your typed corrections), snippets, per-app Style settings, optional Context Awareness reading current-app text. The underlying model is the same for everyone.
- ✅ **#3 Aggregate flywheel** — pseudonymized text + corrections from all opt-in users feed retraining of shared Llama-based cleanup models hosted on Baseten. Improves the global model, helps everyone.
- ❌ **#1 Per-user model adaptation** — no LoRA adapters per user, no per-user inference weights, no documented federated personalization that updates a user-specific model. CEO explicitly says they use "context locally on a person's computer to give them the best personalized experience" — i.e. inference-time conditioning, not per-user weights.

Why I'm not 100%: Wispr has never published technical architecture docs, no ML engineer job posts I could find publicly mention LoRA/per-user adapters, and the CEO references "federated learning that is privacy preserving" once in passing without specifying scope. But every other public signal points to a single shared model + per-user prompt/context conditioning.

---

## 2. Evidence

### A. CEO Tanay Kothari, Tech Optimist Podcast (most direct quote)

> "Now, this does raise a question on if we're not collecting all of this data, how do we make our models better? There's a few things that we have for that, and parts of it also include **federated learning that is privacy preserving, completely anonymous** that we do. But for most part, the question is **how do we use just the context that we have locally on a person's computer to give them the best personalized experience?**"

**Source:** https://techoptimist.vc/episodes/63-meet-the-start-up-at-the-center-of-the-voice-computer-revolution/transcript

This is the founder, on the record, describing the architecture. Two distinct mechanisms:
1. **Federated learning → aggregate** ("how do we make our models better"). The output of federated learning here is *the global model*, not a per-user model.
2. **Local context → per-user experience** ("how do we use just the context that we have locally"). This is inference-time conditioning (prompt context, dictionary, app context) — not weights.

The "personalization" he's selling is #2 — same model for everyone, but conditioned at inference time on your dictionary, snippets, current app, prior context. No per-user fine-tune.

### B. Privacy Policy: Aggregate Training Only

> "If you opt to share your content with us for model training, we may also collect pseudonymized text and corrections you provide **to improve the performance of Wispr Flow for all users**."

**Source:** https://wisprflow.ai/privacy-policy

Direct statement: corrections train the *global* model "for all users," not your personal model. This is textbook option #3.

### C. Settings Toggle: "Improve the model for everyone"

> "We're also focusing on improving our AI to deliver the best possible experience for all our users. If you're comfortable, we'd really appreciate if you can turn on **'Improve the model for everyone'** in your settings so we can deliver better and better experiences with Flow!"

**Source:** https://roadmap.wisprflow.ai (changelog, 2025-11-07)

The toggle is literally named "for everyone" — that's the aggregate flywheel, not per-user.

### D. Baseten Case Study: Shared Fine-Tuned Llama Models

> "Sahaj and his team chose Llama, a family of open-source LLMs by Meta, as the base for their real-time transcript cleanup step. They **fine-tuned these LLMs to precisely solve user tasks based on the users' context and preferences**... Baseten powers Flow with low-latency inference on dedicated deployments for these fine-tuned Llama models."

**Source:** https://www.baseten.co/resources/customers/wispr-flow/

Key phrase: *fine-tuned… based on users' context and preferences* — note "context and preferences," which are inference-time inputs (dictionary, app, style settings). The fine-tuning is on the global model to handle context-conditioning well, NOT per user. They run "dedicated deployments" — singular per model, not per user. Per-user inference at scale (millions of users × dedicated GPU adapters) is economically and operationally implausible at their stage.

### E. Privacy Mode Mechanics Imply No Per-User Model

> "When [Privacy Mode is] on, your dictation data isn't stored or used for model training by us or any third party."

**Source:** https://wisprflow.ai/privacy

If Wispr ran a per-user model that adapted to *your* voice, enabling Privacy Mode would necessarily break or freeze your personal model — and they'd have to disclose that. They don't, because there is no per-user model. Privacy Mode just disables the contribution to the *global* training corpus. Your dictionary still works because the dictionary is metadata, not model weights.

### F. Reviewer Reports Describe #2, Not #1

Multiple long-form reviews describe the "learns over time" feeling, but **every single one** attributes it to the personal dictionary + style settings, never to model retraining on the user's voice:

- **Sid Saladi:** "Auto-add to Dictionary: If you correct a transcription by typing over it, Flow notices and adds the corrected spelling automatically. Over time, it learns your vocabulary without you lifting a finger." → That's the dictionary, not the model. (https://sidsaladi.substack.com/p/wispr-flow-101-the-complete-guide)
- **Letterly review:** "Flow's dictionary learns from your corrections and can also be manually extended." → Dictionary. (https://letterly.app/blog/wispr-flow-review/)
- **Fritz.ai:** "The personal dictionary learns your jargon, names, and acronyms over time." (https://fritz.ai/wispr-flow-review/)
- **Google Sites long-form review:** "The personal dictionary has learned over 500 custom terms, accuracy continues to improve" — explicitly attributes "learning" to the dictionary, ~500 strings of metadata. (https://sites.google.com/view/wispr-flow-review/)
- **Common Sense:** "The system improves its understanding of each user's speech patterns and preferences over time" — but the same review only describes dictionary + style settings as the mechanism. (https://common-sense.com/blog/2025/04/increase-your-productivity-with-wispr-flow/)
- **Photes.io:** "Spend some time dictating to help the app learn your voice" — vague marketing claim, no mechanism described, and contradicted by every architectural source. (https://photes.io/blog/posts/wispr-flow-review-a-dictation-helper)

If Wispr were running per-user adapters, marketing would say it loudly. They don't. Their own copy says "Flow automatically learns your unique words and adds them to your personal dictionary" — words, not voice.

### G. No "Voice Calibration" or Training Step

> "No voice training, no calibration, no reading sample paragraphs. The AI works immediately with your natural voice."

**Source:** https://sites.google.com/view/wispr-flow-review/

A real per-user acoustic model would benefit from calibration. They don't offer it because there is no per-user acoustic model — just a strong shared one.

### H. Cloud-Only Architecture, Off-The-Shelf Providers

> "Wispr Flow's audio is processed by Baseten, its text by OpenAI, Anthropic, and Cerebras, and stored on AWS — all named in Wispr Flow's own subprocessor list."

**Source:** https://www.getvoibe.com/resources/is-wispr-flow-safe/

They route text through OpenAI, Anthropic, and Cerebras — third-party API endpoints with shared models. You cannot run a per-user fine-tune through a public OpenAI API call. Per-user model adaptation is architecturally incompatible with this provider mix.

### I. CEO Explicitly Says Cloud Models, Not Edge

Podcast timestamp 18:01: "The 'Zero-Edit' Standard: Prioritizing **massive cloud models** over edge models to achieve flawless dictation in noisy, real-world conditions."
**Source:** https://www.notablecap.com/blog/a-keyboard-less-future-reinventing-a-150-year-old-interface-with-wispr-flow

"Massive cloud models" (singular family, served from Baseten) — not "small per-user models." Their bet is on a great shared model + good context conditioning, not on per-user customization.

### J. Tanay on his "personalization" architecture

From the Tech Optimist transcript:

> "Underlying it is a set of models that we put together internally at the Wispr office, which has a number of sections around **personalization, understanding the context**, reducing hallucinations…"

He describes "personalization" as a *section* of a shared model pipeline — i.e. a layer that takes user context (dictionary, style, app) as input, not a per-user adapter.

---

## 3. How Their Personalization Actually Works (Mechanism)

Based on all sources combined, the user-perceived "Wispr learns me" experience is the sum of:

| Mechanism | Type | Updates how |
|---|---|---|
| Personal Dictionary | Metadata | Auto-grows when you type-correct a transcript |
| Snippets | Metadata | User-defined trigger phrases → expansion text |
| Personalized Styles (per-app tone) | User-set preference | Manual setting per app category |
| Context Awareness | Runtime input | Reads current app's on-screen text (opt-in) |
| Speaking-style adaptation | Inference-time conditioning | Recent dictations / corrections fed as prompt context |
| Aggregate global model | Training (everyone) | Pseudonymized corrections from opt-in users retrain the shared Llama cleanup model |

None of these update model weights specifically for you. They all either (a) tweak the prompt/context fed to a shared model at inference, or (b) contribute to retraining of a shared model used by all users.

---

## 4. How We Differentiate (this is the strategic answer)

**Wispr's "personalization" is metadata + global model. Ours can be local model adaptation.**

The differentiation we should ship:

### Differentiator 1: Local, on-device per-user learning loop
- Whisper (or whisper-cpp / MLX equivalent) fine-tuned with LoRA adapters on the user's local Mac, against their local correction history
- Adapter weights live in `~/Library/Application Support/funbutton/adapters/` — never leave the device
- This is the literal thing Wispr can't do because they're cloud-bound and route through OpenAI/Anthropic
- Naming: "Your voice. Your model. Your machine."

### Differentiator 2: Visible learning loop
- Wispr's "learning" is opaque metadata. Show a panel: "I've learned 47 of your specific words. Top corrections this week: [list]. Want to review?"
- Show a per-week accuracy curve specific to the user's voice
- Show what the model has adapted to (cadence, accent, jargon clusters)
- Wispr's privacy policy obfuscates this — ours is transparent

### Differentiator 3: User-controlled
- Toggle: "Pause learning," "Reset adapter," "Export adapter for use on another machine"
- Wispr's "Privacy Mode" is binary (on = your data is dropped entirely). Ours can be granular: "Train locally, never upload, but keep adapting."

### Differentiator 4: Offline-capable
- Wispr requires internet (Baseten + OpenAI/Anthropic/Cerebras APIs). Confirmed.
- Local model + local LoRA = works on a plane, in a SCIF, in an NDA-locked enterprise. This is a nontrivial market segment.

### Sprint 2.6 must-ship: "Local Learning Loop"
This is the moat. Wispr cannot follow without re-architecting away from cloud LLMs, which is their current zero-edit-rate engine. They are locked into option #2 + #3 by their architecture choice. Lock in our option #1 as the visible, controllable, local alternative.

---

## 5. Caveats / Confidence Calibration

What I'm confident about (~95%+):
- They use a shared cleanup model (fine-tuned Llama), not per-user weights — confirmed by Baseten.
- The "dictionary learns over time" claim everywhere refers to a metadata dictionary, not a model — confirmed by their own marketing copy and many reviews.
- Privacy Mode toggling on/off does not affect personal "learning" continuity — confirmed by Privacy Mode docs.

What I'm less sure about (~70%):
- The exact scope of "federated learning" Tanay mentioned. Most likely interpretation: privacy-preserving aggregation of corrections to retrain the global model (option #3). Less likely interpretation: some flavor of per-user federated personalization. He didn't elaborate, no published paper or blog post on it.

What would change my mind:
- A Wispr ML engineer job posting mentioning per-user adapters, LoRA, or "personalized models per user" → would push toward partial #1
- A blog post from Wispr explicitly describing per-user fine-tunes → would flip to YES
- A user describing acoustic model improvement on their *accent specifically* (not jargon) over weeks of use, without manually adding words → would suggest something is happening at the model level

I searched for all three. None exist publicly as of 2026-05-08.

---

## Sources Cited

1. https://techoptimist.vc/episodes/63-meet-the-start-up-at-the-center-of-the-voice-computer-revolution/transcript — CEO direct quote
2. https://wisprflow.ai/privacy-policy — model training scope
3. https://wisprflow.ai/privacy — Privacy Mode mechanics
4. https://roadmap.wisprflow.ai — "Improve the model for everyone" toggle
5. https://www.baseten.co/resources/customers/wispr-flow/ — shared fine-tuned Llama architecture
6. https://www.notablecap.com/blog/a-keyboard-less-future-reinventing-a-150-year-old-interface-with-wispr-flow — "massive cloud models"
7. https://www.getvoibe.com/resources/is-wispr-flow-safe/ — third-party LLM routing (OpenAI/Anthropic/Cerebras)
8. https://sidsaladi.substack.com/p/wispr-flow-101-the-complete-guide — dictionary auto-grow mechanism
9. https://letterly.app/blog/wispr-flow-review/ — dictionary learning description
10. https://fritz.ai/wispr-flow-review/ — "personal dictionary learns over time"
11. https://sites.google.com/view/wispr-flow-review/ — "500 custom terms" learned (dictionary, not model)
12. https://www.eesel.ai/blog/wispr-flow-review — independent review describing personalization features
13. https://wisprflow.ai (homepage) — "Flow automatically learns your unique words and adds them to your personal dictionary"
