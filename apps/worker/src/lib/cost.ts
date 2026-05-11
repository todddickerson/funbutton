import type { ModelPref } from '../types';

// cents per 10K combined input+output words; matches PRD pricing table.
const RATES: Record<Exclude<ModelPref, 'fast'>, number> = {
  'premium-haiku': 40,
  'premium-sonnet': 60,
  'premium-opus': 99,
  'premium-gpt41': 50,
};

export function calcCost(model: ModelPref, words_in: number, words_out: number): number {
  if (model === 'fast') return 0;
  const rate = RATES[model];
  const total = words_in + words_out;
  return Math.ceil((total / 10000) * rate);
}

export function projectedCost(model: ModelPref, words_in: number): number {
  // Estimate output ≈ input — conservative for cap pre-check.
  return calcCost(model, words_in, words_in);
}
