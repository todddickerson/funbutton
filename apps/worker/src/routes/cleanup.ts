import { Hono } from 'hono';
import type { CleanupRequest, Env, UsageRecord } from '../types';
import { authenticate, authError } from '../lib/auth';
import { rateLimitExceeded } from '../lib/rate-limit';
import { callProvider, ProviderError } from '../lib/providers';
import { calcCost, projectedCost } from '../lib/cost';
import { countWords, todayISO } from '../lib/words';
import { meteredPriceForModel, reportMeteredUsage } from '../lib/stripe';

const app = new Hono<{ Bindings: Env }>();

app.post('/', async (c) => {
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);
  const license = auth.license;

  if (await rateLimitExceeded(license.license_id, c.env.RATE_LIMIT_KV)) {
    return c.json({ error: 'rate_limited' }, 429);
  }

  let body: CleanupRequest;
  try {
    body = await c.req.json<CleanupRequest>();
  } catch {
    return c.json({ error: 'invalid_json' }, 400);
  }
  const model = body.model;
  const transcript = body.transcript ?? '';
  const mode = body.mode ?? 'auto';
  const dictionary = body.dictionary ?? [];

  if (!model || !transcript) {
    return c.json({ error: 'missing_fields', required: ['model', 'transcript'] }, 400);
  }

  const words_in = countWords(transcript);

  // Pull usage for cap checks (also needed downstream).
  const doStub = c.env.USAGE_DO.get(c.env.USAGE_DO.idFromName(license.license_id));
  const usage = (await (await doStub.fetch('https://do/get')).json()) as UsageRecord;

  if (model.startsWith('premium-')) {
    // Pro tier: check included quota first (counted across all premium models).
    let withinIncluded = false;
    if (license.tier.startsWith('pro_') && license.included_premium_words > 0) {
      const premiumUsed =
        usage['premium-haiku'].words_in + usage['premium-haiku'].words_out +
        usage['premium-sonnet'].words_in + usage['premium-sonnet'].words_out +
        usage['premium-opus'].words_in + usage['premium-opus'].words_out +
        usage['premium-gpt41'].words_in + usage['premium-gpt41'].words_out;
      if (premiumUsed + words_in * 2 <= license.included_premium_words) {
        withinIncluded = true;
      }
    }

    if (!withinIncluded) {
      const proj = projectedCost(model, words_in);
      const capCents = parseInt((await c.env.CAP_KV.get(license.license_id)) ?? '2000', 10);
      if (capCents === 0 || usage.premium_spend_cents + proj > capCents) {
        return c.json(
          {
            fallback: 'fast',
            reason: 'cap_exceeded',
            usage,
            cap_cents: capCents,
          },
          402
        );
      }
    }
  }

  let result;
  try {
    result = await callProvider({ model, transcript, mode, dictionary, env: c.env });
  } catch (e) {
    if (e instanceof ProviderError) {
      return c.json(
        { error: 'cleanup_failed', provider: e.provider, status: e.status },
        502
      );
    }
    throw e;
  }
  const words_out = countWords(result.text);
  const cost_cents = calcCost(model, words_in, words_out);

  // Fire-and-forget side effects: DO counter, D1 audit, Stripe metered.
  const ctx = c.executionCtx;
  ctx.waitUntil(
    doStub.fetch('https://do/record', {
      method: 'POST',
      body: JSON.stringify({ model, words_in, words_out, cost_cents }),
    })
  );

  const idem = `${license.license_id}:${todayISO()}:${model}`;
  ctx.waitUntil(
    c.env.D1_DATABASE.prepare(
      'INSERT INTO usage_log (license_id, model, words_in, words_out, cost_cents, ts, stripe_idempotency_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)'
    )
      .bind(license.license_id, model, words_in, words_out, cost_cents, Date.now(), idem)
      .run()
      .catch((err) => console.error('d1 insert failed', err))
  );

  if (
    model.startsWith('premium-') &&
    cost_cents > 0 &&
    license.stripe_subscription_id &&
    c.env.STRIPE_SECRET_KEY
  ) {
    // We resolve subscription_item lazily via metadata stored in LICENSE_KV under
    // `${license_id}:sub_items` as a JSON map { model: subscription_item_id }.
    ctx.waitUntil(
      (async () => {
        try {
          const map = await c.env.LICENSE_KV.get(`${license.license_id}:sub_items`, 'json') as
            | Record<string, string>
            | null;
          const subItem = map?.[model];
          if (!subItem) return; // not configured; webhook will hydrate on subscription created
          // For prepaid Pro included words, we report 0 (Stripe knows usage but bills only above included).
          await reportMeteredUsage(c.env, {
            subscriptionItemId: subItem,
            quantity: words_in + words_out,
            idempotencyKey: idem,
          });
        } catch (err) {
          console.error('stripe usage record failed', err);
        }
      })()
    );
    // silence unused-import warning when we have stripe wired but want symmetry
    void meteredPriceForModel;
  }

  return c.json({
    text: result.text,
    model_used: model,
    words_in,
    words_out,
    cost_cents,
  });
});

export default app;
