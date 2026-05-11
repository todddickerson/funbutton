import type { Env, ModelPref, Mode } from '../types';
import { systemPrompt, withDictionary } from './prompts';

interface CleanupArgs {
  model: ModelPref;
  transcript: string;
  mode: Mode;
  dictionary: string[];
  env: Env;
}

export interface CleanupResult {
  text: string;
}

export async function callProvider(args: CleanupArgs): Promise<CleanupResult> {
  const sys = withDictionary(systemPrompt(args.mode), args.dictionary);
  switch (args.model) {
    case 'fast':
      return callGroqLlama(sys, args.transcript, args.env);
    case 'premium-haiku':
      return callAnthropic('claude-haiku-4-5', sys, args.transcript, args.env);
    case 'premium-sonnet':
      return callAnthropic('claude-sonnet-4-7', sys, args.transcript, args.env);
    case 'premium-opus':
      return callAnthropic('claude-opus-4-7', sys, args.transcript, args.env);
    case 'premium-gpt41':
      return callOpenAI('gpt-4.1', sys, args.transcript, args.env);
  }
}

async function callGroqLlama(sys: string, user: string, env: Env): Promise<CleanupResult> {
  const res = await fetch('https://api.groq.com/openai/v1/chat/completions', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${env.GROQ_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model: 'llama-3.3-70b-versatile',
      temperature: 0.2,
      messages: [
        { role: 'system', content: sys },
        { role: 'user', content: user },
      ],
    }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new ProviderError('groq', res.status, body);
  }
  const json = (await res.json()) as {
    choices?: Array<{ message?: { content?: string } }>;
  };
  const text = json.choices?.[0]?.message?.content?.trim() ?? '';
  return { text };
}

async function callAnthropic(
  model: string,
  sys: string,
  user: string,
  env: Env
): Promise<CleanupResult> {
  const res = await fetch('https://api.anthropic.com/v1/messages', {
    method: 'POST',
    headers: {
      'x-api-key': env.ANTHROPIC_API_KEY,
      'anthropic-version': '2023-06-01',
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model,
      max_tokens: 4096,
      system: sys,
      messages: [{ role: 'user', content: user }],
    }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new ProviderError('anthropic', res.status, body);
  }
  const json = (await res.json()) as {
    content?: Array<{ type: string; text?: string }>;
  };
  const text = (json.content ?? [])
    .filter((b) => b.type === 'text')
    .map((b) => b.text ?? '')
    .join('')
    .trim();
  return { text };
}

async function callOpenAI(
  model: string,
  sys: string,
  user: string,
  env: Env
): Promise<CleanupResult> {
  const res = await fetch('https://api.openai.com/v1/chat/completions', {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${env.OPENAI_API_KEY}`,
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      model,
      temperature: 0.2,
      messages: [
        { role: 'system', content: sys },
        { role: 'user', content: user },
      ],
    }),
  });
  if (!res.ok) {
    const body = await res.text();
    throw new ProviderError('openai', res.status, body);
  }
  const json = (await res.json()) as {
    choices?: Array<{ message?: { content?: string } }>;
  };
  const text = json.choices?.[0]?.message?.content?.trim() ?? '';
  return { text };
}

export async function transcribeAudio(
  bytes: ArrayBuffer,
  contentType: string,
  env: Env
): Promise<{ text: string }> {
  // Groq Whisper Turbo expects multipart/form-data.
  const ext = contentType.includes('flac')
    ? 'flac'
    : contentType.includes('mp3') || contentType.includes('mpeg')
    ? 'mp3'
    : 'wav';
  const fd = new FormData();
  fd.append('file', new Blob([bytes], { type: contentType }), `audio.${ext}`);
  fd.append('model', 'whisper-large-v3-turbo');
  fd.append('response_format', 'json');
  fd.append('temperature', '0');

  const res = await fetch('https://api.groq.com/openai/v1/audio/transcriptions', {
    method: 'POST',
    headers: { Authorization: `Bearer ${env.GROQ_API_KEY}` },
    body: fd,
  });
  if (!res.ok) {
    const body = await res.text();
    throw new ProviderError('groq-whisper', res.status, body);
  }
  const json = (await res.json()) as { text?: string };
  return { text: (json.text ?? '').trim() };
}

export class ProviderError extends Error {
  constructor(
    public provider: string,
    public status: number,
    public body: string
  ) {
    super(`${provider} ${status}: ${body.slice(0, 200)}`);
  }
}
