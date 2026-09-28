import { useEffect, useState } from 'react';
import { connectMidi, midiStatus, onStatus, onNote } from './input';
import { initAudio } from './audio';
import { midiName } from './notation';
import { Keyboard } from './Keyboard';

export function useMidiStatus() {
  const [s, set] = useState(midiStatus());
  useEffect(() => onStatus(() => set(midiStatus())), []);
  return s;
}

export function MidiBadge() {
  const s = useMidiStatus();
  return (
    <span className="flex items-center gap-2 text-xs px-2 py-1 rounded-full border border-edge/40" title={s.connected ? s.names.join(', ') : 'Using computer keyboard / mouse'}>
      <span className={`w-2.5 h-2.5 rounded-full ${s.connected ? 'bg-good shadow-[0_0_8px_#4ADE80]' : 'bg-muted/40'}`} />
      {s.connected ? 'MIDI connected' : `Computer keys (octave ${s.octave})`}
    </span>
  );
}

export function MidiSetupPanel() {
  const s = useMidiStatus();
  const [last, setLast] = useState<string | null>(null);
  useEffect(() => { void connectMidi(); }, []);
  useEffect(() => onNote((e) => { if (e.type === 'on') setLast(`${midiName(e.midi)} (${e.source === 'midi' ? 'MIDI keyboard' : e.source === 'keys' ? 'computer keyboard' : 'mouse'})`); }), []);
  return (
    <div className="space-y-4">
      <div className="flex items-center gap-4">
        <div className={`w-5 h-5 rounded-full ${s.connected ? 'bg-good shadow-[0_0_14px_#4ADE80]' : 'bg-bad/70'}`} />
        <div className="flex-1">
          <div className="font-bold">{s.connected ? `Connected: ${s.names.join(', ')}` : 'No MIDI keyboard detected'}</div>
          <div className="text-sm muted">{s.error ?? (s.connected ? 'Press any key on your piano to test it.' : 'Plug in your keyboard with USB, turn it on, then click "Detect keyboard". Works in Chrome and Edge.')}</div>
        </div>
        <button className="btn" onClick={() => { void initAudio(); void connectMidi(); }}>🔌 Detect keyboard</button>
      </div>
      <div className="rounded-xl bg-navy border border-edge/30 p-3 text-sm">
        <b>Test:</b> {last ? <>You pressed <span className="text-good font-bold">{last}</span> ✓</> : 'press a key on your piano (or computer keys A–K)…'}
      </div>
      <Keyboard low={36} high={96} height={110} showKeys />
      <div className="text-sm muted">
        <b>No MIDI keyboard?</b> Use the computer keyboard: <b>A S D F G H J K</b> = white keys, <b>W E T Y U O P</b> = black keys. <b>Z / X</b> move down/up an octave (now octave {s.octave}). Hold <b>Shift</b> for the sustain pedal. A real sustain pedal on your MIDI keyboard also works.
      </div>
    </div>
  );
}
