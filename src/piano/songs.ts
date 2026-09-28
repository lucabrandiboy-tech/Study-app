import { COURSE, lessonPiece } from './course';
import { makePiece, Piece } from './notation';

export interface SongEntry { id: string; title: string; composer: string; level: 'Beginner' | 'Intermediate' | 'Advanced'; piece: () => Piece }

const extras: SongEntry[] = [
  {
    id: 'saints', title: 'When the Saints Go Marching In', composer: 'Traditional', level: 'Beginner',
    piece: () => makePiece('saints', 'When the Saints', { rh: 'r:q C4:q E4 F4 | G4:w | r:q C4 E4 F4 | G4:w | r:q C4 E4 F4 | G4:h E4:h | C4:h E4:h | D4:w', lh: 'r:w | C3:w | r:w | C3:w | r:w | C3:w | C3:w | G2:w', bpm: 110 }),
  },
  {
    id: 'greensleeves', title: 'Greensleeves', composer: 'Traditional English', level: 'Intermediate',
    piece: () => makePiece('greensleeves', 'Greensleeves', { rh: 'r:h A4:q | C5:h D5:q | E5:q. F5:e E5:q | D5:h B4:q | G4:q. A4:e B4:q | C5:h A4:q | A4:q. G#4:e A4:q | B4:h G#4:q | E4:h A4:q', lh: 'r:h. | A2:h. | C3:h. | G2:h. | E2:h. | A2:h. | F2:h. | E2:h. | E2:h.', time: [3, 4], bpm: 96 }),
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
    return { id: `rep-${l.id}`, title, composer, level: 'Advanced' as const, piece: () => lessonPiece(`rep-${l.id}`, title, l.ex) };
  });
  const order = { Beginner: 0, Intermediate: 1, Advanced: 2 };
  return [...fromCourse, ...extraCourse, ...extras, ...rep].sort((a, b) => order[a.level] - order[b.level]);
}
