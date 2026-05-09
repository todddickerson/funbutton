# WORKER-SPEC.md — FunButton.ai Cloudflare Worker (Week 2 Monetization)

**Status:** Implementation-ready. The Week 2 coding agent works from this file.
**Repo target:** `~/src/Github/funbutton/apps/worker/` (new Tauri sibling to `apps/desktop`)
**Deploy target:** `api.funbutton.ai` on Cloudflare Workers (personal CF account `e03523c149209369c46ebc10b8a30b43`)
**Pricing source of truth:** `PRD.md` → "Pricing (LOCKED)" + `PRICING-RESEARCH.md` (40 sources)

---

## 1. Architecture Overview

```
                                    ┌──────────────────────┐
                                    │  api.funbutton.ai    │
                                    │  Cloudflare Worker   │
┌──────────────┐    JWT Bearer      │                      │      ┌─────────────┐
│ FunButton.app│ ──────────────────▶│ /v1/transcribe       │─────▶│ Groq        │
│ (Tauri/Rust) │                    │ /v1/cleanup          │      │ Anthropic   │
│              │ ◀──────────────────│ /v1/license/verify   │      │ OpenAI      │
└──────────────┘  cleaned text +    │ /v1/license/refresh  │      └─────────────┘
                  usage metadata    │ /v1/usage            │
                                    │ /v1/stripe/webhook   │      ┌─────────────┐
                                    │                      │─────▶│ Stripe API  │
                                    └─────┬─────┬──────────┘      └─────────────┘
                                          │     │
                            ┌─────────────┘     └─────────────┐
                            ▼                                 ▼
                    ┌───────────────┐              ┌─────────────────┐
                    │  KV / DO      │              │  D1 Database    │
                    │  - LICENSE_KV │              │  usage_log      │
                    │  - USAGE_KV   │              │  (audit trail)  │
                    │  - CAP_KV     │              └─────────────────┘
                    │  - USAGE_DO   │
                    │    (counters) │
                    └───────────────┘
```

### Bindings

| Type | Name | Purpose |
|---|---|---|
| KV | `LICENSE_KV` | License JWT state, cancellation flags. Key = `license_id`. |
| KV | `USAGE_KV` | Per-user word counts (rolling 30-day TTL). Key = `${license_id}:${YYYY-MM}`. |
| KV | `CAP_KV` | User-configured monthly caps in cents. Key = `license_id`. Default `2000` ($20). |
| KV | `RATE_LIMIT_KV` | Per-license rate limit tokens. Key = `${license_id}:${minute_bucket}`. |
| Durable Object | `USAGE_DO` | Atomic per-license counter for current month. Class `UsageCounter`. |
| D1 | `D1_DATABASE` | Audit log of every usage event for billing reconciliation. |
| Secret | `JWT_SECRET` | HS256 signing key for license JWTs. |
| Secret | `STRIPE_SECRET_KEY` | Stripe live key. |
| Secret | `STRIPE_WEBHOOK_SECRET` | Stripe webhook signing secret. |
| Secret | `GROQ_API_KEY` | Groq Whisper Turbo + Llama 3.3 70B. |
| Secret | `ANTHROPIC_API_KEY` | Haiku 4.5 / Sonnet 4.7 / Opus 4.7. |
| Secret | `OPENAI_API_KEY` | GPT-4.1 (replaces deprecating GPT-4o). |

---

## 2. Endpoints

### `POST /v1/license/verify`
Verifies a JWT is still valid (signature + not expired + not cancelled). Called by desktop app at launch.

**Request:** `Authorization: Bearer <jwt>`
**Response:** `{ valid: true, tier: "pro_annual", expires_at: 1234567890, included_premium_words: 50000, words_used_this_month: 12450, cap_cents: 2000 }`

### `POST /v1/license/refresh`
Issues a fresh 30-day JWT. Called by desktop app every 30 days (or earlier on 401).

**Request:** `Authorization: Bearer <jwt>` (existing, may be near expiry)
**Response:** `{ jwt: "<new>", refresh_at: 1234567890 }`

### `POST /v1/transcribe`
Audio → text via Groq Whisper Turbo. Always uses Groq fast tier (transcription cost is negligible; we don't meter it).

**Request:**
```
Authorization: Bearer <jwt>
Content-Type: audio/wav | audio/flac | audio/mp3
Body: raw audio bytes (≤25 MB, ≤60 s)
```
**Response:** `{ text: "raw transcript...", duration_ms: 1234, words: 87 }`

### `POST /v1/cleanup`
Transcript → cleaned text. Hot path. **THIS IS WHERE METERING HAPPENS.** See pseudocode in §6.

**Request:**
```json
{
  "model": "fast" | "premium-haiku" | "premium-sonnet" | "premium-opus" | "premium-gpt41",
  "transcript": "...",
  "mode": "auto" | "email" | "slack" | "code" | "raw",
  "dictionary": ["Spontent", "ClickFunnels", ...]
}
```
**Response (success):** `{ text: "cleaned text", model_used: "premium-haiku", words_in: 87, words_out: 84, cost_cents: 4 }`
**Response (cap exceeded):** `{ fallback: "fast", reason: "cap_exceeded", usage: {...}, cap_cents: 2000 }` (HTTP 402)

### `POST /v1/usage`
Returns current month usage for the dashboard. Called by app Settings UI.

**Response:**
```json
{
  "month": "2026-05",
  "fast_words": 124500,
  "premium_words": { "haiku": 8200, "sonnet": 0, "opus": 0, "gpt41": 0 },
  "premium_spend_cents": 33,
  "cap_cents": 2000,
  "included_premium_words_remaining": 41800,
  "history": [{ "day": "2026-05-09", "fast": 4200, "premium": 800, "spend_cents": 3 }, ...]
}
```

### `POST /v1/stripe/webhook`
Stripe webhook handler. Signature-verified. Handles:
- `checkout.session.completed` → mint license JWT, store in `LICENSE_KV`, email JWT activation link
- `customer.subscription.updated` → tier change, update `LICENSE_KV`
- `customer.subscription.deleted` → mark cancelled in `LICENSE_KV` (JWT remains valid until `expires_at`)
- `invoice.paid` → renewal, extend `expires_at`
- `customer.subscription.trial_will_end` → notify app (future: trial mechanic)
- **Lifetime sales counter:** on each `checkout.session.completed` for a Lifetime price, atomically increment counter; at 1000 → swap active price ID from `lifetime_149` → `lifetime_199`; at 5000 → `lifetime_199` → `lifetime_249`. Update via Stripe Price archive + new active.

---

## 3. Auth Flow (License JWT)

1. **Purchase:** User completes Stripe Checkout (Pro monthly/annual or Lifetime tier).
2. **Webhook → mint JWT:** Worker generates an HS256 JWT signed with `JWT_SECRET`:
   ```json
   {
     "license_id": "lic_01HXY...",
     "tier": "pro_annual" | "pro_monthly" | "lifetime_149" | "lifetime_199" | "lifetime_249",
     "stripe_customer_id": "cus_...",
     "stripe_subscription_id": "sub_..." (null for lifetime),
     "included_premium_words": 50000 (Pro) | 0 (Lifetime),
     "issued_at": 1234567890,
     "expires_at": 1234567890 + 30d (Pro) | never (Lifetime; we still rotate every 90d),
     "refresh_at": 1234567890 + 28d
   }
   ```
3. **Activation email:** Worker sends an email via Resend (or Cloudflare Email Workers) with a `funbutton://activate?jwt=...` deep link. Desktop app handles the URL scheme, stores JWT in macOS Keychain via `tauri-plugin-stronghold`.
4. **Every API call:** Desktop sends `Authorization: Bearer <jwt>`. Worker verifies signature, checks `expires_at`, checks cancellation flag in `LICENSE_KV`.
5. **Auto-refresh:** Desktop app checks `refresh_at` on every launch and on a 24h timer; calls `/v1/license/refresh` when `now > refresh_at`. Worker issues a fresh JWT (same `license_id`, fresh `expires_at` and `refresh_at`).
6. **Revocation:** Cancellation, fraud, or chargeback → set `LICENSE_KV[license_id].revoked = true`. Worker rejects all calls. App shows "License revoked" Settings state with "Contact support" link.

---

## 4. Cap Enforcement (THE STRIPE GOTCHA)

**Why this is critical:** Stripe `billing_thresholds` only **invoices** you when crossed — it does NOT pause access. A user could rack up $10,000 in API calls before the threshold-triggered invoice arrives. We MUST enforce caps app-side in the Worker.

**How:**
1. **Before every premium model call**, Worker fetches current month's `premium_spend_cents` from `USAGE_DO` (Durable Object instance keyed by `license_id`).
2. Estimate the projected cost of this call: `cost_cents = ceil(input_words / 1000 * price_per_10k_words / 10)`. (Cleanup output ≈ input length, cost is dominated by input + output combined; conservative estimate uses input × 2.)
3. Fetch user's cap from `CAP_KV[license_id]` (default 2000 = $20).
4. If `current_spend + projected_cost > cap`:
   - Return **HTTP 402 Payment Required** with `{ fallback: "fast", reason: "cap_exceeded", usage, cap_cents }`.
   - Desktop app falls back silently to Groq fast tier and shows a toast: "Monthly cap hit. Switched to fast tier. Adjust in Settings."
5. If under cap: proceed, then atomically increment counter via `USAGE_DO.recordUsage()`.
6. **Hard rate limit (anti-abuse):** 50 req/min per license, sliding window via `RATE_LIMIT_KV`. Return 429 if exceeded. (Independent of cap — applies even to fast tier.)

**Atomic counter via Durable Object** (because multiple Worker instances may serve the same license simultaneously):
```ts
export class UsageCounter {
  state: DurableObjectState
  constructor(state: DurableObjectState) { this.state = state }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url)
    if (url.pathname === '/get') {
      const month = await this.state.storage.get<UsageRecord>('current_month')
      return Response.json(month ?? emptyMonth())
    }
    if (url.pathname === '/record') {
      const { model, words_in, words_out, cost_cents } = await req.json()
      const month = (await this.state.storage.get<UsageRecord>('current_month')) ?? emptyMonth()
      month.premium_spend_cents += cost_cents
      month[model] ||= { words_in: 0, words_out: 0 }
      month[model].words_in += words_in
      month[model].words_out += words_out
      await this.state.storage.put('current_month', month)
      return Response.json(month)
    }
    if (url.pathname === '/reset') {
      await this.state.storage.put('current_month', emptyMonth())
      return Response.json({ ok: true })
    }
    return new Response('Not found', { status: 404 })
  }
}
```
**Monthly reset:** Cron trigger at 00:00 UTC on the 1st calls `/reset` on every active `USAGE_DO`. (For Pro tier, this also resets the 50K included-premium-words allowance.)

---

## 5. Model Routing

App sends a `model` preference; Worker maps to provider + model + price.

| `model` (app) | Provider | API Model | Use For | Price/10K words |
|---|---|---|---|---|
| `fast` | Groq | `llama-3.3-70b-versatile` | Default cleanup, free tier, fallback | $0 (included) |
| `premium-haiku` | Anthropic | `claude-haiku-4-5` | Pro default premium | $0.40 |
| `premium-sonnet` | Anthropic | `claude-sonnet-4-7` | Long-form, nuanced | $0.60 |
| `premium-opus` | Anthropic | `claude-opus-4-7` | Reasoning-heavy | $0.99 |
| `premium-gpt41` | OpenAI | `gpt-4.1` | Alternative provider | $0.50 |

**Transcription is always Groq Whisper Turbo (`whisper-large-v3-turbo`).** Cost is negligible (~$0.04/hr audio); we don't meter it separately.

**Cost calculation** (premium models only):
```ts
function calcCost(model: string, words_in: number, words_out: number): number {
  const total_words = words_in + words_out
  const rates = {
    'premium-haiku': 40,    // cents per 10K words
    'premium-sonnet': 60,
    'premium-opus': 99,
    'premium-gpt41': 50,
  }
  return Math.ceil((total_words / 10000) * rates[model])
}
```

---

## 6. Hot Path Pseudocode (`POST /v1/cleanup`)

```ts
import type { Env } from './env'

export default {
  async fetch(req: Request, env: Env, ctx: ExecutionContext): Promise<Response> {
    // 1. Verify JWT
    const jwt = req.headers.get('Authorization')?.replace('Bearer ', '')
    if (!jwt) return new Response('Missing Authorization', { status: 401 })
    const license = await verifyJWT(jwt, env.JWT_SECRET)
    if (!license || license.expires_at < Date.now()) {
      return new Response('Unauthorized', { status: 401 })
    }
    const revoked = await env.LICENSE_KV.get(`${license.license_id}:revoked`)
    if (revoked) return new Response('License revoked', { status: 403 })

    // 2. Rate limit (50 req/min per license)
    if (await rateLimitExceeded(license.license_id, env.RATE_LIMIT_KV)) {
      return new Response('Rate limited', { status: 429 })
    }

    // 3. Parse request
    const { model, transcript, mode, dictionary } = await req.json<CleanupRequest>()
    const words_in = countWords(transcript)

    // 4. If premium: check cap BEFORE calling provider
    if (model.startsWith('premium-')) {
      const doId = env.USAGE_DO.idFromName(license.license_id)
      const usageStub = env.USAGE_DO.get(doId)
      const usage = await (await usageStub.fetch('https://do/get')).json<UsageRecord>()

      // Pro tier: check included-words quota first
      let useIncluded = false
      if (license.tier.startsWith('pro_') && license.included_premium_words > 0) {
        const used = (usage.premium_haiku?.words_in ?? 0) + (usage.premium_haiku?.words_out ?? 0)
        if (used + words_in * 2 <= license.included_premium_words) {
          useIncluded = true
        }
      }

      if (!useIncluded) {
        // Pay-as-you-go: check cap
        const projectedCost = calcCost(model, words_in, words_in) // estimate output ≈ input
        const capCents = parseInt(await env.CAP_KV.get(license.license_id) ?? '2000', 10)

        if (capCents === 0 || usage.premium_spend_cents + projectedCost > capCents) {
          // Hard stop — fall back
          return Response.json(
            {
              fallback: 'fast',
              reason: 'cap_exceeded',
              usage,
              cap_cents: capCents,
            },
            { status: 402 }
          )
        }
      }
    }

    // 5. Route to provider
    const result = await callProvider(model, transcript, mode, dictionary, env)
    const words_out = countWords(result.text)
    const cost_cents = model.startsWith('premium-') ? calcCost(model, words_in, words_out) : 0

    // 6. Log usage atomically
    const doId = env.USAGE_DO.idFromName(license.license_id)
    const usageStub = env.USAGE_DO.get(doId)
    ctx.waitUntil(
      usageStub.fetch('https://do/record', {
        method: 'POST',
        body: JSON.stringify({ model, words_in, words_out, cost_cents }),
      })
    )

    // 7. Audit log to D1 (fire-and-forget)
    ctx.waitUntil(
      env.D1_DATABASE.prepare(
        'INSERT INTO usage_log (license_id, model, words_in, words_out, cost_cents, ts) VALUES (?, ?, ?, ?, ?, ?)'
      )
        .bind(license.license_id, model, words_in, words_out, cost_cents, Date.now())
        .run()
    )

    // 8. Stripe metered usage record (Pro tier overage only — Lifetime uses a $0/mo sub for tracking too)
    if (model.startsWith('premium-') && cost_cents > 0 && license.stripe_subscription_id) {
      const subItemId = subscriptionItemForModel(license, model) // map model → metered Price's sub_item id
      const idempotencyKey = `${license.license_id}:${todayISO()}:${model}`
      ctx.waitUntil(postStripeUsage(subItemId, words_in + words_out, idempotencyKey, env))
    }

    return Response.json({
      text: result.text,
      model_used: model,
      words_in,
      words_out,
      cost_cents,
    })
  },
}
```

---

## 7. Stripe Schema

**Products & Prices:**

| Product | Price ID (env) | Type | Amount |
|---|---|---|---|
| FunButton Pro | `STRIPE_PRICE_PRO_MONTHLY` | recurring (1mo) | $9.00 |
| FunButton Pro | `STRIPE_PRICE_PRO_ANNUAL` | recurring (1yr) | $79.00 |
| FunButton Lifetime (Founder) | `STRIPE_PRICE_LIFETIME_149` | one_time | $149.00 |
| FunButton Lifetime (Early) | `STRIPE_PRICE_LIFETIME_199` | one_time | $199.00 |
| FunButton Lifetime (Standard) | `STRIPE_PRICE_LIFETIME_249` | one_time | $249.00 |
| FunButton Premium Haiku (metered) | `STRIPE_PRICE_METER_HAIKU` | recurring metered | $0.40 / 10K words |
| FunButton Premium Sonnet (metered) | `STRIPE_PRICE_METER_SONNET` | recurring metered | $0.60 / 10K words |
| FunButton Premium Opus (metered) | `STRIPE_PRICE_METER_OPUS` | recurring metered | $0.99 / 10K words |
| FunButton Premium GPT-4.1 (metered) | `STRIPE_PRICE_METER_GPT41` | recurring metered | $0.50 / 10K words |

**Subscription assembly:**
- **Pro tier subscription:** `[pro_monthly_9 OR pro_annual_79] + [meter_haiku, meter_sonnet, meter_opus, meter_gpt41]` (4 metered items, $0 unless usage reported)
- **Lifetime subscription:** $0/mo recurring sub with the 4 metered items only. The Lifetime tier is purchased as a one-time payment, but we attach a $0 sub to the same customer for usage tracking. When user upgrades premium model usage, those metered records bill the customer's saved card monthly.

**Lifetime ladder auto-bump (in webhook handler):**
```ts
async function handleCheckoutCompleted(session, env) {
  const priceId = session.line_items.data[0].price.id
  if (priceId === env.STRIPE_PRICE_LIFETIME_149 ||
      priceId === env.STRIPE_PRICE_LIFETIME_199) {
    const count = await env.LIFETIME_COUNTER.idFromName('global').get().increment()
    if (count === 1000 && priceId === env.STRIPE_PRICE_LIFETIME_149) {
      await archivePrice(env.STRIPE_PRICE_LIFETIME_149, env)
      await activatePrice(env.STRIPE_PRICE_LIFETIME_199, env)
    } else if (count === 5000 && priceId === env.STRIPE_PRICE_LIFETIME_199) {
      await archivePrice(env.STRIPE_PRICE_LIFETIME_199, env)
      await activatePrice(env.STRIPE_PRICE_LIFETIME_249, env)
    }
  }
  // ... mint license JWT, send activation email
}
```

---

## 8. D1 Schema (audit trail)

```sql
CREATE TABLE usage_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  license_id TEXT NOT NULL,
  model TEXT NOT NULL,
  words_in INTEGER NOT NULL,
  words_out INTEGER NOT NULL,
  cost_cents INTEGER NOT NULL,
  ts INTEGER NOT NULL,
  stripe_idempotency_key TEXT
);
CREATE INDEX idx_usage_license_ts ON usage_log(license_id, ts DESC);
CREATE INDEX idx_usage_ts ON usage_log(ts);

CREATE TABLE licenses (
  license_id TEXT PRIMARY KEY,
  stripe_customer_id TEXT NOT NULL,
  stripe_subscription_id TEXT,
  tier TEXT NOT NULL,
  email TEXT NOT NULL,
  created_at INTEGER NOT NULL,
  expires_at INTEGER,
  cancelled_at INTEGER,
  revoked_at INTEGER
);
```

---

## 9. Compliance UI Requirements (Desktop App Settings)

These are **non-negotiable** for ROSCA + California ARL compliance. The desktop coding agent must build all of these in Week 2 Day 7.

- [ ] **Auto top-up slider** — `$0–$100/mo`, default `$0` (OFF). $0 = hard stop, falls back to Groq fast tier.
- [ ] **Activation disclosure** — when user moves slider above $0:
  > "By enabling auto top-up, you authorize FunButton to charge your saved card up to $X/mo for premium model usage above any included quota. You can change this amount or disable it anytime in Settings. We'll email you a receipt every month."
  Requires explicit "Enable" button click. No dark patterns.
- [ ] **"View usage" link** → opens `https://api.funbutton.ai/dashboard?jwt=...` (or in-app usage view)
- [ ] **"Cancel subscription" button** — one click in Settings. Opens Stripe customer portal via signed link from `/v1/portal`. Confirms via email.
- [ ] **Monthly receipt email** — itemized: total fast words (free), total premium words by model, total spend. Links to invoice in Stripe portal.
- [ ] **"License" panel in Settings** — shows tier, expires_at (if any), included words remaining, current month spend, current cap, "Upgrade" / "Manage subscription" buttons.
- [ ] **Cap-hit toast** — when Worker returns 402 fallback: "Monthly cap hit ($X). Switched to fast tier. Adjust in Settings → Auto top-up."

---

## 10. Week 2 Ship Plan (7 days)

Day-by-day acceptance criteria. Each day ends with a commit + push + brief PROGRESS.md update.

### Day 1 — Worker scaffold + license verify
- [ ] `apps/worker/` directory, `wrangler.toml`, TypeScript build, deploys to `api.funbutton.ai`
- [ ] `LICENSE_KV` namespace created
- [ ] `JWT_SECRET` set
- [ ] `POST /v1/license/verify` endpoint working with HS256 verify
- [ ] Helper script `scripts/mint-test-license.ts` to issue a test JWT for dev
- [ ] **Acceptance:** `curl -H "Authorization: Bearer <test_jwt>" https://api.funbutton.ai/v1/license/verify` returns license info

### Day 2 — Stripe Checkout + webhook (Pro tier end-to-end)
- [ ] Stripe products + prices created (Pro monthly $9, Pro annual $79) in test mode
- [ ] `POST /v1/checkout/create-session` returns Stripe Checkout URL
- [ ] `POST /v1/stripe/webhook` handles `checkout.session.completed` → mints JWT, stores in `LICENSE_KV`, sends activation email via Resend
- [ ] D1 `licenses` table populated on purchase
- [ ] **Acceptance:** Buy Pro Monthly with test card `4242...` → receive activation email → JWT validates against `/v1/license/verify`

### Day 3 — Transcribe + cleanup with Groq routing
- [ ] `GROQ_API_KEY` set in Worker secrets
- [ ] `POST /v1/transcribe` proxies audio to Groq Whisper Turbo
- [ ] `POST /v1/cleanup` with `model: "fast"` routes to Groq Llama 3.3 70B
- [ ] Mode-aware cleanup prompts (auto/email/slack/code/raw) ported from desktop app
- [ ] Dictionary boost integrated into prompt
- [ ] **Acceptance:** End-to-end audio → text → cleanup roundtrip via desktop app pointing at the Worker

### Day 4 — Anthropic + OpenAI premium routing
- [ ] `ANTHROPIC_API_KEY`, `OPENAI_API_KEY` set
- [ ] `model: "premium-haiku" | "premium-sonnet" | "premium-opus"` routes to Anthropic
- [ ] `model: "premium-gpt41"` routes to OpenAI
- [ ] Cost calculation per `calcCost()` in §5
- [ ] **Acceptance:** All 5 model preferences return clean output and accurate `cost_cents`

### Day 5 — Cap enforcement + 402 fallback + rate limiting
- [ ] `USAGE_DO` Durable Object class deployed, atomic counter operations work
- [ ] `CAP_KV` namespace, default $20 cap on new licenses
- [ ] Pre-call cap check returns HTTP 402 with `{ fallback: "fast", ... }` when projected cost would exceed cap
- [ ] 50 req/min rate limit per license (sliding window)
- [ ] Pro tier: included-words quota (50K/mo) checked before metered
- [ ] **Acceptance:** Set cap to $0.10, run premium calls until cap hits, verify 402 fallback. Verify atomic counter under concurrent requests.

### Day 6 — Usage logging + Stripe metered + webhook events
- [ ] D1 `usage_log` populated on every successful call
- [ ] Stripe usage records POST'd via idempotent `usage_records` API (key `${license_id}:${day}:${model}`)
- [ ] Webhook handlers: `customer.subscription.updated`, `customer.subscription.deleted`, `invoice.paid`
- [ ] Lifetime sales counter + auto-bump price ladder ($149 → $199 at 1K, → $249 at 5K)
- [ ] `POST /v1/usage` endpoint returns dashboard data
- [ ] Monthly reset cron at 00:00 UTC on 1st
- [ ] **Acceptance:** End-to-end Pro tier user runs 51K premium words in a month → first 50K free, rest billed via Stripe metered → Stripe invoice next billing cycle reflects overage.

### Day 7 — Desktop Settings UI + license activation + ship
- [ ] Desktop app handles `funbutton://activate?jwt=...` URL scheme, stores JWT in Keychain
- [ ] Settings → License panel: tier, usage, cap slider ($0–$100), "Manage subscription" button
- [ ] Auto top-up activation disclosure modal (§9)
- [ ] Cap-hit toast notification on 402 fallback
- [ ] Monthly receipt email template
- [ ] `POST /v1/portal` endpoint returns Stripe customer portal URL
- [ ] Public landing page on funbutton.ai with new pricing, "Buy Pro" / "Buy Lifetime" buttons
- [ ] Deploy worker to `api.funbutton.ai`, switch desktop app from BYOK-only to JWT-aware
- [ ] **Acceptance:** Real customer can buy Pro Monthly, activate, use premium cleanup, hit cap, fall back, adjust cap, manage sub via portal, cancel — all from desktop app. End-to-end Lifetime $149 purchase also works.

---

## 11. Env Variables Reference

Copy this into `apps/worker/.dev.vars` (gitignored):

```
JWT_SECRET=<openssl rand -hex 32>
STRIPE_SECRET_KEY=sk_test_...
STRIPE_WEBHOOK_SECRET=whsec_...
STRIPE_PRICE_PRO_MONTHLY=price_...
STRIPE_PRICE_PRO_ANNUAL=price_...
STRIPE_PRICE_LIFETIME_149=price_...
STRIPE_PRICE_LIFETIME_199=price_...
STRIPE_PRICE_LIFETIME_249=price_...
STRIPE_PRICE_METER_HAIKU=price_...
STRIPE_PRICE_METER_SONNET=price_...
STRIPE_PRICE_METER_OPUS=price_...
STRIPE_PRICE_METER_GPT41=price_...
GROQ_API_KEY=gsk_...
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
RESEND_API_KEY=re_...
ACTIVATION_EMAIL_FROM=hello@funbutton.ai
```

For production, set via `wrangler secret put <NAME>`. Bindings (`LICENSE_KV`, `USAGE_KV`, `CAP_KV`, `RATE_LIMIT_KV`, `USAGE_DO`, `D1_DATABASE`) go in `wrangler.toml`.

---

## 12. Open Questions for Todd (NOT blockers — defer if needed)

1. **Activation email sender:** Resend vs. Cloudflare Email Workers vs. AgentMail (`hello@funbutton.ai` inbox)?
2. **Customer portal:** Stripe Customer Portal (zero work) vs. custom Worker-served dashboard at `api.funbutton.ai/dashboard`? Recommend Stripe portal for V1.1, custom in V2.
3. **Lifetime $0/mo metered sub:** Stripe allows but requires explicit confirmation flow on purchase. Alternative: track Lifetime usage in Worker only, charge directly via PaymentIntent when threshold hit. Recommend the $0/mo sub approach for cleaner UX.
4. **Trial:** Currently no trial. Wispr/SuperWhisper offer 7-day. Consider in V2.
5. **Annual upgrade incentive:** When user is on Pro Monthly, show in-app "Upgrade to Annual, save $29/yr" prompt every 30 days?

---

## 13. Out of Scope for V1.1 (revisit later)

- Team tier ($7/seat/mo annual, 3-seat min) — V2
- Refund flow automation — manual via Stripe Dashboard until V2
- Per-user dashboard at `api.funbutton.ai/dashboard` — V2 (use Stripe portal for V1.1)
- Telemetry/analytics — V2 (privacy-first PostHog with opt-in)
- Per-region pricing / VAT handling — V2 (Stripe Tax handles for now)
- Multiple payment methods (Apple Pay button on landing) — V2

---

**End of spec.** Read `PRD.md` "Pricing (LOCKED)" + `PRICING-RESEARCH.md` for the why. Ship in 7 days.
