/**
 * Compact song format for the Song Player library.
 * A song is a right-hand melody plus chord names per bar; the left hand is generated from the chords
 * in a style that matches the level (held roots → oom-pah → broken chords).
 */
import { chordTones, spell } from '../theory';
import { makePiece, type Exercise, type Piece } from '../notation';

export type Level = 'Beginner' | 'Intermediate' | 'Pre-Advanced' | 'Advanced';
export type Category = 'Kids' | 'Folk & World' | 'Holiday' | 'Classical' | 'Hymns & Anthems' | 'Originals' | 'Pop Originals' | 'Imported';
export interface SongEntry { id: string; title: string; composer: string; level: Level; category: Category; piece: () => Piece; ex?: Exercise }

type Style = 'root' | 'oompah' | 'broken';
export interface SongDef {
  id: string; t: string; c: string;
  lv: 'B' | 'I' | 'P' | 'A';
  cat: Category;
  rh: string;
  ch?: string; // chords per bar, e.g. "C | G7 | C F | G:2 C:2 | N"  (N = no chord)
  lh?: string; // explicit left hand (overrides ch)
  time?: [number, number]; key?: number; bpm?: number; style?: Style;
}

const LEVEL: Record<SongDef['lv'], Level> = { B: 'Beginner', I: 'Intermediate', P: 'Pre-Advanced', A: 'Advanced' };
const STYLE: Record<SongDef['lv'], Style> = { B: 'root', I: 'broken', P: 'broken', A: 'broken' };

const DUR: [number, string][] = [[4, 'w'], [3, 'h.'], [2, 'h'], [1.5, 'q.'], [1, 'q'], [0.75, 'e.'], [0.5, 'e'], [0.25, 's']];
function durTokens(q: number): string[] {
  const out: string[] = [];
  let r = q;
  while (r > 1e-6) {
    const d = DUR.find(([v]) => v <= r + 1e-6);
    if (!d) break;
    out.push(d[1]);
    r -= d[0];
  }
  return out;
}
/** One note (or chord/rest) lasting `q` beats, tied across tokens if needed. */
export function held(note: string, q: number): string {
  const toks = durTokens(q);
  if (note === 'r') return toks.map((t) => `r:${t}`).join(' ');
  return toks.map((t, i) => `${note}:${t}${i < toks.length - 1 ? '~' : ''}`).join(' ');
}

const QUAL: Record<string, Parameters<typeof chordTones>[1]> = { '': 'major', m: 'minor', '7': 'dom7', maj7: 'maj7', m7: 'min7', dim: 'dim', aug: 'aug' };
const down = (n: string) => n.replace(/(\d)$/, (d) => String(Number(d) - 1));
const up = (n: string) => n.replace(/(\d)$/, (d) => String(Number(d) + 1));

function parseChord(sym: string) {
  const m = sym.match(/^([A-G])([#b]?)(m7|maj7|m|7|dim|aug)?$/);
  if (!m) throw new Error(`Unknown chord "${sym}"`);
  const letter = m[1], acc = m[2] ?? '', q = m[3] ?? '';
  const bass = `${letter}${acc}${'CDEF'.includes(letter) ? 3 : 2}`; // bass root between G2 and F#3
  const mid = `${letter}${acc}3`;
  const tones = chordTones(mid, QUAL[q]); // root, 3rd, 5th, (7th) around octave 3
  const fifthAbove = spell(bass, 4, q === 'dim' ? 6 : q === 'aug' ? 8 : 7);
  return { bass, tones, seventh: q.includes('7'), fifthAlt: bass.endsWith('3') ? down(fifthAbove) : fifthAbove, fifthAbove, tenth: bass.endsWith('2') ? tones[1] : up(tones[1]) }; // tenth = 3rd, an octave above the bass
}

function chordSpan(sym: string, s: number, time: [number, number], style: Style): string {
  if (sym === 'N') return held('r', s);
  const c = parseChord(sym);
  const dyad = `[${c.seventh ? `${c.tones[1]} ${c.tones[3]}` : `${c.tones[1]} ${c.tones[2]}`}]`;
  const compound = time[1] === 8 && time[0] % 3 === 0;
  if (style === 'root') return held(c.bass, s);
  if (style === 'oompah') {
    if (compound && Math.abs(s - 3) < 1e-6) return `${c.bass}:q ${dyad}:e ${c.fifthAlt}:q ${dyad}:e`;
    if (compound && Math.abs(s - 1.5) < 1e-6) return `${c.bass}:q ${dyad}:e`;
    if (Math.abs(s - 4) < 1e-6) return `${c.bass}:q ${dyad}:q ${c.fifthAlt}:q ${dyad}:q`;
    if (Math.abs(s - 3) < 1e-6) return `${c.bass}:q ${dyad}:q ${dyad}:q`;
    if (Math.abs(s - 2) < 1e-6) return `${c.bass}:q ${dyad}:q`;
    return held(c.bass, s);
  }
  // broken chord in eighth notes
  const n = Math.round(s / 0.5);
  if (Math.abs(n * 0.5 - s) > 1e-6 || n < 2) return held(c.bass, s);
  const pat = compound ? [c.bass, c.fifthAbove, c.tenth] : [c.bass, c.fifthAbove, c.tenth, c.fifthAbove];
  return Array.from({ length: n }, (_, i) => `${pat[i % pat.length]}:e`).join(' ');
}

/** Turn "C | G7 | C F | G:2 C:2" into a left-hand part. Each bar's chords share the bar evenly unless given ":beats". */
export function chordLH(ch: string, time: [number, number] = [4, 4], style: Style = 'root'): string {
  const barLen = (time[0] * 4) / time[1];
  return ch.split('|').map((bar) => {
    const syms = bar.trim().split(/\s+/).filter(Boolean);
    const explicit = syms.map((x) => (x.includes(':') ? Number(x.split(':')[1]) : null));
    const given = explicit.reduce<number>((a, b) => a + (b ?? 0), 0);
    const free = explicit.filter((b) => b === null).length;
    const each = free ? (barLen - given) / free : 0;
    return syms.map((x, i) => chordSpan(x.split(':')[0], explicit[i] ?? each, time, style)).join(' ');
  }).join(' | ');
}

/** Split a voice into tokens, keeping [chord] groups together. */
function tokens(src: string): string[] {
  const out: string[] = [];
  for (const t of src.replace(/\[\s*/g, '[').replace(/\s*\]/g, ']').split(/\s+/).filter(Boolean)) {
    if (out.length && out[out.length - 1].startsWith('[') && !out[out.length - 1].includes(']')) out[out.length - 1] += ' ' + t;
    else out.push(t);
  }
  return out;
}
const OCT_MAX = 6; // don't double above octave 6
/** Double the melody in octaves (the top note of each chord gets an octave added) — a classic harder-arrangement technique. */
export function octaves(rh: string): string {
  return tokens(rh).map((tok) => {
    if (tok === '|') return tok;
    const m = tok.match(/^(\[[^\]]+\]|[^:~]+)(.*)$/);
    if (!m || m[1] === 'r') return tok;
    const notes = m[1].startsWith('[') ? m[1].slice(1, -1).split(/\s+/) : [m[1]];
    const top = notes[notes.length - 1];
    const oct = Number(top.match(/(-?\d)$/)?.[1] ?? 9);
    if (oct >= OCT_MAX) return tok;
    return `[${[...notes, up(top)].join(' ')}]${m[2]}`;
  }).join(' ');
}
const bars = (v: string) => v.split('|').length;
const twice = (v: string) => `${v} | ${v}`;

/** A Pre-Advanced arrangement of an easier song: verse as written, then a second verse in octaves, broken-chord left hand, faster. */
export function proArrangement(d: SongDef): SongDef {
  const time = d.time ?? [4, 4];
  const lh1 = d.ch ? chordLH(d.ch, time, 'broken') : d.lh;
  return {
    ...d, id: `${d.id}-pa`, t: `${d.t} — Pre-Advanced Arrangement`, lv: 'P', ch: undefined,
    rh: `${d.rh} | ${octaves(d.rh)}`, lh: lh1 ? twice(lh1) : undefined, bpm: Math.round((d.bpm ?? 90) * 1.12),
  };
}

export function toEntry(d: SongDef): SongEntry {
  const time = d.time ?? [4, 4];
  let rh = d.rh, lh = d.lh ?? (d.ch ? chordLH(d.ch, time, d.style ?? STYLE[d.lv]) : undefined);
  // Short songs play through twice so every song is a real, full-length piece.
  if (d.lv !== 'P' && bars(rh) < 16 && (!lh || bars(lh) === bars(rh))) { rh = twice(rh); if (lh) lh = twice(lh); }
  const ex: Exercise = { rh, lh, time, key: d.key ?? 0, bpm: d.bpm ?? 90 };
  return { id: d.id, title: d.t, composer: d.c, level: LEVEL[d.lv], category: d.cat, ex, piece: () => makePiece(d.id, d.t, ex) };
}

/** Repeat one bar n times: rep('[A2 E3]:w', 4). */
export const rep = (bar: string, n: number) => Array.from({ length: n }, () => bar).join(' | ');
