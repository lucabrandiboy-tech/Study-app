import { useEffect, useMemo, useRef, useState } from 'react';
import { PlayEngine, PlayMode, PlayResult } from './engine';
import { Piece, toMusicXML, pieceLength, measureLen } from './notation';
import { Keyboard } from './Keyboard';
import { SheetMusic, FallingNotes, SheetHandle } from './views';
import { initAudio, isPianoLoaded, onPianoLoad } from './audio';
import { Stars, Tabs } from '../components/ui';
import { MidiBadge } from './MidiSetup';

export const MODE_LABEL: Record<PlayMode, string> = { demo: '🔊 Demo', wait: '⏳ Wait-for-me', perform: '🎯 Perform' };

export function ResultCard({ r, onRetry, extra }: { r: PlayResult; onRetry?: () => void; extra?: React.ReactNode }) {
  return (
    <div className="card2 animate-pop">
      <div className="flex items-center gap-6">
        <Stars n={r.stars} size="text-5xl" />
        <div className="flex-1">
          <div className="h2">{r.summary}</div>
          <div className="flex gap-6 mt-1 text-sm">
            <span>Notes: <b className="text-edge">{r.accuracy}%</b> ({r.hits}/{r.total})</span>
            {r.mode === 'perform' && <span>On time: <b className="text-edge">{r.onTime}%</b></span>}
            <span>Wrong notes: <b className={r.wrong ? 'text-bad' : 'text-good'}>{r.wrong}</b></span>
          </div>
        </div>
        {onRetry && <button className="btn" onClick={onRetry}>↺ Try again</button>}
        {extra}
      </div>
      {(r.missed.length > 0 || r.timing.length > 0) && (
        <div className="grid grid-cols-2 gap-4 mt-4 text-sm">
          <div><div className="font-bold text-bad mb-1">Missed notes</div>{r.missed.length ? <ul className="space-y-0.5 max-h-40 overflow-auto">{r.missed.slice(0, 20).map((m, i) => <li key={i}>• {m}</li>)}</ul> : <div className="muted">None — great!</div>}</div>
          <div><div className="font-bold text-streak mb-1">Rhythm & wrong notes</div>{r.timing.length ? <ul className="space-y-0.5 max-h-40 overflow-auto">{r.timing.slice(0, 20).map((m, i) => <li key={i}>• {m}</li>)}</ul> : <div className="muted">Nice steady rhythm!</div>}</div>
        </div>
      )}
    </div>
  );
}

export function PlayAlong({ piece, modes = ['demo', 'wait', 'perform'], mode: forcedMode, onFinish, full = false, defaultView = 'sheet', keyRange, lockTempo }: {
  piece: Piece; modes?: PlayMode[]; mode?: PlayMode; onFinish?: (r: PlayResult) => void; full?: boolean; defaultView?: 'sheet' | 'falling'; keyRange?: [number, number]; lockTempo?: boolean;
}) {
  const [mode, setMode] = useState<PlayMode>(forcedMode ?? modes[0]);
  const [view, setView] = useState<'sheet' | 'falling'>(defaultView);
  const [tempo, setTempo] = useState(1);
  const [hands, setHands] = useState<'both' | 'R' | 'L'>('both');
  const [metro, setMetro] = useState(true);
  const [names, setNames] = useState(false);
  const [loopOn, setLoopOn] = useState(false);
  const [loop, setLoop] = useState({ from: 1, to: 2 });
  const [playing, setPlaying] = useState(false);
  const [result, setResult] = useState<PlayResult | null>(null);
  const [guide, setGuide] = useState<Map<number, 'R' | 'L'>>(new Map());
  const [marks, setMarks] = useState<Map<number, 'good' | 'bad'>>(new Map());
  const [progress, setProgress] = useState(0);
  const [pianoReady, setPianoReady] = useState(isPianoLoaded());
  const engine = useRef<PlayEngine | null>(null);
  const viewRef = useRef<SheetHandle>(null);
  const guideKey = useRef('');
  const lastProg = useRef(0);

  useEffect(() => onPianoLoad((ok) => setPianoReady(ok)), []);
  useEffect(() => { if (forcedMode) setMode(forcedMode); }, [forcedMode]);
  const xml = useMemo(() => piece.xml ?? toMusicXML(piece), [piece]);
  const total = pieceLength(piece);
  const nMeasures = Math.round(total / measureLen(piece));
  const bothHands = new Set(piece.events.map((e) => e.hand)).size === 2;
  const [low, high] = useMemo<[number, number]>(() => {
    if (keyRange) return keyRange;
    const ms = piece.events.map((e) => e.midi);
    const lo = Math.min(...ms, 60), hi = Math.max(...ms, 72);
    return [Math.max(21, Math.min(lo - 3, 48)), Math.min(108, Math.max(hi + 3, 76))];
  }, [piece, keyRange]);

  useEffect(() => () => engine.current?.stop(false), []);
  useEffect(() => { engine.current?.stop(false); setPlaying(false); setResult(null); viewRef.current?.setBeat(0); }, [piece]);

  const start = async () => {
    await initAudio();
    setResult(null);
    engine.current?.stop(false);
    const e = new PlayEngine(piece, { mode, tempo, hands: bothHands ? hands : 'both', metronome: metro, loop: loopOn ? loop : null, countIn: mode !== 'wait' });
    e.onFrame = (b) => {
      viewRef.current?.setBeat(Math.max(0, b));
      const g = e.guide();
      const k = [...g.entries()].join(',');
      if (k !== guideKey.current) { guideKey.current = k; setGuide(g); }
      const p = Math.max(0, Math.min(1, b / total));
      if (Math.abs(p - lastProg.current) > 0.01) { lastProg.current = p; setProgress(p); }
    };
    e.onMark = (midi, good) => { setMarks((m) => new Map(m).set(midi, good ? 'good' : 'bad')); };
    e.onFinish = (r) => {
      setPlaying(false); setGuide(new Map()); setProgress(1);
      if (mode !== 'demo') setResult(r);
      onFinish?.(r);
    };
    engine.current = e;
    e.start();
    setPlaying(true);
  };
  const stop = () => { engine.current?.stop(false); setPlaying(false); setGuide(new Map()); };

  return (
    <div className="space-y-3">
      <div className="flex items-center gap-3 flex-wrap">
        {!forcedMode && modes.length > 1 && <Tabs<PlayMode> value={mode} onChange={(m) => { stop(); setMode(m); setResult(null); }} tabs={modes.map((m) => ({ id: m, label: MODE_LABEL[m] }))} />}
        <button className="btn w-32" onClick={playing ? stop : start}>{playing ? '⏹ Stop' : mode === 'demo' ? '▶ Listen' : '▶ Start'}</button>
        <Tabs value={view} onChange={setView} tabs={[{ id: 'sheet', label: '🎼 Sheet' }, { id: 'falling', label: '🌧 Falling' }]} />
        <label className="flex items-center gap-1 text-sm"><input type="checkbox" checked={names} onChange={(e) => setNames(e.target.checked)} /> Note names</label>
        <label className="flex items-center gap-1 text-sm"><input type="checkbox" checked={metro} onChange={(e) => setMetro(e.target.checked)} /> Metronome</label>
        <div className="ml-auto flex items-center gap-3">
          {!pianoReady && <span className="text-xs muted">Loading piano sound… (a simple synth plays until then)</span>}
          <MidiBadge />
        </div>
      </div>
      {(full || !lockTempo) && (
        <div className="flex items-center gap-5 flex-wrap text-sm">
          <label className="flex items-center gap-2">Tempo <input type="range" min={0.25} max={1.5} step={0.05} value={tempo} disabled={playing} onChange={(e) => setTempo(Number(e.target.value))} className="accent-[#A970FF] w-40" />
            <b className="whitespace-nowrap">{Math.round(tempo * 100)}% ({Math.round(piece.bpm * tempo)} bpm)</b></label>
          {bothHands && (
            <div className="flex gap-1">
              {(['both', 'R', 'L'] as const).map((h) => (
                <button key={h} disabled={playing} onClick={() => setHands(h)} className={`px-3 py-1 rounded-lg border text-sm font-bold ${hands === h ? 'bg-accent border-accent' : 'border-edge/40'}`}>
                  {h === 'both' ? '🙌 Both' : h === 'R' ? '👉 Right only' : '👈 Left only'}
                </button>
              ))}
            </div>
          )}
          {full && (
            <label className="flex items-center gap-2">
              <input type="checkbox" checked={loopOn} disabled={playing} onChange={(e) => setLoopOn(e.target.checked)} /> Loop measures
              <input type="number" className="input w-16 py-1" min={1} max={nMeasures} value={loop.from} disabled={playing} onChange={(e) => setLoop({ ...loop, from: Math.max(1, Math.min(nMeasures, Number(e.target.value))) })} />
              to
              <input type="number" className="input w-16 py-1" min={loop.from} max={nMeasures} value={loop.to} disabled={playing} onChange={(e) => setLoop({ ...loop, to: Math.max(loop.from, Math.min(nMeasures, Number(e.target.value))) })} />
              <span className="muted">of {nMeasures}</span>
            </label>
          )}
        </div>
      )}
      <div className="h-1.5 bg-navy rounded-full overflow-hidden"><div className="h-full bg-accent transition-all" style={{ width: `${progress * 100}%` }} /></div>
      <div>
        {view === 'sheet'
          ? <SheetMusic ref={viewRef} xml={xml} height={full ? 300 : 230} />
          : <FallingNotes ref={viewRef} piece={piece} low={low} high={high} activeHands={bothHands ? hands : 'both'} showNames={names} height={full ? 300 : 230} />}
        <div onMouseUp={() => setMarks(new Map())}>
          <Keyboard low={low} high={high} highlight={guide} marks={marks} showNames={names} showKeys={false} height={full ? 150 : 130} />
        </div>
        <div className="flex gap-4 text-xs muted mt-1"><span><span style={{ color: '#7FD3FF' }}>■</span> Right hand</span><span><span style={{ color: '#FF9F43' }}>■</span> Left hand</span><span>Computer keys: A W S E D F T G Y H U J K (Z/X change octave, Shift = pedal)</span></div>
      </div>
      {result && <ResultCard r={result} onRetry={start} />}
    </div>
  );
}
