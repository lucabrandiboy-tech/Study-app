export const dayKey = (d = new Date()) => {
  const y = d.getFullYear(), m = String(d.getMonth() + 1).padStart(2, '0'), day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
};
export const parseDay = (k: string) => { const [y, m, d] = k.split('-').map(Number); return new Date(y, m - 1, d); };
export const addDays = (k: string, n: number) => { const d = parseDay(k); d.setDate(d.getDate() + n); return dayKey(d); };
export const daysBetween = (a: string, b: string) => Math.round((parseDay(b).getTime() - parseDay(a).getTime()) / 86400000);
export const lastNDays = (n: number, end = dayKey()) => Array.from({ length: n }, (_, i) => addDays(end, i - n + 1));
export const weekStart = (k: string) => { const d = parseDay(k); d.setDate(d.getDate() - ((d.getDay() + 6) % 7)); return dayKey(d); };
export const shortDay = (k: string) => parseDay(k).toLocaleDateString(undefined, { weekday: 'short' });
