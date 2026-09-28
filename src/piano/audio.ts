import * as Tone from 'tone';
import { getState } from '../lib/store';

let sampler: Tone.Sampler | null = null;
let loaded = false;
let loadPromise: Promise<void> | null = null;
let clickSynth: Tone.MembraneSynth | null = null;
let woodSynth: Tone.MetalSynth | null = null;
let beepSynth: Tone.Synth | null = null;
let metroBeep: Tone.Synth | null = null;
let fallback: Tone.PolySynth | null = null;
const listeners = new Set<(ok: boolean) => void>();

export const midiToName = (m: number) => Tone.Frequency(m, 'midi').toNote();

/** Start audio (must be called from a user gesture) and load the Salamander grand piano samples. */
export function initAudio(): Promise<void> {
  if (loadPromise) return loadPromise;
  loadPromise = (async () => {
    await Tone.start();
    Tone.getContext().lookAhead = 0.05;
    fallback = new Tone.PolySynth(Tone.Synth, { oscillator: { type: 'triangle' }, envelope: { attack: 0.005, decay: 0.3, sustain: 0.3, release: 0.8 } }).toDestination();
    fallback.volume.value = -10;
    // Load the sampled piano in the background; the simple synth plays until it's ready.
    const urls: Record<string, string> = {};
    ['A0', 'C1', 'D#1', 'F#1', 'A1', 'C2', 'D#2', 'F#2', 'A2', 'C3', 'D#3', 'F#3', 'A3', 'C4', 'D#4', 'F#4', 'A4', 'C5', 'D#5', 'F#5', 'A5', 'C6', 'D#6', 'F#6', 'A6', 'C7', 'D#7', 'F#7', 'A7', 'C8']
      .forEach((n) => { urls[n] = `${n.replace('#', 's')}.mp3`; });
    sampler = new Tone.Sampler({
      urls, release: 1.2, baseUrl: 'https://tonejs.github.io/audio/salamander/',
      onload: () => { loaded = true; listeners.forEach((l) => l(true)); },
      onerror: () => listeners.forEach((l) => l(false)),
    }).toDestination();
    clickSynth = new Tone.MembraneSynth({ pitchDecay: 0.008, octaves: 2, envelope: { attack: 0.001, decay: 0.08, sustain: 0 } }).toDestination();
    woodSynth = new Tone.MetalSynth({ envelope: { attack: 0.001, decay: 0.05, release: 0.01 }, harmonicity: 3.1, modulationIndex: 16, resonance: 3000, octaves: 0.5 }).toDestination();
    metroBeep = new Tone.Synth({ oscillator: { type: 'sine' }, envelope: { attack: 0.001, decay: 0.05, sustain: 0, release: 0.05 } }).toDestination();
    if (!beepSynth) beepSynth = new Tone.Synth({ oscillator: { type: 'sine' }, envelope: { attack: 0.01, decay: 0.1, sustain: 0.2, release: 0.2 } }).toDestination();
    setVolume(getState().settings.volume);
  })();
  return loadPromise;
}
export const isPianoLoaded = () => loaded;
export function onPianoLoad(fn: (ok: boolean) => void) { listeners.add(fn); return () => { listeners.delete(fn); }; }

export function setVolume(v: number) { Tone.getDestination().volume.value = v <= 0 ? -Infinity : Tone.gainToDb(v); }

// ---- sustain pedal handling ----
let pedal = false;
const heldByPedal = new Set<number>();
const down = new Set<number>();
export function setPedal(on: boolean) {
  pedal = on;
  if (!on) { heldByPedal.forEach((m) => { if (!down.has(m)) release(m); }); heldByPedal.clear(); }
}
export const isPedalDown = () => pedal;

function inst() { return loaded && sampler ? sampler : fallback; }
function release(m: number, time?: number) { inst()?.triggerRelease(midiToName(m), time); }

export function noteOn(m: number, vel = 0.8) {
  const i = inst(); if (!i) return;
  down.add(m);
  heldByPedal.delete(m);
  i.triggerAttack(midiToName(m), undefined, vel);
}
export function noteOff(m: number) {
  down.delete(m);
  if (pedal) { heldByPedal.add(m); return; }
  release(m);
}
/** Schedule a note at an audio-context time (seconds). */
export function playNote(m: number, durSec: number, time?: number, vel = 0.7) {
  inst()?.triggerAttackRelease(midiToName(m), Math.max(0.05, durSec), time, vel);
}
export function releaseAll() { sampler?.releaseAll(); fallback?.releaseAll(); down.clear(); heldByPedal.clear(); }
export const now = () => Tone.now();

// Monophonic synths throw if two notes start at the exact same time, so keep start times strictly increasing.
let lastClick = 0;
export function click(accent: boolean, time?: number) {
  const kind = getState().settings.metronome;
  const t = Math.max(time ?? Tone.now(), lastClick + 0.002);
  lastClick = t;
  try {
    if (kind === 'click') clickSynth?.triggerAttackRelease(accent ? 'C6' : 'G5', 0.03, t, accent ? 0.9 : 0.5);
    else if (kind === 'wood') woodSynth?.triggerAttackRelease(accent ? 0.05 : 0.03, t, accent ? 0.5 : 0.25);
    else metroBeep?.triggerAttackRelease(accent ? 'A5' : 'E5', 0.05, t, accent ? 0.6 : 0.35);
  } catch { /* never let a metronome tick break playback */ }
}

/** Simple alert sounds (works even if the piano isn't loaded yet). */
export async function beep(kind: 'done' | 'start' | 'good' | 'bad') {
  await Tone.start();
  if (!beepSynth) beepSynth = new Tone.Synth().toDestination();
  const t = Math.max(Tone.now(), lastBeep + 0.01);
  const seq = kind === 'done' ? ['C5', 'E5', 'G5', 'C6'] : kind === 'start' ? ['G5', 'C6'] : kind === 'good' ? ['E5', 'A5'] : ['C4'];
  lastBeep = t + seq.length * 0.13;
  try { seq.forEach((n, i) => beepSynth!.triggerAttackRelease(n, 0.12, t + i * 0.13)); } catch { /* ignore */ }
}
let lastBeep = 0;
