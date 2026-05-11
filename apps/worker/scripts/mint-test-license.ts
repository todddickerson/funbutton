#!/usr/bin/env tsx
// Mint a test JWT for local dev / staging smoke tests.
//
// Usage:
//   pnpm mint-test-license [tier=pro_annual] [email=todd@example.com]
//
// Requires JWT_SECRET in env (read from .dev.vars or shell).
// The script reads it from process.env.JWT_SECRET; export it before running.

import * as fs from 'node:fs';
import * as path from 'node:path';
import { signJWT, newClaims, newLicenseId } from '../src/lib/jwt';
import type { Tier } from '../src/types';

function loadDevVars(): Record<string, string> {
  const file = path.join(__dirname, '..', '.dev.vars');
  if (!fs.existsSync(file)) return {};
  return Object.fromEntries(
    fs
      .readFileSync(file, 'utf8')
      .split('\n')
      .map((l) => l.trim())
      .filter((l) => l && !l.startsWith('#'))
      .map((l) => {
        const i = l.indexOf('=');
        return i < 0 ? [l, ''] : [l.slice(0, i), l.slice(i + 1)];
      })
  );
}

async function main() {
  const dev = loadDevVars();
  const secret = process.env.JWT_SECRET || dev.JWT_SECRET;
  if (!secret) {
    console.error('JWT_SECRET not set. Add to .dev.vars or `export JWT_SECRET=...`');
    process.exit(1);
  }
  const tier = (process.argv[2] as Tier | undefined) ?? 'pro_annual';
  const email = process.argv[3] ?? 'test@funbutton.ai';

  const claims = newClaims({
    license_id: newLicenseId(),
    tier,
    stripe_customer_id: 'cus_test_local',
    stripe_subscription_id: tier.startsWith('pro_') ? 'sub_test_local' : null,
    email,
  });
  const jwt = await signJWT(claims, secret);
  console.log(JSON.stringify({ jwt, claims }, null, 2));
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
