import { Hono } from 'hono';
import type { Env } from '../types';
import { authenticate, authError } from '../lib/auth';
import {
  createBillingPortalSession,
  createCheckoutSession,
  lifetimeTierFromPriceId,
  proTierFromPriceId,
} from '../lib/stripe';

const app = new Hono<{ Bindings: Env }>();

// Unauthenticated: brand-new user buying their first license.
// Authenticated optional — if they pass a JWT we can prefill email.
app.post('/create-session', async (c) => {
  if (!c.env.STRIPE_SECRET_KEY) {
    return c.json({ error: 'stripe_not_configured' }, 503);
  }
  let body: { price_id?: string; email?: string };
  try {
    body = await c.req.json();
  } catch {
    return c.json({ error: 'invalid_json' }, 400);
  }
  const priceId = body.price_id ?? '';
  if (!priceId) return c.json({ error: 'missing_price_id' }, 400);

  const proTier = proTierFromPriceId(c.env, priceId);
  const lifetimeTier = lifetimeTierFromPriceId(c.env, priceId);
  if (!proTier && !lifetimeTier) {
    return c.json({ error: 'unknown_price_id' }, 400);
  }

  const isSubscription = !!proTier;
  const additionalMeteredItems = isSubscription
    ? [
        c.env.STRIPE_PRICE_METER_HAIKU,
        c.env.STRIPE_PRICE_METER_SONNET,
        c.env.STRIPE_PRICE_METER_OPUS,
        c.env.STRIPE_PRICE_METER_GPT41,
      ].filter(Boolean)
    : [];

  const session = await createCheckoutSession(c.env, {
    priceId,
    mode: isSubscription ? 'subscription' : 'payment',
    customerEmail: body.email,
    successUrl: `${c.env.APP_URL}/welcome?session_id={CHECKOUT_SESSION_ID}`,
    cancelUrl: `${c.env.APP_URL}/pricing?cancelled=1`,
    metadata: { tier: proTier ?? lifetimeTier ?? '' },
    additionalSubscriptionItems: additionalMeteredItems,
  });
  return c.json({ url: session.url, id: session.id });
});

// Authenticated: open Stripe portal for the customer.
app.post('/portal', async (c) => {
  if (!c.env.STRIPE_SECRET_KEY) {
    return c.json({ error: 'stripe_not_configured' }, 503);
  }
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);

  const session = await createBillingPortalSession(c.env, {
    customerId: auth.license.stripe_customer_id,
    returnUrl: `${c.env.APP_URL}/`,
  });
  return c.json({ url: session.url });
});

export default app;
