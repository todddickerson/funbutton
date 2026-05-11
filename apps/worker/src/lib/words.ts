// Conservative word counter. Matches what desktop app expects (whitespace-split).
export function countWords(text: string): number {
  if (!text) return 0;
  return text.trim().split(/\s+/).filter(Boolean).length;
}

export function thisMonthKey(now = Date.now()): string {
  const d = new Date(now);
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`;
}

export function todayISO(now = Date.now()): string {
  return new Date(now).toISOString().slice(0, 10);
}
