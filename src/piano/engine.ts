import * as Tone from 'tone';
import { playNote, click, releaseAll } from './audio';

/** Audio-context time without Tone's scheduling look-ahead: matches what the student actually hears. */
const clock = () => Tone.immediate();
import { onNote } from './input';
import { Hand, NoteEv, Piece, measureLen, pieceLength, midiName } from './notation';

export type PlayMode = 'demo' | 'wait' | 'perform';
export interface PlayResult {
  mode: PlayMode; stars: number; accuracy: number; onTime: number; wrong: number; total: number; hits: number;
  missed: string[]; timing: string[]; summary: string;
}
export interface EngineOpts {
  mode: PlayMode; tempo: number; hands: 'both' | Hand; metronome: boolean;
  loop: null | { from: number; to: number }; // measures, 1-indexed inclusive
  countIn: boolean;
}

/**
 * Drives playback + listening. All timing uses the audio clock (Tone.now()).
 * beat = quarter-note beats from the start of the piece.
 */
export class PlayEngine {
  piece: Piece;
  opts: EngineOpts;
  beat = 0;
  playing = false;
  private beat0 = 0; private t0 = 0;
  private startBeat = 0; private endBeat = 0;
  private raf = 0;
  private schedIdx = 0; private nextClick = 0;
  private expected: NoteEv[] = [];
  private groups: { beat: number; notes: NoteEv[] }[] = [];
  private gi = 0; private groupHit = new Set<number>(); private groupErr = false;
  private groupsClean = 0;
  hits = new Map<number, number>(); // event id -> offset ms
  wrongNotes: { beat: number; midi: number }[] = [];
  private unsub: (() => void) | null = null;
  onFrame: (beat: number) => void = () => {};
  onFinish: (r: PlayResult) => void = () => {};
  onMark: (midi: number, good: boolean) => void = () => {};
  /** Called once per expected note: 'hit' when played (in time), 'miss' when its moment passed without it. */
  onNoteResult: (e: NoteEv, result: 'hit' | 'miss') => void = () => {};
  private missIdx = 0;

  constructor(piece: Piece, opts: EngineOpts) { this.piece = piece; this.opts = opts; }

  get bps() { return (this.piece.bpm * this.opts.tempo) / 60; }
  get mLen() { return measureLen(this.piece); }
  private isActive(e: NoteEv) { return this.opts.hands === 'both' || e.hand === this.opts.hands; }
  private beatAt(t: number) { return this.beat0 + (t - this.t0) * this.bps; }
  private timeOf(b: number) { return this.t0 + (b - this.beat0) / this.bps; }

  start(fromBeat?: number) {
    this.stop(false);
    const ml = this.mLen;
    const len = pieceLength(this.piece);
    const loop = this.opts.loop;
    this.startBeat = fromBeat ?? (loop ? (loop.from - 1) * ml : 0);
    this.endBeat = loop ? Math.min(len, loop.to * ml) : len;
    this.expected = this.piece.events.filter((e) => this.isActive(e) && e.beat >= this.startBeat - 1e-6 && e.beat < this.endBeat - 1e-6);
    const gm = new Map<string, NoteEv[]>();
    this.expected.forEach((e) => { const k = e.beat.toFixed(3); gm.set(k, [...(gm.get(k) ?? []), e]); });
    this.groups = [...gm.entries()].map(([b, notes]) => ({ beat: Number(b), notes })).sort((a, b) => a.beat - b.beat);
    this.gi = 0; this.groupHit.clear(); this.groupErr = false; this.groupsClean = 0;
    this.hits.clear(); this.wrongNotes = []; this.missIdx = 0;
    const lead = this.opts.countIn && this.opts.mode !== 'wait' ? ml : this.opts.mode === 'wait' ? 0 : 0.25;
    this.beat0 = this.startBeat - lead; this.beat = this.beat0;
    this.t0 = clock() + 0.15;
    this.schedIdx = this.piece.events.findIndex((e) => e.beat >= this.startBeat - 1e-6);
    if (this.schedIdx < 0) this.schedIdx = this.piece.events.length;
    this.nextClick = Math.ceil(this.beat0 / this.clickStep() - 1e-6) * this.clickStep();
    this.playing = true;
    this.unsub = onNote((ev) => { if (ev.type === 'on') this.handleNote(ev.midi); });
    const loopFn = () => {
      try { this.tick(); } catch (e) { console.error(e); }
      if (this.playing) this.raf = requestAnimationFrame(loopFn);
    };
    this.raf = requestAnimationFrame(loopFn);
  }

  stop(report = false) {
    if (!this.playing) return;
    this.playing = false;
    cancelAnimationFrame(this.raf);
    this.unsub?.(); this.unsub = null;
    releaseAll();
    if (report) this.onFinish(this.result());
  }

  private clickStep() { return this.piece.beatUnit === 8 ? 0.5 : 4 / this.piece.beatUnit; }

  private tick() {
    const now = clock();
    let b = this.beatAt(now);
    const mode = this.opts.mode;
    let clamp = Infinity;
    if (mode === 'wait' && this.gi < this.groups.length) {
      clamp = this.groups[this.gi].beat;
      if (b > clamp) { b = clamp; this.beat0 = clamp; this.t0 = now; }
    }
    this.beat = b;
    const ahead = Math.min(this.beatAt(now + 0.15), clamp - 1e-4);
    const soonest = now + 0.01;

    // metronome (always during count-in)
    const cs = this.clickStep();
    while (this.nextClick <= ahead) {
      if ((this.opts.metronome || this.nextClick < this.startBeat - 1e-6) && this.nextClick < this.endBeat - 1e-6) {
        const inMeasure = (((this.nextClick % this.mLen) + this.mLen) % this.mLen);
        const accent = inMeasure < 1e-6 || (this.piece.beatUnit === 8 && this.piece.beats % 3 === 0 && Math.abs(inMeasure % 1.5) < 1e-6);
        click(accent, Math.max(soonest, this.timeOf(this.nextClick)));
      }
      this.nextClick += cs;
    }
    // auto-play: everything in demo mode, the other hand in wait/perform
    const evs = this.piece.events;
    while (this.schedIdx < evs.length && evs[this.schedIdx].beat <= ahead) {
      const e = evs[this.schedIdx];
      if (e.beat < this.endBeat - 1e-6 && (mode === 'demo' || !this.isActive(e))) {
        playNote(e.midi, (e.dur / this.bps) * 0.95, Math.max(soonest, this.timeOf(e.beat)), e.hand === 'L' ? 0.55 : 0.7);
      }
      this.schedIdx++;
    }
    if (mode === 'perform') {
      const win = Math.max(0.25, Math.min(0.5, 0.4 * this.bps));
      while (this.missIdx < this.expected.length && this.expected[this.missIdx].beat + win < b) {
        const e = this.expected[this.missIdx++];
        if (!this.hits.has(e.id)) this.onNoteResult(e, 'miss');
      }
    }
    this.onFrame(b);

    // end / loop
    const lastEnd = this.endBeat;
    if (mode === 'wait' ? this.gi >= this.groups.length && b >= Math.max(...this.groups.map((g) => g.beat), this.startBeat) + 1 : b >= lastEnd + 0.1) {
      if (this.opts.loop) { this.start(); return; }
      this.stop(true);
    }
  }

  private handleNote(midi: number) {
    if (!this.playing) return;
    const mode = this.opts.mode;
    if (mode === 'demo') return;
    if (mode === 'wait') {
      const g = this.groups[this.gi];
      if (!g) return;
      if (g.notes.some((n) => n.midi === midi)) {
        this.groupHit.add(midi);
        this.onMark(midi, true);
        if (g.notes.every((n) => this.groupHit.has(n.midi))) {
          g.notes.forEach((n) => { this.hits.set(n.id, 0); this.onNoteResult(n, 'hit'); });
          if (!this.groupErr) this.groupsClean++;
          this.gi++; this.groupHit.clear(); this.groupErr = false;
          // if the student played it early, the music catches up to them
          this.beat0 = Math.max(this.beat, g.beat); this.t0 = clock();
        }
      } else {
        this.groupErr = true;
        this.onMark(midi, false);
        this.wrongNotes.push({ beat: g.beat, midi });
      }
      return;
    }
    // perform: match to nearest unplayed expected note of the same pitch
    const b = this.beatAt(clock());
    const win = Math.max(0.25, Math.min(0.5, 0.4 * this.bps));
    let best: NoteEv | null = null, bestD = Infinity;
    for (const e of this.expected) {
      if (e.midi !== midi || this.hits.has(e.id)) continue;
      const d = Math.abs(e.beat - b);
      if (d <= win && d < bestD) { best = e; bestD = d; }
    }
    if (best) { this.hits.set(best.id, ((b - best.beat) / this.bps) * 1000); this.onMark(midi, true); this.onNoteResult(best, 'hit'); }
    else { this.wrongNotes.push({ beat: b, midi }); this.onMark(midi, false); }
  }

  /** Which keys to light up right now. */
  guide(): Map<number, Hand> {
    const m = new Map<number, Hand>();
    if (!this.playing) return m;
    if (this.opts.mode === 'wait') {
      const g = this.groups[this.gi];
      g?.notes.forEach((n) => { if (!this.groupHit.has(n.midi)) m.set(n.midi, n.hand); });
      return m;
    }
    const b = this.beat;
    for (const e of this.piece.events) {
      if (e.beat > b + 0.12) break;
      if (e.beat + e.dur * 0.9 > b && e.beat <= b + 0.12 && (this.opts.mode === 'demo' || this.isActive(e))) m.set(e.midi, e.hand);
    }
    return m;
  }

  result(): PlayResult {
    const mode = this.opts.mode;
    const total = this.expected.length;
    const hits = [...this.hits.values()];
    const ml = this.mLen;
    const meas = (b: number) => Math.floor(b / ml + 1e-6) + 1;
    const missed = this.expected.filter((e) => !this.hits.has(e.id)).map((e) => `Measure ${meas(e.beat)}: ${midiName(e.midi)} (${e.hand === 'R' ? 'right' : 'left'} hand) was missed`);
    const timing: string[] = [];
    this.expected.forEach((e) => {
      const off = this.hits.get(e.id);
      if (off !== undefined && Math.abs(off) > 140) timing.push(`Measure ${meas(e.beat)}: ${midiName(e.midi)} was ${off > 0 ? 'late' : 'early'} by ${Math.round(Math.abs(off))} ms`);
    });
    this.wrongNotes.slice(0, 12).forEach((w) => timing.push(`Measure ${meas(w.beat)}: played ${midiName(w.midi)} (wrong note)`));
    const accuracy = total ? Math.round((hits.length / total) * 100) : 100;
    let score: number, onTime: number;
    if (mode === 'wait') {
      onTime = 100;
      score = this.groups.length ? this.groupsClean / this.groups.length : 1;
    } else {
      onTime = hits.length ? Math.round((hits.filter((o) => Math.abs(o) <= 140).length / hits.length) * 100) : 0;
      const noteScore = total ? hits.length / (total + this.wrongNotes.length * 0.5) : 1;
      score = noteScore * (0.7 + 0.3 * (onTime / 100));
    }
    const stars = score >= 0.9 ? 3 : score >= 0.75 ? 2 : score >= 0.5 ? 1 : 0;
    const summary = stars === 3 ? 'Excellent! Clean and in time.' : stars === 2 ? 'Good job! A few spots to polish.' : stars === 1 ? 'Getting there — slow it down and try again.' : 'Keep practicing: use wait-for-me mode and a slower tempo.';
    return { mode, stars, accuracy, onTime, wrong: this.wrongNotes.length, total, hits: hits.length, missed, timing, summary };
  }
}
