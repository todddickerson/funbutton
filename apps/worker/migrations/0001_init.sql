-- 0001_init.sql — FunButton audit + licenses tables.

CREATE TABLE IF NOT EXISTS usage_log (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  license_id TEXT NOT NULL,
  model TEXT NOT NULL,
  words_in INTEGER NOT NULL,
  words_out INTEGER NOT NULL,
  cost_cents INTEGER NOT NULL,
  ts INTEGER NOT NULL,
  stripe_idempotency_key TEXT
);

CREATE INDEX IF NOT EXISTS idx_usage_license_ts ON usage_log(license_id, ts DESC);
CREATE INDEX IF NOT EXISTS idx_usage_ts ON usage_log(ts);

CREATE TABLE IF NOT EXISTS licenses (
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

CREATE INDEX IF NOT EXISTS idx_licenses_customer ON licenses(stripe_customer_id);
CREATE INDEX IF NOT EXISTS idx_licenses_subscription ON licenses(stripe_subscription_id);
