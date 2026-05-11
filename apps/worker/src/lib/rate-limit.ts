// 50 requests / minute sliding window via KV. Single-bucket implementation —
// "sliding" approximated by storing per-minute counts and summing the current
// and previous bucket weighted by overlap.

const LIMIT = 50;

export async function rateLimitExceeded(licenseId: string, kv: KVNamespace): Promise<boolean> {
  const now = Date.now();
  const minute = Math.floor(now / 60_000);
  const prevMinute = minute - 1;
  const overlap = (now % 60_000) / 60_000;

  const [curStr, prevStr] = await Promise.all([
    kv.get(`${licenseId}:${minute}`),
    kv.get(`${licenseId}:${prevMinute}`),
  ]);
  const cur = curStr ? parseInt(curStr, 10) : 0;
  const prev = prevStr ? parseInt(prevStr, 10) : 0;
  const weighted = cur + prev * (1 - overlap);

  if (weighted >= LIMIT) return true;

  // Increment current bucket (60s TTL keeps KV from growing forever).
  await kv.put(`${licenseId}:${minute}`, String(cur + 1), { expirationTtl: 120 });
  return false;
}
