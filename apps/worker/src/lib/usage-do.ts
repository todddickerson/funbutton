import type { ModelPref, UsageRecord } from '../types';
import { thisMonthKey, todayISO } from './words';

function emptyMonth(month: string): UsageRecord {
  return {
    month,
    premium_spend_cents: 0,
    fast: { words_in: 0, words_out: 0 },
    'premium-haiku': { words_in: 0, words_out: 0 },
    'premium-sonnet': { words_in: 0, words_out: 0 },
    'premium-opus': { words_in: 0, words_out: 0 },
    'premium-gpt41': { words_in: 0, words_out: 0 },
    history: [],
  };
}

interface RecordBody {
  model: ModelPref;
  words_in: number;
  words_out: number;
  cost_cents: number;
}

export class UsageCounter {
  state: DurableObjectState;

  constructor(state: DurableObjectState) {
    this.state = state;
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    const month = thisMonthKey();

    if (url.pathname === '/get') {
      const stored = await this.state.storage.get<UsageRecord>('current_month');
      if (!stored || stored.month !== month) {
        const fresh = emptyMonth(month);
        return Response.json(fresh);
      }
      return Response.json(stored);
    }

    if (url.pathname === '/record') {
      const { model, words_in, words_out, cost_cents } = (await req.json()) as RecordBody;
      let m =
        (await this.state.storage.get<UsageRecord>('current_month')) ??
        emptyMonth(month);
      if (m.month !== month) m = emptyMonth(month);

      m.premium_spend_cents += cost_cents;
      const slot = m[model];
      slot.words_in += words_in;
      slot.words_out += words_out;

      const today = todayISO();
      const isPremium = model !== 'fast';
      const last = m.history[m.history.length - 1];
      if (last && last.day === today) {
        if (isPremium) {
          last.premium += words_in + words_out;
          last.spend_cents += cost_cents;
        } else {
          last.fast += words_in + words_out;
        }
      } else {
        m.history.push({
          day: today,
          fast: isPremium ? 0 : words_in + words_out,
          premium: isPremium ? words_in + words_out : 0,
          spend_cents: cost_cents,
        });
        // Cap history to last 31 days.
        if (m.history.length > 31) m.history = m.history.slice(-31);
      }

      await this.state.storage.put('current_month', m);
      return Response.json(m);
    }

    if (url.pathname === '/reset') {
      await this.state.storage.put('current_month', emptyMonth(month));
      return Response.json({ ok: true });
    }

    return new Response('Not found', { status: 404 });
  }
}

// Global counter for the lifetime sales ladder. Single instance (idFromName('global')).
export class LifetimeCounter {
  state: DurableObjectState;

  constructor(state: DurableObjectState) {
    this.state = state;
  }

  async fetch(req: Request): Promise<Response> {
    const url = new URL(req.url);
    if (url.pathname === '/increment') {
      const current = (await this.state.storage.get<number>('count')) ?? 0;
      const next = current + 1;
      await this.state.storage.put('count', next);
      return Response.json({ count: next });
    }
    if (url.pathname === '/get') {
      const count = (await this.state.storage.get<number>('count')) ?? 0;
      return Response.json({ count });
    }
    return new Response('Not found', { status: 404 });
  }
}
