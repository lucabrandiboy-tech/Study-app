import { useEffect, useMemo, useRef, useState } from 'react';
import { PlayEngine, PlayMode, PlayResult } from './engine';
import { Piece, toMusicXML, pieceLength, measureLen } from './notation';
import { Keyboard } from './Keyboard';
import { SheetMusic, FallingNotes, SheetHandle } from './views';
import { initAudio, isPianoLoaded, onPianoLoad } from './audio';
import { useMidiStatus } from './MidiSetup';
import type { Level } from './songs';
import { LEVELS } from './songs';

const MODES: { id: PlayMode; label: string; hint: string }[] = [
  { id: 'wait', label: '🐢 Wait for me', hint: 'The music waits until you play the right notes.' },
  { id: 'perform', label: '🎯 Play', hint: 'Play along at tempo and earn stars.' },
  { id: 'demo', label: '👂 Listen', hint: 'Hear how it goes first.' },
];

function Pill<T extends string>({ value, onChange, items, disabled }: { value: T; onChange: (v: T) => void; items: { id: T; label: string }[]; disabled?: boolean }) {
  return (
    <div className="flex bg-[#F1F5F9] rounded-full p-1">
      {items.map((it) => (
        <button key={it.id} disabled={disabled} onClick={() => onChange(it.id)}
          className={`px-4 py-1.5 rounded-full text-sm font-bold transition disabled:opacity-50 ${value === it.id ? 'bg-white text-[#111827] shadow' : 'text-[#64748B] hover:text-[#111827]'}`}>
          {it.label}
        </button>
      ))}
    </div>
  );
}

function BigStars({ n }: { n: number }) {
  return (
    <div className="flex justify-center gap-3">
      {[0, 1, 2].map((i) => (
        <span key={i} className={`text-6xl transition ${i < n ? 'text-[#FACC15] drop-shadow-[0_4px_0_#CA8A04]' : 'text-[#E5E7EB]'}`} style={{ animation: i < n ? `pop .45s ${0.15 + i * 0.2}s ease-out both` : undefined }}>★</span>
      ))}
    </div>
  );
}

/** Full-screen, Simply-Piano-style player: white page, black sheet music, notes turn green/red as you play. */
export function SimplyPlayer({ piece, title, composer, level, best, onClose, onFinish, onNext }: {
  piece: Piece; title: string; composer?: string; level?: Level; best?: number;
  onClose: () => void; onFinish?: (r: PlayResult) => void; onNext?: () => void;
}) {
  const [mode, setMode] = useState<PlayMode>('perform');
  const [view, setView] = useState<'sheet' | 'falling'>('sheet');
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
  const [count, setCount] = useState(0);
  const [hits, setHits] = useState({ ok: 0, miss: 0 });
  const [pianoReady, setPianoReady] = useState(isPianoLoaded());
  const midi = useMidiStatus();
  const engine = useRef<PlayEngine | null>(null);
  const viewRef = useRef<SheetHandle>(null);
  const guideKey = useRef('');
  const lastProg = useRef(0);

  useEffect(() => onPianoLoad((ok) => setPianoReady(ok)), []);
  useEffect(() => () => engine.current?.stop(false), []);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => { if (e.key === 'Escape') { engine.current?.stop(false); onClose(); } };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  const xml = useMemo(() => piece.xml ?? toMusicXML(piece), [piece]);
  const total = pieceLength(piece);
  const nMeasures = Math.round(total / measureLen(piece));
  const bothHands = new Set(piece.events.map((e) => e.hand)).size === 2;
  const [low, high] = useMemo<[number, number]>(() => {
    const ms = piece.events.map((e) => e.midi);
    return [Math.max(21, Math.min(Math.min(...ms) - 3, 48)), Math.min(108, Math.max(Math.max(...ms) + 3, 76))];
  }, [piece]);
  const lvl = LEVELS.find((l) => l.id === level);

  const start = async () => {
    await initAudio();
    setResult(null); setHits({ ok: 0, miss: 0 }); setMarks(new Map());
    viewRef.current?.resetColors?.();
    engine.current?.stop(false);
    const e = new PlayEngine(piece, { mode, tempo, hands: bothHands ? hands : 'both', metronome: metro, loop: loopOn ? loop : null, countIn: mode !== 'wait' });
    const first = loopOn ? (loop.from - 1) * measureLen(piece) : 0;
    e.onFrame = (b) => {
      viewRef.current?.setBeat(Math.max(first, b));
      const g = e.guide();
      const k = [...g.entries()].join(',');
      if (k !== guideKey.current) { guideKey.current = k; setGuide(g); }
      const p = Math.max(0, Math.min(1, b / total));
      if (Math.abs(p - lastProg.current) > 0.01) { lastProg.current = p; setProgress(p); }
      const c = b < first ? Math.ceil(first - b - 1e-6) : 0;
      setCount((old) => (old === c ? old : c));
    };
    e.onMark = (m, good) => setMarks((mk) => new Map(mk).set(m, good ? 'good' : 'bad'));
    e.onNoteResult = (ev, r) => {
      viewRef.current?.colorNote?.(ev.beat, ev.midi, r === 'hit' ? '#16A34A' : '#DC2626');
      setHits((h) => (r === 'hit' ? { ...h, ok: h.ok + 1 } : { ...h, miss: h.miss + 1 }));
    };
    e.onFinish = (r) => {
      setPlaying(false); setGuide(new Map()); setProgress(1); setCount(0);
      if (r.mode !== 'demo') setResult(r);
      onFinish?.(r);
    };
    engine.current = e;
    e.start();
    setPlaying(true);
  };
  const stop = () => { engine.current?.stop(false); setPlaying(false); setGuide(new Map()); setCount(0); };

  return (
    <div className="fixed inset-0 z-[45] bg-white text-[#111827] flex flex-col" style={{ minWidth: 1024 }}>
      {/* top bar */}
      <header className="flex items-center gap-4 px-6 h-16 border-b border-[#E5E7EB]">
        <button onClick={() => { stop(); onClose(); }} className="w-10 h-10 rounded-full hover:bg-[#F1F5F9] text-2xl text-[#64748B]" aria-label="Close">✕</button>
        <div className="min-w-0">
          <div className="font-extrabold text-lg leading-tight truncate max-w-[360px]">{title}</div>
          <div className="text-xs text-[#64748B] flex items-center gap-2">
            {lvl && <span className="font-extrabold px-1.5 rounded" style={{ color: '#fff', background: lvl.color === '#7FD3FF' ? '#0EA5E9' : lvl.color === '#4ADE80' ? '#16A34A' : lvl.color }}>{lvl.tag}</span>}
            {composer}
          </div>
        </div>
        <div className="flex-1 h-3 bg-[#F1F5F9] rounded-full overflow-hidden mx-4">
          <div className="h-full bg-[#22C55E] rounded-full transition-all duration-300" style={{ width: `${progress * 100}%` }} />
        </div>
        {playing && mode !== 'demo' && <div className="text-sm font-bold tabular-nums"><span className="text-[#16A34A]">✓ {hits.ok}</span>{mode === 'perform' && <span className="text-[#DC2626] ml-3">✗ {hits.miss}</span>}</div>}
        {best ? <div className="text-[#FACC15] text-xl" title="Your best">{'★'.repeat(best)}<span className="text-[#E5E7EB]">{'★'.repeat(3 - best)}</span></div> : null}
      </header>

      {/* controls */}
      <div className="flex items-center gap-3 px-6 py-3 flex-wrap">
        <Pill value={mode} onChange={(m) => { stop(); setMode(m); setResult(null); }} items={MODES} disabled={playing} />
        <Pill value={view} onChange={setView} items={[{ id: 'sheet', label: '🎼 Sheet' }, { id: 'falling', label: '🌧 Falling' }]} />
        {bothHands && <Pill value={hands} onChange={setHands} disabled={playing} items={[{ id: 'both', label: '🙌 Both' }, { id: 'R', label: 'Right' }, { id: 'L', label: 'Left' }]} />}
        <label className="flex items-center gap-2 text-sm font-semibold text-[#334155]">🐇 Speed
          <input type="range" min={0.25} max={1.5} step={0.05} value={tempo} disabled={playing} onChange={(e) => setTempo(Number(e.target.value))} className="accent-[#22C55E] w-28" />
          <span className="tabular-nums w-10">{Math.round(tempo * 100)}%</span></label>
        <label className="flex items-center gap-1.5 text-sm font-semibold text-[#334155]"><input type="checkbox" className="accent-[#22C55E]" checked={metro} onChange={(e) => setMetro(e.target.checked)} /> Metronome</label>
        <label className="flex items-center gap-1.5 text-sm font-semibold text-[#334155]"><input type="checkbox" className="accent-[#22C55E]" checked={names} onChange={(e) => setNames(e.target.checked)} /> Note names</label>
        <label className="flex items-center gap-1.5 text-sm font-semibold text-[#334155]"><input type="checkbox" className="accent-[#22C55E]" checked={loopOn} disabled={playing} onChange={(e) => setLoopOn(e.target.checked)} /> Loop bars
          <input type="number" min={1} max={nMeasures} value={loop.from} disabled={playing || !loopOn} onChange={(e) => setLoop({ ...loop, from: Math.max(1, Math.min(nMeasures, Number(e.target.value))) })} className="w-14 border border-[#CBD5E1] rounded-md px-1.5 py-0.5 bg-white" />
          –<input type="number" min={loop.from} max={nMeasures} value={loop.to} disabled={playing || !loopOn} onChange={(e) => setLoop({ ...loop, to: Math.max(loop.from, Math.min(nMeasures, Number(e.target.value))) })} className="w-14 border border-[#CBD5E1] rounded-md px-1.5 py-0.5 bg-white" />
        </label>
        <div className="ml-auto text-xs text-[#64748B] flex items-center gap-2">
          <span className={`w-2.5 h-2.5 rounded-full ${midi.connected ? 'bg-[#22C55E]' : 'bg-[#CBD5E1]'}`} />{midi.connected ? 'Piano connected' : 'Computer keys (A–K)'}
          {!pianoReady && <span>· loading piano sound…</span>}
        </div>
      </div>

      {/* music */}
      <main className="flex-1 relative px-6 min-h-0 flex flex-col justify-center">
        <div className="relative rounded-2xl border border-[#E5E7EB] bg-white shadow-[0_2px_12px_rgba(15,23,42,.06)] overflow-hidden">
          {view === 'sheet'
            ? <SheetMusic ref={viewRef} xml={xml} light singleLine height={300} zoom={1.15} />
            : <FallingNotes ref={viewRef} piece={piece} low={low} high={high} activeHands={bothHands ? hands : 'both'} showNames={names} height={300} light />}
          {count > 0 && (
            <div className="absolute inset-0 flex items-center justify-center pointer-events-none">
              <div key={count} className="w-28 h-28 rounded-full bg-[#22C55E] text-white text-6xl font-black flex items-center justify-center shadow-xl animate-pop">{count}</div>
            </div>
          )}
          {!playing && !result && (
            <div className="absolute inset-0 flex items-center justify-center bg-white/60">
              <button onClick={start} className="px-10 py-4 rounded-full bg-[#22C55E] hover:bg-[#16A34A] text-white text-2xl font-black shadow-[0_6px_0_#15803D] active:translate-y-1 active:shadow-[0_2px_0_#15803D] transition">
                ▶ {mode === 'demo' ? 'Listen' : 'Start'}
              </button>
            </div>
          )}
        </div>
        <div className="text-center text-sm text-[#64748B] mt-2 h-5">{playing ? (mode === 'wait' ? 'Play the highlighted keys — the music waits for you.' : mode === 'perform' ? 'Keep going! Notes turn green when you hit them.' : 'Listen and watch the keys.') : MODES.find((m) => m.id === mode)?.hint}</div>
        {playing && <button onClick={stop} className="absolute top-2 right-8 px-4 py-1.5 rounded-full bg-[#F1F5F9] hover:bg-[#E2E8F0] text-sm font-bold">⏸ Stop</button>}
      </main>

      {/* keyboard */}
      <footer className="px-4 pb-4 pt-2" onMouseUp={() => setMarks(new Map())}>
        <Keyboard light low={low} high={high} highlight={guide} marks={marks} showNames={names} height={170} />
        <div className="flex gap-4 text-xs text-[#64748B] mt-1.5 justify-center">
          <span><span className="text-[#3B82F6]">■</span> Right hand</span><span><span className="text-[#F59E0B]">■</span> Left hand</span><span>Esc to close</span>
        </div>
      </footer>

      {/* results */}
      {result && (
        <div className="absolute inset-0 bg-[#0F172A]/40 flex items-center justify-center z-10">
          <div className="bg-white rounded-3xl p-8 w-[520px] text-center shadow-2xl animate-pop">
            <div className="text-sm font-bold text-[#64748B] uppercase tracking-wider">{result.mode === 'wait' ? 'Practice complete' : 'Song complete'}</div>
            <div className="text-3xl font-black mt-1 mb-4">{result.stars === 3 ? 'Amazing!' : result.stars === 2 ? 'Great job!' : result.stars === 1 ? 'Nice try!' : 'Keep practicing!'}</div>
            <BigStars n={result.stars} />
            <div className="grid grid-cols-3 gap-3 mt-6">
              <div className="rounded-2xl bg-[#F0FDF4] p-3"><div className="text-2xl font-black text-[#16A34A]">{result.accuracy}%</div><div className="text-xs text-[#64748B] font-semibold">Notes</div></div>
              <div className="rounded-2xl bg-[#EFF6FF] p-3"><div className="text-2xl font-black text-[#2563EB]">{result.mode === 'perform' ? `${result.onTime}%` : '—'}</div><div className="text-xs text-[#64748B] font-semibold">On time</div></div>
              <div className="rounded-2xl bg-[#FEF2F2] p-3"><div className="text-2xl font-black text-[#DC2626]">{result.wrong}</div><div className="text-xs text-[#64748B] font-semibold">Wrong notes</div></div>
            </div>
            {result.missed.length > 0 && (
              <div className="text-left text-sm text-[#334155] mt-4 max-h-28 overflow-auto bg-[#F8FAFC] rounded-xl p-3">
                <div className="font-bold mb-1">Practice these spots:</div>
                {result.missed.slice(0, 6).map((m, i) => <div key={i}>• {m}</div>)}
              </div>
            )}
            <div className="flex gap-3 justify-center mt-6">
              <button onClick={() => { setResult(null); void start(); }} className="px-6 py-3 rounded-full bg-[#F1F5F9] hover:bg-[#E2E8F0] font-bold">↺ Try again</button>
              {result.mode === 'wait' && <button onClick={() => { setResult(null); setMode('perform'); }} className="px-6 py-3 rounded-full bg-[#22C55E] hover:bg-[#16A34A] text-white font-bold shadow-[0_4px_0_#15803D]">🎯 Play it for stars</button>}
              {result.mode === 'perform' && onNext && <button onClick={() => { setResult(null); onNext(); }} className="px-6 py-3 rounded-full bg-[#22C55E] hover:bg-[#16A34A] text-white font-bold shadow-[0_4px_0_#15803D]">Next song →</button>}
            </div>
            <button onClick={() => { setResult(null); onClose(); }} className="mt-4 text-sm font-semibold text-[#64748B] hover:text-[#111827]">Back to songs</button>
          </div>
        </div>
      )}
    </div>
  );
}
