/** Famous real songs that are in the public domain (written before 1929). */
import type { SongDef } from './make';

export const REAL_CLASSICS: SongDef[] = [
  { id: 'oh-susanna', t: 'Oh! Susanna', c: 'Stephen Foster (1848)', lv: 'I', cat: 'Folk & World', bpm: 112,
    rh: 'r:h r:q C4:e D4 | E4:q G4 G4:q. A4:e | G4:q E4 C4:q. D4:e | E4:q E4 D4 C4 | D4:h. C4:e D4 | E4:q G4 G4:q. A4:e | G4:q E4 C4:q. D4:e | E4:q E4 D4 D4 | C4:h. r:q | F4:h F4:h | A4:q A4:h A4:q | G4:q G4 E4 C4 | D4:h. C4:e D4 | E4:q G4 G4:q. A4:e | G4:q E4 C4:q. D4:e | E4:q E4 D4 D4 | C4:w',
    ch: 'N | C | C | C | G | C | C | G | C | F | F | C | G | C | C | G | C' },
  { id: 'camptown-races', t: 'Camptown Races', c: 'Stephen Foster (1850)', lv: 'I', cat: 'Folk & World', bpm: 116,
    rh: 'G4:e G4 E4 G4 A4 G4 E4:q | E4:e D4:q. E4:e D4:q. | G4:e G4 E4 G4 A4 G4 E4:q | D4:e E4 D4:q C4:h | G4:e G4 E4 G4 A4 G4 E4:q | E4:e D4:q. E4:e D4:q. | G4:e G4 E4 G4 A4 G4 E4:q | D4:e E4 D4:q C4:h | C4:q. C4:e E4:q G4 | C5:w | A4:q. A4:e C5:q A4 | G4:w | G4:q G4 E4:e E4 G4 G4 | A4:q G4 E4:h | D4:q E4:e F4 E4:q D4 | C4:w',
    ch: 'C | G | C | G:2 C:2 | C | G | C | G:2 C:2 | C | C | F | C | C | F:2 C:2 | G | C' },
  { id: 'amazing-grace', t: 'Amazing Grace', c: 'John Newton / Traditional (1779)', lv: 'I', cat: 'Hymns & Anthems', time: [3, 4], key: 1, bpm: 80,
    rh: 'r:h D4:q | G4:h B4:e G4 | B4:h A4:q | G4:h E4:q | D4:h D4:q | G4:h B4:e G4 | B4:h A4:q | D5:h. | D5:h B4:q | D5:h B4:e G4 | B4:h A4:q | G4:h E4:q | D4:h D4:q | G4:h B4:e G4 | B4:h A4:q | G4:h.',
    ch: 'N | G | G | C | G | G | G | D | D | G | G | C | G | G | D | G' },
  { id: 'clementine', t: 'Oh My Darling, Clementine', c: 'Percy Montrose (1884)', lv: 'B', cat: 'Folk & World', time: [3, 4], bpm: 100,
    rh: 'r:h C4:e C4 | C4:q G3 E4:e E4 | E4:q C4 C4:e E4 | G4:q. G4:e F4 E4 | D4:h D4:e E4 | F4:q F4 E4:e D4 | E4:q C4 C4:e E4 | D4:q. G3:e B3 D4 | C4:h.',
    ch: 'N | C | C | C | G | G | C | G | C' },
  { id: 'this-old-man', t: 'This Old Man', c: 'Traditional (1870s)', lv: 'B', cat: 'Kids', bpm: 104,
    rh: 'G4:q E4 G4:h | G4:q E4 G4:h | A4:q G4 F4 E4 | D4:q E4 F4:h | E4:q F4 G4 C4 | C4:e C4 C4:q C4:e D4 E4 F4 | G4:q D4 D4 F4 | E4:q D4 C4:h',
    ch: 'C | C | F | G | C | C | G | G:2 C:2' },
];
