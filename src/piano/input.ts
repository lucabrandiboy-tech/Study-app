import { noteOn, noteOff, setPedal, initAudio } from './audio';

/** Unified note input: USB MIDI keyboard (Web MIDI) + computer keyboard backup. */
export type NoteEvent = { type: 'on' | 'off'; midi: number; vel: number; source: 'midi' | 'keys' | 'mouse' };
type Listener = (e: NoteEvent) => void;

const listeners = new Set<Listener>();
const statusListeners = new Set<() => void>();
export const pressed = new Set<number>();
let midiAccess: MIDIAccess | null = null;
let midiError: string | null = null;
let lastMidiNote: number | null = null;
let muted = false;
let midiAppSound = true; // false = let the keyboard's own speakers make the sound (no double sound)
export function setMidiAppSound(on: boolean) { midiAppSound = on; } // when true, input still reports but the app doesn't play sound (used by nothing currently)

export function onNote(fn: Listener) { listeners.add(fn); return () => { listeners.delete(fn); }; }
export function onStatus(fn: () => void) { statusListeners.add(fn); return () => { statusListeners.delete(fn); }; }
const status = () => statusListeners.forEach((f) => f());

export function emitNote(e: NoteEvent) {
  if (e.type === 'on') {
    if (pressed.has(e.midi)) return;
    pressed.add(e.midi);
    if (!muted && (e.source !== 'midi' || midiAppSound)) noteOn(e.midi, e.vel);
    if (e.source === 'midi') { lastMidiNote = e.midi; status(); }
  } else {
    if (!pressed.has(e.midi)) return;
    pressed.delete(e.midi);
    if (!muted && (e.source !== 'midi' || midiAppSound)) noteOff(e.midi);
  }
  listeners.forEach((l) => l(e));
}

export function midiStatus() {
  const inputs = midiAccess ? [...midiAccess.inputs.values()].filter((i) => i.state === 'connected') : [];
  return { supported: typeof navigator !== 'undefined' && 'requestMIDIAccess' in navigator, connected: inputs.length > 0, names: inputs.map((i) => i.name ?? 'MIDI device'), error: midiError, lastNote: lastMidiNote, octave: keyOctave };
}

function handleMidi(msg: MIDIMessageEvent) {
  const d = msg.data; if (!d || d.length < 2) return;
  if (d[0] >= 0xf0) return; // clock / active sensing (Roland keyboards send these constantly)
  const cmd = d[0] & 0xf0, note = d[1], vel = d[2] ?? 0;
  if (cmd === 0x90 && vel > 0) emitNote({ type: 'on', midi: note, vel: Math.max(0.15, vel / 127), source: 'midi' });
  else if (cmd === 0x80 || (cmd === 0x90 && vel === 0)) emitNote({ type: 'off', midi: note, vel: 0, source: 'midi' });
  else if (cmd === 0xb0 && note === 64) { if (midiAppSound) setPedal(vel >= 64); } // sustain pedal (CC64)
  else if (cmd === 0xb0 && (note === 123 || note === 120)) releaseMidiNotes(); // all notes off
}

const midiHeld = () => [...pressed];
function releaseMidiNotes() { midiHeld().forEach((m) => emitNote({ type: 'off', midi: m, vel: 0, source: 'midi' })); setPedal(false); }
let connecting = false;

export async function connectMidi() {
  if (connecting) return;
  connecting = true;
  try { await connectMidiInner(); } finally { connecting = false; }
}
async function connectMidiInner() {
  if (!('requestMIDIAccess' in navigator)) { midiError = 'Web MIDI is not supported in this browser. Use Chrome or Edge.'; status(); return; }
  try {
    midiAccess = await navigator.requestMIDIAccess();
    const bind = () => {
      midiAccess!.inputs.forEach((i) => { i.onmidimessage = handleMidi; if (i.state === 'connected' && i.connection !== 'open') void i.open().catch(() => { midiError = `"${i.name}" is busy — close other music apps (DAWs, Roland Piano App, other browser tabs) that are using it, then click Detect.`; status(); }); });
      if (![...midiAccess!.inputs.values()].some((i) => i.state === 'connected')) releaseMidiNotes();
      status();
    };
    bind();
    midiAccess.onstatechange = bind;
    midiError = null;
  } catch (e) {
    midiError = 'Permission to use MIDI was blocked. Click the 🔒 / site-settings icon left of the address bar, set "MIDI devices" to Allow, then reload.';
  }
  status();
}

// ---- computer keyboard ----
// Lower row (white keys) + upper row (black keys), like most DAWs.  Z / X shift octaves.  Shift = sustain.
const KEYMAP: Record<string, number> = { a: 0, w: 1, s: 2, e: 3, d: 4, f: 5, t: 6, g: 7, y: 8, h: 9, u: 10, j: 11, k: 12, o: 13, l: 14, p: 15, ';': 16, "'": 17 };
let keyOctave = 4;
let keysEnabled = true;
const keyDown = new Map<string, number>();
export function setComputerKeysEnabled(on: boolean) { keysEnabled = on; }

function isTyping(e: KeyboardEvent) {
  const t = e.target as HTMLElement | null;
  return !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable);
}

export function installComputerKeyboard() {
  const onDown = (e: KeyboardEvent) => {
    if (!keysEnabled || isTyping(e) || e.metaKey || e.ctrlKey || e.altKey) return;
    if (e.key === 'Shift') { setPedal(true); return; }
    const k = e.key.toLowerCase();
    if (k === 'z') { keyOctave = Math.max(1, keyOctave - 1); status(); return; }
    if (k === 'x') { keyOctave = Math.min(7, keyOctave + 1); status(); return; }
    if (k in KEYMAP && !e.repeat && !keyDown.has(k)) {
      void initAudio();
      const m = 12 * (keyOctave + 1) + KEYMAP[k];
      keyDown.set(k, m);
      emitNote({ type: 'on', midi: m, vel: 0.75, source: 'keys' });
      e.preventDefault();
    }
  };
  const onUp = (e: KeyboardEvent) => {
    if (e.key === 'Shift') { setPedal(false); return; }
    const k = e.key.toLowerCase();
    const m = keyDown.get(k);
    if (m !== undefined) { keyDown.delete(k); emitNote({ type: 'off', midi: m, vel: 0, source: 'keys' }); }
  };
  const onBlur = () => { keyDown.forEach((m) => emitNote({ type: 'off', midi: m, vel: 0, source: 'keys' })); keyDown.clear(); setPedal(false); };
  window.addEventListener('keydown', onDown);
  window.addEventListener('keyup', onUp);
  window.addEventListener('blur', onBlur);
  return () => { window.removeEventListener('keydown', onDown); window.removeEventListener('keyup', onUp); window.removeEventListener('blur', onBlur); };
}
export const keyLabelFor = (midi: number) => {
  const off = midi - 12 * (keyOctave + 1);
  return Object.entries(KEYMAP).find(([, v]) => v === off)?.[0]?.toUpperCase() ?? null;
};
