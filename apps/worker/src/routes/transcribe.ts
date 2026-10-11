import { Hono } from 'hono';
import type { Env } from '../types';
import { authenticate, authError } from '../lib/auth';
import { rateLimitExceeded } from '../lib/rate-limit';
import { transcribeAudio, ProviderError } from '../lib/providers';
import { countWords } from '../lib/words';

const app = new Hono<{ Bindings: Env }>();

const MAX_BYTES = 25 * 1024 * 1024;
const MAX_STT_PROMPT_CHARS = 4000;

function decodeSttPrompt(header: string | undefined): string | undefined {
  if (!header) return undefined;
  let decoded: string;
  try {
    decoded = decodeURIComponent(header);
  } catch {
    return undefined; // malformed encoding — drop the bias, still transcribe
  }
  const trimmed = decoded.trim();
  if (!trimmed) return undefined;
  return trimmed.slice(0, MAX_STT_PROMPT_CHARS);
}

app.post('/', async (c) => {
  const auth = await authenticate(c);
  if (!auth.ok) return authError(c, auth.status, auth.error);
  const license = auth.license;

  if (await rateLimitExceeded(license.license_id, c.env.RATE_LIMIT_KV)) {
    return c.json({ error: 'rate_limited' }, 429);
  }

  const contentType = c.req.header('Content-Type') ?? 'audio/wav';
  if (!contentType.startsWith('audio/')) {
    return c.json({ error: 'unsupported_content_type', expected: 'audio/*' }, 415);
  }

  const t0 = Date.now();
  const bytes = await c.req.arrayBuffer();
  if (bytes.byteLength > MAX_BYTES) {
    return c.json({ error: 'audio_too_large', max_bytes: MAX_BYTES }, 413);
  }

  // whisper vocabulary bias (user dictionary + dev terms), built by the desktop
  // app's `build_stt_prompt` and sent percent-encoded in a header (the body is
  // raw audio). Decode defensively: a malformed value must not fail the whole
  // transcription — we just drop the bias. Capped well above the client's
  // ~600-char budget; whisper ignores tokens past ~224 anyway.
  const sttPrompt = decodeSttPrompt(c.req.header('X-Funbutton-Stt-Prompt'));

  try {
    const { text } = await transcribeAudio(bytes, contentType, c.env, sttPrompt);
    const words = countWords(text);
    return c.json({ text, duration_ms: Date.now() - t0, words });
  } catch (e) {
    if (e instanceof ProviderError) {
      return c.json({ error: 'transcription_failed', provider: e.provider, status: e.status }, 502);
    }
    throw e;
  }
});

export default app;
