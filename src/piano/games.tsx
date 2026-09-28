import { useEffect, useMemo, useRef, useState } from 'react';
import { useApp, recordNoteGame, addXp, recordActivity } from '../lib/store';
import { usePage, useActiveMinutes } from '../lib/hooks';
import { PageHeader, Tabs } from '../components/ui';
import { onNote } from './input';
import { initAudio, playNote, now, click, beep } from './audio';
import { Keyboard } from './Keyboard';
import { PlayAlong } from './PlayAlong';
import { lessonPiece } from './course';
import { scale, arpeggio } from './theory';
import { nameToMidi, pitchClass } from './notation';
import { MidiBadge } from './MidiSetup';

type Tab = 'warmup' | 'notes' | 'rhythm' | 'ear';
export function PracticeGames() {
  const [tab, setTab] = useState<Tab>('warmup');
  useActiveMinutes('piano');
  usePage({ label: `Piano > Extra Practice > ${tab}`, subject: 'piano' });
  return (
    <div>
      <PageHeader title="Extra Practice" sub="Warm-ups and games to sharpen your skills." right={<MidiBadge />} />
      <div className="mb-4"><Tabs<Tab> value={tab} onChange={setTab} tabs={[{ id: 'warmup', label: '🔥 Daily warm-ups' }, { id: 'notes', label: '🎯 Note reading' }, { id: 'rhythm', label: '🥁 Rhythm tapping' }, { id: 'ear', label: '👂 Ear training' }]} /></div>
      {tab === 'warmup' && <WarmUps />}
      {tab === 'notes' && <NoteReading />}
      {tab === 'rhythm' && <RhythmGame />}
      {tab === 'ear' && <EarTraining />}
    </div>
  );
}

// ---------------- warm-ups ----------------
const WARMUPS = [
  { id: 'wu-5finger', title: '5-finger pattern (C, G, F)', ex: { rh: 'C4:e D4 E4 F4 G4 F4 E4 D4 | G4 A4 B4 C5 D5 C5 B4 A4 | F4 G4 A4 Bb4 C5 Bb4 A4 G4 | C4:w', lh: 'C3:e D3 E3 F3 G3 F3 E3 D3 | G2 A2 B2 C3 D3 C3 B2 A2 | F2 G2 A2 Bb2 C3 Bb2 A2 G2 | C3:w', bpm: 80 } },
  { id: 'wu-scale', title: 'C major scale, hands together', ex: { rh: scale('C4', 'major', 1, 'e'), lh: scale('C3', 'major', 1, 'e'), bpm: 80 } },
  { id: 'wu-arp', title: 'C major arpeggio', ex: { rh: arpeggio('C4', 'major', 2), bpm: 80 } },
  { id: 'wu-hanon', title: 'Finger independence (Hanon-style)', ex: { rh: 'C4:s E4 F4 G4 A4 G4 F4 E4 D4 F4 G4 A4 B4 A4 G4 F4 | E4 G4 A4 B4 C5 B4 A4 G4 C5:h', bpm: 70 } },
];
function WarmUps() {
  const [i, setI] = useState(0);
  const w = WARMUPS[i];
  const piece = useMemo(() => lessonPiece(w.id, w.title, w.ex), [w]);
  return (
    <div className="space-y-3">
      <div className="flex gap-2 flex-wrap">{WARMUPS.map((x, k) => <button key={x.id} className={k === i ? 'btn' : 'btn-ghost'} onClick={() => setI(k)}>{x.title}</button>)}</div>
      <PlayAlong piece={piece} modes={['wait', 'perform', 'demo']} onFinish={(r) => { if (r.mode !== 'demo') addXp(5 + r.stars * 5); }} />
    </div>
  );
}

// ---------------- note reading speed game ----------------
const TREBLE = ['C4', 'D4', 'E4', 'F4', 'G4', 'A4', 'B4', 'C5', 'D5', 'E5', 'F5', 'G5', 'A5'];
const BASS = ['C2', 'D2', 'E2', 'F2', 'G2', 'A2', 'B2', 'C3', 'D3', 'E3', 'F3', 'G3', 'A3', 'B3', 'C4'];
const DIAT = (n: string) => 'CDEFGAB'.indexOf(n[0]) + 7 * Number(n.slice(-1));
function StaffNote({ note, clef, flash }: { note: string; clef: 'treble' | 'bass'; flash: 'good' | 'bad' | null }) {
  const bottom = clef === 'treble' ? DIAT('E4') : DIAT('G2');
  const step = DIAT(note) - bottom; // 0 = bottom line
  const y = (s: number) => 150 - s * 10;
  const ny = y(step);
  const ledgers: number[] = [];
  for (let s = -2; s >= step; s -= 2) ledgers.push(s);
  for (let s = 10; s <= step; s += 2) ledgers.push(s);
  const color = flash === 'good' ? '#4ADE80' : flash === 'bad' ? '#F87171' : '#E8ECFF';
  return (
    <svg viewBox="0 0 300 230" className="w-[420px] bg-navy/70 rounded-2xl border border-edge/40">
      {[0, 2, 4, 6, 8].map((s) => <line key={s} x1={20} x2={280} y1={y(s)} y2={y(s)} stroke="#A9A8D6" strokeWidth={1.5} />)}
      <text x={28} y={clef === 'treble' ? y(2) + 22 : y(6) + 12} fontSize={clef === 'treble' ? 78 : 50} fill="#A9A8D6">{clef === 'treble' ? '𝄞' : '𝄢'}</text>
      {ledgers.map((s) => <line key={s} x1={160} x2={204} y1={y(s)} y2={y(s)} stroke="#A9A8D6" strokeWidth={1.5} />)}
      <ellipse cx={182} cy={ny} rx={13} ry={9.5} fill={color} transform={`rotate(-20 182 ${ny})`} style={{ filter: `drop-shadow(0 0 6px ${color})` }} />
      <line x1={step >= 4 ? 170 : 194} x2={step >= 4 ? 170 : 194} y1={ny} y2={step >= 4 ? ny + 60 : ny - 60} stroke={color} strokeWidth={2.5} />
    </svg>
  );
}
function NoteReading() {
  const [clef, setClef] = useState<'treble' | 'bass' | 'both'>('treble');
  const [exact, setExact] = useState(false);
  const [running, setRunning] = useState(false);
  const [left, setLeft] = useState(60);
  const [cur, setCur] = useState<{ n: string; c: 'treble' | 'bass' }>({ n: 'C4', c: 'treble' });
  const [score, setScore] = useState({ right: 0, wrong: 0 });
  const [flash, setFlash] = useState<'good' | 'bad' | null>(null);
  const [final, setFinal] = useState<number | null>(null);
  const history = useApp((s) => s.piano.noteGameNPM);
  const best = history.reduce((m, h) => Math.max(m, h.npm), 0);
  const next = () => {
    const c = clef === 'both' ? (Math.random() < 0.5 ? 'treble' : 'bass') : clef;
    const pool = c === 'treble' ? TREBLE : BASS;
    let n = pool[Math.floor(Math.random() * pool.length)];
    if (n === cur.n) n = pool[(pool.indexOf(n) + 1) % pool.length];
    setCur({ n, c });
  };
  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => setLeft((l) => {
      if (l <= 1) { clearInterval(id); setRunning(false); return 0; }
      return l - 1;
    }), 1000);
    return () => clearInterval(id);
  }, [running]);
  useEffect(() => {
    if (!running && left === 0 && final === null) {
      setFinal(score.right);
      recordNoteGame(score.right);
      if (score.right >= 10) recordActivity('piano');
    }
  }, [running, left]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => onNote((e) => {
    if (e.type !== 'on' || !running) return;
    const target = nameToMidi(cur.n);
    const ok = exact ? e.midi === target : e.midi % 12 === target % 12;
    setFlash(ok ? 'good' : 'bad');
    setTimeout(() => setFlash(null), 180);
    if (ok) { setScore((s) => ({ ...s, right: s.right + 1 })); next(); } else setScore((s) => ({ ...s, wrong: s.wrong + 1 }));
  }), [running, cur, exact]); // eslint-disable-line react-hooks/exhaustive-deps
  const start = async () => { await initAudio(); setScore({ right: 0, wrong: 0 }); setLeft(60); setFinal(null); next(); setRunning(true); };
  return (
    <div className="card">
      <div className="flex gap-4 items-center mb-4 flex-wrap">
        <Tabs value={clef} onChange={(c) => !running && setClef(c)} tabs={[{ id: 'treble', label: '𝄞 Treble' }, { id: 'bass', label: '𝄢 Bass' }, { id: 'both', label: 'Both' }]} />
        <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={exact} disabled={running} onChange={(e) => setExact(e.target.checked)} /> Exact octave (for MIDI keyboards)</label>
        <div className="ml-auto text-sm muted">Best: <b className="text-edge">{best}</b> notes/min</div>
      </div>
      <div className="flex gap-8 items-center">
        <StaffNote note={cur.n} clef={cur.c} flash={flash} />
        <div className="space-y-2">
          <div className="text-5xl font-extrabold tabular-nums text-edge">{left}s</div>
          <div className="text-xl">✓ <b className="text-good">{score.right}</b> &nbsp; ✗ <b className="text-bad">{score.wrong}</b></div>
          {!running && <button className="btn" onClick={start}>{final !== null ? 'Play again' : '▶ Start (60 seconds)'}</button>}
          {final !== null && <div className="animate-pop text-lg">🎯 <b>{final}</b> notes per minute{final > best - 1 && final > 0 ? ' — new best!' : ''}</div>}
        </div>
      </div>
      <div className="mt-4"><Keyboard low={36} high={84} height={110} showKeys /></div>
    </div>
  );
}

// ---------------- rhythm tapping ----------------
const PATTERNS: { name: string; d: number[] }[] = [
  { name: 'Quarters', d: [1, 1, 1, 1] }, { name: 'Half & quarters', d: [2, 1, 1] }, { name: 'Eighths', d: [0.5, 0.5, 1, 0.5, 0.5, 1] },
  { name: 'Dotted quarter', d: [1.5, 0.5, 1, 1] }, { name: 'Syncopation', d: [0.5, 1, 0.5, 1, 1] }, { name: 'Sixteenths', d: [0.25, 0.25, 0.25, 0.25, 1, 0.5, 0.5, 1] },
  { name: 'Triplets', d: [1 / 3, 1 / 3, 1 / 3, 1, 1 / 3, 1 / 3, 1 / 3, 1] }, { name: 'Gallop', d: [0.75, 0.25, 0.75, 0.25, 2] },
];
function RhythmGame() {
  const [p, setP] = useState(0);
  const [bpm, setBpm] = useState(80);
  const [phase, setPhase] = useState<'idle' | 'listen' | 'tap' | 'done'>('idle');
  const [taps, setTaps] = useState<number[]>([]);
  const t0 = useRef(0);
  const pat = PATTERNS[p];
  const onsets = pat.d.reduce<number[]>((a, _, i) => [...a, i === 0 ? 0 : a[i - 1] + pat.d[i - 1]], []);
  const spb = 60 / bpm;
  const listen = async () => {
    await initAudio(); setPhase('listen'); setTaps([]);
    const t = now() + 0.2;
    for (let b = 0; b < 4; b++) click(b === 0, t + b * spb);
    onsets.forEach((o) => playNote(nameToMidi('C5'), 0.15, t + (4 + o) * spb, 0.8));
    setTimeout(() => setPhase('idle'), (8 * spb + 0.4) * 1000);
  };
  const tapNow = async () => {
    await initAudio(); setPhase('tap'); setTaps([]);
    const t = now() + 0.2;
    for (let b = 0; b < 4; b++) click(b === 0, t + b * spb);
    t0.current = performance.now() + (0.2 + 4 * spb) * 1000;
    setTimeout(() => setPhase('done'), (8.5 * spb + 0.2) * 1000);
  };
  useEffect(() => {
    if (phase !== 'tap') return;
    const tap = () => { const t = (performance.now() - t0.current) / 1000 / spb; if (t > -0.5) { setTaps((x) => [...x, t]); playNote(nameToMidi('G4'), 0.1); } };
    const kd = (e: KeyboardEvent) => { if (e.code === 'Space' && !e.repeat) { e.preventDefault(); tap(); } };
    const off = onNote((e) => e.type === 'on' && setTaps((x) => [...x, (performance.now() - t0.current) / 1000 / spb]));
    window.addEventListener('keydown', kd); window.addEventListener('mousedown', tap);
    return () => { window.removeEventListener('keydown', kd); window.removeEventListener('mousedown', tap); off(); };
  }, [phase, spb]);
  const results = onsets.map((o) => { const best = taps.reduce((m, t) => (Math.abs(t - o) < Math.abs(m - o) ? t : m), Infinity); const off = (best - o) * spb * 1000; return Math.abs(off) < 400 ? off : null; });
  const good = results.filter((r) => r !== null && Math.abs(r) < 120).length;
  const extra = Math.max(0, taps.length - onsets.length);
  useEffect(() => { if (phase === 'done' && good === onsets.length && extra === 0) { beep('good'); addXp(10); } }, [phase]); // eslint-disable-line react-hooks/exhaustive-deps
  const W = 560, X = (b: number) => 20 + (b / 4) * (W - 40);
  return (
    <div className="card space-y-4">
      <div className="flex gap-2 flex-wrap">{PATTERNS.map((x, k) => <button key={x.name} className={k === p ? 'btn py-1' : 'btn-ghost py-1'} onClick={() => { setP(k); setPhase('idle'); setTaps([]); }}>{x.name}</button>)}</div>
      <svg viewBox={`0 0 ${W} 110`} className="w-full max-w-[600px] bg-navy/70 rounded-xl border border-edge/30">
        {[0, 1, 2, 3, 4].map((b) => <line key={b} x1={X(b)} x2={X(b)} y1={10} y2={100} stroke="#2a3366" />)}
        {onsets.map((o, i) => <g key={i}><rect x={X(o) + 2} y={30} width={Math.max(6, X(o + pat.d[i]) - X(o) - 4)} height={16} rx={6} fill="#A970FF" /><text x={X(o) + 4} y={24} fontSize={10} fill="#A9A8D6">{pat.d[i] === 1 ? '♩' : pat.d[i] === 0.5 ? '♪' : pat.d[i] === 2 ? '𝅗𝅥' : ''}</text></g>)}
        {phase === 'done' && taps.map((t, i) => <circle key={i} cx={X(t)} cy={72} r={6} fill={results.some((r, k) => r !== null && Math.abs(onsets[k] + r / 1000 / spb - t) < 1e-6 && Math.abs(r) < 120) ? '#4ADE80' : '#FF9F43'} />)}
        <text x={10} y={76} fontSize={10} fill="#A9A8D6">you</text>
      </svg>
      <div className="flex gap-3 items-center">
        <button className="btn-ghost" disabled={phase === 'listen' || phase === 'tap'} onClick={listen}>🔊 Listen</button>
        <button className="btn" disabled={phase === 'listen' || phase === 'tap'} onClick={tapNow}>🥁 Tap it</button>
        <label className="flex items-center gap-2 text-sm">Tempo <input type="range" min={50} max={140} value={bpm} onChange={(e) => setBpm(Number(e.target.value))} className="accent-[#A970FF]" /> {bpm}</label>
        <span className="muted text-sm">{phase === 'tap' ? 'After 4 clicks, tap SPACE (or click, or play any key)!' : 'Listen first, then tap along after the 4-beat count-in.'}</span>
      </div>
      {phase === 'done' && (
        <div className={`animate-pop rounded-xl border-2 px-4 py-3 ${good === onsets.length && !extra ? 'border-good bg-good/10' : 'border-streak bg-streak/10'}`}>
          <b>{good}/{onsets.length}</b> taps on time{extra ? `, ${extra} extra tap${extra > 1 ? 's' : ''}` : ''}. {results.map((r, i) => (r === null ? `Note ${i + 1} missed. ` : Math.abs(r) >= 120 ? `Note ${i + 1} ${r > 0 ? 'late' : 'early'} by ${Math.round(Math.abs(r))}ms. ` : '')).join('')}
        </div>
      )}
    </div>
  );
}

// ---------------- ear training ----------------
const INTERVALS: [string, number][] = [['Minor 2nd', 1], ['Major 2nd', 2], ['Minor 3rd', 3], ['Major 3rd', 4], ['Perfect 4th', 5], ['Tritone', 6], ['Perfect 5th', 7], ['Minor 6th', 8], ['Major 6th', 9], ['Minor 7th', 10], ['Major 7th', 11], ['Octave', 12]];
const CHORDS: [string, number[]][] = [['Major', [0, 4, 7]], ['Minor', [0, 3, 7]], ['Diminished', [0, 3, 6]], ['Augmented', [0, 4, 8]], ['Major 7th', [0, 4, 7, 11]], ['Dominant 7th', [0, 4, 7, 10]], ['Minor 7th', [0, 3, 7, 10]]];
function EarTraining() {
  const [kind, setKind] = useState<'interval' | 'chord'>('interval');
  const [level, setLevel] = useState(0);
  const [q, setQ] = useState<{ root: number; ans: string } | null>(null);
  const [picked, setPicked] = useState<string | null>(null);
  const [stats, setStats] = useState({ right: 0, total: 0 });
  const options = kind === 'interval'
    ? INTERVALS.filter(([, s]) => (level === 0 ? [4, 7, 12, 3].includes(s) : level === 1 ? s !== 6 && s !== 1 && s !== 11 : true)).map(([n]) => n)
    : CHORDS.filter((_, i) => (level === 0 ? i < 2 : level === 1 ? i < 4 : true)).map(([n]) => n);
  const play = async (qq = q, mode: 'melodic' | 'harmonic' = 'melodic') => {
    if (!qq) return;
    await initAudio();
    const t = now() + 0.05;
    const semis = kind === 'interval' ? [0, INTERVALS.find(([n]) => n === qq.ans)![1]] : CHORDS.find(([n]) => n === qq.ans)![1];
    if (kind === 'interval' && mode === 'melodic') semis.forEach((s, i) => playNote(qq.root + s, 0.8, t + i * 0.7));
    else semis.forEach((s) => playNote(qq.root + s, 1.4, t));
  };
  const newQ = () => {
    const nq = { root: 48 + Math.floor(Math.random() * 12), ans: options[Math.floor(Math.random() * options.length)] };
    setQ(nq); setPicked(null); void play(nq, kind === 'chord' ? 'harmonic' : 'melodic');
  };
  const choose = (o: string) => {
    if (!q || picked) return;
    setPicked(o);
    const ok = o === q.ans;
    setStats((s) => ({ right: s.right + (ok ? 1 : 0), total: s.total + 1 }));
    if (ok) addXp(3);
    beep(ok ? 'good' : 'bad');
  };
  return (
    <div className="card space-y-4">
      <div className="flex gap-4 items-center flex-wrap">
        <Tabs value={kind} onChange={(k) => { setKind(k); setQ(null); }} tabs={[{ id: 'interval', label: 'Intervals' }, { id: 'chord', label: 'Chords' }]} />
        <Tabs value={String(level)} onChange={(l) => { setLevel(Number(l)); setQ(null); }} tabs={[{ id: '0', label: 'Easy' }, { id: '1', label: 'Medium' }, { id: '2', label: 'Hard' }]} />
        <div className="ml-auto muted">Score: <b className="text-edge">{stats.right}/{stats.total}</b></div>
      </div>
      <div className="flex gap-2">
        <button className="btn" onClick={newQ}>{q ? '⏭ Next' : '▶ Start'}</button>
        {q && <button className="btn-ghost" onClick={() => play(q, 'melodic')}>🔁 Replay{kind === 'chord' ? ' (broken)' : ''}</button>}
        {q && <button className="btn-ghost" onClick={() => play(q, 'harmonic')}>🔁 Together</button>}
      </div>
      {q && (
        <div className="grid grid-cols-4 gap-2">
          {options.map((o) => (
            <button key={o} onClick={() => choose(o)} className={`px-4 py-3 rounded-xl border-2 font-bold transition ${picked ? (o === q.ans ? 'border-good bg-good/15' : o === picked ? 'border-bad bg-bad/15' : 'border-edge/20 opacity-60') : 'border-edge/40 hover:border-edge'}`}>{o}</button>
          ))}
        </div>
      )}
      {picked && q && <div className="text-sm muted">Root note: {pitchClass(q.root)}. {kind === 'interval' ? 'Tip: link intervals to songs — a Perfect 4th starts "Here Comes the Bride", a Major 6th starts "My Bonnie".' : 'Tip: major sounds bright, minor sounds sad, diminished sounds tense, augmented sounds dreamy/unsettled.'}</div>}
    </div>
  );
}
