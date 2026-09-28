import { COURSE, lessonPiece } from './course';
import { makePiece, Piece } from './notation';

export interface SongEntry { id: string; title: string; composer: string; level: Level; piece: () => Piece }
export type Level = 'Beginner' | 'Intermediate' | 'Pre-Advanced' | 'Advanced';
export const LEVELS: { id: Level; tag: string; color: string }[] = [
  { id: 'Beginner', tag: 'BEG', color: '#4ADE80' }, { id: 'Intermediate', tag: 'INT', color: '#7FD3FF' },
  { id: 'Pre-Advanced', tag: 'PRE-ADV', color: '#FF9F43' }, { id: 'Advanced', tag: 'ADV', color: '#F87171' },
];

const extras: SongEntry[] = [
  {
    id: 'saints', title: 'When the Saints Go Marching In', composer: 'Traditional', level: 'Beginner',
    piece: () => makePiece('saints', 'When the Saints', { rh: 'r:q C4:q E4 F4 | G4:w | r:q C4 E4 F4 | G4:w | r:q C4 E4 F4 | G4:h E4:h | C4:h E4:h | D4:w', lh: 'r:w | C3:w | r:w | C3:w | r:w | C3:w | C3:w | G2:w', bpm: 110 }),
  },
  {
    id: 'greensleeves', title: 'Greensleeves', composer: 'Traditional English', level: 'Intermediate',
    piece: () => makePiece('greensleeves', 'Greensleeves', { rh: 'r:h A4:q | C5:h D5:q | E5:q. F5:e E5:q | D5:h B4:q | G4:q. A4:e B4:q | C5:h A4:q | A4:q. G#4:e A4:q | B4:h G#4:q | E4:h A4:q', lh: 'r:h. | A2:h. | C3:h. | G2:h. | E2:h. | A2:h. | F2:h. | E2:h. | E2:h.', time: [3, 4], bpm: 96 }),
  },
  {
    id: 'london-bridge', title: 'London Bridge', composer: 'Traditional', level: 'Beginner',
    piece: () => makePiece('london-bridge', 'London Bridge', { rh: 'G4:q. A4:e G4:q F4 | E4 F4 G4:h | D4:q E4 F4:h | E4:q F4 G4:h | G4:q. A4:e G4:q F4 | E4 F4 G4:h | D4:h G4:h | E4:q C4:h.', lh: 'C3:w | C3:w | G2:w | C3:w | C3:w | C3:w | G2:w | C3:w', bpm: 100 }),
  },
  {
    id: 'row-row', title: 'Row, Row, Row Your Boat', composer: 'Traditional', level: 'Beginner',
    piece: () => makePiece('row-row', 'Row, Row, Row Your Boat', { rh: 'C4:q. C4:q. | C4:q D4:e E4:q. | E4:q D4:e E4:q F4:e | G4:h. | C5:e C5 C5 G4 G4 G4 | E4 E4 E4 C4 C4 C4 | G4:q F4:e E4:q D4:e | C4:h.', time: [6, 8], bpm: 70 }),
  },
  {
    id: 'old-macdonald', title: 'Old MacDonald Had a Farm', composer: 'Traditional', level: 'Beginner',
    piece: () => makePiece('old-macdonald', 'Old MacDonald', { rh: 'C4:q C4 C4 G3 | A3 A3 G3:h | E4:q E4 D4 D4 | C4:w', lh: 'C3:w | F2:h C3:h | G2:w | C3:w', bpm: 110 }),
  },
  {
    id: 'brahms-lullaby', title: 'Lullaby (Wiegenlied)', composer: 'Johannes Brahms', level: 'Intermediate',
    piece: () => makePiece('brahms-lullaby', 'Brahms Lullaby', { rh: 'r:h E4:e E4:e | G4:h E4:e E4:e | G4:h E4:e G4:e | C5:q B4:h | A4:h D4:e E4:e | F4:h D4:e F4:e | B4:e A4:e G4:q B4:q | C5:h.', lh: 'r:h. | C3:h. | C3:h. | G2:h. | F2:h. | G2:h. | G2:h. | C3:h.', time: [3, 4], bpm: 84 }),
  },
  {
    id: 'canon-d', title: 'Canon in D (simplified)', composer: 'Johann Pachelbel', level: 'Intermediate',
    piece: () => makePiece('canon-d', 'Canon in D', { rh: 'F#5:h E5:h | D5:h C#5:h | B4:h A4:h | B4:h C#5:h | [D5 F#5]:h [C#5 E5]:h | [B4 D5]:h [A4 C#5]:h | [G4 B4]:h [F#4 A4]:h | [G4 B4]:h [A4 C#5]:h | [F#4 A4 D5]:w', lh: 'D3:h A2:h | B2:h F#2:h | G2:h D2:h | G2:h A2:h | D3:h A2:h | B2:h F#2:h | G2:h D2:h | G2:h A2:h | D2:w', key: 2, bpm: 66 }),
  },
  {
    id: 'moonlight', title: 'Moonlight Sonata, 1st mvt. (opening)', composer: 'Ludwig van Beethoven', level: 'Advanced',
    piece: () => makePiece('moonlight', 'Moonlight Sonata', { rh: 'G#3:et C#4 E4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 | G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4 | A3 C#4 E4 A3 C#4 E4 A3 D4 F#4 A3 D4 F#4 | G#3 B#3 F#4 G#3 C#4 E4 G#3 C#4 D#4 F#3 B#3 D#4 | E3 G#3 C#4 G#3 C#4 E4 G#3 C#4 E4 G#3 C#4 E4', lh: '[C#2 C#3]:w | [B1 B2]:w | [A1 A2]:h [F#1 F#2]:h | [G#1 G#2]:h [G#1 G#2]:h | [C#2 C#3]:w', key: 4, bpm: 52 }),
  },
  {
    id: 'turkish-march', title: 'Rondo alla Turca (opening)', composer: 'Wolfgang Amadeus Mozart', level: 'Advanced',
    piece: () => makePiece('turkish-march', 'Rondo alla Turca', { rh: 'B4:s A4 G#4 A4 C5:q | D5:s C5 B4 C5 E5:q | F5:s E5 D#5 E5 B5 A5 G#5 A5 | B5 A5 G#5 A5 C6:q | A5:q r:q', lh: 'r:h | A3:e [C4 E4]:e [C4 E4]:e [C4 E4]:e | A3:e [C4 E4]:e [C4 E4]:e [C4 E4]:e | A3:e [C4 E4]:e E3:e [B3 E4]:e | A2:q r:q', time: [2, 4], bpm: 116 }),
  },
];

/** Public-domain library: the course's songs + repertoire + extras, sorted by level. */
export function songLibrary(): SongEntry[] {
  const fromCourse: SongEntry[] = COURSE.filter((u) => u.song && !u.song.composer?.startsWith('Practice')).map((u) => ({
    id: `song-u${u.n}`, title: u.song!.title, composer: u.song!.composer ?? 'Traditional', level: u.level === 'Beginner' ? 'Beginner' : 'Intermediate',
    piece: () => lessonPiece(`song-u${u.n}`, u.song!.title, u.song!.ex),
  }));
  const extraCourse: SongEntry[] = [
    { id: 'lightly-row', title: 'Lightly Row', composer: 'Traditional', level: 'Beginner', piece: () => lessonPiece('lightly-row', 'Lightly Row', COURSE[4].lessons[0].ex) },
    { id: 'frere-jacques', title: 'Frère Jacques', composer: 'Traditional French', level: 'Beginner', piece: () => lessonPiece('frere-jacques', 'Frère Jacques', COURSE[4].lessons[1].ex) },
    { id: 'jingle-bells', title: 'Jingle Bells (chorus)', composer: 'James Lord Pierpont', level: 'Beginner', piece: () => lessonPiece('jingle-bells', 'Jingle Bells', COURSE[4].lessons[3].ex) },
  ];
  const rep: SongEntry[] = COURSE.find((u) => u.n === 24)!.lessons.map((l) => {
    const [composer, title] = l.title.split(' – ');
    const level: Level = /Bach|Mozart|Satie|Chopin/.test(composer) ? 'Pre-Advanced' : 'Advanced';
    return { id: `rep-${l.id}`, title, composer, level, piece: () => lessonPiece(`rep-${l.id}`, title, l.ex) };
  });
  const order = { Beginner: 0, Intermediate: 1, 'Pre-Advanced': 2, Advanced: 3 };
  return [...fromCourse, ...extraCourse, ...extras, ...rep].sort((a, b) => order[a.level] - order[b.level]);
}
