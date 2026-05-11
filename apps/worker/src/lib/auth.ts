import type { Context } from 'hono';
import { verifyJWT } from './jwt';
import type { Env, LicenseClaims } from '../types';

export async function authenticate(c: Context<{ Bindings: Env }>): Promise<
  { ok: true; license: LicenseClaims } | { ok: false; status: 401 | 403; error: string }
> {
  const header = c.req.header('Authorization') ?? '';
  const match = header.match(/^Bearer\s+(.+)$/i);
  if (!match || !match[1]) return { ok: false, status: 401, error: 'missing_authorization' };

  const claims = await verifyJWT(match[1], c.env.JWT_SECRET);
  if (!claims) return { ok: false, status: 401, error: 'invalid_jwt' };
  if (claims.expires_at < Date.now()) return { ok: false, status: 401, error: 'jwt_expired' };

  const revoked = await c.env.LICENSE_KV.get(`${claims.license_id}:revoked`);
  if (revoked) return { ok: false, status: 403, error: 'license_revoked' };

  return { ok: true, license: claims };
}

export function authError(c: Context<{ Bindings: Env }>, status: 401 | 403, error: string) {
  return c.json({ error }, status);
}
