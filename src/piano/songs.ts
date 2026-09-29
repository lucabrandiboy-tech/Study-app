import { COURSE, lessonPiece } from './course';
import { toEntry, proArrangement, type SongEntry, type Level, type Category, type SongDef } from './library/make';
import { KIDS } from './library/kids';
import { FOLK, HOLIDAY } from './library/folkHoliday';
import { ANTHEMS, CLASSICAL } from './library/classical';
import { ORIGINALS } from './library/originals';
import { PRE_ADVANCED } from './library/preAdvanced';

export type { SongEntry, Level, Category };

export const LEVELS: { id: Level; tag: string; color: string }[] = [
  { id: 'Beginner', tag: 'BEG', color: '#4ADE80' }, { id: 'Intermediate', tag: 'INT', color: '#7FD3FF' },
  { id: 'Pre-Advanced', tag: 'PRE-ADV', color: '#FF9F43' }, { id: 'Advanced', tag: 'ADV', color: '#F87171' },
];
export const CATEGORIES: { id: Category; icon: string; from: string; to: string }[] = [
  { id: 'Kids', icon: '🧸', from: '#FDE68A', to: '#FB923C' },
  { id: 'Folk & World', icon: '🌍', from: '#A7F3D0', to: '#10B981' },
  { id: 'Holiday', icon: '🎄', from: '#FECACA', to: '#DC2626' },
  { id: 'Classical', icon: '🎻', from: '#C7D2FE', to: '#6366F1' },
  { id: 'Hymns & Anthems', icon: '🕊️', from: '#BAE6FD', to: '#0284C7' },
  { id: 'Originals', icon: '✨', from: '#F5D0FE', to: '#A855F7' },
  { id: 'Imported', icon: '📂', from: '#E5E7EB', to: '#6B7280' },
];

/** Songs that were already in the app before the big library. */
const CLASSIC_EXTRAS: SongDef[] = [
  { id: 'saints', t: 'When the Saints Go Marching In', c: 'Traditional', lv: 'B', cat: 'Hymns & Anthems', bpm: 110,
    rh: 'r:q C4:q E4 F4 | G4:w | r:q C4 E4 F4 | G4:w | r:q C4 E4 F4 | G4:h E4:h | C4:h E4:h | D4:w', lh: 'r:w | C3:w | r:w | C3:w | r:w | C3:w | C3:w | G2:w' },
  { id: 'greensleeves', t: 'Greensleeves', c: 'Traditional English', lv: 'I', cat: 'Folk & World', time: [3, 4], bpm: 96,
    rh: 'r:h A4:q | C5:h D5:q | E5:q. F5:e E5:q | D5:h B4:q | G4:q. A4:e B4:q | C5:h A4:q | A4:q. G#4:e A4:q | B4:h G#4:q | E4:h A4:q', lh: 'r:h. | A2:h. | C3:h. | G2:h. | E2:h. | A2:h. | F2:h. | E2:h. | E2:h.' },
  { id: 'london-bridge', t: 'London Bridge', c: 'Traditional', lv: 'B', cat: 'Kids', bpm: 100,
    rh: 'G4:q. A4:e G4:q F4 | E4 F4 G4:h | D4:q E4 F4:h | E4:q F4 G4:h | G4:q. A4:e G4:q F4 | E4 F4 G4:h | D4:h G4:h | E4:q C4:h.', lh: 'C3:w | C3:w | G2:w | C3:w | C3:w | C3:w | G2:w | C3:w' },
  { id: 'row-row', t: 'Row, Row, Row Your Boat', c: 'Traditional', lv: 'B', cat: 'Kids', time: [6, 8], bpm: 70,
    rh: 'C4:q. C4:q. | C4:q D4:e E4:q. | E4:q D4:e E4:q F4:e | G4:h. | C5:e C5 C5 G4 G4 G4 | E4 E4 E4 C4 C4 C4 | G4:q F4:e E4:q D4:e | C4:h.' },
  { id: 'brahms-lullaby', t: 'Lullaby (Wiegenlied)', c: 'Johannes Brahms', lv: 'I', cat: 'Classical', time: [3, 4], bpm: 84,
    rh: 'r:h E4:e E4:e | G4:h E4:e E4:e | G4:h E4:e G4:e | C5:q B4:h | A4:h D4:e E4:e | F4:h D4:e F4:e | B4:e A4:e G4:q B4:q | C5:h.', lh: 'r:h. | C3:h. | C3:h. | G2:h. | F2:h. | G2:h. | G2:h. | C3:h.' },
  { id: 'canon-d', t: 'Canon in D (simplified)', c: 'Johann Pachelbel', lv: 'I', cat: 'Classical', key: 2, bpm: 66,
    rh: 'F#5:h E5:h | D5:h C#5:h | B4:h A4:h | B4:h C#5:h | [D5 F#5]:h [C#5 E5]:h | [B4 D5]:h [A4 C#5]:h | [G4 B4]:h [F#4 A4]:h | [G4 B4]:h [A4 C#5]:h | [F#4 A4 D5]:w',
    lh: 'D3:h A2:h | B2:h F#2:h | G2:h D2:h | G2:h A2:h | D3:h A2:h | B2:h F#2:h | G2:h D2:h | G2:h A2:h | D2:w' },
  { id: 'moonlight', t: 'Moonlight Sonata, 1st mvt. (opening)', c: 'Ludwig van Beethoven', lv: 'A', cat: 'Classical', key: 4, bpm: 52,
    rh: 'G#3:et C#4 E4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 | G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 | A3 C#4 E4 A3 C#4 E4 A3 D4 F#4 A3 D4 F#4 | G#3 B#3 F#4 G#3 C#4 E4 G#3 C#4 D#4 F#3 B#3 D#4 | E3 G#3 C#4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4',
    lh: '[C#2 C#3]:w | [B1 B2]:w | [A1 A2]:h [F#1 F#2]:h | [G#1 G#2]:h [G#1 G#2]:h | [C#2 C#3]:w' },
  { id: 'turkish-march', t: 'Rondo alla Turca (opening)', c: 'Wolfgang Amadeus Mozart', lv: 'A', cat: 'Classical', time: [2, 4], bpm: 116,
    rh: 'B4:s A4 G#4 A4 C5:q | D5:s C5 B4 C5 E5:q | F5:s E5 D#5 E5 B5 A5 G#5 A5 | B5 A5 G#5 A5 C6:q | A5:q r:q',
    lh: 'r:h | A3:e [C4 E4]:e [C4 E4]:e [C4 E4]:e | A3:e [C4 E4]:e [C4 E4]:e [C4 E4]:e | A3:e [C4 E4]:e E3:e [B3 E4]:e | A2:q r:q' },
];

const courseCat: Record<number, Category> = { 1: 'Kids', 2: 'Kids', 3: 'Kids', 4: 'Classical', 5: 'Kids', 6: 'Classical', 7: 'Classical', 8: 'Classical', 9: 'Hymns & Anthems' };
const unitLevel = (n: number): Level => (n <= 7 ? 'Beginner' : n <= 15 ? 'Intermediate' : n <= 19 ? 'Pre-Advanced' : 'Advanced');

let cached: SongEntry[] | null = null;
/** The whole public-domain library (+ originals), sorted by level. */
export function songLibrary(): SongEntry[] {
  if (cached) return cached;
  const course: SongEntry[] = COURSE.filter((u) => u.song).map((u) => {
    const original = u.song!.composer?.startsWith('Practice');
    return {
      id: `song-u${u.n}`, title: u.song!.title, composer: original ? 'Study+Piano Original' : u.song!.composer ?? 'Traditional',
      level: original ? unitLevel(u.n) : u.level === 'Beginner' ? 'Beginner' : 'Intermediate',
      category: original ? 'Originals' : courseCat[u.n] ?? 'Classical',
      ex: u.song!.ex, piece: () => lessonPiece(`song-u${u.n}`, u.song!.title, u.song!.ex),
    };
  });
  const jingle: SongEntry = { id: 'jingle-bells', title: 'Jingle Bells (chorus)', composer: 'James Lord Pierpont', level: 'Beginner', category: 'Holiday', ex: COURSE[4].lessons[3].ex, piece: () => lessonPiece('jingle-bells', 'Jingle Bells', COURSE[4].lessons[3].ex) };
  const rep: SongEntry[] = COURSE.find((u) => u.n === 24)!.lessons.map((l) => {
    const [composer, title] = l.title.split(' – ');
    const level: Level = /Bach|Mozart|Satie|Chopin/.test(composer) ? 'Pre-Advanced' : 'Advanced';
    return { id: `rep-${l.id}`, title, composer, level, category: 'Classical', ex: l.ex, piece: () => lessonPiece(`rep-${l.id}`, title, l.ex) };
  });
  const baseDefs = [...KIDS, ...FOLK, ...HOLIDAY, ...ANTHEMS, ...CLASSICAL, ...CLASSIC_EXTRAS, ...ORIGINALS, ...PRE_ADVANCED];
  // Every easier song also gets a harder Pre-Advanced arrangement (octaves, broken chords, faster, two verses).
  const courseDefs: SongDef[] = [...course, jingle].filter((e) => e.level !== 'Advanced' && e.level !== 'Pre-Advanced' && e.ex?.rh).map((e) => ({ id: e.id, t: e.title, c: e.composer, lv: 'I', cat: e.category, rh: e.ex!.rh!, lh: e.ex!.lh, time: e.ex!.time, key: e.ex!.key, bpm: e.ex!.bpm }));
  const arrangements = [...baseDefs.filter((d) => d.lv === 'B' || d.lv === 'I'), ...courseDefs].map(proArrangement);
  const defs = [...baseDefs, ...arrangements].map(toEntry);
  const order = { Beginner: 0, Intermediate: 1, 'Pre-Advanced': 2, Advanced: 3 };
  const all = [...course, jingle, ...rep, ...defs];
  const seen = new Set<string>();
  cached = all.filter((s) => (seen.has(s.id) ? false : (seen.add(s.id), true))).sort((a, b) => order[a.level] - order[b.level] || a.title.localeCompare(b.title));
  return cached;
}
