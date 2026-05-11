// Minimal HS256 JWT signer/verifier built on Web Crypto.
// We intentionally avoid an npm dep for a 60-line primitive.

import type { LicenseClaims, Tier } from '../types';

const enc = new TextEncoder();
const dec = new TextDecoder();

function base64UrlEncode(bytes: Uint8Array): string {
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/=+$/, '').replace(/\+/g, '-').replace(/\//g, '_');
}

function base64UrlDecode(str: string): Uint8Array {
  const pad = str.length % 4 === 0 ? '' : '='.repeat(4 - (str.length % 4));
  const b64 = (str + pad).replace(/-/g, '+').replace(/_/g, '/');
  const bin = atob(b64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

async function key(secret: string): Promise<CryptoKey> {
  return crypto.subtle.importKey(
    'raw',
    enc.encode(secret),
    { name: 'HMAC', hash: 'SHA-256' },
    false,
    ['sign', 'verify']
  );
}

export async function signJWT(claims: LicenseClaims, secret: string): Promise<string> {
  const header = base64UrlEncode(enc.encode(JSON.stringify({ alg: 'HS256', typ: 'JWT' })));
  const payload = base64UrlEncode(enc.encode(JSON.stringify(claims)));
  const data = `${header}.${payload}`;
  const sig = new Uint8Array(
    await crypto.subtle.sign('HMAC', await key(secret), enc.encode(data))
  );
  return `${data}.${base64UrlEncode(sig)}`;
}

export async function verifyJWT(
  token: string,
  secret: string
): Promise<LicenseClaims | null> {
  const parts = token.split('.');
  if (parts.length !== 3) return null;
  const [h, p, s] = parts as [string, string, string];
  const data = `${h}.${p}`;
  const sig = base64UrlDecode(s);
  const ok = await crypto.subtle.verify('HMAC', await key(secret), sig, enc.encode(data));
  if (!ok) return null;
  try {
    const claims = JSON.parse(dec.decode(base64UrlDecode(p))) as LicenseClaims;
    return claims;
  } catch {
    return null;
  }
}

const THIRTY_DAYS_MS = 30 * 24 * 60 * 60 * 1000;
const TWENTY_EIGHT_DAYS_MS = 28 * 24 * 60 * 60 * 1000;
const NINETY_DAYS_MS = 90 * 24 * 60 * 60 * 1000;

export function newClaims(opts: {
  license_id: string;
  tier: Tier;
  stripe_customer_id: string;
  stripe_subscription_id: string | null;
  email: string;
  included_premium_words?: number;
}): LicenseClaims {
  const now = Date.now();
  const isLifetime = opts.tier.startsWith('lifetime_');
  const includedDefault = opts.tier.startsWith('pro_') ? 50000 : 0;
  return {
    license_id: opts.license_id,
    tier: opts.tier,
    stripe_customer_id: opts.stripe_customer_id,
    stripe_subscription_id: opts.stripe_subscription_id,
    email: opts.email,
    included_premium_words: opts.included_premium_words ?? includedDefault,
    issued_at: now,
    expires_at: now + (isLifetime ? NINETY_DAYS_MS : THIRTY_DAYS_MS),
    refresh_at: now + (isLifetime ? NINETY_DAYS_MS - 2 * 24 * 60 * 60 * 1000 : TWENTY_EIGHT_DAYS_MS),
  };
}

export function refreshClaims(prev: LicenseClaims): LicenseClaims {
  return newClaims({
    license_id: prev.license_id,
    tier: prev.tier,
    stripe_customer_id: prev.stripe_customer_id,
    stripe_subscription_id: prev.stripe_subscription_id,
    email: prev.email,
    included_premium_words: prev.included_premium_words,
  });
}

export function newLicenseId(): string {
  // ULID-ish: time + random; not cryptographically reversible but unique enough.
  const t = Date.now().toString(36).padStart(9, '0');
  const r = crypto.getRandomValues(new Uint8Array(10));
  const rand = base64UrlEncode(r).slice(0, 14);
  return `lic_${t}${rand}`;
}
