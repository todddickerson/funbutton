// Shared types for the FunButton Worker.

export type ModelPref =
  | 'fast'
  | 'premium-haiku'
  | 'premium-sonnet'
  | 'premium-opus'
  | 'premium-gpt41';

export type Mode = 'auto' | 'email' | 'slack' | 'code' | 'raw';

export type Tier =
  | 'pro_monthly'
  | 'pro_annual'
  | 'lifetime_149'
  | 'lifetime_199'
  | 'lifetime_249';

export interface LicenseClaims {
  license_id: string;
  tier: Tier;
  stripe_customer_id: string;
  stripe_subscription_id: string | null;
  included_premium_words: number;
  email: string;
  issued_at: number;
  expires_at: number;
  refresh_at: number;
}

export interface CleanupRequest {
  model: ModelPref;
  transcript: string;
  mode?: Mode;
  dictionary?: string[];
}

export interface UsagePerModel {
  words_in: number;
  words_out: number;
}

export interface UsageRecord {
  month: string; // YYYY-MM
  premium_spend_cents: number;
  fast: UsagePerModel;
  'premium-haiku': UsagePerModel;
  'premium-sonnet': UsagePerModel;
  'premium-opus': UsagePerModel;
  'premium-gpt41': UsagePerModel;
  history: Array<{ day: string; fast: number; premium: number; spend_cents: number }>;
}

export interface Env {
  // KV
  LICENSE_KV: KVNamespace;
  USAGE_KV: KVNamespace;
  CAP_KV: KVNamespace;
  RATE_LIMIT_KV: KVNamespace;
  // Durable Objects
  USAGE_DO: DurableObjectNamespace;
  LIFETIME_COUNTER: DurableObjectNamespace;
  // D1
  D1_DATABASE: D1Database;
  // Secrets
  JWT_SECRET: string;
  STRIPE_SECRET_KEY: string;
  STRIPE_WEBHOOK_SECRET: string;
  STRIPE_PRICE_PRO_MONTHLY: string;
  STRIPE_PRICE_PRO_ANNUAL: string;
  STRIPE_PRICE_LIFETIME_149: string;
  STRIPE_PRICE_LIFETIME_199: string;
  STRIPE_PRICE_LIFETIME_249: string;
  STRIPE_PRICE_METER_HAIKU: string;
  STRIPE_PRICE_METER_SONNET: string;
  STRIPE_PRICE_METER_OPUS: string;
  STRIPE_PRICE_METER_GPT41: string;
  GROQ_API_KEY: string;
  ANTHROPIC_API_KEY: string;
  OPENAI_API_KEY: string;
  RESEND_API_KEY: string;
  ACTIVATION_EMAIL_FROM: string;
  APP_URL: string;
  ACTIVATION_URL_SCHEME: string;
}
