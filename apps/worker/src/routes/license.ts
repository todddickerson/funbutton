import { Hono } from 'hono';
import type { Env } from '../types';
import { authenticate, authError } from '../lib/auth';
import { refreshClaims, signJWT } from '../lib/jwt';
import { thisMonthKey } from '../lib/words';

const app = new Hono<{ Bindings: Env }>();

app.post('/verify', async (c) => {
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);
  const license = auth.license;

  // Pull current month usage to surface in the response.
  const stub = c.env.USAGE_DO.get(c.env.USAGE_DO.idFromName(license.license_id));
  const usage = (await (await stub.fetch('https://do/get')).json()) as {
    month: string;
    premium_spend_cents: number;
    'premium-haiku': { words_in: number; words_out: number };
  };
  const capStr = (await c.env.CAP_KV.get(license.license_id)) ?? '2000';
  const wordsHaiku = usage['premium-haiku'].words_in + usage['premium-haiku'].words_out;

  return c.json({
    valid: true,
    license_id: license.license_id,
    tier: license.tier,
    expires_at: license.expires_at,
    refresh_at: license.refresh_at,
    included_premium_words: license.included_premium_words,
    words_used_this_month: wordsHaiku,
    premium_spend_cents: usage.premium_spend_cents,
    cap_cents: parseInt(capStr, 10),
    month: usage.month ?? thisMonthKey(),
  });
});

app.post('/refresh', async (c) => {
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);
  const next = refreshClaims(auth.license);
  const jwt = await signJWT(next, c.env.JWT_SECRET);
  return c.json({ jwt, refresh_at: next.refresh_at, expires_at: next.expires_at });
});

export default app;
