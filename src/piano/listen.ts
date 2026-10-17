// "Listen & make a song": hears music (for example a YouTube video) through the microphone or straight from the
// computer's own sound, finds the melody notes, and turns them into a playable song. Works best when one clear melody stands out.
import type { Piece, NoteEv } from './notation';
import type { ImportedSong } from '../lib/store';
import { estimateLevel } from './importer';

export interface Frame { t: number; midi: number | null } // t in seconds
export interface HeardNote { start: number; end: number; midi: number }

/** Pitch of one short slice of sound (Hz), or null when it's too quiet or not a clear note. YIN-style. */
export function detectPitch(buf: Float32Array, sampleRate: number): number | null {
  const n = buf.length;
  let rms = 0;
  for (let i = 0; i < n; i++) rms += buf[i] * buf[i];
  if (Math.sqrt(rms / n) < 0.01) return null;
  const minLag = Math.floor(sampleRate / 1400), maxLag = Math.min(Math.floor(sampleRate / 65), Math.floor(n / 2));
  const d = new Float32Array(maxLag + 1);
  for (let tau = 1; tau <= maxLag; tau++) {
    let s = 0;
    for (let i = 0; i < n - maxLag; i++) { const x = buf[i] - buf[i + tau]; s += x * x; }
    d[tau] = s;
  }
  let sum = 0, tau = -1;
  const cm = new Float32Array(maxLag + 1);
  cm[0] = 1;
  for (let t = 1; t <= maxLag; t++) { sum += d[t]; cm[t] = sum ? (d[t] * t) / sum : 1; }
  for (let t = minLag; t <= maxLag; t++) {
    if (cm[t] < 0.15) { while (t + 1 <= maxLag && cm[t + 1] < cm[t]) t++; tau = t; break; }
  }
  if (tau < 0) return null;
  // parabolic interpolation for a finer pitch
  const a = cm[tau - 1] ?? cm[tau], b = cm[tau], c = cm[tau + 1] ?? cm[tau];
  const shift = (a - c) / (2 * (a - 2 * b + c) || 1);
  return sampleRate / (tau + (Number.isFinite(shift) ? Math.max(-1, Math.min(1, shift)) : 0));
}

export const hzToMidi = (hz: number) => Math.round(69 + 12 * Math.log2(hz / 440));

/** Joins steady stretches of the same pitch into notes; smooths out one-frame glitches and drops blips. */
export function framesToNotes(frames: Frame[], minDur = 0.09): HeardNote[] {
  // median of 5 removes single-frame octave jumps and dropouts
  const m = frames.map((f, i) => {
    const w = frames.slice(Math.max(0, i - 2), i + 3).map((x) => x.midi);
    if (w.filter((x) => x === null).length > w.length / 2) return null;
    const v = w.filter((x): x is number => x !== null).sort((x, y) => x - y);
    return v[Math.floor(v.length / 2)];
  });
  const notes: HeardNote[] = [];
  let cur: HeardNote | null = null;
  frames.forEach((f, i) => {
    const midi = m[i];
    if (cur && midi === cur.midi) { cur.end = f.t; return; }
    if (cur && cur.end - cur.start >= minDur) notes.push(cur);
    cur = midi === null ? null : { start: f.t, end: f.t, midi };
  });
  const last = cur as HeardNote | null;
  if (last && last.end - last.start >= minDur) notes.push(last);
  return notes.filter((n) => n.midi >= 36 && n.midi <= 96);
}

/** Notes (in seconds) → a playable right-hand piece, timed to the beat at `bpm` (rounded to sixteenths). */
export function notesToPiece(notes: HeardNote[], title: string, bpm = 90): Piece {
  const t0 = notes[0]?.start ?? 0;
  const q = (sec: number) => Math.round(((sec - t0) * bpm) / 60 * 4) / 4;
  const events: NoteEv[] = [];
  notes.forEach((n, i) => {
    const beat = q(n.start);
    let dur = Math.max(0.25, q(n.end + 0.03) - beat);
    const next = notes[i + 1];
    if (next) dur = Math.max(0.25, Math.min(dur, q(next.start) - beat));
    if (events.length && events[events.length - 1].beat === beat) return; // two notes rounded onto the same spot: keep the first
    events.push({ id: events.length, beat, dur, midi: n.midi, hand: 'R' });
  });
  return { id: `yt-${Date.now()}`, title, composer: 'Learned by ear', bpm, beats: 4, beatUnit: 4, key: 0, events, hands: ['R'] };
}

export function pieceToSong(p: Piece): ImportedSong {
  return { id: p.id, title: p.title, composer: p.composer ?? 'Learned by ear', level: estimateLevel(p), added: new Date().toISOString().slice(0, 10), piece: p };
}

/** 'mic' hears the room (the song has to come out of speakers). 'computer' hears the computer's own sound through the
 *  browser's tab / screen sharing, so it works with headphones on (Chrome and Edge on computers). */
export type ListenSource = 'mic' | 'computer';

/** The computer's own sound can only be shared where the browser has screen sharing (computers, not phones). */
export const canHearComputer = () => typeof navigator !== 'undefined' && !!navigator.mediaDevices?.getDisplayMedia;

/** The sharing box was used but no sound came with it (tab audio left off, or a browser that can't share sound). */
export class NoSoundShared extends Error {}

// While listening, the app's own background music stays quiet so it isn't heard as part of the song.
let listeningNow = false;
const listenSubs = new Set<() => void>();
export const isListening = () => listeningNow;
export function onListening(f: () => void) { listenSubs.add(f); return () => { listenSubs.delete(f); }; }
function setListening(v: boolean) { listeningNow = v; listenSubs.forEach((f) => f()); }

const RAW = { echoCancellation: false, noiseSuppression: false, autoGainControl: false };

async function computerSound(): Promise<MediaStream> {
  // Chrome only shares sound along with a picture, so ask for both and drop the picture straight away.
  // The tab list comes first and includes this tab (the mini player plays here); the person still hears it in their headphones.
  const opts = { video: { displaySurface: 'browser' }, audio: { ...RAW, suppressLocalAudioPlayback: false }, selfBrowserSurface: 'include', systemAudio: 'include' };
  const stream = await navigator.mediaDevices.getDisplayMedia(opts as DisplayMediaStreamOptions);
  stream.getVideoTracks().forEach((t) => t.stop());
  if (!stream.getAudioTracks().length) throw new NoSoundShared();
  return stream;
}

/** Starts listening. `onPitch` gets the note heard right now (for a live display). `onEnded` is called if listening
 *  ends without `stop()`: the person pressed the browser's "Stop sharing", the mic went away, or 3 minutes passed. */
export async function startListening(source: ListenSource, onPitch: (midi: number | null) => void, onEnded: () => void): Promise<{ stop: () => Frame[] }> {
  // The background jazz pauses as soon as listening starts but takes 0.8 s to fade out, so what's heard before it's gone is skipped.
  setListening(true);
  const quietAt = performance.now() + 1200;
  let stream: MediaStream;
  try {
    stream = source === 'computer' ? await computerSound() : await navigator.mediaDevices.getUserMedia({ audio: RAW });
  } catch (e) { setListening(false); throw e; }
  const ctx = new AudioContext();
  void ctx.resume();
  const src = ctx.createMediaStreamSource(stream);
  const an = ctx.createAnalyser();
  an.fftSize = 2048;
  src.connect(an);
  const buf = new Float32Array(an.fftSize);
  const frames: Frame[] = [];
  const t0 = performance.now();
  let done = false;
  const end = () => { if (!done) onEnded(); };
  const id = setInterval(() => {
    if (performance.now() < quietAt) return;
    an.getFloatTimeDomainData(buf);
    const hz = detectPitch(buf, ctx.sampleRate);
    const midi = hz ? hzToMidi(hz) : null;
    frames.push({ t: (performance.now() - t0) / 1000, midi });
    onPitch(midi);
    if (frames.length === 6000) end(); // about 3 minutes
  }, 30);
  const stop = () => {
    if (done) return frames;
    done = true;
    clearInterval(id);
    stream.getTracks().forEach((t) => t.stop());
    void ctx.close();
    setListening(false);
    return frames;
  };
  stream.getAudioTracks().forEach((t) => t.addEventListener('ended', end));
  return { stop };
}

/** The video id from any YouTube link (watch, youtu.be, shorts, embed, music.youtube.com), or null. */
export function youTubeId(link: string): string | null {
  const s = link.trim();
  if (/^[\w-]{11}$/.test(s)) return s;
  const m = s.match(/(?:youtube(?:-nocookie)?\.com\/(?:watch\?(?:.*&)?v=|shorts\/|embed\/|live\/|v\/)|youtu\.be\/)([\w-]{11})/i);
  return m ? m[1] : null;
}
