/**
 * Original pop-song generator. Every song here is written by this generator (seeded, so each song is
 * always the same): real pop chord progressions + a hook-based verse/chorus melody, sized to the level.
 */
import { midiName } from '../notation';
import { held, octaves, type SongDef } from './make';

function rng(seed: number) {
  let a = seed >>> 0;
  return () => { a = (a + 0x6d2b79f5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
}

type Key = { tonic: number; minor: boolean; sig: number; name: string };
const KEYS: Key[] = [
  { tonic: 60, minor: false, sig: 0, name: 'C' }, { tonic: 67, minor: false, sig: 1, name: 'G' }, { tonic: 62, minor: false, sig: 2, name: 'D' },
  { tonic: 65, minor: false, sig: -1, name: 'F' }, { tonic: 69, minor: false, sig: 3, name: 'A' }, { tonic: 70, minor: false, sig: -2, name: 'Bb' }, { tonic: 63, minor: false, sig: -3, name: 'Eb' },
  { tonic: 69, minor: true, sig: 0, name: 'Am' }, { tonic: 64, minor: true, sig: 1, name: 'Em' }, { tonic: 62, minor: true, sig: -1, name: 'Dm' },
  { tonic: 71, minor: true, sig: 2, name: 'Bm' }, { tonic: 67, minor: true, sig: -2, name: 'Gm' }, { tonic: 60, minor: true, sig: -3, name: 'Cm' },
];
const MAJ = [0, 2, 4, 5, 7, 9, 11], MIN = [0, 2, 3, 5, 7, 8, 10];
const QUAL_MAJ = ['', 'm', 'm', '', '', 'm', 'dim'], QUAL_MIN = ['m', 'dim', '', 'm', 'm', '', ''];

// Famous pop progressions (scale degrees, 0 = I/i)
const PROG_MAJ = [[0, 4, 5, 3], [5, 3, 0, 4], [0, 5, 3, 4], [0, 3, 4, 3], [3, 0, 4, 5], [0, 4, 3, 4], [0, 2, 3, 4], [1, 4, 0, 5]];
const PROG_MIN = [[0, 5, 2, 6], [0, 3, 4, 0], [0, 6, 5, 6], [0, 5, 3, 4], [0, 3, 6, 2], [5, 6, 0, 0]];

const STYLES = [
  { name: 'Dance Pop', bpm: [116, 128] }, { name: 'Power Ballad', bpm: [66, 80] }, { name: 'Dark Pop', bpm: [70, 90] }, { name: 'Alt-Pop', bpm: [96, 116] },
  { name: 'Synth Pop', bpm: [104, 120] }, { name: 'Pop Rock', bpm: [110, 132] }, { name: 'Country Pop', bpm: [92, 108] }, { name: 'K-Pop', bpm: [112, 128] },
  { name: 'R&B Groove', bpm: [80, 96] }, { name: 'Indie Pop', bpm: [98, 116] }, { name: 'Summer Anthem', bpm: [108, 124] }, { name: 'Movie Theme', bpm: [72, 92] },
];
const ADJ = ['Neon', 'Midnight', 'Golden', 'Electric', 'Silver', 'Broken', 'Wild', 'Paper', 'Velvet', 'Crystal', 'Summer', 'Lonely', 'Cosmic', 'Sugar', 'Echo', 'Faded', 'Burning', 'Frozen', 'Hidden', 'Starlit', 'Rainy', 'Sunset', 'Glitter', 'Violet', 'Runaway', 'Endless', 'Secret', 'Shadow', 'Ocean', 'Thunder', 'Candy', 'Late-Night', 'Brave', 'Satellite', 'Lucky', 'Afterglow'];
const NOUN = ['Heartbeat', 'Drive', 'Skyline', 'Dreams', 'Hearts', 'Lights', 'Parade', 'Rewind', 'Signal', 'Getaway', 'Fever', 'Horizon', 'Kiss', 'Memories', 'Rebel', 'Rocket', 'Paradise', 'Waves', 'Promise', 'Galaxy', 'Echoes', 'Highway', 'Summer', 'Crush', 'City', 'Mirror', 'Storm', 'Anthem', 'Motel', 'Satellites', 'Letters', 'Radio', 'Fireworks', 'Roses', 'Daydream', 'Stadium'];

type Lv = 'B' | 'I' | 'P' | 'A';
const RHY: Record<Lv, number[][]> = {
  B: [[2, 2], [1, 1, 2], [1, 1, 1, 1], [2, 1, 1], [3, 1]],
  I: [[1, 1, 2], [1, 1, 1, 1], [1.5, 0.5, 1, 1], [0.5, 0.5, 1, 2], [1, 0.5, 0.5, 1, 1], [2, 1, 1]],
  P: [[1.5, 0.5, 1, 1], [0.5, 1, 0.5, 1, 1], [1.5, 1.5, 1], [0.5, 0.5, 0.5, 0.5, 1, 1], [0.75, 0.25, 1, 0.5, 0.5, 1], [1, 0.5, 1, 0.5, 1]],
  A: [[0.5, 1, 0.5, 1, 1], [0.25, 0.25, 0.5, 1, 0.5, 0.5, 1], [0.75, 0.25, 0.75, 0.25, 1, 1], [0.5, 0.5, 0.25, 0.25, 0.5, 1.5, 0.5], [1.5, 0.5, 0.5, 0.5, 1], [0.25, 0.25, 0.25, 0.25, 1, 0.5, 0.5, 1]],
};
const RANGE: Record<Lv, [number, number]> = { B: [0, 5], I: [-1, 8], P: [-3, 10], A: [-4, 12] };

function makeSong(lv: Lv, n: number): SongDef {
  const r = rng(n * 7919 + { B: 1, I: 2, P: 3, A: 4 }[lv] * 104729);
  const pick = <T,>(a: T[]) => a[Math.floor(r() * a.length)];
  const easyKeys = KEYS.filter((k) => k.sig >= -1 && k.sig <= 1);
  const key = lv === 'B' ? pick(easyKeys.filter((k) => !k.minor || r() < 0.3)) : pick(KEYS);
  const scale = key.minor ? MIN : MAJ, qual = key.minor ? QUAL_MIN : QUAL_MAJ;
  const flats = key.sig < 0;
  const style = pick(STYLES);
  const bpm = Math.round((style.bpm[0] + r() * (style.bpm[1] - style.bpm[0])) * (lv === 'B' ? 0.8 : 1));
  const base = key.tonic >= 67 ? key.tonic - 12 : key.tonic; // melody home note around C4–F#4
  const deg2midi = (d: number) => base + Math.floor(d / 7) * 12 + scale[((d % 7) + 7) % 7];
  const chordName = (d: number) => `${midiName(key.tonic + scale[d], flats).replace(/-?\d+$/, '')}${qual[d]}`;
  const verseProg = pick(key.minor ? PROG_MIN : PROG_MAJ), chorusProg = pick(key.minor ? PROG_MIN : PROG_MAJ);
  const [lo, hi] = RANGE[lv];
  let cur = 2;

  const bar = (chord: number, rhythm: number[], lift: number, end = false): string => {
    if (end) { cur = 7 * Math.round(cur / 7); return held(midiName(deg2midi(Math.max(lo, Math.min(hi, cur))), flats), 4); }
    const tones = [chord, chord + 2, chord + 4].flatMap((t) => [t - 7, t, t + 7]);
    let beat = 0;
    const toks: string[] = [];
    rhythm.forEach((d, i) => {
      if (i === 0 && lv !== 'B' && r() < (lv === 'I' ? 0.1 : 0.2)) { toks.push(held('r', d)); beat += d; return; }
      const strong = Math.abs(beat - Math.round(beat)) < 1e-6;
      let target: number;
      if (strong) target = tones.reduce((b, t) => (Math.abs(t - cur - lift * 0.3) < Math.abs(b - cur - lift * 0.3) ? t : b), tones[0]);
      else target = cur + (r() < 0.5 ? 1 : -1) * (lv === 'A' && r() < 0.3 ? 2 : 1);
      if (lv === 'B' && Math.abs(target - cur) > 2) target = cur + Math.sign(target - cur) * 2;
      cur = Math.max(lo + lift, Math.min(hi, target));
      const m = deg2midi(cur);
      const name = midiName(m, flats);
      const harm = lv === 'A' && d >= 1 && r() < 0.6 ? `[${midiName(deg2midi(cur - 2), flats)} ${name}]` : name;
      toks.push(held(harm, d));
      beat += d;
    });
    return toks.join(' ');
  };

  // Section = 4 bars over a 4-chord progression. Hook-based: bars 1 & 3 reuse the same rhythm.
  const section = (prog: number[], lift: number, cadence: boolean) => {
    const a = pick(RHY[lv]), b = pick(RHY[lv]), c = pick(RHY[lv]);
    const start = cur;
    const bars = [bar(prog[0], a, lift), bar(prog[1], b, lift)];
    cur = start; // repeat the hook shape
    bars.push(bar(prog[2], a, lift), cadence ? bar(prog[3], [2, 2], lift) : bar(prog[3], c, lift));
    return bars;
  };
  const verse = section(verseProg, 0, false);
  const verse2 = section(verseProg, 0, true);
  cur += 2;
  const chorus = section(chorusProg, 2, false);
  const chorusEnd = section(chorusProg, 2, true);
  let chorusRh = [...chorus, ...chorusEnd].join(' | ');
  if (lv === 'P' || lv === 'A') chorusRh = octaves(chorusRh);
  const endBar = bar(0, [4], 0, true);

  const vCh = verseProg.map(chordName).join(' | '), cCh = chorusProg.map(chordName).join(' | ');
  const tonicCh = chordName(0);
  let rh: string, ch: string;
  if (lv === 'B') { rh = [...verse, ...verse2, chorusRh, endBar].join(' | '); ch = [vCh, vCh, cCh, cCh, tonicCh].join(' | '); }
  else { // verse, chorus, verse, chorus, outro
    rh = [...verse, ...verse2, chorusRh, ...verse, ...verse2, chorusRh, endBar].join(' | ');
    ch = [vCh, vCh, cCh, cCh, vCh, vCh, cCh, cCh, tonicCh].join(' | ');
  }
  const t = `${ADJ[(n * 7 + { B: 0, I: 9, P: 18, A: 27 }[lv]) % ADJ.length]} ${NOUN[(n * 11 + Math.floor(n / ADJ.length) + { B: 3, I: 5, P: 7, A: 13 }[lv]) % NOUN.length]}`;
  return { id: `pop-${lv}-${n}`, t, c: `Original · ${style.name} (${key.name})`, lv, cat: 'Pop Originals', rh, ch, key: key.sig, bpm };
}

/** 200 original pop songs per level. */
export function popSongs(perLevel = 200): SongDef[] {
  const out: SongDef[] = [];
  for (const lv of ['B', 'I', 'P', 'A'] as Lv[]) {
    const seen = new Set<string>();
    for (let n = 0; n < perLevel; n++) {
      const s = makeSong(lv, n);
      let t = s.t, k = 2;
      while (seen.has(t)) t = `${s.t} ${['II', 'III', 'IV', 'V', 'VI'][k++ - 2] ?? k}`;
      seen.add(t);
      out.push({ ...s, t });
    }
  }
  return out;
}
