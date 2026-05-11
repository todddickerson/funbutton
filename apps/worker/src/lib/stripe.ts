// Minimal Stripe REST client + webhook verifier. We avoid the official SDK
// because it pulls Node-only deps; the few endpoints we need are simple form-urlencoded.

import type { Env, Tier } from '../types';

const STRIPE_API = 'https://api.stripe.com/v1';

interface StripeRequest {
  path: string;
  method: 'GET' | 'POST' | 'DELETE';
  form?: Record<string, string>;
  idempotencyKey?: string;
}

async function stripeFetch(
  env: Env,
  { path, method, form, idempotencyKey }: StripeRequest
): Promise<unknown> {
  const headers: Record<string, string> = {
    Authorization: `Bearer ${env.STRIPE_SECRET_KEY}`,
  };
  let body: string | undefined;
  if (form) {
    body = new URLSearchParams(form).toString();
    headers['Content-Type'] = 'application/x-www-form-urlencoded';
  }
  if (idempotencyKey) headers['Idempotency-Key'] = idempotencyKey;

  const res = await fetch(`${STRIPE_API}${path}`, { method, headers, body });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`stripe ${res.status} ${path}: ${text.slice(0, 300)}`);
  }
  return res.json();
}

export interface CheckoutSession {
  id: string;
  url: string;
}

export async function createCheckoutSession(
  env: Env,
  args: {
    priceId: string;
    mode: 'subscription' | 'payment';
    customerEmail?: string;
    successUrl: string;
    cancelUrl: string;
    metadata?: Record<string, string>;
    additionalSubscriptionItems?: string[]; // metered price IDs to attach
  }
): Promise<CheckoutSession> {
  const form: Record<string, string> = {
    mode: args.mode,
    success_url: args.successUrl,
    cancel_url: args.cancelUrl,
    'line_items[0][price]': args.priceId,
    'line_items[0][quantity]': '1',
  };
  if (args.customerEmail) form.customer_email = args.customerEmail;
  // Attach metered items so subscription has them from day 1 (subscription mode only).
  if (args.mode === 'subscription' && args.additionalSubscriptionItems) {
    args.additionalSubscriptionItems.forEach((priceId, i) => {
      form[`line_items[${i + 1}][price]`] = priceId;
    });
  }
  if (args.metadata) {
    for (const [k, v] of Object.entries(args.metadata)) {
      form[`metadata[${k}]`] = v;
    }
  }
  return stripeFetch(env, { path: '/checkout/sessions', method: 'POST', form }) as Promise<CheckoutSession>;
}

export async function createBillingPortalSession(
  env: Env,
  args: { customerId: string; returnUrl: string }
): Promise<{ url: string }> {
  const form: Record<string, string> = {
    customer: args.customerId,
    return_url: args.returnUrl,
  };
  return stripeFetch(env, {
    path: '/billing_portal/sessions',
    method: 'POST',
    form,
  }) as Promise<{ url: string }>;
}

export async function reportMeteredUsage(
  env: Env,
  args: {
    subscriptionItemId: string;
    quantity: number;
    idempotencyKey: string;
  }
): Promise<void> {
  await stripeFetch(env, {
    path: `/subscription_items/${args.subscriptionItemId}/usage_records`,
    method: 'POST',
    form: {
      quantity: String(args.quantity),
      action: 'increment',
      timestamp: String(Math.floor(Date.now() / 1000)),
    },
    idempotencyKey: args.idempotencyKey,
  });
}

export async function retrieveCheckoutSession(env: Env, sessionId: string): Promise<unknown> {
  return stripeFetch(env, {
    path: `/checkout/sessions/${sessionId}?expand[]=line_items&expand[]=subscription`,
    method: 'GET',
  });
}

export async function archivePrice(env: Env, priceId: string): Promise<void> {
  await stripeFetch(env, {
    path: `/prices/${priceId}`,
    method: 'POST',
    form: { active: 'false' },
  });
}

export async function activatePrice(env: Env, priceId: string): Promise<void> {
  await stripeFetch(env, {
    path: `/prices/${priceId}`,
    method: 'POST',
    form: { active: 'true' },
  });
}

// -------- Webhook signature verification --------

export interface VerifiedEvent {
  raw: string;
  parsed: {
    id: string;
    type: string;
    data: { object: Record<string, unknown> };
  };
}

export async function verifyWebhook(
  rawBody: string,
  signatureHeader: string,
  secret: string,
  toleranceSeconds = 300
): Promise<VerifiedEvent | null> {
  // Header format: t=...,v1=...,v1=...
  const parts = Object.fromEntries(
    signatureHeader.split(',').map((p) => {
      const i = p.indexOf('=');
      return i < 0 ? [p, ''] : [p.slice(0, i), p.slice(i + 1)];
    })
  ) as Record<string, string>;
  const t = parts.t;
  const v1Sigs = signatureHeader
    .split(',')
    .filter((p) => p.startsWith('v1='))
    .map((p) => p.slice(3));
  if (!t || v1Sigs.length === 0) return null;

  const ts = parseInt(t, 10);
  if (!Number.isFinite(ts)) return null;
  if (Math.abs(Math.floor(Date.now() / 1000) - ts) > toleranceSeconds) return null;

  const signedPayload = `${t}.${rawBody}`;
  const keyObj = await crypto.subtle.importKey(
    'raw',
    new TextEncoder().encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign']
  );
  const macBuf = await crypto.subtle.sign('HMAC', keyObj, new TextEncoder().encode(signedPayload));
  const expected = toHex(new Uint8Array(macBuf));

  let match = false;
  for (const sig of v1Sigs) {
    if (timingSafeEqual(sig, expected)) {
      match = true;
      break;
    }
  }
  if (!match) return null;

  try {
    return { raw: rawBody, parsed: JSON.parse(rawBody) };
  } catch {
    return null;
  }
}

function toHex(bytes: Uint8Array): string {
  let s = '';
  for (const b of bytes) s += b.toString(16).padStart(2, '0');
  return s;
}

function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) return false;
  let result = 0;
  for (let i = 0; i < a.length; i++) result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  return result === 0;
}

// -------- Lifetime tier helpers --------

export function lifetimeTierFromPriceId(env: Env, priceId: string): Tier | null {
  if (priceId === env.STRIPE_PRICE_LIFETIME_149) return 'lifetime_149';
  if (priceId === env.STRIPE_PRICE_LIFETIME_199) return 'lifetime_199';
  if (priceId === env.STRIPE_PRICE_LIFETIME_249) return 'lifetime_249';
  return null;
}

export function proTierFromPriceId(env: Env, priceId: string): Tier | null {
  if (priceId === env.STRIPE_PRICE_PRO_MONTHLY) return 'pro_monthly';
  if (priceId === env.STRIPE_PRICE_PRO_ANNUAL) return 'pro_annual';
  return null;
}

export function meteredPriceForModel(env: Env, model: string): string | null {
  switch (model) {
    case 'premium-haiku':
      return env.STRIPE_PRICE_METER_HAIKU || null;
    case 'premium-sonnet':
      return env.STRIPE_PRICE_METER_SONNET || null;
    case 'premium-opus':
      return env.STRIPE_PRICE_METER_OPUS || null;
    case 'premium-gpt41':
      return env.STRIPE_PRICE_METER_GPT41 || null;
    default:
      return null;
  }
}
