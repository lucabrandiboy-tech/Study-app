/** Original Pre-Advanced pieces written in modern pop styles (all melodies are original to this app). */
import type { SongDef } from './make';

const x2 = (s: string) => `${s} | ${s}`;

export const STYLE_POP: SongDef[] = [
  // ---- Dark, whispery bedroom-pop (in the style of Billie Eilish) ----
  { id: 'sp-whisper-room', t: 'Whisper Room', c: 'Original · dark bedroom-pop style (like Billie Eilish)', lv: 'P', cat: 'Originals', bpm: 72,
    rh: x2('r:e E4:e E4 D4 E4:q. C4:e | D4:e D4 C4 D4 B3:h | r:e E4:e E4 D4 E4:q G4:e F4 | E4:h. r:q') + ' | ' + x2('A4:e. G4:s E4:e D4 E4:q r:e C4:e | D4:e E4 G4 E4 D4:h | C4:e. D4:s E4:e G4 A4:q G4:e E4 | E4:w') + ' | [A3 C4 E4]:w',
    lh: x2('[A1 A2]:w | [F1 F2]:w | [C2 C3]:w | [E1 E2]:w') + ' | ' + x2('A2:e E3 A3 E3 A2 E3 A3 E3 | F2:e C3 F3 C3 F2 C3 F3 C3 | C2:e G2 C3 G2 C2 G2 C3 G2 | E2:e B2 E3 B2 E2 B2 G#3 B2') + ' | [A1 A2]:w' },
  { id: 'sp-ocean-static', t: 'Ocean Static', c: 'Original · dark bedroom-pop style (like Billie Eilish)', lv: 'P', cat: 'Originals', key: -3, bpm: 66, time: [6, 8],
    rh: x2('C5:e Bb4 G4 G4:q. | Ab4:e G4 F4 G4:q. | Eb4:e F4 G4 Bb4:q G4:e | F4:h.') + ' | ' + x2('G4:q G4:e Ab4:q G4:e | F4:e Eb4 D4 Eb4:q. | C4:e D4 Eb4 G4:q F4:e | Eb4:h.') + ' | [C4 Eb4 G4]:h.',
    ch: x2('Cm | Ab | Eb | Bb') + ' | ' + x2('Cm | Ab | Fm | Cm') + ' | Cm' },
  { id: 'sp-lights-out', t: 'Lights Out', c: 'Original · dark bedroom-pop style (like Billie Eilish)', lv: 'P', cat: 'Originals', key: -1, bpm: 84,
    rh: x2('D4:s D4 F4:e F4:s E4 D4:e A4:q r:q | G4:s G4 F4:e E4:s D4 C#4:e D4:h | D4:s D4 F4:e A4:s A4 Bb4:e A4:q F4:q | E4:e F4 E4 D4 C#4:h') + ' | [D4 F4 A4]:w',
    ch: x2('Dm | Gm:2 A:2 | Dm | Bb:2 A:2') + ' | Dm' },

  // ---- Quirky offbeat alt-pop/rock (in the style of Oliver Tree) ----
  { id: 'sp-scooter-hero', t: 'Scooter Hero', c: 'Original · quirky alt-pop style (like Oliver Tree)', lv: 'P', cat: 'Originals', bpm: 116,
    rh: x2('r:e G4:e G4 G4 A4:q G4:e E4 | r:e D4:e E4 G4 A4:h | r:e C5:e C5 C5 B4:q A4:e G4 | A4:e G4 E4 D4 E4:h') + ' | ' + x2('C5:q. B4:e A4:q G4 | A4:e G4 E4 D4 C4:h | E4:q. G4:e A4:q C5 | B4:e A4 G4 A4 G4:h') + ' | [C4 E4 G4 C5]:w',
    ch: x2('C | Am | F | G') + ' | ' + x2('F | C | Am | G') + ' | C' },
  { id: 'sp-bowl-cut', t: 'Bowl Cut Blues', c: 'Original · quirky alt-pop style (like Oliver Tree)', lv: 'P', cat: 'Originals', key: 1, bpm: 104,
    rh: x2('G4:e B4 D5 B4 C5:q B4 | A4:e. G4:s E4:e G4 A4:h | G4:e B4 D5 E5 D5:q B4 | A4:e G4 F#4 A4 G4:h') + ' | ' + x2('E5:e D5 B4 D5 E5:q. D5:e | B4:e A4 G4 A4 B4:h | C5:e B4 A4 G4 E4:q. G4:e | A4:e B4 A4 F#4 G4:h') + ' | [G4 B4 D5 G5]:w',
    ch: x2('G | C:2 D:2 | G | D:2 G:2') + ' | ' + x2('Em | C | Am | D') + ' | G' },
  { id: 'sp-alien-mall', t: 'Alien at the Mall', c: 'Original · quirky alt-pop style (like Oliver Tree)', lv: 'P', cat: 'Originals', key: -1, bpm: 120,
    rh: x2('F4:s F4 A4:e C5:s C5 A4:e F4:q r:q | G4:s G4 Bb4:e D5:s D5 Bb4:e G4:q r:q | A4:e C5 F5 E5 D5:q C5 | Bb4:e A4 G4 A4 F4:h') + ' | [F4 A4 C5 F5]:w',
    ch: x2('F | Gm | F:2 Bb:2 | C:2 F:2') + ' | F' },

  // ---- Big dance-pop anthems (in the style of Lady Gaga) ----
  { id: 'sp-disco-heart', t: 'Disco Heart', c: 'Original · dance-pop anthem style (like Lady Gaga)', lv: 'P', cat: 'Originals', bpm: 120,
    rh: x2('A4:e A4 C5 A4 E5:q D5:e C5 | B4:e B4 D5 B4 E5:h | A4:e A4 C5 E5 G5:q F5:e E5 | D5:e E5 D5 C5 B4:h') + ' | ' + x2('[A4 A5]:q [G4 G5] [E4 E5] [C5 C6] | [B4 B5]:h. [G4 G5]:q | [A4 A5]:q [G4 G5] [E4 E5] [D5 D6] | [C5 C6]:e [B4 B5] [A4 A5] [G4 G5] [A4 A5]:h') + ' | [A4 C5 E5 A5]:w',
    ch: x2('Am | Em | F | G') + ' | ' + x2('Am | Em | F | G') + ' | Am' },
  { id: 'sp-runway', t: 'Runway Lights', c: 'Original · dance-pop anthem style (like Lady Gaga)', lv: 'P', cat: 'Originals', key: 2, bpm: 124,
    rh: x2('B4:e B4 B4 A4 B4:q D5 | C#5:e B4 A4 F#4 A4:h | B4:e B4 B4 D5 F#5:q E5 | D5:e C#5 B4 A4 B4:h') + ' | ' + x2('F#5:e. E5:s D5:e E5 F#5:q A5 | G5:e F#5 E5 D5 E5:h | D5:e. C#5:s B4:e C#5 D5:q F#5 | E5:e D5 C#5 A4 B4:h') + ' | [B4 D5 F#5 B5]:w',
    ch: x2('Bm | A | G | F#m') + ' | ' + x2('D | A | G | A') + ' | Bm' },
  { id: 'sp-monster-waltz', t: 'Masquerade Monster', c: 'Original · theatrical pop style (like Lady Gaga)', lv: 'P', cat: 'Originals', time: [3, 4], bpm: 132,
    rh: x2('E5:q D#5 E5 | B4:h. | C5:q B4 A4 | G#4:h E4:q | A4:q C5 E5 | A5:h G5:q | F5:q E5 D5 | E5:h.') + ' | [A4 C5 E5 A5]:h.',
    ch: x2('Am | E | Am | E | Am | C | Dm:2 E:1 | Am') + ' | Am' },
];
