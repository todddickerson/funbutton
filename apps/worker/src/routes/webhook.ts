import { Hono } from 'hono';
import type { Env, Tier } from '../types';
import { newClaims, newLicenseId, signJWT } from '../lib/jwt';
import {
  archivePrice,
  activatePrice,
  lifetimeTierFromPriceId,
  proTierFromPriceId,
  retrieveCheckoutSession,
  verifyWebhook,
} from '../lib/stripe';
import { sendActivationEmail } from '../lib/email';

const app = new Hono<{ Bindings: Env }>();

app.post('/', async (c) => {
  const sig = c.req.header('stripe-signature') ?? '';
  if (!sig) return c.text('missing signature', 400);
  const raw = await c.req.text();
  const verified = await verifyWebhook(raw, sig, c.env.STRIPE_WEBHOOK_SECRET);
  if (!verified) return c.text('invalid signature', 400);

  const event = verified.parsed;
  try {
    switch (event.type) {
      case 'checkout.session.completed':
        await handleCheckoutCompleted(c.env, event);
        break;
      case 'customer.subscription.updated':
        await handleSubscriptionUpdated(c.env, event);
        break;
      case 'customer.subscription.deleted':
        await handleSubscriptionDeleted(c.env, event);
        break;
      case 'invoice.paid':
        await handleInvoicePaid(c.env, event);
        break;
      default:
        // No-op for other events; Stripe still expects 2xx.
        break;
    }
  } catch (err) {
    console.error('webhook handler error', event.type, err);
    return c.text('handler error', 500);
  }
  return c.text('ok', 200);
});

interface StripeEvent {
  id: string;
  type: string;
  data: { object: Record<string, unknown> };
}

async function handleCheckoutCompleted(env: Env, event: StripeEvent) {
  const sessionRef = event.data.object as {
    id: string;
    customer?: string;
    subscription?: string | null;
    customer_email?: string | null;
    customer_details?: { email?: string };
    metadata?: Record<string, string>;
  };
  // Re-fetch with line_items + subscription expansions.
  const session = (await retrieveCheckoutSession(env, sessionRef.id)) as {
    id: string;
    customer: string | null;
    subscription: { id: string; items: { data: Array<{ id: string; price: { id: string } }> } } | string | null;
    customer_email: string | null;
    customer_details?: { email?: string };
    line_items?: { data: Array<{ price: { id: string } }> };
    metadata?: Record<string, string>;
  };

  const email = session.customer_email || session.customer_details?.email || '';
  const customerId = session.customer ?? '';
  const lineItem = session.line_items?.data[0];
  const priceId = lineItem?.price?.id ?? '';
  if (!email || !priceId || !customerId) {
    console.error('checkout missing fields', { email, priceId, customerId });
    return;
  }

  const proTier = proTierFromPriceId(env, priceId);
  const lifetimeTier = lifetimeTierFromPriceId(env, priceId);
  const tier: Tier | null = proTier ?? lifetimeTier;
  if (!tier) {
    console.error('checkout unknown tier for price', priceId);
    return;
  }

  const subscriptionId =
    typeof session.subscription === 'string'
      ? session.subscription
      : session.subscription?.id ?? null;

  const licenseId = newLicenseId();
  const claims = newClaims({
    license_id: licenseId,
    tier,
    stripe_customer_id: customerId,
    stripe_subscription_id: subscriptionId,
    email,
  });
  const jwt = await signJWT(claims, env.JWT_SECRET);

  // Persist license rows + KV.
  await env.LICENSE_KV.put(licenseId, JSON.stringify(claims));
  await env.LICENSE_KV.put(`stripe_customer:${customerId}`, licenseId);

  // Map subscription_item ids per metered price for later usage reporting.
  if (typeof session.subscription === 'object' && session.subscription) {
    const map: Record<string, string> = {};
    for (const item of session.subscription.items.data) {
      const pid = item.price.id;
      if (pid === env.STRIPE_PRICE_METER_HAIKU) map['premium-haiku'] = item.id;
      else if (pid === env.STRIPE_PRICE_METER_SONNET) map['premium-sonnet'] = item.id;
      else if (pid === env.STRIPE_PRICE_METER_OPUS) map['premium-opus'] = item.id;
      else if (pid === env.STRIPE_PRICE_METER_GPT41) map['premium-gpt41'] = item.id;
    }
    if (Object.keys(map).length > 0) {
      await env.LICENSE_KV.put(`${licenseId}:sub_items`, JSON.stringify(map));
    }
  }

  // D1 row
  await env.D1_DATABASE.prepare(
    'INSERT INTO licenses (license_id, stripe_customer_id, stripe_subscription_id, tier, email, created_at, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)'
  )
    .bind(
      licenseId,
      customerId,
      subscriptionId,
      tier,
      email,
      claims.issued_at,
      claims.expires_at
    )
    .run()
    .catch((err) => console.error('d1 licenses insert failed', err));

  // Default cap $20.
  await env.CAP_KV.put(licenseId, '2000');

  // Lifetime ladder auto-bump.
  if (lifetimeTier === 'lifetime_149' || lifetimeTier === 'lifetime_199') {
    try {
      const id = env.LIFETIME_COUNTER.idFromName('global');
      const stub = env.LIFETIME_COUNTER.get(id);
      const { count } = (await (await stub.fetch('https://do/increment')).json()) as { count: number };
      if (count === 1000 && priceId === env.STRIPE_PRICE_LIFETIME_149) {
        await archivePrice(env, env.STRIPE_PRICE_LIFETIME_149);
        await activatePrice(env, env.STRIPE_PRICE_LIFETIME_199);
      } else if (count === 5000 && priceId === env.STRIPE_PRICE_LIFETIME_199) {
        await archivePrice(env, env.STRIPE_PRICE_LIFETIME_199);
        await activatePrice(env, env.STRIPE_PRICE_LIFETIME_249);
      }
    } catch (err) {
      console.error('lifetime counter/bump failed', err);
    }
  }

  await sendActivationEmail(env, { to: email, jwt, tier });
}

async function handleSubscriptionUpdated(env: Env, event: StripeEvent) {
  const sub = event.data.object as {
    id: string;
    customer: string;
    cancel_at_period_end?: boolean;
  };
  const licenseId = await env.LICENSE_KV.get(`stripe_customer:${sub.customer}`);
  if (!licenseId) return;
  const raw = await env.LICENSE_KV.get(licenseId);
  if (!raw) return;
  const claims = JSON.parse(raw);
  claims.stripe_subscription_id = sub.id;
  await env.LICENSE_KV.put(licenseId, JSON.stringify(claims));
}

async function handleSubscriptionDeleted(env: Env, event: StripeEvent) {
  const sub = event.data.object as { id: string; customer: string };
  const licenseId = await env.LICENSE_KV.get(`stripe_customer:${sub.customer}`);
  if (!licenseId) return;
  await env.LICENSE_KV.put(`${licenseId}:revoked`, '1');
  await env.D1_DATABASE.prepare(
    'UPDATE licenses SET cancelled_at = ?1 WHERE license_id = ?2'
  )
    .bind(Date.now(), licenseId)
    .run()
    .catch((err) => console.error('cancellation d1 update failed', err));
}

async function handleInvoicePaid(env: Env, event: StripeEvent) {
  // Pro tier renewal — extend JWT expiry.
  const invoice = event.data.object as {
    customer: string;
    subscription: string;
    period_end?: number;
  };
  const licenseId = await env.LICENSE_KV.get(`stripe_customer:${invoice.customer}`);
  if (!licenseId) return;
  const raw = await env.LICENSE_KV.get(licenseId);
  if (!raw) return;
  const claims = JSON.parse(raw);
  if (invoice.period_end) {
    claims.expires_at = invoice.period_end * 1000;
  }
  await env.LICENSE_KV.put(licenseId, JSON.stringify(claims));
}

export default app;
