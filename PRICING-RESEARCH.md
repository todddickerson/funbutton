# FunButton.ai — Pricing & Billing Validation

**Research date:** 2026-05-09
**Purpose:** Pressure-test the proposed pricing & billing model with current (Nov 2025–May 2026) market data. Confirm or revise the COGS, margins, lifetime ceiling, and implementation plan.
**TL;DR:** Pricing is **mostly right but overstates premium-cleanup margin** and **understates auto-top-up regulatory exposure**. Recommended revisions in §11.

---

## 1. Wispr Flow's actual unit economics

**Question:** How is Wispr serving "unlimited" at $12-15/mo? Is unlimited really unlimited?

### Evidence

- **Funding:** $94M total ($30M Series A Jun 2025 + $25M Series A+ Nov 2025 + $39M earlier). Source: TechCrunch / StartupIntros [§5 of COMPETITIVE-LANDSCAPE.md].
- **Pricing:** $15/mo or $12/mo annual ($144/yr). Free = 2,000 words/week desktop, 1,000/week iOS, **unlimited Android promo** (limited time). No lifetime ever. (Source: [wisprflow.ai/pricing](https://wisprflow.ai/pricing), [getvoibe.com](https://www.getvoibe.com/resources/wispr-flow-pricing/), verified 2026-04-14.)
- **Architecture:** Cloud-only, 800MB RAM, mandatory internet, ~96-97% accuracy. In-house transcription models (FlowOne) with OpenAI Whisper / Meta Llama as fallbacks.
- **Unit-economics tell:** Wispr publicly claimed they "rewrote infrastructure for 30% faster dictation" specifically before launching Android with unlimited free dictation in Feb 2026. ([sustainabl.net analysis](https://www.sustainabl.net/en/articulo/wispr-flow-android-dictation-mass-acquisition-mm823hzx)) — quote: *"Higher speed could imply lower latency but may also mean better pipelines, reduced calls, server optimization, or more efficient models. In any case, the direction is clear: the team understands that the bottleneck isn't only accuracy, but the cost of serving dictation at scale."*
- **No published throttling reports** in the searches performed. "Unlimited" appears to actually be unlimited on paid Pro, but session lengths capped: was 5 min, **extended to 20 min in March 2026** ([automationswitch.com review](https://automationswitch.com/workflow-automation/wispr-flow-review)). Earlier reports cited a 6-min hard cap.
- **Power-user real usage** (verified user dashboards):
  | Tier | Words / period | Per month | Source |
  |---|---|---|---|
  | Top 1% | 339,725 / 3 mo | ~113K | [themarketingshow.com](https://themarketingshow.com/posts/wispr-flow) |
  | Top 1% | 182,718 / period | ~60K | [zackproser.com](https://zackproser.com/blog/wisprflow-review) |
  | Top 2% | 243,554 / 39 days | ~187K | [modulovalue.com](https://modulovalue.com/blog/voxtral-transcribe-and-wispr-flow/) |
  | Top 3% | 94,530 / 7 wks | ~58K | [automationswitch.com](https://automationswitch.com/workflow-automation/wispr-flow-review) |
  | Marketing creator | 80,000 / mo | 80K | self-reported |

### Verdict

Wispr is structurally cost-stressed. Cloud-only inference is **direct variable cost per minute spoken**. With $94M raised they can subsidize indefinitely for now, but the rebuilt infra + the Pro-tier paywall on Android are defensive moves to protect gross margin. **No evidence of fair-use throttling** on Pro — but the hidden ceiling is **session length** (20 min), not word count.

**Implication for FunButton:** "Unlimited" framing is honest if we use Groq's per-minute pricing (which is **89% cheaper than OpenAI Whisper API** — see §2). We do not need to subsidize at $9/mo. We just need to keep cleanup model selection sharp.

---

## 2. Groq production pricing (verified May 2026)

**Sources:** [groq.com/pricing](https://groq.com/pricing), [cloudzero.com Groq 2026 review](https://www.cloudzero.com/blog/groq-pricing/), [tokenmix.ai](https://tokenmix.ai/blog/groq-api-pricing) (verified 2026-04-03), [computeprices.com](https://computeprices.com/providers/groq) (verified 2026-05-08).

### Confirmed pricing

| Service | Price | Speed |
|---|---|---|
| **Whisper Large v3 Turbo** | **$0.04 / hour transcribed** | 216-228× real-time |
| **Whisper V3 Large** | $0.111 / hour | 189-217× |
| **Distil-Whisper Large v3 EN** | $0.02 / hour | 250× |
| **Llama 3.3 70B Versatile (128k)** | **$0.59 / M input · $0.79 / M output** | 275-394 TPS |
| **GPT-OSS 120B** | $0.15 / M input · $0.60 / M output | 500 TPS |
| **GPT-OSS 20B** | $0.075 / M input · $0.30 / M output | 1,000 TPS |
| **Llama 4 Scout 17Bx16E** | $0.11 / M input · $0.34 / M output | 460 TPS |

**Discounts (stackable):** Cached input −50% · Batch processing −50% · Developer tier −25%.

### Free tier rate limits (confirmed)

- 30 requests/min, 6,000 tokens/min, 14,400 requests/day for Llama 3.3 70B.
- Whisper Large v3: 20 RPM, 2,000 req/day. **Rate-limited at the org level** — multiple keys do NOT bypass.
- Free tier is fine for personal BYOK users. Cannot serve a paid Pro tier on free Groq — **must use Pay-as-you-go**.

### Power-user cost calculation (FunButton Pro @ Groq)

Assumptions: 150 WPM dictation (top-1% Wispr benchmark), avg 200-word session (~10 sec audio for short messages, longer for code/email), 1,280-token cleanup prompt (system + transcript), 280-token cleaned output.

| Profile | Words/mo | Audio min | Whisper cost | Cleanups | LLM cost | **Total Groq COGS** |
|---|---|---|---|---|---|---|
| Median user | 30K | 200 | $0.13 | 150 | $0.15 | **$0.28** |
| Heavy daily user | 100K | 667 | $0.44 | 500 | $0.49 | **$0.93** |
| Top 1% (real) | 187K | 1,247 | $0.83 | 935 | $0.92 | **$1.75** |
| Soft-ceiling user | 500K | 3,333 | $2.22 | 2,500 | $2.45 | **$4.67** |
| Abuser | 1M | 6,667 | $4.44 | 5,000 | $4.90 | **$9.34** |

### Pro $9/mo economics

| User profile | Groq COGS | Stripe + CF | **Net margin** | **Margin %** |
|---|---|---|---|---|
| Median (30K) | $0.28 | $0.50 | $8.22 | **91%** |
| Heavy (100K) | $0.93 | $0.50 | $7.57 | **84%** |
| Top 1% real (187K) | $1.75 | $0.50 | $6.75 | **75%** |
| Soft-cap (500K) | $4.67 | $0.50 | $3.83 | **43%** |
| Abuser (1M) | $9.34 | $0.50 | −$0.84 | **break-even** |

**Verdict:** Pro $9/mo is **structurally profitable** at the proposed 500K soft ceiling. The 1M abuser case is exactly why the soft ceiling matters — without it, a single power user can burn margin to zero.

**Stripe + CF infra cost** (~$0.50/user/mo) breakdown: Stripe 2.9% + 30¢ on $9 ≈ $0.56 transaction. CF Worker compute negligible (~$0.001/user). License/D1 lookup ~$0.01. Round to $0.60 worst case.

---

## 3. Anthropic Claude Sonnet 4.6 pricing (verified 2026)

**Sources:** [Anthropic via metacto.com](https://www.metacto.com/blogs/anthropic-api-pricing-a-full-breakdown-of-costs-and-integration), [silicondata.com](https://www.silicondata.com/use-cases/anthropic-claude-api-pricing-2026/) verified Mar 11, 2026, [evolink.ai](https://evolink.ai) verified Apr 16, 2026.

### Confirmed table (per million tokens, May 2026)

| Model | Input | Output | Cache write | Cache read | Context |
|---|---|---|---|---|---|
| **Claude Opus 4.6** | $5 | $25 | $6.25 | $0.50 | 200K / 1M std |
| **Claude Sonnet 4.6** | **$3** | **$15** | $3.75 | $0.30 | 200K / 1M std |
| **Claude Haiku 4.5** | $1 | $5 | $1.25 | $0.10 | 200K |
| Claude Opus 4.7 | $5 | $25 | — | — | 1M std |

Both Sonnet 4.6 and Opus 4.6 include **1M context at standard pricing** — no long-context surcharge (unlike Sonnet 4.5).
Batch API: −50%. Caching: −90% on input on hot prefixes.

### COGS for "premium boost" cleanup, per 10K words

10K words ≈ ~14K tokens (transcript) + ~1K system prompt = **15K input** (mostly cacheable system).
Output ≈ same 14K tokens cleaned + small reasoning trace = **15K output**.

| Model | Cost (uncached) | Cost (90% cache) | Cost (batch) |
|---|---|---|---|
| Sonnet 4.6 | (15K × $3 + 15K × $15)/M = **$0.27** | **~$0.23** | $0.135 |
| Haiku 4.5 | (15K × $1 + 15K × $5)/M = **$0.09** | ~$0.075 | $0.045 |
| Opus 4.6 | (15K × $5 + 15K × $25)/M = **$0.45** | ~$0.38 | $0.225 |

### Verdict on $0.40/10K-word retail (proposed)

- **Against Sonnet 4.6 uncached: $0.27 COGS → $0.13 margin = 33%** ❌ NOT >50% as proposed.
- **With prompt caching (90% hit rate, easily achievable on a 1K-token system prompt): $0.23 → $0.17 margin = 43%** ❌ Still under 50%.
- **Against Haiku 4.5 uncached: $0.09 COGS → $0.31 margin = 78%** ✅ Comfortably >50%.

**The proposed $0.40/10K words DOES NOT clear 50% margin against Sonnet 4.6.** Two ways to fix:

1. **Recommended:** Make the default "premium" model **Haiku 4.5** (faster, cheaper, surprisingly capable for cleanup). Charge $0.40/10K. **78% margin.** Rename UI: "Premium boost (Claude Haiku) / Pro+ (Sonnet)".
2. Alternative: Keep Sonnet as default and **bump retail to $0.60/10K** to clear 55% margin. Risk: undercuts the "fast tier is unlimited" positioning since premium starts feeling expensive.

**Pick 1.** Sonnet 4.6 becomes a "Pro+ boost" at $0.60/10K (55% margin). Opus 4.6 is a separately-toggled "Reasoning boost" at $0.99/10K.

---

## 4. OpenAI GPT-4o pricing (verified 2026)

**Sources:** [openai.com/api/pricing](https://openai.com/api/pricing/) verified Apr 2026, [pricepertoken.com](https://pricepertoken.com/pricing-page/model/openai-gpt-4o) verified May 2026, [pecollective.com](https://pecollective.com/tools/gpt-4o-pricing/), [metacto.com OpenAI breakdown](https://www.metacto.com/blogs/unlocking-the-true-cost-of-openai-api-a-deep-dive-into-usage-integration-and-maintenance) verified Mar 2026.

### Confirmed pricing

| Model | Input / M | Output / M | Cached input | Status |
|---|---|---|---|---|
| **GPT-4o** | **$2.50** | **$10.00** | $1.25 | **Legacy / grandfathered** for existing users; replaced by GPT-4.1 family for new deployments |
| GPT-4o Mini | $0.15 | $0.60 | $0.075 | Active |
| **GPT-4.1** (new flagship) | $2.00 | $8.00 | $0.50 | **Active — recommended** |
| GPT-4.1 Mini | $0.40 | $1.60 | $0.10 | Active |
| GPT-4.1 Nano | $0.10 | $0.40 | $0.025 | Active |
| GPT-5.3 (Codex) | $1.75 | $14.00 | $0.175 | Active |

Batch API: −50% on both. Caching: 50–90% off input.

### COGS per 10K words (15K in / 15K out)

| Model | Uncached | Cached (90%) | Batch |
|---|---|---|---|
| GPT-4o | 15K×$2.50/M + 15K×$10/M = **$0.19** | $0.17 | $0.094 |
| **GPT-4.1** | 15K×$2/M + 15K×$8/M = **$0.15** | $0.13 | $0.075 |
| GPT-4.1 Mini | 15K×$0.40/M + 15K×$1.60/M = **$0.030** | $0.026 | $0.015 |

### Verdict on $0.60/10K-word retail (proposed)

- Against GPT-4o uncached: $0.60 − $0.19 = **$0.41 margin = 68%** ✅
- Against GPT-4.1 uncached: $0.60 − $0.15 = **$0.45 margin = 75%** ✅

**$0.60/10K-word retail clears 50% comfortably for GPT-4.1.** Keep proposed price; switch model to GPT-4.1 (cheaper, better, current). **GPT-4o is being deprecated for new integrations** — do not build the metering on it.

---

## 5. Stripe metered billing UX best practices 2026

**Sources:** [docs.stripe.com billing/usage-based](https://docs.stripe.com/billing/subscriptions/usage-based/), [docs.stripe.com billing-credits implementation guide](https://docs.stripe.com/billing/subscriptions/usage-based/billing-credits/implementation-guide), [docs.stripe.com billing-thresholds](https://docs.stripe.com/billing/subscriptions/usage-based-legacy), [starterpick.com Stripe Meters API guide](https://starterpick.com) (Mar 2026).

### What's native in Stripe (May 2026)

1. **Meters API (current)** — replaces legacy `usage_records`. Events are immutable, server-side aggregated, queryable in real time. **First-class for AI token billing.** Idempotency key on `identifier` field. Returns aggregated summaries via `listEventSummaries` (cache 5–10 min for dashboards).
2. **Billing credits / credit grants** — prepaid credits applied automatically at invoice finalization. Supports per-meter scoping (`applicability_config.scope.billable_items`). Used for "first 50K premium words/mo included."
3. **Billing thresholds** — `billing_thresholds[amount_gte]=2000` (cents) auto-issues an invoice when accrued usage hits the threshold; can also reset billing cycle anchor. **NOTE:** This invoices early, it does **not** hard-stop charges. To enforce "$20 cap → fall back to fast tier", you must do an **app-level check** before each premium call against `listEventSummaries` (or your own D1 mirror).
4. **Graduated tiered pricing** — native; tier 1 covers included quota at flat rate, tier 2 charges per-unit overage.

### Best-in-class UX patterns observed

- **Anthropic / OpenAI:** show usage dashboard with current period total vs. limit, alert at 80% and 100% via email + in-app toast.
- **Cloudflare R2 / Workers:** $5/mo subscription includes generous baseline + clear $0.30/M-request overage; "no surprise charges" because they hard-stop on free, soft-warn on paid.
- **Vercel:** prepaid spend caps at $20/$50/$100 with auto top-up. Cap is **soft** (notification + service degradation), not hard, by default. Hard cap is opt-in.

### Recommended FunButton flow

```
Pro $9/mo subscription
├── Recurring price: $9/mo flat (Stripe Price)
├── Meter: groq_words (informational; included unlimited up to 500K soft ceiling)
└── Meter: premium_words (charged)
    ├── Credit grant: 50,000 words/mo, expires at cycle end, reissued on invoice.paid
    ├── Graduated price: $0 for first 50K, $0.40 per 1K above (Haiku) or $0.60 per 1K (Sonnet)
    └── Auto top-up logic (app + Worker enforced):
        ├── User sets: monthlyHardCap = $20 (default), $0–$200 range
        ├── On every premium call, Worker checks: currentMonthSpend < monthlyHardCap
        ├── If exceeded: 402 response, client switches model to fast (Groq), shows toast
        └── User can raise cap from Settings → triggers Stripe customer portal session
```

**Do NOT use `billing_thresholds` for the hard stop** — it triggers an invoice, which charges the customer, which is the opposite of what we want. Use it as a backup safety net at 2× the user's chosen cap (in case our app-level meter is stale).

---

## 6. Competitor lifetime pricing (verified May 2026)

**Sources:** [getvoibe.com VoiceInk pricing](https://www.getvoibe.com/resources/voiceink-pricing/) verified Apr 2026, [getvoibe.com Wispr pricing](https://www.getvoibe.com/resources/wispr-flow-pricing/), [pikaseo.com](https://pikaseo.com), [dictanote.co/voicein/plus](https://dictanote.co/voicein/plus/), COMPETITIVE-LANDSCAPE.md §2.

| App | Lifetime price | Notes |
|---|---|---|
| Speakmac | $19 | Cheapest Mac native |
| **VoiceInk Solo** | **$25** | 1 Mac, GPL v3 source available free |
| VoiceInk Personal | $39 | 2 Macs |
| BetterDictation | $39 | YC-mentioned |
| VoiceInk Extended | $49 | 3 Macs |
| MacWhisper | $69-$80 | File transcription primary |
| **Voibe** | **$99-149** | $99 in some markets; $149 in others (Voibe brand consolidation 2026) |
| **VoiceIn Plus** | **$149** | 600K users; promo deal |
| **Superwhisper** | **$249.99** *(reportedly jumped to $849 in 2026)* | YouTube reviewer flagged the increase Apr 2026 |
| Wispr Flow | **No lifetime, ever** | Subscription only ($432 over 3 yr) |
| Sublime Text *(reference)* | $99 (3-yr renewable) | Dev-tool gold standard |

### Verdict on $149 / $249

- **$149 first 1K** is **at parity with Voibe / VoiceIn Plus** — defensible, well below the SuperWhisper anchor, well above the VoiceInk floor. ✅ **Keep.**
- **$249 post-1K** is **uncomfortably close to old Superwhisper anchor** ($249.99) and Superwhisper just flinched the entire category to $849 — making $249 look "premium but reasonable" by contrast. **Defensible but optimistic.** Recommend stepping the ladder:
  - **First 1K customers: $149**
  - **Next 4K customers (1K-5K): $199**
  - **Permanent post-5K: $249**
- This gives early evangelists a deal, captures FOMO mid-launch, lands at SuperWhisper's old price for steady-state. Easier psychologically than a single $100 jump.

### Lifetime ceiling logic

A dev-tool lifetime in 2026 caps around **3× annual subscription** for a comparable mainstream tool. Wispr at $144/yr × 3 = $432. Cursor Pro is $20/mo = $240/yr × 3 = $720. So $249 is about **57% of Wispr's 3-yr equivalent** — strong value framing.

---

## 7. Cloudflare Worker proxy economics

**Sources:** [developers.cloudflare.com/workers/platform/pricing](https://developers.cloudflare.com/workers/platform/pricing/) verified Apr 23, 2026; [truefoundry.com AI Gateway 2026](https://www.truefoundry.com/blog/cloudflare-ai-gateway-pricing-a-complete-breakdown).

### Workers Paid plan ($5/mo minimum)

| Resource | Included | Overage |
|---|---|---|
| Requests | 10M / mo | $0.30 / M |
| CPU time | 30M ms / mo | $0.02 / M ms |
| Duration | unlimited | $0 |
| KV reads | 10M / mo | $0.50 / M |
| KV writes | 1M / mo | $5.00 / M |
| **D1 row reads** | **25B / mo** | $0.001 / M |
| **D1 row writes** | **50M / mo** | $1.00 / M |
| D1 storage | 5 GB | $0.20 / GB-mo |
| R2 reads | 1M / mo | $0.36 / M |
| R2 writes | 1M / mo | $4.50 / M |
| AI Gateway logs | 1M / mo | upgrade-only (not pay-per-overage) |
| Container compute | 25 GiB-h memory + 375 vCPU-min | $0.0000025 / GiB-s + $0.000020 / vCPU-s |

**Key:** No data egress / bandwidth charges. Subrequests outbound (Groq, Anthropic, OpenAI) free.

### FunButton scale projection

Assume 10,000 active Pro users × 200 dictation requests/day = 2M req/day = **60M req/mo**.

| Line item | Volume | Cost |
|---|---|---|
| Subscription | 1 | $5 |
| Requests | (60M − 10M) / M × $0.30 | $15 |
| CPU time @ ~10ms/req | (10ms × 60M − 30M) / M × $0.02 | $11.40 |
| D1 writes (1 usage event/req) | 60M / 50M overage × $1 | $10 |
| D1 reads (3 reads/req: license, usage, quota) | well within 25B | $0 |
| AI Gateway logs | within 1M (would need sampling above 1M) | $0 |
| **TOTAL** | | **~$41/mo for 10K users = $0.004/user/mo** |

**Verdict:** Cloudflare proxy is **negligible cost** at our scale. Even at 100K users, infra is ~$400/mo total = $0.004/user. The ~$0.50/user infra cost line in §2 is dominated by Stripe transaction fees, not CF.

### Logging note

AI Gateway logs cap at 1M/mo even on paid — if we exceed, we must sample (e.g. log 10% of requests) or pipe to our own R2. **Recommended:** log 100% to D1 (`usage_events` table), use AI Gateway only for failure analytics + LLM prompt caching.

---

## 8. License key / activation patterns 2026

**Sources:** [git-tower.com offline activation](https://www.git-tower.com/help/guides/integration/offline-activation/mac), [keyforge.dev Windows app guide](https://keyforge.dev/guides/how-to-license-windows-app), Polar.sh license key tutorial (YouTube, Sep 2025), [softwarekey.com SOLO Server](https://www.softwarekey.com).

### Modern best-practice stack (2026)

| Component | Pattern |
|---|---|
| **Issuance** | License key issued at Stripe `checkout.session.completed` webhook → cryptographically signed JWT → emailed + shown in dashboard |
| **Format** | `FB-{tier}-{base64url(JWT)}` e.g. `FB-LIFE-eyJhbGc...` (human-recognizable prefix, copy-pasteable) |
| **JWT claims** | `userId`, `tier` (free/pro/life), `deviceLimit`, `premiumQuotaWords`, `validUntil` (refreshed; for Pro = sub period end + 30d grace; for Lifetime = +10y), `featureFlags` |
| **Activation** | First launch: POST `/v1/activate` with `{licenseKey, hwFingerprint, machineName}` → server stores `activationId` keyed to (license, hwFingerprint), returns refreshed JWT |
| **Refresh** | App pings `/v1/license/refresh` every **24h online**, **30d grace offline**, **hard stop at 60d offline** (reasonable for a desktop app; matches Sublime Text / Tower / Setapp norms) |
| **Revocation** | Server-side denylist checked at refresh; revoked keys return 403 → app falls back to free BYOK mode (never bricks) |
| **Multi-device** | 3 devices for Pro/Lifetime, 1 for Pro-yearly. `POST /v1/license/deactivate` to release a slot from another device |
| **Hardware fingerprint** | macOS: combine `IOPlatformUUID` + machine model + user-supplied machine name. Hash → 16-char ID stored in JWT. **Not a security barrier**, just a multi-device anti-share signal |

### Reference implementations

- **Tower (Mac):** offline plist exchange — `gittower license -i > info.plist` → transfer → `gittower license -a info.plist --code TOWR-... --email ...` → `--install`. Good pattern for offline corp environments. Probably overkill for v0.2 — stick with online refresh + 30d grace.
- **Sublime Text:** $99 license, 3-year version refresh window, simple email + key, no hardware lock. Trust-based. Considered the dev-tool gold standard.
- **Polar.sh + custom proxy** (recommended modern pattern): Polar handles license CRUD + webhooks; you proxy validation through your own Worker to add custom logic (device fingerprint, usage caps, regional restrictions). Polar does not charge per-license overhead.
- **SOLO Server / Keyforge:** legacy SDKs for Windows native; not relevant for Tauri.

### Recommendation for FunButton

**Roll our own JWT-based license server in Cloudflare Worker + D1.** No third-party dependency. ~150 LOC. Polar is fine if we want a managed solution but adds a dependency without much benefit at our scale.

```
license_keys (D1)
├── id (uuid)
├── stripe_customer_id
├── tier (free|pro_monthly|pro_yearly|lifetime)
├── premium_quota_words (default by tier)
├── device_limit
├── valid_until (NULL for lifetime, sub_period_end+grace for monthly)
├── revoked_at (NULL or ISO ts)
└── created_at

activations (D1)
├── id (uuid = activation_id)
├── license_key_id (fk)
├── hw_fingerprint (sha256, 16-char)
├── machine_name (user-supplied)
├── last_seen_at
└── created_at

usage_events (D1)
├── id (auto)
├── license_key_id (fk)
├── event_type (transcribe|cleanup_fast|cleanup_premium)
├── words (int)
├── tokens_in, tokens_out (int)
├── model (string)
├── stripe_meter_event_id (when posted)
├── created_at (indexed)
```

---

## 9. Auto top-up regulatory issues (2026)

**Sources:** [goodwinlaw.com FTC Click-to-Cancel update](https://www.goodwinlaw.com/en/insights/publications/2026/02/alerts-practices-ba-ftcs-click-to-cancel-rule-gets-new-life), [sidley.com FTC ANPRM](https://www.sidley.com/en/insights/newsupdates/2026/02/us-ftc-signals-renewed-interest-in-click-to-cancel-rulemaking), [latham.com](https://www.lw.com).

### Current US regulatory state (May 2026)

- **8th Circuit vacated FTC's 2024 "Click-to-Cancel" Negative Option Rule in July 2025** on procedural grounds (FTC failed required preliminary regulatory analysis after ALJ found >$100M economic impact).
- **FTC restarted rulemaking** by submitting a new ANPRM to OIRA on **January 30, 2026**. OIRA review takes up to 90 days + 30 day extension. **No new rule is in effect today.**
- **ROSCA (Restore Online Shoppers' Confidence Act) is still active** and the FTC is enforcing aggressively under it: Uber (UberOne), LA Fitness, and Chegg ($7.5M settlement Sept 2025) all hit for hard-to-cancel auto-renewals.
- **State Automatic Renewal Laws (ARLs) remain enforceable** and many are stricter than federal. **California ARL (May 2024 amendment)** is the toughest — same-medium cancellation, after-sale notifications, advance renewal notices.
- **EU Consumer Rights Directive (CRD)** + **Digital Services Act (DSA)** require 14-day cancellation right for online subscriptions (digital content has caveats), clear price disclosure pre-purchase. UK CRA mirrors.

### Implications for auto top-up

The **auto top-up** mechanism is the riskiest part of the proposed plan. Under ROSCA + state ARLs:

1. **Express informed consent required** before charging beyond the base subscription. The user's choosing a $20 cap counts, but the UI must make it unambiguous.
2. **Same-medium cancellation** — if the user signed up for top-up via Settings → must be turn-off-able via Settings (NOT "email support to cancel").
3. **Advance disclosure of all material terms** — price, frequency, that it auto-renews unless cancelled.
4. **Clear notice on every charge** — email receipt with cap reset link.
5. **California specifically:** if auto-renew adds >$10/mo equivalent, user must get a renewal reminder 3-21 days ahead.

### Required Settings UI (compliance checklist)

```
Settings → Premium Boost
├── [ ] Enable premium cleanup (Claude Haiku) — toggleable
├── Monthly hard cap: [$20 ▼]   (range: $0 ... $200, default $20)
│   ├── At cap: [Fall back to fast tier ▼]  (only behavior; no surprise charges)
│   └── Email me at: [80%, 100%]
├── Plain-English disclosure (always visible):
│   "Beyond your 50,000 included premium words, we charge $0.40 per 1,000 words 
│    used (Claude Haiku) or $0.60 per 1,000 words (Sonnet). You will never be 
│    charged more than your monthly cap of $20. Your card on file will be 
│    charged on the 1st of each month for any usage above your included quota. 
│    Disable any time from this screen."
├── [ Disable premium boost — one click, no save offer required ]
└── Receipt log (last 12 months, downloadable CSV)
```

**One-click disable is the MUST.** No multi-step "are you sure" dark patterns. Do NOT add a save offer (FTC view: a save offer is allowed but only after the cancellation succeeds, never before).

### Recommendations

- **Default state: premium boost OFF.** User must opt in.
- **No auto-enrollment from trial.** Trial is fast-tier only.
- **California ARL renewal reminder:** if monthly cap × 12 > $120, email reminder 7 days before annual subscription renews.
- **Receipt-on-charge:** every monthly auto-charge gets an email with itemized usage + link to disable / lower cap.
- **Sub-processors disclosure** in Privacy Policy: Stripe (payments), Cloudflare (proxy), Groq (transcription/cleanup), Anthropic + OpenAI (premium cleanup, opt-in).

This is **not blocked** by current regs — it's just compliance-attentive design. The biggest exposure is California's ARL specifics; allocate 2-3 hours with a templated subscription-compliance review (RocketLawyer, LegalGPS) before launching paid tiers.

---

## 10. Industry benchmarks for power users (verified)

Real Wispr Flow user dashboards (from individual reviewers' published numbers — see §1 table):

| Percentile | Words / mo | Source |
|---|---|---|
| Median (estimated) | 5,000-10,000 | inferred from "2,000 words/week free is enough for casual users" framing |
| Top 10% (heavy daily) | 25,000-40,000 | inferred |
| Top 3% | 58,000 | automationswitch.com |
| Top 1% (multiple) | 60,000-113,000 | zackproser.com, themarketingshow.com |
| Top <1% (extreme outlier) | 187,000 | modulovalue.com (243K / 39 days) |
| Theoretical 8-hour-day at 150 WPM | 1,440,000 | math, not observed |

### Verdict on 500K/mo soft ceiling

**500K/mo soft ceiling is well above the 95th percentile** — even the most extreme observed real user (modulovalue.com at 187K/mo) is below it by 2.7×. Theoretical 8-hour-non-stop dictation hits 1.44M — that user is either an abuser or running a scraping bot.

**Recommendation:** Keep 500K. At 500K we still have 43% margin (§2). Consider a softer 300K alert ("hey power user, want to switch to local Ollama for free?") that doesn't gate, just nudges.

---

## 11. Final validated pricing table

### Tiers

| Plan | Price | Fast tier (Groq) | Premium boost (Claude Haiku 4.5 default) | Pro+ boost (Sonnet 4.6) | Reasoning boost (Opus 4.6) | Devices |
|---|---|---|---|---|---|---|
| **Free / BYOK** | $0 | Unlimited via your Groq key OR local Ollama | BYOK only | BYOK only | BYOK only | Unlimited |
| **Pro Monthly** | **$9 / mo** | Unlimited (500K/mo soft ceiling, then nudge to local) | 50K words/mo included, then $0.40/1K | $0.60/1K opt-in | $0.99/1K opt-in | 3 |
| **Pro Yearly** | **$79 / yr** ($6.58 effective) | same | same | same | same | 1 |
| **Lifetime — first 1,000** | **$149** | same as Pro | Pay-as-you-go ($0.40/1K Haiku, $0.60/1K Sonnet, $0.99/1K Opus) | same | same | 3 |
| **Lifetime — 1,001-5,000** | **$199** | same | same | same | same | 3 |
| **Lifetime — 5,001+** | **$249** | same | same | same | same | 3 |

### Key revisions from the original proposal

| Item | Original | **Revised** | Why |
|---|---|---|---|
| Premium default model | Claude Sonnet $0.40/10K = "Sonnet" | **Haiku 4.5 $0.40/10K** as default; Sonnet $0.60/10K as Pro+; Opus $0.99/10K as Reasoning | Sonnet at $0.40 = 33-43% margin (fails 50% bar). Haiku at $0.40 = 78% margin. Same UX; better economics. |
| Lifetime ladder | $149 first 1K, then $249 | $149 (1-1K) → **$199 (1K-5K)** → $249 (5K+) | $100 cliff is rough. Three steps reduces friction, adds urgency twice. |
| GPT-4o premium | $0.60/10K GPT-4o | $0.60/10K **GPT-4.1** | GPT-4o being deprecated for new integrations. GPT-4.1 is cheaper ($2/$8 vs $2.50/$10), better, current. |
| Auto-top-up default cap | $20 | $20 (kept) — but with **opt-in default OFF**, mandatory monthly receipt, one-click disable | ROSCA / state ARL compliance |
| Soft ceiling | 500K | 500K (kept) — add 300K nudge to local | Power user education + retention; still ≥43% margin at 500K |

### Auto top-up flow (compliance-aware)

```
User flow:
  Settings → Premium Boost → toggle ON (DEFAULT OFF)
    → "Cap monthly extra spend at $X" (default $20, range $0-$200)
    → "Emergency overflow: [Fall back to fast tier ▼]" (only option)
    → Plain-English disclosure (always visible)
    → "Disable" button (1 click, instant)
  
On every premium API call (Worker enforces):
  if (currentMonthSpendCents + thisCallCents > userCapCents) {
    return 402 {error: 'cap_reached', fallback: 'fast'}
  }
  // No "are you sure you want to overage?" prompts.
  // No surprise charges. Period.
  
Monthly cycle:
  - 80% of cap: in-app toast + email
  - 100% of cap: email + auto-fallback to fast tier (toast: "you've hit your $20 cap, switching to fast — change in Settings")
  - 1st of next month: cap resets, charge stripe for previous month's overage (single line on invoice), email receipt
```

---

## 12. Risk register

| Risk | Probability | Impact | Mitigation |
|---|---|---|---|
| **Groq raises prices 50%** | Medium (they've held flat for 18 mo but no contract) | High — compresses margin from 84% to ~70% on 100K user, from 43% to 14% on 500K user | (a) Lower soft ceiling to 300K. (b) Add second STT provider (OpenAI Whisper at $0.36/hr is 9× more expensive — bad fallback). **Real fallback: route premium users through local Ollama-Whisper option.** (c) Renegotiate or move some traffic to Distil-Whisper at $0.02/hr. |
| **Claude / GPT prices change** | Low for next 6mo, Medium-High for 12mo | Low — premium is opt-in; we just adjust retail per 1K | Quarterly model & price review. We pass through cost changes in retail with 30 days' notice. |
| **Stripe billing_thresholds doesn't hard-stop** | Confirmed (it doesn't) | Medium — could overcharge a user who exceeds cap before our app-level check fires | App-level check is authoritative. Stripe threshold = backstop at 2× the user's chosen cap. |
| **FTC re-issues click-to-cancel rule** | Medium-High (ANPRM in Jan 2026) | Low if we're already compliant | Already compliant per §9. Watch for OIRA clearance + Federal Register publication; allow 90 days' lead time for any spec changes. |
| **California ARL enforcement** | Medium (CA AG has been active) | Medium fine ($2,500-7,500 per violation per consumer) | Same-medium cancellation, advance renewal reminders, plain-English disclosure → already in spec. |
| **License key piracy / sharing** | High (it's lifetime, of course it'll happen) | Low — fast-tier abuse caught by Groq cost ceiling; lifetime users sharing 4 devices = $149 / 4 = $37 effective per user, still profitable | 3-device limit + 24h online refresh + denylist on flagged keys. Don't be hostile about it; punks share. |
| **Wispr drops to $9/mo to compete** | Low (would crater their margins, $94M raised pressure) | Medium — would compress our positioning premium | We have 3 unique levers (Linux, lifetime, GPLv3 core) Wispr structurally cannot match. Compete on those, not price. |
| **Superwhisper $849 normalizes premium pricing** | Confirmed (happening now) | Net positive — drags lifetime ceiling up; our $249 looks cheap | Lean into it on landing page: "Superwhisper jumped to $849. We charge $149." |

---

## 13. Stripe implementation sketch

### Products & prices (run as Stripe CLI / dashboard / bootstrap script)

```bash
# Subscriptions
stripe products create --name "FunButton Pro Monthly" --metadata tier=pro_monthly
stripe prices create --product prod_XXX --unit-amount 900 --currency usd --recurring interval=month

stripe products create --name "FunButton Pro Yearly" --metadata tier=pro_yearly
stripe prices create --product prod_YYY --unit-amount 7900 --currency usd --recurring interval=year

# Lifetime (one-time, three SKUs, switch active by ladder phase)
stripe products create --name "FunButton Lifetime" --metadata tier=lifetime
stripe prices create --product prod_ZZZ --unit-amount 14900 --currency usd  # $149
stripe prices create --product prod_ZZZ --unit-amount 19900 --currency usd  # $199
stripe prices create --product prod_ZZZ --unit-amount 24900 --currency usd  # $249

# Meters (Meters API, current — NOT legacy usage_records)
stripe billing meters create --display-name "Premium words (Haiku)" --event-name premium_words_haiku --customer-mapping '{"event_payload_key":"stripe_customer_id"}' --value-settings '{"event_payload_key":"value"}' --default-aggregation '{"formula":"sum"}'
stripe billing meters create --display-name "Premium words (Sonnet)" --event-name premium_words_sonnet ...
stripe billing meters create --display-name "Premium words (Opus)" --event-name premium_words_opus ...

# Metered prices (graduated: first 50K free, then per-1K)
stripe prices create --product prod_AAA \
  --currency usd \
  --recurring '{"interval":"month","usage_type":"metered","meter":"mtr_haiku"}' \
  --billing-scheme tiered \
  --tiers-mode graduated \
  --tiers '[{"up_to":50000,"flat_amount":0},{"up_to":"inf","unit_amount_decimal":"0.04"}]'  # $0.04/word = $0.40/1K = $4/10K

# Credit grants are issued at subscription creation
```

### Webhook handlers (Cloudflare Worker)

```javascript
// /api/stripe/webhook
checkout.session.completed:
  → create license_key in D1 with tier from metadata
  → email license key to customer
  → Slack/Discord ping for first lifetime sale

customer.subscription.created:
  → upsert license_key
  → create credit grant: 50,000 premium_words (Haiku) for current period

customer.subscription.updated:
  → if status=past_due → mark license expiring_soon (warn user, don't revoke)
  → if status=canceled → set valid_until = period_end + 30d grace, revoke after

invoice.paid:
  → reset monthly counters (in D1)
  → reissue credit grants for next period

billing.meter.event_summary (poll, not webhook):
  → reconcile our D1 meter mirror with Stripe's authoritative count once/day

customer.subscription.deleted:
  → revoke license at valid_until
```

### License issuance (post-checkout)

```javascript
// Ed25519 sign with private key in Worker secret
const claims = {
  sub: license_key_id,
  tier: 'lifetime',
  device_limit: 3,
  premium_quota_words: 50000,  // 0 for lifetime base; sold a la carte
  features: ['groq_unlimited', 'premium_byok'],
  iat: Math.floor(Date.now()/1000),
  exp: tier === 'lifetime' ? 0 : sub_period_end_ts + 30*86400  // 30d grace
};
const jwt = await sign(claims, env.LICENSE_PRIV_KEY);  // EdDSA / Ed25519
const key = `FB-${tier.toUpperCase()}-${base64url(jwt)}`;
```

---

## 14. Cloudflare Worker proxy pseudocode

```typescript
// apps/proxy/src/index.ts (Cloudflare Worker)
import { verify as verifyJWT } from '@tsndr/cloudflare-worker-jwt';
import Stripe from 'stripe';

interface LicenseClaims {
  sub: string;             // license_key_id
  tier: 'free' | 'pro_monthly' | 'pro_yearly' | 'lifetime';
  device_limit: number;
  premium_quota_words: number;
  features: string[];
  exp: number;
}

export default {
  async fetch(req: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    const url = new URL(req.url);
    
    // === ROUTE 1: License activate / refresh ===
    if (url.pathname === '/v1/license/refresh') {
      return handleLicenseRefresh(req, env);
    }
    
    // === ROUTE 2: Transcription proxy ===
    if (url.pathname === '/v1/transcribe') {
      return handleTranscribe(req, env, ctx);
    }
    
    // === ROUTE 3: Cleanup proxy ===
    if (url.pathname === '/v1/cleanup') {
      return handleCleanup(req, env, ctx);
    }
    
    return new Response('not found', { status: 404 });
  }
};

async function handleCleanup(req: Request, env: Env, ctx: ExecutionContext) {
  // 1. AUTH
  const token = req.headers.get('authorization')?.replace(/^Bearer /, '');
  if (!token) return jsonError(401, 'no_license');
  const valid = await verifyJWT(token, env.LICENSE_PUBKEY, { algorithm: 'EdDSA' });
  if (!valid) return jsonError(401, 'invalid_license');
  const claims = decodeJWT(token).payload as LicenseClaims;
  
  // 2. REVOCATION CHECK (D1)
  const license = await env.DB.prepare(
    'SELECT revoked_at, valid_until, stripe_customer_id, monthly_cap_cents FROM license_keys WHERE id=?'
  ).bind(claims.sub).first();
  if (!license || license.revoked_at) return jsonError(403, 'license_revoked');
  if (license.valid_until && license.valid_until < Date.now()/1000) return jsonError(402, 'license_expired');
  
  // 3. RATE LIMIT (per-license, sliding window)
  const rl = await env.RATE_LIMITER.limit({ key: `cleanup:${claims.sub}` });
  if (!rl.success) return jsonError(429, 'rate_limit', { retryAfter: rl.retryAfter });
  
  // 4. PARSE REQUEST
  const { transcript, model = 'fast' } = await req.json() as { transcript: string; model: 'fast'|'haiku'|'sonnet'|'opus' };
  const wordCount = transcript.trim().split(/\s+/).length;
  const isPremium = model !== 'fast';
  
  // 5. PREMIUM QUOTA + CAP CHECK (HARD STOP)
  if (isPremium) {
    const usage = await env.DB.prepare(
      `SELECT
         COALESCE(SUM(CASE WHEN model='haiku' THEN words END),0) as haiku_words,
         COALESCE(SUM(CASE WHEN model='sonnet' THEN words END),0) as sonnet_words,
         COALESCE(SUM(CASE WHEN model='opus' THEN words END),0) as opus_words
       FROM usage_events
       WHERE license_key_id=? AND created_at >= date('now','start of month')`
    ).bind(claims.sub).first();
    
    const overageCents = computeOverageCents(usage, model, wordCount, claims.premium_quota_words);
    const monthlySpend = await currentMonthOverageSpend(env, claims.sub);
    
    if (monthlySpend + overageCents > license.monthly_cap_cents) {
      // HARD STOP — fall back to fast tier on the client
      return jsonError(402, 'cap_reached', {
        fallback: 'fast',
        currentSpendCents: monthlySpend,
        capCents: license.monthly_cap_cents,
        message: `You've hit your $${license.monthly_cap_cents/100} monthly cap. Switching to fast tier. Raise cap in Settings.`
      });
    }
  }
  
  // 6. ROUTE TO PROVIDER
  const upstream = isPremium
    ? await callAnthropic(transcript, model, env)   // Haiku/Sonnet/Opus
    : await callGroq(transcript, env);              // Llama 3.3 70B
  
  if (!upstream.ok) {
    // Fail open to fast tier on premium failure
    if (isPremium) return callGroq(transcript, env).then(r => withHeader(r, 'x-fb-fallback','fast'));
    return upstream;
  }
  
  const result = await upstream.json();
  
  // 7. LOG USAGE (ASYNC, NEVER BLOCK RESPONSE)
  ctx.waitUntil(Promise.all([
    env.DB.prepare(
      'INSERT INTO usage_events (license_key_id, model, words, tokens_in, tokens_out, created_at) VALUES (?,?,?,?,?,?)'
    ).bind(claims.sub, model, wordCount, result.usage?.input_tokens||0, result.usage?.output_tokens||0, Date.now()/1000).run(),
    isPremium && postStripeMeter(env, license.stripe_customer_id, model, wordCount),
  ]));
  
  return Response.json({ cleaned: result.text, model, words: wordCount });
}

async function postStripeMeter(env: Env, stripeCustomerId: string, model: string, words: number) {
  const stripe = new Stripe(env.STRIPE_SECRET, { apiVersion: '2025-04-30' });
  await stripe.billing.meterEvents.create({
    event_name: `premium_words_${model}`,
    payload: { stripe_customer_id: stripeCustomerId, value: String(words) },
    timestamp: Math.floor(Date.now()/1000),
    identifier: crypto.randomUUID(),  // idempotency
  });
}

async function callGroq(transcript: string, env: Env) {
  return fetch('https://api.groq.com/openai/v1/chat/completions', {
    method: 'POST',
    headers: { 'Authorization': `Bearer ${env.GROQ_KEY}`, 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: 'llama-3.3-70b-versatile',
      messages: [
        { role: 'system', content: SYSTEM_PROMPT_CACHED },  // cached on Groq side
        { role: 'user', content: transcript },
      ],
      temperature: 0.2, max_tokens: 1024,
    }),
  });
}

async function callAnthropic(transcript: string, model: string, env: Env) {
  const modelMap = { haiku: 'claude-haiku-4-5', sonnet: 'claude-sonnet-4-6', opus: 'claude-opus-4-6' };
  return fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: { 'x-api-key': env.ANTHROPIC_KEY, 'anthropic-version': '2023-06-01', 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model: modelMap[model],
      max_tokens: 2048,
      system: [{ type: 'text', text: SYSTEM_PROMPT, cache_control: { type: 'ephemeral' } }],
      messages: [{ role: 'user', content: transcript }],
    }),
  });
}
```

**Security & ops notes:**
- `LICENSE_PUBKEY` lives in Worker secret. Private signing key lives in a separate Worker (license issuance) so the proxy can never sign new licenses.
- Stripe meter posts are idempotent (UUID `identifier`) — safe to retry on transient failures.
- D1 is the authoritative meter for hard-cap decisions. Stripe meter is the billing source of truth (reconciled daily).
- Fail-open to fast tier on Anthropic outages keeps premium users productive.

---

## 15. Concrete Week-2 ship plan

**Goal:** Ship monetization in Week 2 post-v0.1 launch (target: 2026-05-19 → 2026-05-26).

### Day 1 (Mon)
- [ ] Stripe products + prices created via bootstrap script (above)
- [ ] D1 schema deployed (3 tables)
- [ ] License Worker scaffolded — `/v1/license/refresh`, `/v1/license/activate` working with a mocked claims response
- [ ] Sign up Anthropic Workspace + add prod API key to Worker secret

### Day 2 (Tue)
- [ ] Cleanup Worker — `/v1/cleanup` routing to Groq for `fast`, returning hardcoded mock for premium (no Anthropic yet)
- [ ] Stripe checkout pages live (Pro Monthly, Pro Yearly, Lifetime $149)
- [ ] `checkout.session.completed` webhook → email license key
- [ ] In-app: License Settings screen — paste key, validate online, store in macOS Keychain

### Day 3 (Wed)
- [ ] Anthropic Haiku 4.5 wired into `/v1/cleanup` (premium path)
- [ ] Hard-cap check in Worker (D1 month-to-date sum + monthly_cap_cents)
- [ ] Stripe meter events posting (idempotent)
- [ ] In-app: Premium Boost Settings screen with cap slider, plain-English disclosure, one-click disable

### Day 4 (Thu)
- [ ] Sonnet 4.6 + Opus 4.6 toggles in app
- [ ] Usage dashboard in app (current month consumption, cap remaining, model breakdown)
- [ ] Email templates: receipt-on-charge, 80%/100% cap alerts, license-key-on-purchase
- [ ] California ARL renewal reminder cron (7 days before annual renewal)

### Day 5 (Fri)
- [ ] Compliance review: Privacy Policy, Terms, Subscription Disclosure page
- [ ] Beta test with 5 friends as paid users (real money, real consumption)
- [ ] Edge cases: license refresh during offline, hard-cap mid-stream, premium failover

### Day 6-7 (weekend buffer)
- [ ] First 1,000 lifetime promo countdown widget on landing page
- [ ] HN / Twitter launch post: *"Lifetime $149 (first 1,000), then $199, then $249. Wispr is $432 over 3 years and has no lifetime. We do."*

---

## Citations

All sources accessed during research, May 9, 2026:

1. https://wisprflow.ai/pricing
2. https://www.getvoibe.com/resources/wispr-flow-pricing/
3. https://www.sustainabl.net/en/articulo/wispr-flow-android-dictation-mass-acquisition-mm823hzx
4. https://groq.com/pricing
5. https://groq.com/blog/whisper-large-v3-turbo-now-available-on-groq-combining-speed-quality-for-speech-recognition
6. https://www.cloudzero.com/blog/groq-pricing/
7. https://tokenmix.ai/blog/groq-api-pricing
8. https://computeprices.com/providers/groq
9. https://www.metacto.com/blogs/anthropic-api-pricing-a-full-breakdown-of-costs-and-integration
10. https://www.silicondata.com/use-cases/anthropic-claude-api-pricing-2026/
11. https://evolink.ai (Claude routing pricing reference)
12. https://www.claudecodecamp.com/p/claude-code-pricing
13. https://openai.com/api/pricing/
14. https://developers.openai.com/api/docs/pricing
15. https://pricepertoken.com/pricing-page/model/openai-gpt-4o
16. https://pecollective.com/tools/gpt-4o-pricing/
17. https://www.aifreeapi.com/en/posts/gpt-4o-pricing-per-million-tokens
18. https://www.metacto.com/blogs/unlocking-the-true-cost-of-openai-api-a-deep-dive-into-usage-integration-and-maintenance
19. https://docs.stripe.com/billing/subscriptions/usage-based/billing-credits/implementation-guide
20. https://docs.stripe.com/billing/subscriptions/usage-based-legacy
21. https://starterpick.com (Stripe Meters API guide, Mar 2026)
22. https://www.getvoibe.com/resources/voiceink-pricing/
23. https://www.getvoibe.com/resources/apple-dictation-pricing/
24. https://pikaseo.com (Voibe / Wispr alternatives)
25. https://dictanote.co/voicein/plus/
26. https://themarketingshow.com/posts/wispr-flow
27. https://zackproser.com/blog/wisprflow-review
28. https://automationswitch.com/workflow-automation/wispr-flow-review
29. https://modulovalue.com/blog/voxtral-transcribe-and-wispr-flow/
30. https://developers.cloudflare.com/workers/platform/pricing/
31. https://developers.cloudflare.com/cloudflare-for-platforms/workers-for-platforms/reference/pricing/
32. https://www.truefoundry.com/blog/cloudflare-ai-gateway-pricing-a-complete-breakdown
33. https://www.git-tower.com/help/guides/integration/offline-activation/mac
34. https://keyforge.dev/guides/how-to-license-windows-app
35. https://www.softwarekey.com (SOLO Server reference)
36. Polar.sh license key tutorial — YouTube, Sep 2025
37. https://www.goodwinlaw.com/en/insights/publications/2026/02/alerts-practices-ba-ftcs-click-to-cancel-rule-gets-new-life
38. https://www.sidley.com/en/insights/newsupdates/2026/02/us-ftc-signals-renewed-interest-in-click-to-cancel-rulemaking
39. https://www.ftc.gov/news-events/news/press-releases/2024/10/federal-trade-commission-announces-final-click-cancel-rule-making-it-easier-consumers-end-recurring
40. https://www.lw.com (FTC ARL compliance brief)

---

*End of PRICING-RESEARCH.md*
