import * as Tone from 'tone';

/**
 * Generative smooth-jazz background music for the menus.
 * Electric-piano chords, walking bass, soft brushes and a mellow sax-like melody that improvises
 * over a ii–V–I progression (different every time through). All synthesized, no audio files needed.
 */
const PROG: { name: string; chord: string[]; bass: string[]; scale: string[] }[] = [
  { name: 'Dm9', chord: ['F3', 'A3', 'C4', 'E4'], bass: ['D2', 'A2', 'F2', 'A2'], scale: ['D4', 'E4', 'F4', 'A4', 'C5', 'D5'] },
  { name: 'G13', chord: ['F3', 'B3', 'E4', 'A4'], bass: ['G2', 'D3', 'B2', 'D3'], scale: ['D4', 'E4', 'G4', 'A4', 'B4', 'D5'] },
  { name: 'Cmaj9', chord: ['E3', 'G3', 'B3', 'D4'], bass: ['C2', 'G2', 'E2', 'G2'], scale: ['C4', 'D4', 'E4', 'G4', 'A4', 'C5'] },
  { name: 'Am9', chord: ['C4', 'E4', 'G4', 'B4'], bass: ['A1', 'E2', 'C2', 'E2'], scale: ['A4', 'B4', 'C5', 'E5', 'G4', 'D5'] },
  { name: 'Fmaj9', chord: ['E3', 'A3', 'C4', 'G4'], bass: ['F2', 'C3', 'A2', 'C3'], scale: ['F4', 'G4', 'A4', 'C5', 'E5', 'D5'] },
  { name: 'Em7', chord: ['D3', 'G3', 'B3', 'E4'], bass: ['E2', 'B2', 'G2', 'B2'], scale: ['E4', 'G4', 'A4', 'B4', 'D5', 'E5'] },
  { name: 'Dm9', chord: ['F3', 'A3', 'C4', 'E4'], bass: ['D2', 'A2', 'F2', 'A2'], scale: ['D4', 'F4', 'A4', 'C5', 'E5', 'D5'] },
  { name: 'G7sus', chord: ['F3', 'A3', 'C4', 'D4'], bass: ['G2', 'D3', 'G2', 'F2'], scale: ['D4', 'F4', 'G4', 'A4', 'C5', 'D5'] },
];

let built = false;
let running = false;
let out: Tone.Volume;
let keys: Tone.PolySynth<Tone.FMSynth>, bass: Tone.MonoSynth, sax: Tone.MonoSynth, hat: Tone.NoiseSynth, kick: Tone.MembraneSynth;
let loopId: number | null = null;
let volume = 0.5;
const listeners = new Set<(on: boolean) => void>();
export const onJazz = (fn: (on: boolean) => void) => { listeners.add(fn); return () => { listeners.delete(fn); }; };
export const isJazzPlaying = () => running;

function build() {
  if (built) return;
  built = true;
  out = new Tone.Volume(-60).toDestination();
  const reverb = new Tone.Reverb({ decay: 3.5, wet: 0.35 }).connect(out);
  const chorus = new Tone.Chorus(2.5, 2.5, 0.4).connect(reverb).start();
  const trem = new Tone.Tremolo(4, 0.25).connect(chorus).start();
  keys = new Tone.PolySynth(Tone.FMSynth, {
    harmonicity: 3, modulationIndex: 2.5, oscillator: { type: 'sine' }, modulation: { type: 'sine' },
    envelope: { attack: 0.005, decay: 1.8, sustain: 0.15, release: 1.4 }, modulationEnvelope: { attack: 0.002, decay: 0.4, sustain: 0.1, release: 0.5 },
  }).connect(trem);
  keys.volume.value = -14;
  bass = new Tone.MonoSynth({ oscillator: { type: 'triangle' }, filter: { Q: 1, type: 'lowpass' }, envelope: { attack: 0.01, decay: 0.4, sustain: 0.5, release: 0.3 }, filterEnvelope: { baseFrequency: 180, octaves: 1.5, attack: 0.01, decay: 0.2, sustain: 0.3 } }).connect(reverb);
  bass.volume.value = -9;
  const saxFilter = new Tone.Filter(1800, 'lowpass').connect(reverb);
  const vib = new Tone.Vibrato(5, 0.08).connect(saxFilter);
  sax = new Tone.MonoSynth({ oscillator: { type: 'fatsawtooth', count: 2, spread: 12 } as unknown as Tone.OmniOscillatorOptions, filter: { Q: 2, type: 'lowpass' }, envelope: { attack: 0.06, decay: 0.3, sustain: 0.7, release: 0.5 }, filterEnvelope: { baseFrequency: 500, octaves: 2.2, attack: 0.08, decay: 0.3, sustain: 0.5 }, portamento: 0.03 }).connect(vib);
  sax.volume.value = -20;
  hat = new Tone.NoiseSynth({ noise: { type: 'pink' }, envelope: { attack: 0.005, decay: 0.12, sustain: 0 } }).connect(new Tone.Filter(7000, 'highpass').connect(reverb));
  hat.volume.value = -30;
  kick = new Tone.MembraneSynth({ pitchDecay: 0.03, octaves: 3, envelope: { attack: 0.001, decay: 0.3, sustain: 0 } }).connect(out);
  kick.volume.value = -20;
}

let bar = 0;
function scheduleBar(time: number) {
  const p = PROG[bar % PROG.length];
  const beat = Tone.Time('4n').toSeconds();
  // Rhodes: chord on 1, a soft "and of 2" push sometimes
  keys.triggerAttackRelease(p.chord, '2n', time + Math.random() * 0.02, 0.45);
  if (Math.random() < 0.5) keys.triggerAttackRelease(p.chord.slice(1), '8n', time + beat * 2.5, 0.3);
  // walking bass
  p.bass.forEach((n, i) => bass.triggerAttackRelease(n, '4n', time + i * beat, 0.7));
  // brushes: swung ride pattern, soft kick on 1 and 3
  [0, 1, 1.66, 2, 3, 3.66].forEach((b) => hat.triggerAttackRelease('16n', time + b * beat, b % 1 ? 0.25 : 0.4));
  [0, 2].forEach((b) => kick.triggerAttackRelease('C1', '8n', time + b * beat, 0.5));
  // sax: plays in phrases of two bars, rests for two
  if (Math.floor(bar / 2) % 2 === 0) {
    let t = Math.random() < 0.5 ? 0.5 : 1;
    while (t < 4) {
      const dur = [0.5, 0.5, 1, 1.5][Math.floor(Math.random() * 4)];
      const swing = t % 1 ? 0.17 : 0;
      if (Math.random() < 0.8) sax.triggerAttackRelease(p.scale[Math.floor(Math.random() * p.scale.length)], dur * beat * 0.9, time + (t + swing) * beat, 0.5 + Math.random() * 0.2);
      t += dur;
    }
  }
  bar++;
}

export async function startJazz() {
  if (running) return;
  await Tone.start();
  build();
  running = true;
  const tr = Tone.getTransport();
  tr.bpm.value = 84;
  if (loopId === null) loopId = tr.scheduleRepeat((time) => scheduleBar(time), '1m');
  if (tr.state !== 'started') tr.start('+0.1');
  out.volume.cancelScheduledValues(Tone.now());
  out.volume.rampTo(volToDb(volume), 1.5);
  listeners.forEach((l) => l(true));
}
export function stopJazz() {
  if (!running) return;
  running = false;
  out.volume.rampTo(-60, 0.8);
  setTimeout(() => { if (!running) Tone.getTransport().pause(); }, 900);
  listeners.forEach((l) => l(false));
}
const volToDb = (v: number) => (v <= 0 ? -60 : Tone.gainToDb(v * 0.6));
export function setJazzVolume(v: number) {
  volume = v;
  if (built && running) out.volume.rampTo(volToDb(v), 0.3);
}
