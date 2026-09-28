/**
 * Tiny music notation used to author lessons and songs, plus MusicXML/MIDI import and MusicXML export.
 *
 * Tokens (space separated, "|" barlines are optional and ignored):
 *   C4:q  F#4:e  Bb3:h.   [C4 E4 G4]:w (chord)   r:q (rest)   C4:h~ (tie into next)
 * Durations: w h q e s z(32nd), add "." for dotted, "t" for triplet (e.g. "et" = triplet eighth).
 * A token without a duration reuses the previous one.
 */
export type Hand = 'R' | 'L';
export interface NoteEv { id: number; beat: number; dur: number; midi: number; hand: Hand }
export interface Piece {
  id: string; title: string; composer?: string; level?: string;
  bpm: number; beats: number; beatUnit: number; key?: number;
  events: NoteEv[];
  xml?: string; // original MusicXML (imports); otherwise generated
  hands?: Hand[]; // which hands have notes
}
export interface Exercise { rh?: string; lh?: string; bpm?: number; time?: [number, number]; key?: number }

const STEP: Record<string, number> = { C: 0, D: 2, E: 4, F: 5, G: 7, A: 9, B: 11 };
const DUR: Record<string, number> = { w: 4, h: 2, q: 1, e: 0.5, s: 0.25, z: 0.125 };

export function nameToMidi(n: string): number {
  const m = n.match(/^([A-Ga-g])(#|b|n|##|bb)?(-?\d)$/);
  if (!m) throw new Error(`Bad note name: ${n}`);
  const acc = m[2] === '#' ? 1 : m[2] === '##' ? 2 : m[2] === 'b' ? -1 : m[2] === 'bb' ? -2 : 0;
  return 12 * (Number(m[3]) + 1) + STEP[m[1].toUpperCase()] + acc;
}
const SHARP_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
const FLAT_NAMES = ['C', 'Db', 'D', 'Eb', 'E', 'F', 'Gb', 'G', 'Ab', 'A', 'Bb', 'B'];
export const midiName = (m: number, flats = false) => `${(flats ? FLAT_NAMES : SHARP_NAMES)[m % 12]}${Math.floor(m / 12) - 1}`;
export const pitchClass = (m: number, flats = false) => (flats ? FLAT_NAMES : SHARP_NAMES)[m % 12];

function parseDur(s: string): number {
  let d = DUR[s[0]];
  if (d === undefined) throw new Error(`Bad duration: ${s}`);
  if (s.includes('.')) d *= 1.5;
  if (s.includes('t')) d *= 2 / 3;
  return d;
}

let nextId = 1;
export function parseVoice(src: string, hand: Hand, startBeat = 0): { events: NoteEv[]; length: number } {
  const events: NoteEv[] = [];
  let beat = startBeat, dur = 1;
  const tied = new Map<number, NoteEv>();
  const tokens = src.replace(/\|/g, ' ').replace(/\[\s*/g, '[').replace(/\s*\]/g, ']').split(/\s+/).filter(Boolean);
  // re-join chord tokens that were split by spaces inside brackets
  const joined: string[] = [];
  for (let i = 0; i < tokens.length; i++) {
    if (tokens[i].startsWith('[') && !tokens[i].includes(']')) {
      let t = tokens[i];
      while (!t.includes(']') && i + 1 < tokens.length) t += ' ' + tokens[++i];
      joined.push(t);
    } else joined.push(tokens[i]);
  }
  for (const tok of joined) {
    const tie = tok.endsWith('~');
    const t = tie ? tok.slice(0, -1) : tok;
    const [pitchPart, durPart] = t.split(':');
    if (durPart) dur = parseDur(durPart);
    if (pitchPart !== 'r') {
      const names = pitchPart.startsWith('[') ? pitchPart.slice(1, pitchPart.indexOf(']')).split(/\s+/) : [pitchPart];
      const newTied = new Map<number, NoteEv>();
      for (const n of names) {
        const midi = nameToMidi(n);
        const prev = tied.get(midi);
        if (prev) { prev.dur += dur; if (tie) newTied.set(midi, prev); continue; }
        const ev: NoteEv = { id: nextId++, beat, dur, midi, hand };
        events.push(ev);
        if (tie) newTied.set(midi, ev);
      }
      tied.clear(); newTied.forEach((v, k) => tied.set(k, v));
    } else tied.clear();
    beat += dur;
  }
  return { events, length: beat - startBeat };
}

export function makePiece(id: string, title: string, ex: Exercise, extra: Partial<Piece> = {}): Piece {
  const r = ex.rh ? parseVoice(ex.rh, 'R') : { events: [], length: 0 };
  const l = ex.lh ? parseVoice(ex.lh, 'L') : { events: [], length: 0 };
  const events = [...r.events, ...l.events].sort((a, b) => a.beat - b.beat || a.midi - b.midi);
  const [beats, beatUnit] = ex.time ?? [4, 4];
  return { id, title, bpm: ex.bpm ?? 80, beats, beatUnit, key: ex.key ?? 0, events, hands: [...(ex.rh ? ['R'] : []), ...(ex.lh ? ['L'] : [])] as Hand[], ...extra };
}

export const measureLen = (p: Pick<Piece, 'beats' | 'beatUnit'>) => (p.beats * 4) / p.beatUnit;
export const pieceLength = (p: Piece) => {
  const end = p.events.reduce((m, e) => Math.max(m, e.beat + e.dur), 0);
  const ml = measureLen(p);
  return Math.max(ml, Math.ceil(end / ml - 1e-6) * ml);
};

// ---------------- MusicXML export ----------------
const DIV = 24; // divisions per quarter
const TYPES: [number, string, number, boolean][] = [
  // [divisions, type, dots, triplet]
  [96, 'whole', 0, false], [72, 'half', 1, false], [48, 'half', 0, false], [36, 'quarter', 1, false], [32, 'half', 0, true],
  [24, 'quarter', 0, false], [18, 'eighth', 1, false], [16, 'quarter', 0, true], [12, 'eighth', 0, false], [9, '16th', 1, false],
  [8, 'eighth', 0, true], [6, '16th', 0, false], [4, '16th', 0, true], [3, '32nd', 0, false],
];
function splitDur(d: number): number[] {
  const out: number[] = [];
  let rest = d;
  while (rest > 0) {
    const t = TYPES.find(([v]) => v <= rest) ?? [rest];
    out.push(t[0]);
    rest -= t[0];
    if (t[0] < 3) break;
  }
  return out;
}
function pitchXml(m: number, key: number) {
  const flats = key < 0;
  const name = pitchClass(m, flats);
  const step = name[0], alter = name.length > 1 ? (name[1] === '#' ? 1 : -1) : 0;
  return `<pitch><step>${step}</step>${alter ? `<alter>${alter}</alter>` : ''}<octave>${Math.floor(m / 12) - 1}</octave></pitch>`;
}

/** Build a monophonic-rhythm voice (chords allowed) per hand from events, quantized to 1/24 quarter. */
function voiceFor(events: NoteEv[], total: number) {
  const q = (b: number) => Math.round(b * DIV);
  const groups = new Map<number, NoteEv[]>();
  events.forEach((e) => { const s = q(e.beat); groups.set(s, [...(groups.get(s) ?? []), e]); });
  const starts = [...groups.keys()].sort((a, b) => a - b);
  const items: { start: number; dur: number; notes: number[] }[] = [];
  let cursor = 0;
  starts.forEach((s, i) => {
    if (s > cursor) items.push({ start: cursor, dur: s - cursor, notes: [] });
    const next = starts[i + 1] ?? q(total);
    const g = groups.get(s)!;
    const longest = Math.max(...g.map((e) => q(e.dur)));
    // small gaps before the next note (common in MIDI files) are absorbed so the rhythm reads cleanly
    const d = Math.max(3, next - s - longest <= DIV / 4 ? next - s : Math.min(longest, next - s));
    items.push({ start: s, dur: d, notes: [...new Set(g.map((e) => e.midi))].sort((a, b) => a - b) });
    cursor = s + d;
  });
  if (cursor < q(total)) items.push({ start: cursor, dur: q(total) - cursor, notes: [] });
  return items;
}

export function toMusicXML(p: Piece): string {
  const total = pieceLength(p);
  const ml = measureLen(p) * DIV;
  const nMeasures = Math.round((total * DIV) / ml);
  const hands: Hand[] = p.hands?.length ? p.hands : ([...new Set(p.events.map((e) => e.hand))] as Hand[]);
  const staves = hands.length === 2 ? ['R', 'L'] as Hand[] : hands.length ? hands : ['R'] as Hand[];
  const voices = staves.map((h) => voiceFor(p.events.filter((e) => e.hand === h), total));
  const measures: string[] = [];
  for (let m = 0; m < nMeasures; m++) {
    const mStart = m * ml, mEnd = mStart + ml;
    let xml = `<measure number="${m + 1}">`;
    if (m === 0) {
      xml += `<attributes><divisions>${DIV}</divisions><key><fifths>${p.key ?? 0}</fifths></key><time><beats>${p.beats}</beats><beat-type>${p.beatUnit}</beat-type></time>`;
      xml += staves.length === 2 ? '<staves>2</staves><clef number="1"><sign>G</sign><line>2</line></clef><clef number="2"><sign>F</sign><line>4</line></clef>'
        : staves[0] === 'L' ? '<clef><sign>F</sign><line>4</line></clef>' : '<clef><sign>G</sign><line>2</line></clef>';
      xml += `</attributes><direction placement="above"><direction-type><metronome><beat-unit>quarter</beat-unit><per-minute>${p.bpm}</per-minute></metronome></direction-type><sound tempo="${p.bpm}"/></direction>`;
    }
    staves.forEach((_, si) => {
      if (si > 0) xml += `<backup><duration>${ml}</duration></backup>`;
      const staffTag = staves.length === 2 ? `<staff>${si + 1}</staff>` : '';
      for (const it of voices[si]) {
        const s = Math.max(it.start, mStart), e = Math.min(it.start + it.dur, mEnd);
        if (e <= s) continue;
        const pieces = splitDur(e - s);
        pieces.forEach((d, pi) => {
          const t = TYPES.find(([v]) => v === d);
          const typeXml = t ? `<type>${t[1]}</type>${'<dot/>'.repeat(t[2])}${t[3] ? '<time-modification><actual-notes>3</actual-notes><normal-notes>2</normal-notes></time-modification>' : ''}` : '';
          if (!it.notes.length) {
            const whole = d === ml && pieces.length === 1;
            xml += `<note><rest${whole ? ' measure="yes"' : ''}/><duration>${d}</duration><voice>${si + 1}</voice>${whole ? '' : typeXml}${staffTag}</note>`;
            return;
          }
          const tieStart = pi < pieces.length - 1 || it.start + it.dur > mEnd;
          const tieStop = pi > 0 || it.start < mStart;
          it.notes.forEach((midi, ni) => {
            xml += `<note>${ni > 0 ? '<chord/>' : ''}${pitchXml(midi, p.key ?? 0)}<duration>${d}</duration>${tieStop ? '<tie type="stop"/>' : ''}${tieStart ? '<tie type="start"/>' : ''}<voice>${si + 1}</voice>${typeXml}${staffTag}`;
            if (tieStart || tieStop) xml += `<notations>${tieStop ? '<tied type="stop"/>' : ''}${tieStart ? '<tied type="start"/>' : ''}</notations>`;
            xml += '</note>';
          });
        });
      }
    });
    measures.push(xml + '</measure>');
  }
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 3.1 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="3.1"><part-list><score-part id="P1"><part-name></part-name></score-part></part-list><part id="P1">${measures.join('')}</part></score-partwise>`;
}

// ---------------- MusicXML import ----------------
export function parseMusicXML(text: string, fileName = 'Imported song'): Piece {
  const doc = new DOMParser().parseFromString(text, 'application/xml');
  if (doc.querySelector('parsererror')) throw new Error('This file is not valid MusicXML.');
  const title = doc.querySelector('work-title')?.textContent || doc.querySelector('movement-title')?.textContent || fileName.replace(/\.(musicxml|xml|mxl)$/i, '');
  const composer = doc.querySelector('creator[type="composer"]')?.textContent ?? undefined;
  const parts = [...doc.querySelectorAll('part')];
  if (!parts.length) throw new Error('No parts found in this MusicXML file.');
  const events: NoteEv[] = [];
  let bpm = 90, beats = 4, beatUnit = 4, key = 0;
  const partStaves = parts.map((p) => Number(p.querySelector('staves')?.textContent ?? 1));
  parts.slice(0, 2).forEach((part, pi) => {
    let divisions = 1, pos = 0, lastStart = 0;
    const tieOpen = new Map<number, NoteEv>();
    for (const measure of part.querySelectorAll(':scope > measure')) {
      for (const el of measure.children) {
        if (el.tagName === 'attributes') {
          const d = el.querySelector('divisions'); if (d) divisions = Number(d.textContent);
          const b = el.querySelector('time > beats'), bt = el.querySelector('time > beat-type');
          if (b && bt && pi === 0 && events.length === 0) { beats = Number(b.textContent); beatUnit = Number(bt.textContent); }
          const f = el.querySelector('key > fifths'); if (f && pi === 0 && events.length === 0) key = Number(f.textContent);
        } else if (el.tagName === 'direction' || el.tagName === 'sound') {
          const s = el.tagName === 'sound' ? el : el.querySelector('sound');
          const t = s?.getAttribute('tempo'); if (t && events.length === 0) bpm = Math.round(Number(t));
        } else if (el.tagName === 'backup') pos -= Number(el.querySelector('duration')?.textContent ?? 0) / divisions;
        else if (el.tagName === 'forward') pos += Number(el.querySelector('duration')?.textContent ?? 0) / divisions;
        else if (el.tagName === 'note') {
          if (el.querySelector('grace') || el.querySelector('cue')) continue;
          const dur = Number(el.querySelector(':scope > duration')?.textContent ?? 0) / divisions;
          const isChord = !!el.querySelector(':scope > chord');
          const start = isChord ? lastStart : pos;
          if (!isChord) { lastStart = pos; pos += dur; }
          const pitch = el.querySelector(':scope > pitch');
          if (!pitch || el.querySelector(':scope > rest')) continue;
          const midi = 12 * (Number(pitch.querySelector('octave')!.textContent) + 1) + STEP[pitch.querySelector('step')!.textContent!.trim()] + Number(pitch.querySelector('alter')?.textContent ?? 0);
          const staff = Number(el.querySelector(':scope > staff')?.textContent ?? 1);
          const hand: Hand = partStaves[pi] >= 2 ? (staff >= 2 ? 'L' : 'R') : parts.length >= 2 ? (pi === 0 ? 'R' : 'L') : midi >= 60 ? 'R' : 'L';
          const ties = [...el.querySelectorAll(':scope > tie')].map((t) => t.getAttribute('type'));
          const open = tieOpen.get(midi);
          if (ties.includes('stop') && open) {
            open.dur += dur;
            if (!ties.includes('start')) tieOpen.delete(midi);
            continue;
          }
          const ev: NoteEv = { id: nextId++, beat: start, dur, midi, hand };
          events.push(ev);
          if (ties.includes('start')) tieOpen.set(midi, ev);
        }
      }
    }
  });
  if (!events.length) throw new Error('No notes found in this file.');
  events.sort((a, b) => a.beat - b.beat || a.midi - b.midi);
  return { id: `import-${Date.now()}`, title, composer, bpm, beats, beatUnit, key, events, xml: text, hands: [...new Set(events.map((e) => e.hand))] as Hand[], level: 'Imported' };
}

export async function parseMidiFile(buf: ArrayBuffer, fileName = 'Imported MIDI'): Promise<Piece> {
  const { Midi } = await import('@tonejs/midi');
  const midi = new Midi(buf);
  const ppq = midi.header.ppq;
  const tracks = midi.tracks.filter((t) => t.notes.length && t.instrument.percussion !== true);
  if (!tracks.length) throw new Error('No notes found in this MIDI file.');
  const events: NoteEv[] = [];
  tracks.forEach((t, ti) => t.notes.forEach((n) => {
    const hand: Hand = tracks.length >= 2 ? (ti === 0 ? 'R' : 'L') : n.midi >= 60 ? 'R' : 'L';
    events.push({ id: nextId++, beat: Math.round((n.ticks / ppq) * 4) / 4, dur: Math.max(0.25, Math.round((n.durationTicks / ppq) * 4) / 4), midi: n.midi, hand });
  }));
  events.sort((a, b) => a.beat - b.beat || a.midi - b.midi);
  const ts = midi.header.timeSignatures[0]?.timeSignature ?? [4, 4];
  return {
    id: `import-${Date.now()}`, title: midi.name || fileName.replace(/\.midi?$/i, ''), bpm: Math.round(midi.header.tempos[0]?.bpm ?? 100),
    beats: ts[0], beatUnit: ts[1], key: 0, events, hands: [...new Set(events.map((e) => e.hand))] as Hand[], level: 'Imported',
  };
}
