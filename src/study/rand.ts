export const ri = (a: number, b: number) => Math.floor(Math.random() * (b - a + 1)) + a;
export const pick = <T,>(arr: readonly T[]): T => arr[Math.floor(Math.random() * arr.length)];
export const shuffle = <T,>(arr: readonly T[]): T[] => {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [a[i], a[j]] = [a[j], a[i]]; }
  return a;
};
export const round = (n: number, d = 1) => Math.round(n * 10 ** d) / 10 ** d;
export const fmt = (n: number) => (Number.isInteger(n) ? String(n) : String(round(n, 2)));
export const LETTERS = 'ABCDEFGHJKLMNPQRSTUVWXYZ';
export const pickLetters = (n: number) => shuffle(LETTERS.split('')).slice(0, n);

/** Build a multiple-choice answer: correct first in list, shuffled for display. */
export function mc(correct: string, wrong: string[], why?: Record<string, string>) {
  const uniqWrong = [...new Set(wrong.filter((w) => w !== correct))].slice(0, 3);
  const all = shuffle([correct, ...uniqWrong]);
  return { kind: 'choice' as const, choices: all, correct: all.indexOf(correct), why: why ? all.map((c) => why[c] ?? '') : undefined };
}

/** Parse student input like "12", "3/4", "5√2", "5sqrt(3)", "4pi", "2.5". Returns NaN if unreadable. */
export function parseNum(input: string): number {
  let s = input.toLowerCase().replace(/\s+/g, '').replace(/,/g, '').replace(/°|degrees?|units?|cm|in|ft|m\b/g, '');
  if (!s) return NaN;
  s = s.replace(/π/g, 'pi').replace(/√\(?(\d+(?:\.\d+)?)\)?/g, 'sqrt($1)').replace(/sqrt(\d+(?:\.\d+)?)/g, 'sqrt($1)');
  s = s.replace(/(\d|\))(?=sqrt|pi|\()/g, '$1*').replace(/pi(?=\d|sqrt)/g, 'pi*');
  if (!/^[0-9+\-*/().a-z]*$/.test(s) || /[a-z]/.test(s.replace(/sqrt|pi/g, ''))) return NaN;
  s = s.replace(/sqrt/g, 'Math.sqrt').replace(/pi/g, 'Math.PI');
  try {
    const v = Function(`"use strict";return (${s});`)();
    return typeof v === 'number' && isFinite(v) ? v : NaN;
  } catch { return NaN; }
}
export const close = (a: number, b: number) => Math.abs(a - b) <= Math.max(0.051, Math.abs(b) * 0.002);
