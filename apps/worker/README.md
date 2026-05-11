# FunButton Worker (`apps/worker`)

The Cloudflare Worker that powers paid-tier FunButton. Spec: [`WORKER-SPEC.md`](../../WORKER-SPEC.md).

## Layout

- `src/index.ts` — Hono router, DO exports, cron handler
- `src/routes/*` — endpoint controllers (license, transcribe, cleanup, usage, checkout, webhook)
- `src/lib/*` — JWT, auth, providers (Groq/Anthropic/OpenAI), Stripe, prompts, cost, usage DO
- `migrations/0001_init.sql` — D1 schema
- `scripts/mint-test-license.ts` — issue a dev JWT
- `wrangler.toml` — bindings + cron + production route to `api.funbutton.ai`

## Local dev

```bash
cd apps/worker
pnpm install        # or npm install
cp .dev.vars.example .dev.vars
# Fill JWT_SECRET (openssl rand -hex 32), GROQ_API_KEY, ANTHROPIC_API_KEY, OPENAI_API_KEY
# Stripe vars optional for local — Stripe routes return 503 until set.

pnpm dev            # wrangler dev — runs on http://localhost:8787

# Mint a license for curl testing
pnpm mint-test-license pro_annual todd@example.com
```

## Provisioning (one-time per environment)

```bash
export CLOUDFLARE_API_TOKEN=...   # from ~/clawd/.env CLOUDFLARE_WORKERS_API_TOKEN

# KV
wrangler kv namespace create LICENSE_KV
wrangler kv namespace create USAGE_KV
wrangler kv namespace create CAP_KV
wrangler kv namespace create RATE_LIMIT_KV
# Paste returned IDs into wrangler.toml (both top-level and env.production).

# D1
wrangler d1 create funbutton-prod
# Paste database_id into wrangler.toml; then:
wrangler d1 migrations apply funbutton-prod --remote

# Secrets (production)
wrangler secret put JWT_SECRET --env production
wrangler secret put STRIPE_SECRET_KEY --env production
wrangler secret put STRIPE_WEBHOOK_SECRET --env production
wrangler secret put GROQ_API_KEY --env production
wrangler secret put ANTHROPIC_API_KEY --env production
wrangler secret put OPENAI_API_KEY --env production
wrangler secret put RESEND_API_KEY --env production
# Plus the 9 STRIPE_PRICE_* IDs
```

## Deploy

```bash
pnpm deploy:staging   # *.workers.dev URL for smoke test
pnpm deploy           # production: api.funbutton.ai
```

## Endpoints (all under `/v1`)

| Method | Path | Auth | Purpose |
|---|---|---|---|
| POST | `/license/verify` | Bearer | Validate JWT, return live usage snapshot |
| POST | `/license/refresh` | Bearer | Issue fresh JWT (auto-rotate every 28d) |
| POST | `/transcribe` | Bearer | Audio → text via Groq Whisper Turbo |
| POST | `/cleanup` | Bearer | Transcript → cleaned text (meters + caps premium) |
| GET\|POST | `/usage` | Bearer | Dashboard data: usage, cap, history |
| POST | `/usage/cap` | Bearer | Update user-set monthly cap ($0–$100) |
| POST | `/checkout/create-session` | — | Stripe Checkout URL for Pro / Lifetime |
| POST | `/portal/portal` | Bearer | Stripe Customer Portal URL |
| POST | `/stripe/webhook` | Signature | Stripe events (checkout, sub lifecycle, invoice paid) |

## Cap enforcement

Cap check happens **before** any premium provider call (`src/routes/cleanup.ts`).
Returns HTTP 402 with `{ fallback: "fast", reason: "cap_exceeded", usage, cap_cents }`.
The desktop client must silently retry with `model: "fast"` on 402 and surface a toast.

## Stripe model

Each customer gets at most one license. Pro = subscription with 4 metered items
pre-attached at checkout; Lifetime = one-time payment (the `$0/mo + metered items`
sub is deferred to V2 — for now Lifetime customers pay-per-call via their saved
card on a separate billing relationship, recorded only in the D1 audit log).
