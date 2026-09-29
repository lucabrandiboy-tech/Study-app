import { useEffect, useState } from 'react';
import { connectMidi, midiStatus, onStatus, onNote } from './input';
import { useApp, setSettings } from '../lib/store';
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
      <MidiSoundToggle />
      <Keyboard low={21} high={108} height={110} showKeys />
      <RolandHelp />
      <div className="text-sm muted">
        <b>No MIDI keyboard?</b> Use the computer keyboard: <b>A S D F G H J K</b> = white keys, <b>W E T Y U O P</b> = black keys. <b>Z / X</b> move down/up an octave (now octave {s.octave}). Hold <b>Shift</b> for the sustain pedal. A real sustain pedal on your MIDI keyboard also works.
      </div>
    </div>
  );
}

function MidiSoundToggle() {
  const on = useApp((s) => s.settings.midiAppSound);
  return (
    <div className="rounded-xl bg-navy border border-edge/30 p-3 text-sm flex items-center gap-3">
      <span className="flex-1"><b>Sound for your MIDI keyboard:</b> {on ? 'the app plays its piano sound.' : "your keyboard's own speakers (app stays quiet)."} If you hear every note twice, switch to your keyboard's speakers — or turn your keyboard's volume down.</span>
      <button className="btn-ghost" onClick={() => setSettings({ midiAppSound: !on })}>{on ? "🔈 Use keyboard's speakers" : '🎹 Use app sound'}</button>
    </div>
  );
}

function RolandHelp() {
  return (
    <details className="rounded-xl bg-navy border border-edge/30 p-3 text-sm">
      <summary className="font-bold cursor-pointer">🎹 Roland keyboard not showing up? Step-by-step fix</summary>
      <ol className="list-decimal pl-5 mt-2 space-y-1">
        <li>Use a <b>USB cable</b> in the port labeled <b>USB COMPUTER</b> / <b>USB to Host</b> (the square "USB-B" plug). The flat "USB MEMORY" port does NOT send notes.</li>
        <li>Turn the keyboard <b>on</b> before opening the app, then click <b>🔌 Detect keyboard</b>.</li>
        <li>Open the app in <b>Google Chrome</b> or <b>Microsoft Edge</b> (Safari and Firefox don't support MIDI keyboards). The downloaded .html file works too — just open it in Chrome/Edge.</li>
        <li>When the browser asks "use your MIDI devices?" click <b>Allow</b>. If you blocked it: click the icon left of the address bar → Site settings → MIDI devices → Allow, then reload.</li>
        <li><b>Only one app</b> can use the keyboard at a time on Windows. Close the Roland Piano App, GarageBand, DAWs, or other tabs using MIDI.</li>
        <li>Windows didn't find it? Some older Roland models (FP-30, RP-30, F-140R, GO:PIANO…) need the <b>Roland USB driver</b> from roland.com → Support → your model → Updates & Drivers. Newer models (FP-10, FP-30X, FP-60X, RD-88) work without one.</li>
        <li>Bluetooth Roland pianos: Bluetooth MIDI works in Chrome on a Mac after pairing in "Audio MIDI Setup". On Windows, use the USB cable.</li>
        <li>Still nothing? Unplug the USB cable, wait 5 seconds, plug it back in — the app reconnects automatically.</li>
      </ol>
    </details>
  );
}
