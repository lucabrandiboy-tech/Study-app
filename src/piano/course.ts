import { Exercise, makePiece, Piece } from './notation';
import { scale, chordTones, invert, chord, broken, arpeggio, down8, mulberry32, spell } from './theory';

export interface Lesson { id: string; title: string; text: string; ex: Exercise }
export interface Unit {
  n: number; level: 'Beginner' | 'Intermediate' | 'Advanced' | 'Final Exam'; title: string;
  lessons: Lesson[]; song?: { title: string; composer?: string; ex: Exercise }; test?: Exercise; exam?: boolean;
}

const L = (unit: number, i: number, title: string, text: string, ex: Exercise): Lesson => ({ id: `u${unit}-l${i}`, title, text, ex });
const lh8 = (s: string) => s.replace(/([A-G][#b]?)(\d)/g, (_, n, o) => `${n}${Number(o) - 1}`);
const HS = (rh: string, lh: string, bars: number): Exercise => ({ rh: `${rh} ${'r:w '.repeat(bars)}`, lh: `${'r:w '.repeat(bars)}${lh}` });

// Daily sight-reading: a new melody every day, seeded by the date.
function sightReading(seedOffset: number, key: 'C' | 'G' | 'F', bothHands: boolean): Exercise {
  const d = new Date();
  const rnd = mulberry32(d.getFullYear() * 1000 + d.getMonth() * 40 + d.getDate() + seedOffset * 7919);
  const tonic = { C: 'C4', G: 'G4', F: 'F4' }[key];
  const scaleN = [0, 1, 2, 3, 4, 5, 6, 7].map((i) => spell(tonic, i, [0, 2, 4, 5, 7, 9, 11, 12][i]));
  const rhythms = [['q', 'q', 'q', 'q'], ['h', 'q', 'q'], ['q', 'q', 'h'], ['q.', 'e', 'q', 'q'], ['e', 'e', 'q', 'h'], ['h', 'h']];
  let idx = 0; const bars: string[] = [];
  for (let b = 0; b < 3; b++) {
    const r = rhythms[Math.floor(rnd() * rhythms.length)];
    bars.push(r.map((dur) => { idx = Math.max(0, Math.min(7, idx + [-2, -1, -1, 1, 1, 2, 0][Math.floor(rnd() * 7)])); return `${scaleN[idx]}:${dur}`; }).join(' '));
  }
  bars.push(`${scaleN[Math.min(idx, 4) === idx ? 2 : 4]}:h ${scaleN[0]}:h`);
  const root3 = down8(down8(tonic));
  const five3 = spell(root3, 4, 7);
  return { rh: bars.join(' | '), lh: bothHands ? `${root3}:w | ${five3}:w | ${root3}:w | ${five3}:h ${root3}:h` : undefined, bpm: 72, key: key === 'G' ? 1 : key === 'F' ? -1 : 0 };
}

const C = chordTones('C4', 'major'), F = chordTones('F4', 'major'), G = chordTones('G3', 'major'), Am = chordTones('A3', 'minor');

export const COURSE: Unit[] = [
  // ------------------------------- BEGINNER -------------------------------
  {
    n: 1, level: 'Beginner', title: 'Keyboard Basics',
    lessons: [
      L(1, 1, 'Finding C', `The piano repeats the same 12 keys over and over. **C** is the white key just to the LEFT of every group of **two black keys**.
**Middle C** (C4) is the C closest to the middle of the piano — your home base.
Play middle C, then the C one octave higher, then middle C again.`, { rh: 'C4:h C5:h | C4:w', bpm: 70 }),
      L(1, 2, 'Black key groups', `Black keys come in groups of **two** and **three**. These groups are your map of the keyboard.
Play the group of two black keys (C# and D#), then the group of three (F#, G#, A#).`, { rh: 'C#4:q D#4:q r:h | F#4:q G#4:q A#4:q r:q', bpm: 70 }),
      L(1, 3, 'Posture & finger numbers', `Sit tall at the front half of the bench, elbows level with the keys, feet flat. Curve your fingers like you're holding a bubble.
Fingers are numbered **1 (thumb) to 5 (pinky)** on both hands.
**C position (right hand):** thumb on middle C, then 2 on D, 3 on E, 4 on F, 5 on G. Play up and back down, one finger per key.`, { rh: 'C4:q D4 E4 F4 | G4:w | G4:q F4 E4 D4 | C4:w', bpm: 70 }),
      L(1, 4, 'Left-hand C position', `**Left hand C position:** pinky (5) on the C below middle C (C3), then 4 on D, 3 on E, 2 on F, thumb (1) on G.
Play up and back down. Keep your wrist loose and fingers curved.`, { lh: 'C3:q D3 E3 F3 | G3:w | G3:q F3 E3 D3 | C3:w', bpm: 70 }),
      L(1, 5, 'Steady pulse', `Music has a steady **beat**, like a heartbeat. Count "1, 2, 3, 4" out loud and play middle C on every beat with finger 1.
Listen to the metronome clicks and land exactly with them.`, { rh: 'C4:q C4 C4 C4 | C4:q C4 C4 C4 | C4:h C4:h | C4:w', bpm: 72 }),
    ],
    song: { title: 'Hot Cross Buns', composer: 'Traditional', ex: { rh: 'E4:q D4 C4:h | E4:q D4 C4:h | C4:q C4 D4 D4 | E4:q D4 C4:h', bpm: 80 } },
    test: { rh: 'C4:q D4 E4 F4 | G4:h E4:h | C#4:q D#4 r:h | C4:w', bpm: 76 },
  },
  {
    n: 2, level: 'Beginner', title: 'Reading Notes: Treble Clef',
    lessons: [
      L(2, 1, 'C, D, E', `The **treble clef** (𝄞) shows notes for the right hand. Middle C sits on a little line below the staff.
**D** hangs just below the bottom line, and **E** sits ON the bottom line.
Notes move by **steps** (line → space → line). Read and play C–D–E.`, { rh: 'C4:q D4 E4 D4 | C4:w', bpm: 76 }),
      L(2, 2, 'F and G', `**F** is in the first space, **G** is on the second line (the treble clef curls around the G line!).
Play E–F–G and back.`, { rh: 'E4:q F4 G4 F4 | E4:h C4:h', bpm: 76 }),
      L(2, 3, 'Skips', `A **skip** jumps over one note: line to line, or space to space.
C–E–G is a chain of skips. Watch how the notes jump on the staff.`, { rh: 'C4:q E4 G4 E4 | C4:w', bpm: 76 }),
      L(2, 4, 'A, B, and high C', `Keep climbing: **A** (2nd space), **B** (middle line), and **C5** (3rd space).
Move your hand up so your thumb is on G.`, { rh: 'G4:q A4 B4 C5 | B4:q A4 G4:h', bpm: 76 }),
      L(2, 5, 'The whole octave', `Now read the full octave from middle C up to C5 and back down. Cross your thumb under after E (finger 3) going up, and cross 3 over the thumb coming down.`, { rh: 'C4:q D4 E4 F4 | G4 A4 B4 C5 | C5 B4 A4 G4 | F4 E4 D4 C4', bpm: 72 }),
    ],
    song: { title: 'Mary Had a Little Lamb', composer: 'Traditional', ex: { rh: 'E4:q D4 C4 D4 | E4 E4 E4:h | D4:q D4 D4:h | E4:q G4 G4:h | E4:q D4 C4 D4 | E4 E4 E4 E4 | D4 D4 E4 D4 | C4:w', bpm: 90 } },
    test: { rh: 'C4:q E4 D4 F4 | E4 G4 F4 A4 | G4 B4 A4 C5 | G4:h C4:h', bpm: 76 },
  },
  {
    n: 3, level: 'Beginner', title: 'Reading Notes: Bass Clef',
    lessons: [
      L(3, 1, 'C, B, A', `The **bass clef** (𝄢) shows lower notes for the left hand. Its two dots surround the **F line**.
Middle C sits on a little line ABOVE the bass staff. Going down: **B** hangs just above the top line, **A** is ON the top line.`, { lh: 'C4:q B3 A3 B3 | C4:w', bpm: 76 }),
      L(3, 2, 'G and F', `**G** is in the top space and **F** is on the 4th line — the one between the bass clef's two dots.`, { lh: 'A3:q G3 F3 G3 | A3:h C4:h', bpm: 76 }),
      L(3, 3, 'E, D, C', `Keep going down: **E** (3rd space), **D** (middle line), **C3** (2nd space). Put your left pinky on C3 — that's LH C position!`, { lh: 'E3:q D3 C3 D3 | E3:w', bpm: 76 }),
      L(3, 4, 'Octave down', `Play the whole octave from middle C down to C3 and back up with your left hand. Cross finger 3 over your thumb going down.`, { lh: 'C4:q B3 A3 G3 | F3 E3 D3 C3 | C3 D3 E3 F3 | G3 A3 B3 C4', bpm: 72 }),
      L(3, 5, 'Skips in the bass', `Skips in bass clef: C3–E3–G3 are all in spaces. Read the pattern, not just the letters!`, { lh: 'C3:q E3 G3 E3 | C3:w', bpm: 76 }),
    ],
    song: { title: 'Au Clair de la Lune', composer: 'Traditional French', ex: { lh: 'C3:q C3 C3 D3 | E3:h D3:h | C3:q E3 D3 D3 | C3:w', bpm: 84 } },
    test: { lh: 'C4:q A3 B3 G3 | A3 F3 G3 E3 | F3 D3 E3 C3 | G3:h C3:h', bpm: 76 },
  },
  {
    n: 4, level: 'Beginner', title: 'Rhythm',
    lessons: [
      L(4, 1, 'Whole, half, quarter', `**Whole note** = 4 beats (hollow, no stem). **Half note** = 2 beats (hollow with stem). **Quarter note** = 1 beat (filled with stem).
Count out loud: "1-2-3-4" and hold each note its full length.`, { rh: 'C4:w | C4:h C4:h | C4:q C4 C4 C4 | C4:w', bpm: 80 }),
      L(4, 2, 'Rests', `**Rests** are silent beats — count them just like notes! A quarter rest (𝄽) = 1 beat, half rest (sits on the line) = 2 beats, whole rest (hangs from the line) = a whole measure.`, { rh: 'C4:q r:q C4:q r:q | C4:h r:h | r:q C4:q r:q C4:q | C4:w', bpm: 80 }),
      L(4, 3, '3/4 time', `A **time signature** tells you the beats per measure (top number) and which note gets the beat (bottom number).
**4/4**: 4 quarter-note beats per measure. **3/4**: 3 beats — the feel of a waltz: ONE-two-three.`, { rh: 'C4:q E4 G4 | C4:h. | G4:q E4 C4 | C4:h.', bpm: 90, time: [3, 4] }),
      L(4, 4, 'Dotted half notes', `A **dot** adds half the note's value. A dotted half note = 2 + 1 = **3 beats**. It fills a whole measure of 3/4.`, { rh: 'E4:h. | D4:h E4:q | C4:h D4:q | E4:h.', bpm: 90, time: [3, 4] }),
      L(4, 5, 'Counting out loud', `Mix all the rhythms. Say the counts out loud as you play: "1, 2, 3-4" for a quarter, quarter, half.`, { rh: 'C4:q C4 D4:h | E4:q E4 F4:h | G4:q r:q G4:q r:q | C5:w', bpm: 84 }),
    ],
    song: { title: 'Ode to Joy (simplified)', composer: 'Ludwig van Beethoven', ex: { rh: 'E4:q E4 F4 G4 | G4 F4 E4 D4 | C4 C4 D4 E4 | E4:h D4:h | E4:q E4 F4 G4 | G4 F4 E4 D4 | C4 C4 D4 E4 | D4:h C4:h', bpm: 96 } },
    test: { rh: 'C4:h D4:q E4:q | F4:w | E4:q r:q D4:q r:q | C4:w', bpm: 84 },
  },
  {
    n: 5, level: 'Beginner', title: 'Right-Hand Melodies (C Position)',
    lessons: [
      L(5, 1, 'Lightly Row', `A melody is a tune. Keep your right hand in C position and let each finger own one key.
Play **legato** — smooth and connected, lifting one finger as the next goes down.`, { rh: 'G4:q E4 E4:h | F4:q D4 D4:h | C4:q D4 E4 F4 | G4 G4 G4:h', bpm: 90 }),
      L(5, 2, 'Frère Jacques', `A **round** — the same tune can start at different times. Notice the repeated patterns: every phrase is played twice.`, { rh: 'C4:q D4 E4 C4 | C4 D4 E4 C4 | E4 F4 G4:h | E4:q F4 G4:h', bpm: 96 }),
      L(5, 3, 'Stepwise phrase', `Melodies are made of **phrases** — musical sentences. Breathe (lift slightly) at the end of each phrase.`, { rh: 'C4:q D4 E4 C4 | E4 F4 G4:h | G4:q F4 E4 D4 | C4:w', bpm: 92 }),
      L(5, 4, 'Jingle Bells', `This chorus repeats notes a lot. Keep the repeated E's even and relaxed — don't press harder, just lift and drop.`, { rh: 'E4:q E4 E4:h | E4:q E4 E4:h | E4:q G4 C4 D4 | E4:w | F4:q F4 F4 F4 | F4 E4 E4 E4 | E4 D4 D4 E4 | D4:h G4:h', bpm: 100 }),
    ],
    song: { title: 'Twinkle, Twinkle, Little Star', composer: 'Traditional', ex: { rh: 'C4:q C4 G4 G4 | A4 A4 G4:h | F4:q F4 E4 E4 | D4 D4 C4:h | G4:q G4 F4 F4 | E4 E4 D4:h | G4:q G4 F4 F4 | E4 E4 D4:h | C4:q C4 G4 G4 | A4 A4 G4:h | F4:q F4 E4 E4 | D4 D4 C4:h', bpm: 96 } },
    test: { rh: 'E4:q F4 G4 E4 | D4:h G4:h | C4:q E4 D4 F4 | E4:h C4:h', bpm: 90 },
  },
  {
    n: 6, level: 'Beginner', title: 'Left-Hand Melodies (C Position)',
    lessons: [
      L(6, 1, 'LH five-finger warm-up', `Left hand in C position: pinky on C3. Play up to G3 slowly and evenly. The left hand is usually weaker — go slow!`, { lh: 'C3:q D3 E3 F3 | G3:w | G3:q F3 E3 D3 | C3:w', bpm: 84 }),
      L(6, 2, 'Mary in the bass', `The same melody you played with the right hand — now in the left hand, one octave lower. Read it in bass clef.`, { lh: 'E3:q D3 C3 D3 | E3 E3 E3:h | D3:q D3 D3:h | E3:q G3 G3:h', bpm: 88 }),
      L(6, 3, 'Down and up', `Practice changing direction smoothly. Keep your pinky curved — don't let it collapse.`, { lh: 'G3:q F3 E3 D3 | C3:w | E3:q F3 G3 E3 | C3:w', bpm: 88 }),
      L(6, 4, 'Hot Cross Buns (LH)', `A familiar tune in the left hand. Thumb plays E3? No — in LH C position, finger 3 plays E3, 4 plays D3, 5 plays C3.`, { lh: 'E3:q D3 C3:h | E3:q D3 C3:h | C3:q C3 D3 D3 | E3:q D3 C3:h', bpm: 90 }),
    ],
    song: { title: 'Ode to Joy (left hand)', composer: 'Ludwig van Beethoven', ex: { lh: 'E3:q E3 F3 G3 | G3 F3 E3 D3 | C3 C3 D3 E3 | E3:h D3:h', bpm: 90 } },
    test: { lh: 'C3:q E3 D3 F3 | E3 G3 F3 D3 | E3:q D3 C3 D3 | C3:w', bpm: 86 },
  },
  {
    n: 7, level: 'Beginner', title: 'Both Hands Together',
    lessons: [
      L(7, 1, 'Taking turns', `First, hands take turns: right hand plays, then left hand answers. This helps your brain switch between clefs.`, { rh: 'C4:q D4 E4:h | r:w | E4:q D4 C4:h | r:w', lh: 'r:w | C3:q D3 E3:h | r:w | E3:q D3 C3:h', bpm: 84 }),
      L(7, 2, 'Parallel motion', `Now both hands at the SAME time, moving the same direction. Both thumbs are on the inside — think "mirror fingers" (RH 1 with LH 5).`, { rh: 'C4:q D4 E4 F4 | G4:w', lh: 'C3:q D3 E3 F3 | G3:w', bpm: 72 }),
      L(7, 3, 'Melody over a held note', `Right hand plays the melody while the left hand holds a long C. Keep holding for all 4 beats — count!`, { rh: 'E4:q D4 C4 D4 | E4 E4 E4:h', lh: 'C3:w | C3:w', bpm: 80 }),
      L(7, 4, 'Bass notes on beat 1', `The left hand plays one bass note at the start of each measure. The bass note G2 is below C position — reach down with finger 5.`, { rh: 'C4:q E4 G4 E4 | F4 D4 B3 D4 | C4:w', lh: 'C3:w | G2:w | C3:w', bpm: 76 }),
      L(7, 5, 'Twinkle with bass', `Twinkle with a left-hand part in half notes. Practice hands separately first if it feels tricky!`, { rh: 'C4:q C4 G4 G4 | A4 A4 G4:h | F4:q F4 E4 E4 | D4 D4 C4:h', lh: 'C3:h E3:h | F3:h E3:h | D3:h C3:h | G2:h C3:h', bpm: 80 }),
    ],
    song: { title: 'Ode to Joy (hands together)', composer: 'Ludwig van Beethoven', ex: { rh: 'E4:q E4 F4 G4 | G4 F4 E4 D4 | C4 C4 D4 E4 | E4:h D4:h | E4:q E4 F4 G4 | G4 F4 E4 D4 | C4 C4 D4 E4 | D4:h C4:h', lh: 'C3:w | G2:w | C3:w | G2:w | C3:w | G2:w | C3:w | G2:h C3:h', bpm: 88 } },
    test: { rh: 'C4:q E4 D4 F4 | E4:h D4:h | E4:q G4 F4 D4 | C4:w', lh: 'C3:w | G2:w | C3:w | C3:w', bpm: 80 },
  },

  // ----------------------------- INTERMEDIATE -----------------------------
  {
    n: 8, level: 'Intermediate', title: 'Eighth Notes, Dotted Notes & Ties',
    lessons: [
      L(8, 1, 'Eighth notes', `An **eighth note** is half a beat. Two eighths = one quarter. Count "1-and-2-and": numbers on the beat, "and" in between.`, { rh: 'C4:e D4 E4 F4 G4:q G4 | A4:e A4 G4:q F4:e F4 E4:q | D4:e E4 D4 C4 C4:h', bpm: 84 }),
      L(8, 2, 'Dotted quarter + eighth', `A **dotted quarter** = 1½ beats. It's usually followed by an eighth note to finish the beat: "1 - (2) and".`, { rh: 'C4:q. D4:e E4:q. F4:e | G4:h G4:h | A4:q. G4:e F4:q. E4:e | D4:w', bpm: 80 }),
      L(8, 3, 'Ties', `A **tie** is a curved line connecting two of the SAME note. Play it once and hold for both values combined.`, { rh: 'C4:h~ C4:q D4:q | E4:h~ E4:h | G4:q F4 E4 D4 | C4:w', bpm: 84 }),
      L(8, 4, 'Ode to Joy — real rhythm', `The real rhythm of Ode to Joy uses a dotted quarter + eighth at the end of the phrase.`, { rh: 'E4:q E4 F4 G4 | G4 F4 E4 D4 | C4 C4 D4 E4 | E4:q. D4:e D4:h', lh: 'C3:w | G2:w | C3:w | G2:w', bpm: 96 }),
    ],
    song: { title: 'Minuet in G (opening)', composer: 'Christian Petzold (attr. J.S. Bach)', ex: { rh: 'D5:q G4:e A4 B4 C5 | D5:q G4 G4 | E5:q C5:e D5 E5 F#5 | G5:q G4 G4 | C5:q D5:e C5 B4 A4 | B4:q C5:e B4 A4 G4 | F#4:q G4:e A4 B4 G4 | A4:h.', lh: 'G3:h A3:q | B3:h. | C4:h. | B3:h. | A3:h. | G3:h. | D4:h B3:q | D4:h D3:q', bpm: 100, time: [3, 4], key: 1 } },
    test: { rh: 'E4:e F4 G4 E4 C4:q. D4:e | E4:h~ E4:q r:q | G4:q. F4:e E4:e D4 C4:q | C4:w', bpm: 84 },
  },
  {
    n: 9, level: 'Intermediate', title: 'Sharps, Flats, Naturals & Key Signatures',
    lessons: [
      L(9, 1, 'Sharps', `A **sharp** (♯) raises a note by a half step — to the very next key on the right (often black).`, { rh: 'F4:q F#4 G4:h | C4:q C#4 D4:h | G4:q G#4 A4:h | F#4:w', bpm: 80 }),
      L(9, 2, 'Flats', `A **flat** (♭) lowers a note by a half step — to the very next key on the left.`, { rh: 'B4:q Bb4 A4:h | E4:q Eb4 D4:h | A4:q Ab4 G4:h | Bb4:w', bpm: 80 }),
      L(9, 3, 'Naturals & key of F', `A **key signature** at the start of each line means "play these notes sharp/flat all the time." **F major** has one flat: B♭.
A **natural** (♮) cancels a sharp or flat. Watch for the B natural in measure 2!`, { rh: 'F4:q G4 A4 Bb4 | C5:h B4:h | Bb4:q A4 G4 F4 | F4:w', bpm: 80, key: -1 }),
      L(9, 4, 'Key of G', `**G major** has one sharp: F♯. Every F in this piece is F♯, even without a sign next to it.`, { rh: 'G4:q A4 B4 C5 | D5:q F#4 G4:h | B4:q A4 G4 F#4 | G4:w', bpm: 84, key: 1 }),
      L(9, 5, 'Chromatic scale', `The **chromatic scale** plays every key, white and black. Fingering: 3 on black keys, 1 on white keys (2 when two whites are next to each other: E–F, B–C).`, { rh: 'C4:e C#4 D4 D#4 E4 F4 F#4 G4 | G4 F#4 F4 E4 D#4 D4 C#4 C4 | C4:w', bpm: 72 }),
    ],
    song: { title: 'Amazing Grace', composer: 'Traditional (John Newton)', ex: { rh: 'r:h D4:q | G4:h B4:e G4:e | B4:h A4:q | G4:h E4:q | D4:h D4:q | G4:h B4:e G4:e | B4:h A4:q | D5:h.', lh: 'r:h. | G3:h. | G3:h. | C3:h. | G3:h. | G3:h. | E3:h. | D3:h.', bpm: 84, time: [3, 4], key: 1 } },
    test: { rh: 'G4:q F#4 G4 A4 | Bb4:h A4:h | G4:q F4 E4 Eb4 | D4:w', bpm: 80 },
  },
  {
    n: 10, level: 'Intermediate', title: 'Major Scales',
    lessons: [
      L(10, 1, 'C major (hands separate)', `A **major scale** follows the pattern W-W-H-W-W-W-H (whole and half steps).
**Fingering RH**: 1-2-3, thumb under, 1-2-3-4-5. **LH**: 5-4-3-2-1, 3 crosses over, 3-2-1. Right hand first, then left.`, HS(scale('C4', 'major'), scale('C3', 'major'), 4)),
      L(10, 2, 'G major (hands together)', `G major has one sharp (F♯). Same fingering as C. Now play **hands together** — the thumbs cross at different times, so go slow!`, { rh: scale('G4', 'major'), lh: scale('G3', 'major'), key: 1, bpm: 72 }),
      L(10, 3, 'D major', `D major has two sharps: F♯ and C♯. Same fingering as C and G.`, { ...HS(scale('D4', 'major'), scale('D3', 'major'), 4), key: 2, bpm: 72 }),
      L(10, 4, 'F major', `F major has one flat: B♭. **Special RH fingering**: 1-2-3-4 (on B♭), thumb under to C, 1-2-3-4. LH uses normal fingering.`, { rh: scale('F4', 'major'), lh: scale('F3', 'major'), key: -1, bpm: 72 }),
      L(10, 5, 'B♭ major', `B♭ major has two flats: B♭ and E♭. RH: 2 on B♭, then 1 on C, 2-3 (E♭ is 4? no — RH: 4-1-2-3-1-2-3-4). LH: 3-2-1-4-3-2-1-3. Take it slow!`, { rh: scale('Bb3', 'major'), lh: scale('Bb2', 'major'), key: -2, bpm: 66 }),
    ],
    song: { title: 'Scale Etude in G', composer: 'Practice piece', ex: { rh: `${scale('G4', 'major', 1, 'e')} | ${chord(['G4', 'B4', 'D5'], 'w')}`, lh: 'G3:w | D3:w | G2:w', key: 1, bpm: 80 } },
    test: { rh: scale('D4', 'major'), lh: scale('D3', 'major'), key: 2, bpm: 76 },
  },
  {
    n: 11, level: 'Intermediate', title: 'Minor Scales',
    lessons: [
      L(11, 1, 'A natural minor', `Every major key has a **relative minor** that shares its key signature. A minor is the relative minor of C major — all white keys, from A to A.`, { rh: scale('A3', 'natural'), bpm: 76 }),
      L(11, 2, 'A harmonic minor', `**Harmonic minor** raises the 7th note by a half step. In A minor, G becomes **G♯**. It gives the scale an exotic sound.`, { rh: scale('A3', 'harmonic'), lh: scale('A2', 'harmonic'), bpm: 72 }),
      L(11, 3, 'E natural minor', `E minor is the relative minor of G major — one sharp (F♯).`, { ...HS(scale('E4', 'natural'), scale('E3', 'natural'), 4), key: 1, bpm: 72 }),
      L(11, 4, 'E harmonic minor', `Raise the 7th: D becomes **D♯**.`, { rh: scale('E4', 'harmonic'), lh: scale('E3', 'harmonic'), key: 1, bpm: 72 }),
      L(11, 5, 'D natural & harmonic minor', `D minor is the relative minor of F major (one flat, B♭). For harmonic minor, C becomes **C♯**. Natural first, then harmonic.`, { rh: `${scale('D4', 'natural')} | ${scale('D4', 'harmonic')}`, key: -1, bpm: 72 }),
    ],
    song: { title: 'Minor Melody (A harmonic minor)', composer: 'Practice piece', ex: { rh: 'A4:q B4 C5 B4 | A4:h G#4:h | A4:q C5 E5 D5 | C5:q B4 A4:h', lh: 'A2:w | E3:w | A2:w | E3:h A2:h', bpm: 84 } },
    test: { rh: scale('E4', 'harmonic'), key: 1, bpm: 76 },
  },
  {
    n: 12, level: 'Intermediate', title: 'Chords: Triads & Inversions',
    lessons: [
      L(12, 1, 'C major triad', `A **triad** has 3 notes stacked in thirds: **root, third, fifth**. C major = C–E–G. Play it **blocked** (together) and **broken** (one at a time). RH fingers 1-3-5.`, { rh: `${chord(C, 'h')} ${chord(C, 'h')} | ${broken(C, 'q')} r:q | ${chord(C, 'w')}`, bpm: 76 }),
      L(12, 2, 'Major triads: F and G', `Build triads on F (F–A–C) and G (G–B–D). Same shape, different starting key.`, { rh: `${chord(F, 'w')} | ${chord(chordTones('G4', 'major'), 'w')} | ${chord(C, 'w')}`, lh: 'F3:w | G3:w | C3:w', bpm: 72 }),
      L(12, 3, 'Minor triads', `A **minor triad** lowers the 3rd by a half step. C minor = C–E♭–G. Try A minor (A–C–E), D minor (D–F–A), and E minor (E–G–B).`, { rh: `${chord(chordTones('A3', 'minor'), 'w')} | ${chord(chordTones('D4', 'minor'), 'w')} | ${chord(chordTones('E4', 'minor'), 'w')} | ${chord(chordTones('C4', 'minor'), 'w')}`, bpm: 72 }),
      L(12, 4, 'Inversions of C', `**Inversions** rearrange the notes. Root position: C–E–G. **1st inversion**: E–G–C (3rd on bottom). **2nd inversion**: G–C–E (5th on bottom).`, { rh: `${chord(C, 'h')} ${chord(invert(C, 1), 'h')} | ${chord(invert(C, 2), 'h')} ${chord(invert(C, 3), 'h')} | ${chord(invert(C, 2), 'h')} ${chord(invert(C, 1), 'h')} | ${chord(C, 'w')}`, bpm: 72 }),
      L(12, 5, 'Inversions of G and F', `Same idea on G and F. Notice how inversions let your hand stay in one place.`, { rh: `${chord(invert(chordTones('G3', 'major'), 1), 'h')} ${chord(invert(chordTones('G3', 'major'), 2), 'h')} | ${chord(F, 'h')} ${chord(invert(F, 1), 'h')} | ${chord(invert(F, 2), 'w')}`, bpm: 72 }),
    ],
    song: { title: 'Chord Waltz', composer: 'Practice piece', ex: { rh: `r:q ${chord(['E4', 'G4'], 'q')} ${chord(['E4', 'G4'], 'q')} | r:q ${chord(['F4', 'A4'], 'q')} ${chord(['F4', 'A4'], 'q')} | r:q ${chord(['D4', 'G4'], 'q')} ${chord(['D4', 'G4'], 'q')} | r:q ${chord(['E4', 'G4'], 'q')} ${chord(['E4', 'G4'], 'q')}`, lh: 'C3:h. | F3:h. | G2:h. | C3:h.', time: [3, 4], bpm: 100 } },
    test: { rh: `${chord(C, 'h')} ${chord(invert(C, 1), 'h')} | ${chord(chordTones('A3', 'minor'), 'h')} ${chord(F, 'h')} | ${chord(invert(chordTones('G3', 'major'), 1), 'w')}`, bpm: 72 },
  },
  {
    n: 13, level: 'Intermediate', title: 'Chord Progressions & Accompaniment',
    lessons: [
      L(13, 1, 'I – IV – V – I', `Roman numerals name chords by scale step. In C: **I** = C, **IV** = F, **V** = G. Use inversions so your right hand barely moves: C–E–G → C–F–A → B–D–G → C–E–G. LH plays the roots.`, { rh: `${chord(C, 'w')} | ${chord(['C4', 'F4', 'A4'], 'w')} | ${chord(['B3', 'D4', 'G4'], 'w')} | ${chord(C, 'w')}`, lh: 'C3:w | F2:w | G2:w | C3:w', bpm: 72 }),
      L(13, 2, 'I – V – vi – IV', `This is the progression in hundreds of pop songs! In C: C – G – Am – F. Lowercase **vi** means minor.`, { rh: `${chord(C, 'w')} | ${chord(['B3', 'D4', 'G4'], 'w')} | ${chord(['C4', 'E4', 'A4'], 'w')} | ${chord(['C4', 'F4', 'A4'], 'w')}`, lh: 'C3:w | G2:w | A2:w | F2:w', bpm: 76 }),
      L(13, 3, 'Alberti bass', `**Alberti bass** breaks a chord into the pattern low–high–middle–high. Mozart loved it! LH: C–G–E–G.`, { rh: 'E4:h G4:h | F4:h D4:h | C4:w', lh: 'C3:e G3 E3 G3 C3 G3 E3 G3 | B2 G3 D3 G3 B2 G3 D3 G3 | C3 G3 E3 G3 C3:h', bpm: 80 }),
      L(13, 4, 'Broken-chord accompaniment', `A flowing LH pattern (root–5th–octave) under a simple melody. Keep the LH soft so the melody sings.`, { rh: 'G4:h A4:h | B4:h A4:h | G4:w', lh: 'G2:e D3 G3 D3 G2 D3 G3 D3 | C3 G3 C4 G3 D3 A3 D4 A3 | G2 D3 G3 D3 G2:h', key: 1, bpm: 80 }),
      L(13, 5, 'Progression in G', `I–IV–V–I in G major: G – C – D – G. Transpose the same hand shapes up.`, { rh: `${chord(['G4', 'B4', 'D5'], 'w')} | ${chord(['G4', 'C5', 'E5'], 'w')} | ${chord(['F#4', 'A4', 'D5'], 'w')} | ${chord(['G4', 'B4', 'D5'], 'w')}`, lh: 'G2:w | C3:w | D3:w | G2:w', key: 1, bpm: 72 }),
    ],
    song: { title: 'Pop Progression Groove', composer: 'Practice piece', ex: { rh: 'E5:q D5 C5 D5 | D5:h B4:h | C5:q B4 A4 C5 | A4:w', lh: 'C3:e G3 E3 G3 C3 G3 E3 G3 | G2 D3 B2 D3 G2 D3 B2 D3 | A2 E3 C3 E3 A2 E3 C3 E3 | F2 C3 A2 C3 F2:h', bpm: 84 } },
    test: { rh: `${chord(C, 'h')} ${chord(['C4', 'F4', 'A4'], 'h')} | ${chord(['B3', 'D4', 'G4'], 'h')} ${chord(C, 'h')}`, lh: 'C3:h F2:h | G2:h C3:h', bpm: 72 },
  },
  {
    n: 14, level: 'Intermediate', title: 'Dynamics & Articulation',
    lessons: [
      L(14, 1, 'Staccato', `**Staccato** (a dot above/below the note) means short and detached — bounce off the key. Here the short notes are written as eighths with rests.`, { rh: 'C4:e r:e D4:e r:e E4:e r:e F4:e r:e | G4:e r:e G4:e r:e G4:h | E4:e r:e E4:e r:e C4:h', bpm: 90 }),
      L(14, 2, 'Legato', `**Legato** (under a slur — a curved line over different notes) means smooth and connected. Overlap each note with the next.`, { rh: 'C4:q D4 E4 F4 | G4 F4 E4 D4 | E4 F4 G4 A4 | G4:w', bpm: 76 }),
      L(14, 3, 'Piano & forte', `**p (piano)** = soft. **f (forte)** = loud. Also **mp** (medium soft) and **mf** (medium loud). Play the first phrase **p** and the echo **f** — using a MIDI keyboard, press gently vs. firmly.`, { rh: 'C4:q E4 G4:h | C5:q G4 E4:h | C4:q E4 G4:h | C5:q G4 E4:h', bpm: 84 }),
      L(14, 4, 'Crescendo & accents', `**Crescendo** (<) = get gradually louder; **decrescendo** (>) = gradually softer. An **accent** (>) above one note means play it with extra weight. Grow louder as you climb, softer as you come down.`, { rh: 'C4:q D4 E4 F4 | G4 A4 B4 C5 | C5 B4 A4 G4 | F4 E4 D4 C4', bpm: 84 }),
    ],
    song: { title: 'Dynamics Study', composer: 'Practice piece', ex: { rh: 'G4:e r:e G4:e r:e A4:q G4 | C5:w | C5:q B4 A4 G4 | E4:h C4:h', lh: 'C3:w | E3:w | F3:h G3:h | C3:w', bpm: 88 } },
    test: { rh: 'E4:e r:e E4:e r:e F4:q G4 | G4:q F4 E4 D4 | C4:e r:e E4:e r:e G4:h | C5:w', bpm: 88 },
  },
  {
    n: 15, level: 'Intermediate', title: 'The Sustain Pedal',
    lessons: [
      L(15, 1, 'Pedal basics', `The **sustain pedal** (right pedal) keeps notes ringing after you let go. Keep your heel on the floor and press with the ball of your foot. On a computer keyboard, hold **Shift**.
Press the pedal, play each chord, then lift the pedal before the next chord.`, { rh: `${chord(C, 'w')} | ${chord(F, 'w')} | ${chord(['B3', 'D4', 'G4'], 'w')} | ${chord(C, 'w')}`, lh: 'C3:w | F2:w | G2:w | C3:w', bpm: 60 }),
      L(15, 2, 'Legato pedaling', `**Legato (syncopated) pedaling**: play the new chord, THEN quickly lift and re-press the pedal. "Hands down, foot up-down." This avoids blurry sound and gaps.`, { rh: `${chord(C, 'h')} ${chord(['C4', 'F4', 'A4'], 'h')} | ${chord(['B3', 'D4', 'G4'], 'h')} ${chord(C, 'h')}`, lh: 'C3:h F2:h | G2:h C3:h', bpm: 66 }),
      L(15, 3, 'Broken chords with pedal', `With pedal held for the whole measure, the broken chord blends into a rich sound. Change pedal at each new measure.`, { rh: '', lh: `${broken(['C3', 'G3', 'C4', 'E4', 'G4', 'E4', 'C4', 'G3'])} | ${broken(['A2', 'E3', 'A3', 'C4', 'E4', 'C4', 'A3', 'E3'])} | ${broken(['F2', 'C3', 'F3', 'A3', 'C4', 'A3', 'F3', 'C3'])} | ${broken(['G2', 'D3', 'G3', 'B3', 'D4', 'B3', 'G3', 'D3'])}`, bpm: 72 }),
      L(15, 4, 'Pedal + melody', `Melody in the right hand, LH chord on beat one, pedal changing every measure.`, { rh: 'E5:h D5:h | C5:h A4:h | C5:q B4 A4 B4 | C5:w', lh: `${chord(['C3', 'G3'], 'w')} | ${chord(['A2', 'E3'], 'w')} | ${chord(['F2', 'C3'], 'h')} ${chord(['G2', 'D3'], 'h')} | ${chord(['C3', 'G3'], 'w')}`, bpm: 70 }),
    ],
    song: { title: 'Gymnopédie-style Study', composer: 'Practice piece (in the style of Satie)', ex: { rh: 'r:q r:q r:q | r:q F#5:q A5:q | G5:q F#5:q C#5:q | B4:q C#5:q D5:q | A4:h.', lh: `G2:q ${chord(['B3', 'D4', 'F#4'], 'h')} | D2:q ${chord(['A3', 'C#4', 'F#4'], 'h')} | G2:q ${chord(['B3', 'D4', 'F#4'], 'h')} | D2:q ${chord(['A3', 'C#4', 'F#4'], 'h')} | G2:h.`, time: [3, 4], key: 2, bpm: 72 } },
    test: { rh: `${chord(C, 'h')} ${chord(chordTones('A3', 'minor'), 'h')} | ${chord(['C4', 'F4', 'A4'], 'h')} ${chord(['B3', 'D4', 'G4'], 'h')}`, lh: 'C3:h A2:h | F2:h G2:h', bpm: 66 },
  },

  // ------------------------------- ADVANCED -------------------------------
  {
    n: 16, level: 'Advanced', title: 'All Scales & Arpeggios',
    lessons: [
      L(16, 1, 'Two-octave C major (eighths)', `Two octaves means crossing the thumb twice. Keep your wrist level and let the thumb glide under early.`, { rh: scale('C4', 'major', 2, 'e'), lh: scale('C3', 'major', 2, 'e'), bpm: 72 }),
      L(16, 2, 'E major & A major', `E major (4 sharps) and A major (3 sharps) use standard 1-2-3-1-2-3-4-5 fingering in the RH.`, { rh: `${scale('E4', 'major', 1, 'e')} | ${scale('A3', 'major', 1, 'e')}`, key: 4, bpm: 76 }),
      L(16, 3, 'E♭ major & A♭ major', `Flat keys start on black keys. RH E♭: 3-1-2-3-4-1-2-3. RH A♭: 3-4-1-2-3-1-2-3.`, { rh: `${scale('Eb4', 'major', 1, 'e')} | ${scale('Ab3', 'major', 1, 'e')}`, key: -3, bpm: 72 }),
      L(16, 4, 'Major arpeggios', `An **arpeggio** is a broken chord spread across octaves. RH fingering: 1-2-3, thumb under, 1-2-3-5.`, { rh: `${arpeggio('C4', 'major', 2)} | ${arpeggio('G3', 'major', 2)}`, bpm: 80 }),
      L(16, 5, 'Minor arpeggios', `Minor arpeggios: A minor (A–C–E) and E minor (E–G–B). Same fingering as major.`, { rh: `${arpeggio('A3', 'minor', 2)} | ${arpeggio('E4', 'minor', 1)}`, bpm: 80 }),
      L(16, 6, 'Circle of scales: B major & F# major', `B major (5 sharps) and F♯ major (6 sharps). RH B major: 1-2-3-1-2-3-4-5. RH F♯: 2-3-4-1-2-3-1-2.`, { rh: `${scale('B3', 'major', 1, 'e')} | ${scale('F#4', 'major', 1, 'e')}`, key: 5, bpm: 68 }),
    ],
    song: { title: 'Arpeggio Etude', composer: 'Practice piece', ex: { rh: `${broken(['C4', 'E4', 'G4', 'C5', 'E5', 'C5', 'G4', 'E4'])} | ${broken(['A3', 'C4', 'E4', 'A4', 'C5', 'A4', 'E4', 'C4'])} | ${broken(['F3', 'A3', 'C4', 'F4', 'A4', 'F4', 'C4', 'A3'])} | ${broken(['G3', 'B3', 'D4', 'G4', 'B4', 'G4', 'D4', 'B3'])} | ${chord(C, 'w')}`, lh: 'C3:w | A2:w | F2:w | G2:w | C2:w', bpm: 84 } },
    test: { rh: scale('G3', 'major', 2, 'e'), lh: scale('G2', 'major', 2, 'e'), key: 1, bpm: 76 },
  },
  {
    n: 17, level: 'Advanced', title: 'Seventh Chords & Voicings',
    lessons: [
      L(17, 1, 'Major 7th', `A **seventh chord** adds a 7th above the root. **Cmaj7** = C–E–G–B. Dreamy and jazzy!`, { rh: `${chord(chordTones('C4', 'maj7'), 'w')} | ${chord(chordTones('F4', 'maj7'), 'w')}`, lh: 'C3:w | F2:w', bpm: 66 }),
      L(17, 2, 'Dominant 7th', `**Dominant 7th** (C7) = C–E–G–B♭. It wants to resolve — G7 pulls strongly to C.`, { rh: `${chord(chordTones('G3', 'dom7'), 'w')} | ${chord(C, 'w')} | ${chord(chordTones('C4', 'dom7'), 'w')} | ${chord(F, 'w')}`, lh: 'G2:w | C3:w | C3:w | F2:w', bpm: 66 }),
      L(17, 3, 'Minor 7th & half-diminished', `**Minor 7th** (Dm7) = D–F–A–C. **Half-diminished** (Bm7♭5) = B–D–F–A.`, { rh: `${chord(chordTones('D4', 'min7'), 'w')} | ${chord(chordTones('A3', 'min7'), 'w')} | ${chord(chordTones('B3', 'm7b5'), 'w')}`, bpm: 66 }),
      L(17, 4, 'ii – V – I voicings', `The jazz progression **ii–V–I** in C: Dm7 – G7 – Cmaj7. **Voice leading**: keep common tones and move others by step. LH plays roots.`, { rh: `${chord(['F4', 'A4', 'C5'], 'w')} | ${chord(['F4', 'B4', 'D5'], 'w')} | ${chord(['E4', 'G4', 'B4'], 'w')}`, lh: 'D3:w | G2:w | C3:w', bpm: 66 }),
      L(17, 5, 'Extended chords (9ths)', `Add the **9th** (the 2nd, an octave up) for color. Cmaj9 = C–E–G–B–D. Rootless voicing in RH: E–G–B–D with C in the LH.`, { rh: `${chord(['E4', 'G4', 'B4', 'D5'], 'w')} | ${chord(['F4', 'A4', 'C5', 'E5'], 'w')}`, lh: 'C3:w | D3:w', bpm: 60 }),
    ],
    song: { title: 'Jazz ii–V–I Study', composer: 'Practice piece', ex: { rh: `r:h ${chord(['F4', 'A4', 'C5'], 'h')} | r:h ${chord(['F4', 'B4', 'D5'], 'h')} | r:h ${chord(['E4', 'G4', 'B4'], 'h')} | ${chord(['E4', 'G4', 'B4', 'D5'], 'w')}`, lh: 'D3:q A3 D3 A3 | G2 D3 G2 D3 | C3 G3 C3 G3 | C3:w', bpm: 80 } },
    test: { rh: `${chord(chordTones('D4', 'min7'), 'w')} | ${chord(chordTones('G3', 'dom7'), 'w')} | ${chord(chordTones('C4', 'maj7'), 'w')}`, lh: 'D3:w | G2:w | C3:w', bpm: 66 },
  },
  {
    n: 18, level: 'Advanced', title: 'Complex Rhythms',
    lessons: [
      L(18, 1, 'Triplets', `A **triplet** fits 3 notes into the space of 2. Eighth-note triplets = 3 per beat: "tri-po-let, tri-po-let".`, { rh: 'C4:et D4 E4 F4:et G4 A4 G4:q F4 | E4:et F4 G4 E4:et D4 C4 D4:h | C4:w', bpm: 72 }),
      L(18, 2, 'Sixteenth notes', `**Sixteenth notes** = 4 per beat: "1-e-and-a". Keep them even and light.`, { rh: 'C4:s D4 E4 F4 G4:q G4:s A4 G4 F4 E4:q | D4:s E4 F4 E4 D4:q C4:h | C4:w', bpm: 66 }),
      L(18, 3, 'Syncopation', `**Syncopation** puts the accent OFF the beat. Notes on the "and" that are tied or held create that bouncy feel.`, { rh: 'C4:e E4:q E4:e~ E4:q G4:q | G4:e F4:q E4:e~ E4:h | D4:e D4:q E4:e~ E4:q C4:q | C4:w', lh: 'C3:h C3:h | F2:h F2:h | G2:h G2:h | C3:w', bpm: 84 }),
      L(18, 4, '6/8 time', `**6/8** has 6 eighth notes per measure, felt in **2 big beats** (ONE-two-three FOUR-five-six).`, { rh: 'C4:e E4 G4 C5:q. | B4:e G4 D4 G4:q. | A4:e F4 C4 F4:q. | G4:q. C4:q.', lh: 'C3:q. C3:q. | G2:q. G2:q. | F2:q. F2:q. | G2:q. C3:q.', time: [6, 8], bpm: 72 }),
      L(18, 5, 'Dotted eighth + sixteenth', `A **dotted eighth + sixteenth** gives a "long-short" skip rhythm, like a gallop.`, { rh: 'C4:e. D4:s E4:e. F4:s G4:h | A4:e. G4:s F4:e. E4:s D4:h | C4:w', bpm: 72 }),
    ],
    song: { title: 'Syncopated Rag Study', composer: 'Practice piece (ragtime style)', ex: { rh: 'E4:e G4:q E4:e C5:e B4:q G4:e | A4:e G4:q E4:e~ E4:h | D4:e F4:q D4:e B4:e A4:q F4:e | G4:w', lh: 'C3:q G3 C3 G3 | C3 G3 C3 G3 | G2 D3 G2 D3 | G2 D3 C3:h', bpm: 84 } },
    test: { rh: 'C4:et D4 E4 F4:q G4:s F4 E4 D4 C4:q | E4:e G4:q E4:e~ E4:h | C4:w', bpm: 72 },
  },
  {
    n: 19, level: 'Advanced', title: 'Hand Independence',
    lessons: [
      L(19, 1, 'Quarters vs. eighths', `LH plays steady quarters while RH plays eighths. Count the eighths out loud and let the LH lock onto the numbers.`, { rh: 'E4:e F4 G4 E4 F4 G4 A4 F4 | G4 A4 B4 G4 C5:h', lh: 'C3:q G2 C3 G2 | C3 G2 C3:h', bpm: 76 }),
      L(19, 2, 'Long vs. short', `RH holds long notes while LH moves in quarters. Don't let the RH lift early!`, { rh: 'G4:w | A4:w | G4:h F4:h | E4:w', lh: 'C3:q E3 G3 E3 | F3 A3 C4 A3 | G2 B2 D3 B2 | C3 G2 C3:h', bpm: 80 }),
      L(19, 3, 'Triplets vs. duplets', `RH plays triplets while LH plays straight eighths — 3 against 2! Practice slowly: "nice cup of tea" rhythm.`, { rh: 'C5:et G4 E4 C5:et G4 E4 D5:et B4 G4 D5:et B4 G4 | C5:w', lh: 'C3:e G3 C3 G3 G2 D3 G2 D3 | C3:w', bpm: 56 }),
      L(19, 4, 'Staccato vs. legato', `LH plays **staccato** (short) quarters while the RH plays a **legato** melody. Two different touches at once!`, { rh: 'E4:q F4 G4 A4 | G4 F4 E4 D4 | C4:w', lh: 'C3:e r:e G2:e r:e C3:e r:e G2:e r:e | C3:e r:e G2:e r:e C3:e r:e G2:e r:e | C3:w', bpm: 76 }),
    ],
    song: { title: 'Two-Voice Invention Study', composer: 'Practice piece (Baroque style)', ex: { rh: 'C5:s D5 E5 F5 D5:e E5 C5:q r:q | G4:s A4 B4 C5 A4:e B4 G4:q r:q | E5:s F5 G5 F5 E5:e D5 C5:h', lh: 'r:h C4:s D4 E4 F4 D4:e E4 | C4:q G3 r:h | C3:q G2 C3:h', bpm: 60 } },
    test: { rh: 'E4:e F4 G4 E4 F4 G4 A4 F4 | G4:w', lh: 'C3:q G2 C3 G2 | C3:w', bpm: 76 },
  },
  {
    n: 20, level: 'Advanced', title: 'Sight-Reading (new pieces every day)',
    lessons: [
      L(20, 1, "Today's piece: C major (RH)", `Sight-reading tips: **before playing**, check the key signature, time signature, and scan for patterns (steps, skips, repeated notes). Then keep a steady beat — **don't stop** to fix mistakes! This melody changes every day.`, sightReading(1, 'C', false)),
      L(20, 2, "Today's piece: G major (RH)", `Key of G — watch for F♯! Look ahead one beat as you play.`, sightReading(2, 'G', false)),
      L(20, 3, "Today's piece: F major (hands together)", `Key of F — watch for B♭. The left hand plays whole-note roots.`, sightReading(3, 'F', true)),
      L(20, 4, "Today's piece: C major (hands together)", `Keep your eyes on the music, not your hands. Trust your hand position!`, sightReading(4, 'C', true)),
    ],
    test: sightReading(5, 'G', true),
  },
  {
    n: 21, level: 'Advanced', title: 'Ornaments',
    lessons: [
      L(21, 1, 'Trills', `A **trill** (tr) rapidly alternates the written note with the note above it. Use fingers 2-3 or 3-4 and keep the wrist relaxed. Here it's written out in 32nd notes.`, { rh: 'E4:q D4:z E4 D4 E4 D4 E4 D4 E4 C4:h | C4:w', bpm: 60 }),
      L(21, 2, 'Grace notes', `A **grace note** is a tiny quick note played just before the main note. Here it's written as a fast 16th leading into the main note.`, { rh: 'C4:q D#4:s E4:e. G4:q r:q | F#4:s G4:e. E4:q C4:h', bpm: 72 }),
      L(21, 3, 'Mordents', `A **mordent** is a quick "main–lower–main" flick. Written out: C–B–C, very fast, then hold the C.`, { rh: 'C5:s B4 C5:e G4:q A4:s G4 A4:e F4:q | E4:s D4 E4:e~ E4:q C4:h', bpm: 66 }),
      L(21, 4, 'Turns', `A **turn** (∽) circles around a note: above–main–below–main. Written out in 16ths.`, { rh: 'E4:q F4:s E4 D4 E4 G4:h | D4:s C4 B3 C4 E4:q C4:h', bpm: 66 }),
    ],
    song: { title: 'Baroque Ornament Study', composer: 'Practice piece', ex: { rh: 'G4:q A4:s G4 F#4 G4 B4:q D5:q | C5:s D5 C5 B4 A4:q G4:h | F#4:z G4 F#4 G4 F#4 G4 F#4 G4 A4:q G4:h', lh: 'G2:h B2:h | C3:h D3:h | D3:h G2:h', key: 1, bpm: 60 } },
    test: { rh: 'C5:s B4 C5:e G4:q E4:q F4:s E4 D4 E4 | C4:w', bpm: 66 },
  },
  {
    n: 22, level: 'Advanced', title: 'Music Theory',
    lessons: [
      L(22, 1, 'Intervals', `An **interval** is the distance between two notes. From C: C–D major 2nd, C–E major 3rd, C–F perfect 4th, C–G perfect 5th, C–A major 6th, C–B major 7th, C–C octave. Play each interval as a pair.`, { rh: 'C4:e D4 C4 E4 C4 F4 C4 G4 | C4 A4 C4 B4 C4:h | [C4 C5]:w', bpm: 72 }),
      L(22, 2, 'Circle of fifths', `The **circle of fifths** goes up a perfect 5th each step: C → G → D → A → E → B → F♯… Each step adds one sharp. Going the other way (down a 5th) adds flats: C → F → B♭ → E♭…`, { lh: 'C3:q G3 D3 A3 | E3 B2 F#3 C#3 | C3 F2 Bb2 Eb3 | Ab2 Db3 C3:h', bpm: 72 }),
      L(22, 3, 'Cadences', `A **cadence** ends a phrase. **Authentic** (V → I) sounds final. **Plagal** (IV → I) is the "Amen" cadence. **Half** cadence ends on V — like a question.`, { rh: `${chord(['B3', 'D4', 'G4'], 'h')} ${chord(C, 'h')} | ${chord(['C4', 'F4', 'A4'], 'h')} ${chord(C, 'h')} | ${chord(C, 'h')} ${chord(['B3', 'D4', 'G4'], 'h')}`, lh: 'G2:h C3:h | F2:h C3:h | C3:h G2:h', bpm: 66 }),
      L(22, 4, 'Modulation', `**Modulation** changes key mid-piece. A common trick: use the V7 of the new key. C major → **D7** (V7 of G) → G major.`, { rh: `${chord(C, 'w')} | ${chord(['C4', 'D4', 'F#4', 'A4'], 'w')} | ${chord(['B3', 'D4', 'G4'], 'w')} | ${chord(['D4', 'G4', 'B4'], 'w')}`, lh: 'C3:w | D3:w | G2:w | G2:w', bpm: 66 }),
    ],
    song: { title: 'Cadence Chorale', composer: 'Practice piece', ex: { rh: `${chord(['E4', 'G4', 'C5'], 'h')} ${chord(['F4', 'A4', 'C5'], 'h')} | ${chord(['D4', 'G4', 'B4'], 'h')} ${chord(['E4', 'G4', 'C5'], 'h')} | ${chord(['E4', 'A4', 'C5'], 'h')} ${chord(['F4', 'A4', 'D5'], 'h')} | ${chord(['D4', 'G4', 'B4'], 'w')}`, lh: 'C3:h F2:h | G2:h C3:h | A2:h D3:h | G2:w', bpm: 66 } },
    test: { rh: `${chord(['C4', 'F4', 'A4'], 'h')} ${chord(C, 'h')} | ${chord(['B3', 'D4', 'G4'], 'h')} ${chord(C, 'h')}`, lh: 'F2:h C3:h | G2:h C3:h', bpm: 66 },
  },
  {
    n: 23, level: 'Advanced', title: 'Playing by Ear & Improvising',
    lessons: [
      L(23, 1, 'Echo game', `**Playing by ear** starts with listening. Press **Demo** to hear the short melody, then close your eyes and try to find it yourself before you look at the music.`, { rh: 'C4:q E4 G4 E4 | D4 F4 E4:h', bpm: 80 }),
      L(23, 2, 'Find the melody', `Listen to the Demo a few times. Hum it. Which note does it start on? Does it go up or down? Then play it back.`, { rh: 'G4:q G4 A4 G4 | C5:h B4:h', bpm: 84 }),
      L(23, 3, 'The pentatonic scale', `The **C pentatonic scale** (C–D–E–G–A) has no "wrong" notes over C, F, G and Am chords. Learn the pattern here, then go to **Free Play** and improvise with it!`, { rh: 'C4:e D4 E4 G4 A4 C5 A4 G4 | E4 D4 C4:q C4:h', lh: 'C3:w | C3:w', bpm: 84 }),
      L(23, 4, 'Improvising over chords', `Your left hand plays I–vi–IV–V. Your right hand plays this sample pentatonic melody — then make up your own in Free Play! Tip: start and end phrases on a chord tone.`, { rh: 'E4:q G4 A4 G4 | E4:h C4:h | A4:q G4 E4 D4 | D4:w', lh: 'C3:w | A2:w | F2:w | G2:w', bpm: 84 }),
    ],
    song: { title: 'Blues Riff in C', composer: 'Practice piece', ex: { rh: 'C4:e Eb4 E4 G4 A4 G4 E4:q | F4:e Ab4 A4 C5 D5 C5 A4:q | C4:e Eb4 E4 G4 A4 G4 E4:q | G4:e F4 E4 D4 C4:h', lh: 'C3:q G3 A3 G3 | F2 C3 D3 C3 | C3 G3 A3 G3 | G2 D3 C3:h', bpm: 90 } },
    test: { rh: 'C4:q E4 G4 A4 | G4:h E4:h | D4:q E4 C4:h', bpm: 84 },
  },
  {
    n: 24, level: 'Advanced', title: 'Advanced Repertoire (Public Domain)',
    lessons: [
      L(24, 1, 'Bach – Prelude in C (BWV 846)', `J.S. Bach (1685–1750). This prelude is one long flowing pattern of broken chords. Hold the bass notes and let the harmony change each measure.`, { rh: 'r:e G4:s C5 E5 G4 C5 E5 r:e G4:s C5 E5 G4 C5 E5 | r:e A4:s D5 F5 A4 D5 F5 r:e A4:s D5 F5 A4 D5 F5 | r:e G4:s D5 F5 G4 D5 F5 r:e G4:s D5 F5 G4 D5 F5 | r:e G4:s C5 E5 G4 C5 E5 r:e G4:s C5 E5 G4 C5 E5', lh: '[C4 E4]:h [C4 E4]:h | [C4 D4]:h [C4 D4]:h | [B3 D4]:h [B3 D4]:h | [C4 E4]:h [C4 E4]:h', bpm: 66 }),
      L(24, 2, 'Beethoven – Für Elise (opening)', `Ludwig van Beethoven (1770–1827). The famous E–D♯ alternation. Keep it light, with the pedal changing on each broken chord.`, { rh: 'r:e r:e E5:s D#5 | E5 D#5 E5 B4 D5 C5 | A4:e r:s C4 E4 A4 | B4:e r:s E4 G#4 B4 | C5:e r:s E4 E5 D#5 | E5 D#5 E5 B4 D5 C5 | A4:e r:s C4 E4 A4 | B4:e r:s E4 C5 B4 | A4:q.', lh: 'r:q. | r:q. | A2:s E3 A3 r:e. | E2:s E3 G#3 r:e. | A2:s E3 A3 r:e. | r:q. | A2:s E3 A3 r:e. | E2:s E3 G#3 r:e. | A2:s E3 A3 r:e.', time: [3, 8], bpm: 66 }),
      L(24, 3, 'Mozart – Sonata K.545 (opening)', `Wolfgang Amadeus Mozart (1756–1791). Classic Alberti bass under a singing melody. Keep the LH even and quiet.`, { rh: 'C5:h E5:q. G5:e | B4:q. C5:s D5 C5:h | A5:h G5:q C6:q | G5:q F5:e G5:e E5:h', lh: 'C4:e G4 E4 G4 C4 G4 E4 G4 | D4 G4 F4 G4 C4 G4 E4 G4 | C4 A4 F4 A4 C4 G4 E4 G4 | B3 G4 D4 G4 C4 G4 E4 G4', bpm: 88 }),
      L(24, 4, 'Satie – Gymnopédie No. 1 (opening)', `Erik Satie (1866–1925). Slow and dreamy in 3/4. LH plays a low bass note then a soft chord; RH floats a simple melody. Change the pedal every measure.`, { rh: 'r:h. | r:h. | r:h. | r:h. | r:q F#5 A5 | G5 F#5 C#5 | B4 C#5 D5 | A4:h.', lh: `G2:q ${chord(['B3', 'D4', 'F#4'], 'h')} | D2:q ${chord(['A3', 'C#4', 'F#4'], 'h')} | G2:q ${chord(['B3', 'D4', 'F#4'], 'h')} | D2:q ${chord(['A3', 'C#4', 'F#4'], 'h')} | G2:q ${chord(['B3', 'D4', 'F#4'], 'h')} | D2:q ${chord(['A3', 'C#4', 'F#4'], 'h')} | G2:q ${chord(['B3', 'D4', 'F#4'], 'h')} | D2:q ${chord(['A3', 'C#4', 'F#4'], 'h')}`, time: [3, 4], key: 2, bpm: 72 }),
      L(24, 5, 'Chopin – Prelude in E minor, Op. 28 No. 4 (opening)', `Frédéric Chopin (1810–1849). A sad, slow melody over pulsing LH chords that slowly slide down by half steps. (Simplified excerpt.)`, { rh: 'B4:h. C5:q | B4:w | B4:h. C5:q | B4:w', lh: `${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} ${chord(['G3', 'B3', 'E4'], 'e')} | ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} ${chord(['G3', 'B3', 'D#4'], 'e')} | ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} ${chord(['G3', 'A3', 'D#4'], 'e')} | ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')} ${chord(['F#3', 'A3', 'D#4'], 'e')}`, key: 1, bpm: 60 }),
      L(24, 6, 'Joplin – The Entertainer (opening)', `Scott Joplin (c.1868–1917), the "King of Ragtime." Syncopated RH over an "oom-pah" LH. Not too fast — Joplin wrote "never play ragtime fast!"`, { rh: 'r:q r:s D4:s D#4:s E4:s | C5:e E4:s C5:e E4:s C5:e~ | C5:q C5:s D5:s D#5:s E5:s | C5:s D5:s E5:e B4:s D5:e C5:s | C5:q r:s D4:s D#4:s E4:s | C5:e E4:s C5:e E4:s C5:e~ | C5:q A4:s G4:s F#4:s A4:s | C5:s E5:e D5:s C5:s A4:s D5:e | D5:h', lh: `r:h | C3:e ${chord(['E3', 'G3', 'C4'], 'e')} G2:e ${chord(['E3', 'G3', 'C4'], 'e')} | C3:e ${chord(['E3', 'G3', 'C4'], 'e')} G2:e ${chord(['E3', 'G3', 'C4'], 'e')} | G2:e ${chord(['F3', 'G3', 'B3'], 'e')} D2:e ${chord(['F3', 'G3', 'B3'], 'e')} | C3:e ${chord(['E3', 'G3', 'C4'], 'e')} G2:e ${chord(['E3', 'G3', 'C4'], 'e')} | C3:e ${chord(['E3', 'G3', 'C4'], 'e')} G2:e ${chord(['E3', 'G3', 'C4'], 'e')} | D3:e ${chord(['F#3', 'A3', 'C4'], 'e')} A2:e ${chord(['F#3', 'A3', 'C4'], 'e')} | G2:e ${chord(['G3', 'B3', 'D4'], 'e')} D3:e ${chord(['G3', 'B3', 'D4'], 'e')} | G2:h`, time: [2, 4], bpm: 70 }),
      L(24, 7, 'Debussy – Clair de Lune (opening, simplified)', `Claude Debussy (1862–1918). Soft, floating, with lots of pedal. This simplified excerpt keeps the gentle falling thirds of the opening in 9/8 time.`, { rh: `r:e ${chord(['F4', 'Ab4'], 'q')}~ ${chord(['F4', 'Ab4'], 'h.')} | r:e ${chord(['Eb4', 'F4'], 'e')} ${chord(['Db4', 'F4'], 'e')} ${chord(['Eb4', 'F4'], 'h.')} | r:e ${chord(['Db4', 'F4'], 'e')} ${chord(['C4', 'F4'], 'e')} ${chord(['Db4', 'F4'], 'q.')} ${chord(['C4', 'F4'], 'e')} ${chord(['Bb3', 'F4'], 'e')} ${chord(['C4', 'F4'], 'e')} | ${chord(['Bb3', 'Db4'], 'h.')} r:q.`, time: [9, 8], key: -5, bpm: 50 }),
    ],
    test: { rh: 'C5:h E5:q. G5:e | B4:q. C5:s D5 C5:h', lh: 'C4:e G4 E4 G4 C4 G4 E4 G4 | D4 G4 F4 G4 C4 G4 E4 G4', bpm: 80 },
  },
  {
    n: 25, level: 'Final Exam', title: 'Final Exam', exam: true,
    lessons: [], // filled below: perform 3 pieces from Unit 24
  },
];

// Final exam: perform three repertoire pieces, each needs 2+ stars.
const u24 = COURSE.find((u) => u.n === 24)!;
COURSE.find((u) => u.n === 25)!.lessons = [u24.lessons[2], u24.lessons[1], u24.lessons[3]].map((l, i) => ({
  ...l, id: `u25-l${i + 1}`, title: `Exam piece ${i + 1}: ${l.title}`,
  text: `Perform this piece at full tempo. You need **2 or more stars** on all three exam pieces to earn the **Advanced Pianist** badge. Warm up with Demo and Wait-for-me first!`,
}));

export const TOTAL_UNITS = COURSE.length;

const cache = new Map<string, Piece>();
export function lessonPiece(id: string, title: string, ex: Exercise): Piece {
  const key = `${id}:${new Date().toDateString()}`;
  if (!cache.has(key)) cache.set(key, makePiece(id, title, ex));
  return cache.get(key)!;
}
export const findLesson = (id: string) => {
  for (const u of COURSE) {
    const l = u.lessons.find((x) => x.id === id);
    if (l) return { unit: u, lesson: l };
  }
  return null;
};

/** Placement test: one check-piece per stage. Passing (2+ stars) moves you past that unit. */
export const PLACEMENT: { unit: number; label: string; ex: Exercise }[] = [
  { unit: 2, label: 'Treble-clef reading', ex: COURSE[1].test! },
  { unit: 4, label: 'Rhythm', ex: COURSE[3].test! },
  { unit: 7, label: 'Both hands together', ex: COURSE[6].test! },
  { unit: 9, label: 'Sharps & flats', ex: COURSE[8].test! },
  { unit: 12, label: 'Chords', ex: COURSE[11].test! },
  { unit: 16, label: 'Two-octave scales', ex: COURSE[15].test! },
];

/** For the rest of the app: lh8 converts a melody down an octave. */
export { lh8 };
export const G_TRIAD = G, AM_TRIAD = Am;
