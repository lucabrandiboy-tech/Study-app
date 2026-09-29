/** Hand-arranged Pre-Advanced pieces (public-domain classics, arranged for 8th-grade pianists). */
import type { SongDef } from './make';

const join = (bars: string[]) => bars.join(' | ');

// ---- Für Elise (A + B sections), 3/8 ----
const feA = ['E5:s D#5 E5 B4 D5 C5', 'A4:e r:s C4:s E4 A4', 'B4:e r:s E4:s G#4 B4', 'C5:e r:s E4:s E5 D#5', 'E5:s D#5 E5 B4 D5 C5', 'A4:e r:s C4:s E4 A4', 'B4:e r:s E4:s C5 B4'];
const feAL = ['r:q.', 'A2:s E3 A3 r:s r:e', 'E2:s E3 G#3 r:s r:e', 'A2:s E3 A3 r:s r:e', 'r:q.', 'A2:s E3 A3 r:s r:e', 'E2:s E3 G#3 r:s r:e'];
const feRH = join(['r:e r:e E5:s D#5', ...feA, 'A4:e r:e E5:s D#5', ...feA, 'A4:e r:s B4:s C5 D5',
  'E5:e. G4:s F5 E5', 'D5:e. F4:s E5 D5', 'C5:e. E4:s D5 C5', 'B4:e r:s E4:s E5:s r:s', 'r:e r:e E5:s D#5', ...feA, 'A4:q.']);
const feLH = join(['r:q.', ...feAL, 'A2:s E3 A3 r:s r:e', ...feAL, 'A2:s E3 A3 r:s r:e',
  'C3:s G3 C4 r:s r:e', 'G2:s G3 B3 r:s r:e', 'A2:s E3 A3 r:s r:e', 'E2:s E3 E4 r:s r:e', 'r:q.', ...feAL, '[A2 A3]:q.']);

// ---- Prelude in C (after Bach) ----
const PRE: [string, string, string, string][] = [
  ['G4', 'C5', 'E5', '[C3 E3]'], ['A4', 'D5', 'F5', '[C3 D3]'], ['G4', 'D5', 'F5', '[B2 D3]'], ['G4', 'C5', 'E5', '[C3 E3]'],
  ['A4', 'E5', 'A5', '[C3 E3]'], ['F#4', 'A4', 'D5', '[C3 D3]'], ['G4', 'D5', 'G5', '[B2 D3]'], ['E4', 'G4', 'C5', '[B2 C3]'],
  ['E4', 'G4', 'C5', '[A2 C3]'], ['D4', 'F#4', 'C5', '[D2 A2]'], ['D4', 'G4', 'B4', '[G2 B2]'], ['E4', 'G4', 'C#5', '[G2 Bb2]'],
  ['D4', 'A4', 'D5', '[F2 A2]'], ['D4', 'F4', 'B4', '[F2 Ab2]'], ['C4', 'G4', 'C5', '[E2 G2]'], ['C4', 'F4', 'A4', '[E2 F2]'],
  ['C4', 'F4', 'A4', '[D2 F2]'], ['B3', 'D4', 'G4', '[G1 D2]'], ['C4', 'E4', 'G4', '[C2 E2]'], ['Bb3', 'E4', 'G4', '[C2 G2]'],
  ['A3', 'C4', 'F4', '[F1 F2]'], ['C4', 'F4', 'A4', '[F1 F2]'], ['B3', 'F4', 'G4', '[G1 G2]'],
];
const halfArp = (a: string, b: string, c: string) => `r:e ${a}:s ${b} ${c} ${a} ${b} ${c}`;
const preRH = join([...PRE.map(([a, b, c]) => `${halfArp(a, b, c)} ${halfArp(a, b, c)}`), '[E4 G4 C5]:w']);
const preLH = join([...PRE.map(([, , , l]) => `${l}:h ${l}:h`), '[C2 C3]:w']);

// ---- Gymnopédie No. 1 (after Satie), 3/4 ----
const gymL = (n: number) => Array.from({ length: n }, (_, i) => (i % 2 ? 'D2:q [A3 C#4 F#4]:h' : 'G2:q [B3 D4 F#4]:h'));
const gymRH = join(['r:h.', 'r:h.', 'r:q F#5:q A5', 'G5 F#5 C#5', 'B4 C#5 D5', 'A4:h.', 'F#4:h.~', 'F#4:h.', 'r:h.', 'r:h.',
  'r:q F#5:q A5', 'G5 F#5 C#5', 'B4 C#5 D5', 'A4:h.', 'C#5:h.', 'D5:h.', 'E5:q D5 C#5', 'B4:h.', 'A4:h. ', 'F#4:h.~', 'F#4:h.', 'r:h.', '[D4 F#4 A4 D5]:h.']);
const gymLH = join([...gymL(22), '[D2 A2]:h.']);

// ---- Chopin Prelude in E minor (Op. 28 No. 4, arr.) ----
const rc = (c: string) => Array.from({ length: 8 }, (_, i) => (i ? c : `${c}:e`)).join(' ');
const chopinLH = join(['[G3 B3 E4]', '[G3 B3 E4]', '[F#3 A3 E4]', '[F#3 A3 D#4]', '[F3 A3 D#4]', '[F3 A3 D4]', '[E3 A3 C4]', '[E3 G3 C4]',
  '[D#3 G3 C4]', '[D3 G3 B3]', '[D3 F#3 B3]', '[C3 F#3 A3]', '[B2 D#3 A3]', '[B2 D#3 A3]', '[B2 F#3 A3]'].map(rc).concat('[E2 E3]:w'));
const chopinRH = join(['B4:h. C5:q', 'B4:w', 'B4:h. C5:q', 'B4:w', 'B4:h. C5:q', 'B4:h A4:h', 'A4:h. B4:q', 'A4:w', 'A4:h. B4:q', 'A4:h G4:h',
  'F#4:h. G4:q', 'F#4:w', 'F#4:h E4:h', 'D#4:h E4:h', '[D#4 F#4 B4]:w', '[E4 G4 B4]:w']);

// ---- Sonata in C, K. 545 (1st mvt. opening, after Mozart) ----
const k545RH = join(['C5:h E5:q G5', 'B4:q. C5:s D5 C5:h', 'A5:h G5:q C6', 'G5:q F5:e G5 E5:h',
  'A4:s B4 C5 D5 E5 F5 G5 A5 G5 F5 E5 D5 C5 B4 A4 G4', 'F4:s G4 A4 B4 C5 D5 E5 F5 E5 D5 C5 B4 A4 G4 F4 E4',
  'D4:s E4 F4 G4 A4 B4 C5 D5 C5 B4 A4 G4 F4 E4 D4 C4', 'B3:s C4 D4 E4 F4 G4 A4 F4 E4:e G4 C5:q',
  'C5:h E5:q G5', 'B4:q. C5:s D5 C5:h', 'A5:h G5:q C6', 'G5:q F5:e G5 E5:h', '[E4 G4 C5]:w']);
const alb = (a: string, b: string, c: string) => `${a}:e ${c} ${b} ${c} ${a} ${c} ${b} ${c}`;
const k545LH = join([alb('C3', 'E3', 'G3'), 'D3:e G3 F3 G3 C3 G3 E3 G3', 'C3:e A3 F3 A3 C3 G3 E3 G3', 'B2:e G3 D3 G3 C3 G3 E3 G3',
  'A2:h F2:h', 'G2:h C3:h', 'F2:h G2:h', 'G2:h C3:h', alb('C3', 'E3', 'G3'), 'D3:e G3 F3 G3 C3 G3 E3 G3', 'C3:e A3 F3 A3 C3 G3 E3 G3', 'B2:e G3 D3 G3 C3 G3 E3 G3', '[C2 C3]:w']);

const canonV1 = 'F#5:q E5 D5 C#5 | B4 A4 B4 C#5 | D5 C#5 B4 A4 | G4 F#4 G4 E4';
const canonV2 = 'D4:e F#4 A4 G4 F#4 D4 F#4 E4 | D4 B3 D4 A4 G4 B4 A4 G4 | F#4 D4 E4 C#5 D5 F#5 A5 A4 | B4 G4 A4 F#4 D4 D5 C#5 A4';
const canonV3 = 'D5:s C#5 D5 D4 C#4 A4 E4 F#4 D4:e D5 C#5 B4 | C#5:s F#5 A5 B5 G5 F#5 E5 G5 F#5 E5 D5 C#5 B4 A4 G4 F#4 | E4:s G4 F#4 E4 D4 E4 F#4 G4 A4 E4 A4 G4 F#4 B4 A4 G4 | A4:s G4 F#4 E4 D4 B3 B4 C#5 D5 C#5 B4 A4 G4 F#4 E4 A4';
const canonCh = 'D:2 A:2 | Bm:2 F#m:2 | G:2 D:2 | G:2 A:2';

export const PRE_ADVANCED: SongDef[] = [
  { id: 'fur-elise', t: 'Für Elise', c: 'Ludwig van Beethoven', lv: 'P', cat: 'Classical', time: [3, 8], bpm: 66, rh: feRH, lh: feLH },
  { id: 'minuet-g-full', t: 'Minuet in G (complete A section)', c: 'Christian Petzold', lv: 'P', cat: 'Classical', time: [3, 4], key: 1, bpm: 108,
    rh: 'D5:q G4:e A4 B4 C5 | D5:q G4 G4 | E5:q C5:e D5 E5 F#5 | G5:q G4 G4 | C5:q D5:e C5 B4 A4 | B4:q C5:e B4 A4 G4 | F#4:q G4:e A4 B4 G4 | A4:h. | D5:q G4:e A4 B4 C5 | D5:q G4 G4 | E5:q C5:e D5 E5 F#5 | G5:q G4 G4 | C5:q D5:e C5 B4 A4 | B4:q C5:e B4 A4 G4 | A4:q B4:e A4 G4 F#4 | G4:h.',
    ch: 'G | G | C | G | C | G | D | D | G | G | C | G | C | G | D | G' },
  { id: 'prelude-c', t: 'Prelude in C (Well-Tempered Clavier)', c: 'Johann Sebastian Bach', lv: 'P', cat: 'Classical', bpm: 66, rh: preRH, lh: preLH },
  { id: 'gymnopedie-1', t: 'Gymnopédie No. 1', c: 'Erik Satie', lv: 'P', cat: 'Classical', time: [3, 4], key: 2, bpm: 72, rh: gymRH, lh: gymLH },
  { id: 'chopin-e-minor', t: 'Prelude in E minor, Op. 28 No. 4', c: 'Frédéric Chopin', lv: 'P', cat: 'Classical', key: 1, bpm: 56, rh: chopinRH, lh: chopinLH },
  { id: 'k545', t: 'Sonata in C, K. 545 (opening)', c: 'Wolfgang Amadeus Mozart', lv: 'P', cat: 'Classical', bpm: 100, rh: k545RH, lh: k545LH },
  { id: 'entertainer', t: 'The Entertainer', c: 'Scott Joplin', lv: 'P', cat: 'Classical', time: [2, 4], bpm: 76,
    rh: 'r:q. D4:s D#4 | E4:s C5:e E4:s C5:e E4:s C5:s~ | C5:q r:s C5:s D5 D#5 | E5:s C5 D5 E5:e B4:s D5:e | C5:h | r:q. D4:s D#4 | E4:s C5:e E4:s C5:e E4:s C5:s~ | C5:q r:e A4:s G4 | F#4:s A4 C5 E5:e D5:s C5 A4 | D5:h | r:q. D4:s D#4 | E4:s C5:e E4:s C5:e E4:s C5:s~ | C5:q r:s C5:s D5 D#5 | E5:s C5 D5 E5:e B4:s D5:e | C5:h',
    ch: 'N | C | C | G7 | C | N | C | C | D7 | G | N | C | C | G7 | C' },
  { id: 'canon-d-variations', t: 'Canon in D (three variations)', c: 'Johann Pachelbel', lv: 'P', cat: 'Classical', key: 2, bpm: 72,
    rh: `${canonV1} | ${canonV2} | ${canonV3} | ${canonV1} | D5:w`, ch: `${canonCh} | ${canonCh} | ${canonCh} | ${canonCh} | D` },
  { id: 'william-tell', t: 'William Tell Overture (finale)', c: 'Gioachino Rossini', lv: 'P', cat: 'Classical', time: [2, 4], key: 3, bpm: 132,
    rh: 'E4:s E4 E4:e E4:s E4 E4:e | E4:s E4 A4:e B4 C#5 | E4:s E4 E4:e E4:s E4 A4:e | C#5:s C#5 B4:e G#4 E4 | E4:s E4 E4:e E4:s E4 E4:e | E4:s E4 A4:e B4 C#5 | B4:s B4 A4:e G#4 B4 | A4:q A4:q | E5:s E5 E5:e E5:s E5 E5:e | E5:s E5 A5:e B5 C#6 | E5:s E5 E5:e E5:s E5 A5:e | C#6:s C#6 B5:e G#5 E5 | E5:s E5 E5:e E5:s E5 E5:e | E5:s E5 A5:e B5 C#6 | B5:s B5 A5:e G#5 B5 | [A4 A5]:q r:q',
    ch: 'A | A | A | E | A | A | E | A | A | A | A | E | A | A | E | A' },
  { id: 'blue-danube', t: 'The Blue Danube', c: 'Johann Strauss II', lv: 'P', cat: 'Classical', time: [3, 4], key: 2, bpm: 132,
    rh: 'D4:q D4 F#4 | A4:h A4:q | r:q A5 A5 | r:q F#5 F#5 | D4:q D4 F#4 | A4:h A4:q | r:q A5 A5 | r:q G5 G5 | C#4:q C#4 E4 | B4:h B4:q | r:q B5 B5 | r:q G5 G5 | C#4:q C#4 E4 | B4:h B4:q | r:q B5 B5 | r:q F#5 F#5 | D4:q D4 F#4 | A4:h D5:q | r:q D6 D6 | r:q A5 A5 | D4:q D4 F#4 | A4:h D5:q | r:q D6 D6 | r:q B5 B5 | E4:q E4 G4 | B4:h. | B4:h A4:q | F#5:h. | E5:h C#5:q | D5:h.',
    ch: 'D | D | D | D | D | D | D | D | A7 | A7 | A7 | A7 | A7 | A7 | A7 | A7 | D | D | D | D | D | D | D | D | G | G | D | D | A7 | D' },
];
