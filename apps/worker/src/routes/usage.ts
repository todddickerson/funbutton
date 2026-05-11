import { Hono } from 'hono';
import type { Context } from 'hono';
import type { Env, UsageRecord } from '../types';
import { authenticate, authError } from '../lib/auth';

const app = new Hono<{ Bindings: Env }>();

async function handleUsage(c: Context<{ Bindings: Env }>) {
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);
  const license = auth.license;

  const stub = c.env.USAGE_DO.get(c.env.USAGE_DO.idFromName(license.license_id));
  const usage = (await (await stub.fetch('https://do/get')).json()) as UsageRecord;
  const capStr = (await c.env.CAP_KV.get(license.license_id)) ?? '2000';
  const capCents = parseInt(capStr, 10);

  const premiumTotalWords =
    usage['premium-haiku'].words_in + usage['premium-haiku'].words_out +
    usage['premium-sonnet'].words_in + usage['premium-sonnet'].words_out +
    usage['premium-opus'].words_in + usage['premium-opus'].words_out +
    usage['premium-gpt41'].words_in + usage['premium-gpt41'].words_out;

  const includedRemaining = Math.max(0, license.included_premium_words - premiumTotalWords);

  return c.json({
    month: usage.month,
    fast_words: usage.fast.words_in + usage.fast.words_out,
    premium_words: {
      haiku: usage['premium-haiku'].words_in + usage['premium-haiku'].words_out,
      sonnet: usage['premium-sonnet'].words_in + usage['premium-sonnet'].words_out,
      opus: usage['premium-opus'].words_in + usage['premium-opus'].words_out,
      gpt41: usage['premium-gpt41'].words_in + usage['premium-gpt41'].words_out,
    },
    premium_spend_cents: usage.premium_spend_cents,
    cap_cents: capCents,
    included_premium_words: license.included_premium_words,
    included_premium_words_remaining: includedRemaining,
    history: usage.history,
  });
}

app.get('/', handleUsage);
app.post('/', handleUsage);

// Cap update (slider in Settings) — guarded by JWT.
app.post('/cap', async (c) => {
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);
  let body: { cap_cents?: number };
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'invalid_json' }, 400);
  }
  const cap = body.cap_cents;
  if (typeof cap !== 'number' || cap < 0 || cap > 10000) {
    return c.json({ error: 'cap_out_of_range', range: [0, 10000] }, 400);
  }
  await c.env.CAP_KV.put(auth.license.license_id, String(Math.floor(cap)));
  return c.json({ ok: true, cap_cents: Math.floor(cap) });
});

export default app;
