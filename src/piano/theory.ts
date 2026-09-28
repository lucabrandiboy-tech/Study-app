/** Music theory helpers that output notation-string note names with correct spelling. */
const LETTERS = ['C', 'D', 'E', 'F', 'G', 'A', 'B'];
const NAT = [0, 2, 4, 5, 7, 9, 11];

function parse(n: string) {
  const m = n.match(/^([A-G])(#|b)?(\d)$/)!;
  const acc = m[2] === '#' ? 1 : m[2] === 'b' ? -1 : 0;
  return { li: LETTERS.indexOf(m[1]), oct: Number(m[3]), pc: NAT[LETTERS.indexOf(m[1])] + acc, abs: (Number(m[3]) + 1) * 12 + NAT[LETTERS.indexOf(m[1])] + acc };
}
/** Note that is `steps` letter-steps above tonic and `semis` semitones above it. */
export function spell(tonic: string, steps: number, semis: number): string {
  const t = parse(tonic);
  const li = (t.li + steps) % 7;
  const oct = t.oct + Math.floor((t.li + steps) / 7);
  const natAbs = (oct + 1) * 12 + NAT[li];
  const acc = t.abs + semis - natAbs;
  return `${LETTERS[li]}${acc === 1 ? '#' : acc === -1 ? 'b' : acc === 2 ? '##' : acc === -2 ? 'bb' : ''}${oct}`;
}
const PATTERNS = { major: [0, 2, 4, 5, 7, 9, 11], natural: [0, 2, 3, 5, 7, 8, 10], harmonic: [0, 2, 3, 5, 7, 8, 11] };
export type ScaleType = keyof typeof PATTERNS;
export function scaleNotes(tonic: string, type: ScaleType, octaves = 1): string[] {
  const up: string[] = [];
  for (let o = 0; o < octaves; o++) PATTERNS[type].forEach((s, i) => up.push(spell(tonic, o * 7 + i, o * 12 + s)));
  up.push(spell(tonic, octaves * 7, octaves * 12));
  return up;
}
/** Up and down, formatted with a duration; ends on a held tonic so it fills whole measures of 4/4. */
export function scale(tonic: string, type: ScaleType, octaves = 1, dur = 'q'): string {
  const up = scaleNotes(tonic, type, octaves);
  const down = [...up].reverse().slice(1);
  const all = [...up, ...down];
  const last = all.pop()!;
  const per = dur === 'q' ? 1 : dur === 'e' ? 0.5 : 0.25;
  const used = all.length * per;
  const fill = Math.ceil((used + 1e-9) / 4) * 4 - used;
  const lastDur = fill === 4 ? 'w' : fill === 3 ? 'h.' : fill === 2 ? 'h' : fill === 1 ? 'q' : fill === 0.5 ? 'e' : 'q';
  return `${all.map((n, i) => (i === 0 ? `${n}:${dur}` : n)).join(' ')} ${last}:${lastDur}${fill > 4 ? '' : ''}`;
}
const TRIADS = { major: [[2, 4], [4, 7]], minor: [[2, 3], [4, 7]], dim: [[2, 3], [4, 6]], aug: [[2, 4], [4, 8]] } as const;
const SEVENTHS = { maj7: [[2, 4], [4, 7], [6, 11]], dom7: [[2, 4], [4, 7], [6, 10]], min7: [[2, 3], [4, 7], [6, 10]], m7b5: [[2, 3], [4, 6], [6, 10]], dim7: [[2, 3], [4, 6], [6, 9]] } as const;
export function chordTones(root: string, q: keyof typeof TRIADS | keyof typeof SEVENTHS): string[] {
  const pat = (q in TRIADS ? TRIADS[q as keyof typeof TRIADS] : SEVENTHS[q as keyof typeof SEVENTHS]) as readonly (readonly [number, number])[];
  return [root, ...pat.map(([st, se]) => spell(root, st, se))];
}
/** Inversion: move the lowest note(s) up an octave. */
export function invert(notes: string[], inv: number): string[] {
  const out = [...notes];
  for (let i = 0; i < inv; i++) { const n = out.shift()!; out.push(spell(n, 7, 12)); }
  return out;
}
export const chord = (notes: string[], dur: string) => `[${notes.join(' ')}]:${dur}`;
export const broken = (notes: string[], dur = 'e') => notes.map((n, i) => (i === 0 ? `${n}:${dur}` : n)).join(' ');
export function arpeggio(root: string, q: 'major' | 'minor', octaves = 1, dur = 'e'): string {
  const t = chordTones(root, q);
  const up: string[] = [];
  for (let o = 0; o < octaves; o++) t.forEach((n, i) => up.push(spell(root, o * 7 + [0, 2, 4][i], o * 12 + (parse(n).abs - parse(root).abs))));
  up.push(spell(root, octaves * 7, octaves * 12));
  const all = [...up, ...[...up].reverse().slice(1)];
  const last = all.pop()!;
  const per = dur === 'e' ? 0.5 : dur === 's' ? 0.25 : 1;
  const used = all.length * per, fill = Math.ceil((used + 1e-9) / 4) * 4 - used;
  const pad = fill >= 1 ? `${last}:q${fill - 1 > 0 ? ` r:${fill - 1 === 3 ? 'h.' : fill - 1 === 2 ? 'h' : fill - 1 === 1 ? 'q' : 'e'}` : ''}` : `${last}:${fill === 0.5 ? 'e' : 's'}`;
  return `${broken(all, dur)} ${pad}`;
}
export const down8 = (n: string) => { const m = n.match(/^(.*?)(\d)$/)!; return `${m[1]}${Number(m[2]) - 1}`; };

/** Deterministic random for "new sight-reading pieces every day". */
export function mulberry32(a: number) {
  return () => { a |= 0; a = (a + 0x6d2b79f5) | 0; let t = Math.imul(a ^ (a >>> 15), 1 | a); t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t; return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}
