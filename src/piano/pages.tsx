import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useApp, recordLesson, recordUnitTest, recordActivity, setPlacement, recordSong, setRecordings, celebrate, openChat, getState } from '../lib/store';
import { useActiveMinutes, usePage } from '../lib/hooks';
import { COURSE, TOTAL_UNITS, lessonPiece, PLACEMENT, Unit } from './course';
import { PlayAlong, ResultCard } from './PlayAlong';
import { PlayResult } from './engine';
import { Keyboard } from './Keyboard';
import { PageHeader, Rich, Stars, Tabs, ProgressBar } from '../components/ui';
import { initAudio, playNote, now, click, setPedal, isPedalDown } from './audio';
import { onNote, emitNote } from './input';
import { songLibrary, SongEntry } from './songs';
import { parseMusicXML, parseMidiFile, Piece, midiName } from './notation';
import { MidiBadge } from './MidiSetup';

// ---------------------------------------------------------------- Course map
export function unitStatus(u: Unit, s: ReturnType<typeof getState>) {
  const unlocked = u.n <= s.piano.unlockedUnit;
  const passed = u.exam ? u.lessons.every((l) => (s.piano.lessons[l.id] ?? 0) >= 2) : (s.piano.unitTests[u.n] ?? 0) >= 2;
  return { unlocked, passed };
}

export function PianoCourse() {
  const piano = useApp((s) => s.piano);
  const state = useApp((s) => s);
  const [placing, setPlacing] = useState(false);
  usePage({ label: 'Piano > Course Map', subject: 'piano' });
  const levels = ['Beginner', 'Intermediate', 'Advanced', 'Final Exam'] as const;
  const totalLessons = COURSE.reduce((n, u) => n + u.lessons.length, 0);
  const doneLessons = COURSE.reduce((n, u) => n + u.lessons.filter((l) => (piano.lessons[l.id] ?? 0) > 0).length, 0);
  if (placing) return <PlacementTest onDone={() => setPlacing(false)} />;
  return (
    <div>
      <PageHeader title="Piano Course" sub={`${doneLessons}/${totalLessons} lessons complete · Pass each Unit Test with 2+ stars to unlock the next unit.`}
        right={<button className="btn-ghost" onClick={() => setPlacing(true)}>🧭 {piano.placementDone ? 'Retake' : 'Take'} placement test</button>} />
      {!piano.placementDone && piano.unlockedUnit === 1 && (
        <div className="card2 mb-6 flex items-center gap-4">
          <div className="text-4xl">🧭</div>
          <div className="flex-1"><div className="font-bold">Already play a bit?</div><div className="muted text-sm">Take the short placement test to skip units you already know.</div></div>
          <button className="btn" onClick={() => setPlacing(true)}>Start placement test</button>
        </div>
      )}
      {levels.map((lvl) => (
        <div key={lvl} className="mb-8">
          <h2 className="h2 mb-3">{lvl === 'Beginner' ? '🌱' : lvl === 'Intermediate' ? '🌿' : lvl === 'Advanced' ? '🌳' : '👑'} {lvl}</h2>
          <div className="grid grid-cols-4 gap-3">
            {COURSE.filter((u) => u.level === lvl).map((u) => {
              const { unlocked, passed } = unitStatus(u, state);
              const done = u.lessons.filter((l) => (piano.lessons[l.id] ?? 0) > 0).length;
              const stars = u.exam ? Math.min(...u.lessons.map((l) => piano.lessons[l.id] ?? 0)) : piano.unitTests[u.n] ?? 0;
              const inner = (
                <div className={`card h-full ${!unlocked ? 'opacity-45' : passed ? 'border-good/70' : 'border-accent/70 shadow-[0_0_14px_rgba(169,112,255,.35)]'}`}>
                  <div className="flex justify-between items-start">
                    <div className="text-xs muted">{u.exam ? 'FINAL' : `UNIT ${u.n}`}</div>
                    <div>{!unlocked ? '🔒' : passed ? '✅' : '▶️'}</div>
                  </div>
                  <div className="font-bold mt-1 mb-2 leading-tight">{u.title}</div>
                  <ProgressBar value={done} max={u.lessons.length} />
                  <div className="flex justify-between items-center mt-1 text-xs muted"><span>{done}/{u.lessons.length} lessons</span>{passed && <Stars n={stars} size="text-sm" />}</div>
                </div>
              );
              return unlocked ? <Link key={u.n} to={`/piano/unit/${u.n}`}>{inner}</Link> : <div key={u.n} title="Pass the previous unit test to unlock">{inner}</div>;
            })}
          </div>
        </div>
      ))}
    </div>
  );
}

function PlacementTest({ onDone }: { onDone: () => void }) {
  const [i, setI] = useState(-1);
  const [last, setLast] = useState<PlayResult | null>(null);
  const item = PLACEMENT[i];
  const piece = useMemo(() => (item ? lessonPiece(`placement-${i}`, item.label, item.ex) : null), [i]); // eslint-disable-line react-hooks/exhaustive-deps
  usePage({ label: 'Piano > Placement Test', subject: 'piano' });
  const finish = (unit: number) => { setPlacement(unit); celebrate(unit > 1 ? `Placed at Unit ${unit}!` : 'Starting at Unit 1', '🧭', 'Your course map has been updated.'); onDone(); };
  if (i < 0) return (
    <div>
      <PageHeader title="Placement Test" sub="Play up to 6 short pieces that get harder. Stop whenever it gets too tough." />
      <div className="card space-y-3 w-[700px]">
        <p>Each piece checks a skill. If you earn <b>2+ stars</b>, you skip ahead past that unit. Use wait-for-me mode to warm up — only Perform mode counts.</p>
        <div className="flex gap-2"><button className="btn" onClick={() => setI(0)}>Start</button><button className="btn-ghost" onClick={onDone}>Cancel</button></div>
      </div>
    </div>
  );
  return (
    <div>
      <PageHeader title={`Placement ${i + 1}/${PLACEMENT.length}: ${item.label}`} sub="Perform it with 2+ stars to move on." right={<button className="btn-ghost" onClick={() => finish(i === 0 ? 1 : PLACEMENT[i - 1].unit + 1)}>I'll stop here</button>} />
      {piece && <PlayAlong key={i} piece={piece} modes={['wait', 'perform']} onFinish={(r) => { if (r.mode === 'perform') setLast(r); }} />}
      {last && (
        <div className="mt-3 flex gap-3">
          {last.stars >= 2
            ? (i + 1 < PLACEMENT.length ? <button className="btn" onClick={() => { setLast(null); setI(i + 1); }}>Passed! Next piece →</button> : <button className="btn" onClick={() => finish(Math.min(TOTAL_UNITS, item.unit + 1))}>Finish — place me!</button>)
            : <button className="btn" onClick={() => finish(i === 0 ? 1 : PLACEMENT[i - 1].unit + 1)}>Place me here (Unit {i === 0 ? 1 : PLACEMENT[i - 1].unit + 1})</button>}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------- Unit page
export function UnitPage() {
  const { n } = useParams();
  const unit = COURSE.find((u) => u.n === Number(n));
  const piano = useApp((s) => s.piano);
  const state = useApp((s) => s);
  usePage({ label: `Piano > Unit ${n}: ${unit?.title}`, subject: 'piano' });
  if (!unit) return <div>Unit not found</div>;
  const { unlocked, passed } = unitStatus(unit, state);
  if (!unlocked) return <div className="card">🔒 This unit is locked. Pass the previous unit test first. <Link className="text-edge" to="/piano">Back to map</Link></div>;
  return (
    <div>
      <PageHeader title={unit.exam ? '👑 Final Exam' : `Unit ${unit.n}: ${unit.title}`} sub={unit.level} right={<Link to="/piano" className="btn-ghost">← Course map</Link>} />
      <div className="grid grid-cols-2 gap-3 mb-6">
        {unit.lessons.map((l, i) => (
          <Link key={l.id} to={`/piano/lesson/${l.id}`} className="card flex items-center gap-4 py-4">
            <div className="w-10 h-10 rounded-full bg-card2 border border-edge/50 flex items-center justify-center font-extrabold text-edge">{i + 1}</div>
            <div className="flex-1 font-bold">{l.title}</div>
            <Stars n={piano.lessons[l.id] ?? 0} size="text-lg" />
          </Link>
        ))}
      </div>
      {!unit.exam && (
        <div className="grid grid-cols-2 gap-3">
          {unit.song && (
            <Link to={`/piano/lesson/u${unit.n}-song`} className="card2 flex items-center gap-4">
              <div className="text-4xl">🎵</div>
              <div className="flex-1"><div className="text-xs muted">PRACTICE SONG</div><div className="font-bold">{unit.song.title}</div><div className="text-xs muted">{unit.song.composer}</div></div>
              <Stars n={piano.lessons[`u${unit.n}-song`] ?? 0} size="text-lg" />
            </Link>
          )}
          {unit.test && (
            <Link to={`/piano/lesson/u${unit.n}-test`} className={`card2 flex items-center gap-4 ${passed ? 'border-good' : 'border-streak'}`}>
              <div className="text-4xl">🏁</div>
              <div className="flex-1"><div className="text-xs muted">UNIT TEST · need 2+ stars</div><div className="font-bold">{passed ? 'Passed!' : 'Take the unit test'}</div></div>
              <Stars n={piano.unitTests[unit.n] ?? 0} size="text-lg" />
            </Link>
          )}
        </div>
      )}
      {unit.exam && <div className="card2">{passed ? '👑 You passed the Final Exam! You are an Advanced Pianist.' : 'Earn 2+ stars on all three exam pieces to pass.'}</div>}
    </div>
  );
}

// ---------------------------------------------------------------- Lesson page
type Step = 'learn' | 'demo' | 'practice' | 'perform' | 'feedback';
export function LessonPage() {
  const { id = '' } = useParams();
  const nav = useNavigate();
  const m = id.match(/^u(\d+)-(song|test)$/);
  const unit = COURSE.find((u) => (m ? u.n === Number(m[1]) : u.lessons.some((l) => l.id === id)))!;
  const lesson = m ? null : unit?.lessons.find((l) => l.id === id);
  const kind: 'lesson' | 'song' | 'test' = m ? (m[2] as 'song' | 'test') : 'lesson';
  const title = kind === 'song' ? `🎵 ${unit.song?.title}` : kind === 'test' ? `🏁 Unit ${unit.n} Test` : lesson?.title ?? '';
  const text = kind === 'song' ? `Practice song for this unit${unit.song?.composer ? ` by **${unit.song.composer}**` : ''}. Listen to the demo, practice with wait-for-me, then perform it for stars.`
    : kind === 'test' ? `**Unit Test**: perform this piece at tempo and earn **2 or more stars** to unlock the next unit. Warm up first with the demo and wait-for-me mode.`
    : lesson?.text ?? '';
  const ex = kind === 'song' ? unit.song!.ex : kind === 'test' ? unit.test! : lesson!.ex;
  const piece = useMemo(() => lessonPiece(id, title, ex), [id]); // eslint-disable-line react-hooks/exhaustive-deps
  const stars = useApp((s) => (kind === 'test' ? s.piano.unitTests[unit.n] : s.piano.lessons[id]) ?? 0);
  const [step, setStep] = useState<Step>('learn');
  const [result, setResult] = useState<PlayResult | null>(null);
  useActiveMinutes('piano');
  usePage({ label: `Piano > Unit ${unit?.n}: ${unit?.title} > ${title}`, detail: `Step: ${step}${result ? `. Last score: ${result.stars} stars, ${result.accuracy}% notes, ${result.onTime}% on time. Missed: ${result.missed.slice(0, 5).join('; ')}` : ''}`, subject: 'piano' }, { kind: 'piano', path: `/piano/lesson/${id}` });
  useEffect(() => { setStep('learn'); setResult(null); }, [id]);

  // "Learn" animation: light up each note of the exercise in order on the keyboard
  const [anim, setAnim] = useState<Map<number, 'R' | 'L'>>(new Map());
  const animRef = useRef(0);
  const animate = async () => {
    await initAudio();
    const token = ++animRef.current;
    const groups = new Map<number, typeof piece.events>();
    piece.events.forEach((e) => groups.set(e.beat, [...(groups.get(e.beat) ?? []), e]));
    const spb = 60 / (piece.bpm * 0.8);
    for (const [, evs] of [...groups.entries()].sort((a, b) => a[0] - b[0])) {
      if (token !== animRef.current) return;
      setAnim(new Map(evs.map((e) => [e.midi, e.hand])));
      evs.forEach((e) => playNote(e.midi, e.dur * spb * 0.9, undefined, 0.5));
      await new Promise((r) => setTimeout(r, Math.max(...evs.map((e) => e.dur)) * spb * 1000));
    }
    setAnim(new Map());
  };
  useEffect(() => () => { animRef.current++; }, []);

  const onFinish = (r: PlayResult) => {
    if (r.mode === 'perform') {
      setResult(r);
      if (kind === 'test' || (unit.exam && kind === 'lesson')) {
        if (kind === 'test') recordUnitTest(unit.n, r.stars, TOTAL_UNITS);
        else { recordLesson(id, r.stars); const s = getState(); if (unit.lessons.every((l) => (s.piano.lessons[l.id] ?? 0) >= 2)) recordUnitTest(unit.n, 3, TOTAL_UNITS); }
        if (r.stars >= 2) celebrate(kind === 'test' ? `Unit ${unit.n} passed!` : 'Exam piece passed!', '🏁', kind === 'test' && unit.n < TOTAL_UNITS ? `Unit ${unit.n + 1} is unlocked.` : undefined);
      } else recordLesson(id, r.stars);
      if (r.stars >= 1) recordActivity('piano');
      setStep('feedback');
    } else if (r.mode === 'wait') {
      if (r.accuracy === 100 && step === 'practice') setTimeout(() => setStep('perform'), 800);
    }
  };

  const idx = unit.lessons.findIndex((l) => l.id === id);
  const nextId = kind === 'lesson' ? (unit.lessons[idx + 1]?.id ?? (unit.song ? `u${unit.n}-song` : unit.test ? `u${unit.n}-test` : null)) : kind === 'song' ? (unit.test ? `u${unit.n}-test` : null) : null;
  const stepList: { id: Step; label: string }[] = [
    { id: 'learn', label: '1. Learn' }, { id: 'demo', label: '2. Demo' }, { id: 'practice', label: '3. Practice' }, { id: 'perform', label: '4. Perform' }, { id: 'feedback', label: '5. Feedback' },
  ];
  return (
    <div>
      <PageHeader title={title} sub={<span>Unit {unit.n}: {unit.title} · Best: <Stars n={stars} size="text-base" /></span>} right={<Link to={`/piano/unit/${unit.n}`} className="btn-ghost">← Unit</Link>} />
      <div className="mb-4"><Tabs<Step> value={step} onChange={setStep} tabs={stepList.map((s) => ({ ...s, disabled: s.id === 'feedback' && !result }))} /></div>

      {step === 'learn' && (
        <div className="space-y-4">
          <div className="card"><Rich text={text} className="text-[17px]" /></div>
          <div className="card">
            <div className="flex items-center gap-3 mb-3"><button className="btn" onClick={animate}>▶ Show me on the keyboard</button><span className="muted text-sm">Watch which keys light up (blue = right hand, orange = left hand).</span></div>
            <Keyboard low={Math.min(48, ...piece.events.map((e) => e.midi)) - 2} high={Math.max(84, ...piece.events.map((e) => e.midi)) + 2} highlight={anim} showNames />
          </div>
          <div className="flex gap-2"><button className="btn" onClick={() => setStep('demo')}>Next: hear the demo →</button><button className="btn-ghost" onClick={() => openChat('Can you explain this piano lesson another way?')}>🤖 Piano help</button></div>
        </div>
      )}
      {step === 'demo' && <><PlayAlong piece={piece} mode="demo" /><button className="btn mt-4" onClick={() => setStep('practice')}>Next: practice it →</button></>}
      {step === 'practice' && <><div className="card2 mb-3 text-sm">⏳ <b>Wait-for-me mode:</b> the music waits until you play the right note(s). Take your time!</div><PlayAlong piece={piece} mode="wait" onFinish={onFinish} /><button className="btn mt-4" onClick={() => setStep('perform')}>Ready to perform →</button></>}
      {step === 'perform' && <><div className="card2 mb-3 text-sm">🎯 <b>Performance:</b> after a 1-measure count-in, play along at tempo. Stars are based on correct notes and timing. {kind === 'test' ? 'You need 2+ stars to pass.' : ''}</div><PlayAlong piece={piece} mode="perform" onFinish={onFinish} lockTempo={kind === 'test'} /></>}
      {step === 'feedback' && result && (
        <div className="space-y-4">
          <ResultCard r={result} onRetry={() => setStep('perform')} />
          <div className="flex gap-2">
            {nextId && <button className="btn" onClick={() => nav(`/piano/lesson/${nextId}`)}>Next →</button>}
            <button className="btn-ghost" onClick={() => setStep('practice')}>Practice more (wait-for-me)</button>
            <button className="btn-ghost" onClick={() => openChat(`I just played "${title}" and got ${result.stars} stars (${result.accuracy}% notes, ${result.onTime}% on time). What should I practice?`)}>🤖 What should I work on?</button>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------- Song player
export function SongPlayer() {
  const lib = useMemo(songLibrary, []);
  const songs = useApp((s) => s.piano.songs);
  const [sel, setSel] = useState<SongEntry | null>(null);
  const [imported, setImported] = useState<Piece | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const piece = useMemo(() => imported ?? sel?.piece() ?? null, [sel, imported]);
  const [last, setLast] = useState<PlayResult | null>(null);
  useActiveMinutes('piano');
  usePage({ label: piece ? `Piano > Song Player: ${piece.title}` : 'Piano > Song Player', detail: last ? `Last result: ${last.stars} stars, ${last.accuracy}% notes. ${last.missed.slice(0, 5).join('; ')}` : undefined, subject: 'piano' }, { kind: 'piano', path: '/songs' });

  const onFile = async (f: File) => {
    setErr(null);
    try {
      if (/\.midi?$/i.test(f.name)) setImported(await parseMidiFile(await f.arrayBuffer(), f.name));
      else if (/\.mxl$/i.test(f.name)) throw new Error('Compressed .mxl files are not supported yet — export as uncompressed .musicxml or .xml.');
      else setImported(parseMusicXML(await f.text(), f.name));
      setSel(null);
    } catch (e) { setErr((e as Error).message); }
  };
  const onFinish = (r: PlayResult) => {
    if (r.mode !== 'perform' || !piece) return;
    setLast(r);
    recordSong(piece.id, r.stars, r.accuracy, r.missed.slice(0, 10));
    if (r.stars >= 1) recordActivity('piano');
  };

  if (!piece) return (
    <div>
      <PageHeader title="Song Player" sub="Public-domain songs sorted by level — or import your own MusicXML / MIDI." right={
        <label className="btn cursor-pointer">📂 Import MusicXML / MIDI<input type="file" accept=".xml,.musicxml,.mid,.midi,.mxl" className="hidden" onChange={(e) => e.target.files?.[0] && onFile(e.target.files[0])} /></label>
      } />
      {err && <div className="card border-bad text-bad mb-4">{err}</div>}
      <p className="muted text-sm mb-4">Tip: free, legal public-domain sheet music is available as MusicXML/MIDI on sites like MuseScore (public-domain filter), IMSLP, and Mutopia.</p>
      {(['Beginner', 'Intermediate', 'Advanced'] as const).map((lvl) => (
        <div key={lvl} className="mb-6">
          <h2 className="h2 mb-3">{lvl}</h2>
          <div className="grid grid-cols-3 gap-3">
            {lib.filter((s) => s.level === lvl).map((s) => {
              const rec = songs[s.id];
              return (
                <button key={s.id} className="card text-left flex items-center gap-3" onClick={() => { setSel(s); setImported(null); setLast(null); }}>
                  <div className="text-3xl">🎼</div>
                  <div className="flex-1"><div className="font-bold">{s.title}</div><div className="text-xs muted">{s.composer}</div></div>
                  {rec && <div className="text-right"><Stars n={rec.stars} size="text-sm" /><div className="text-xs muted">{rec.accuracy}%</div></div>}
                </button>
              );
            })}
          </div>
        </div>
      ))}
    </div>
  );
  return (
    <div>
      <PageHeader title={piece.title} sub={[piece.composer ?? sel?.composer, `${piece.bpm} bpm`, `${piece.beats}/${piece.beatUnit}`].filter(Boolean).join(' · ')} right={<button className="btn-ghost" onClick={() => { setSel(null); setImported(null); setLast(null); }}>← Library</button>} />
      <PlayAlong piece={piece} full onFinish={onFinish} defaultView="sheet" />
      {last && <div className="mt-2 flex justify-end"><button className="btn-ghost" onClick={() => openChat(`I played "${piece.title}" and got ${last.stars} stars (${last.accuracy}% notes, ${last.onTime}% on time). Missed: ${last.missed.slice(0, 6).join('; ')}. How should I practice the tricky parts?`)}>🤖 Ask for practice tips</button></div>}
    </div>
  );
}

// ---------------------------------------------------------------- Free play
export function FreePlay() {
  const recs = useApp((s) => s.recordings);
  const [recording, setRecording] = useState(false);
  const [events, setEvents] = useState<{ t: number; midi: number; on: boolean; vel: number }[]>([]);
  const [metro, setMetro] = useState(false);
  const [bpm, setBpm] = useState(90);
  const [pedal, setPedalUi] = useState(false);
  const [playingId, setPlayingId] = useState<string | null>(null);
  const t0 = useRef(0);
  const evRef = useRef(events);
  evRef.current = events;
  useActiveMinutes('piano');
  usePage({ label: 'Piano > Free Play', subject: 'piano' });

  useEffect(() => onNote((e) => { if (recording) setEvents((ev) => [...ev, { t: performance.now() - t0.current, midi: e.midi, on: e.type === 'on', vel: e.vel }]); }), [recording]);
  useEffect(() => {
    if (!metro) return;
    let beat = 0;
    const id = setInterval(() => { click(beat % 4 === 0); beat++; }, 60000 / bpm);
    return () => clearInterval(id);
  }, [metro, bpm]);
  useEffect(() => { const id = setInterval(() => setPedalUi(isPedalDown()), 100); return () => clearInterval(id); }, []);

  const startRec = async () => { await initAudio(); setEvents([]); t0.current = performance.now(); setRecording(true); };
  const stopRec = () => {
    setRecording(false);
    if (evRef.current.length) {
      const name = `Recording ${recs.length + 1}`;
      setRecordings([{ id: `rec-${Date.now()}`, name, date: new Date().toLocaleString(), events: evRef.current }, ...recs].slice(0, 20));
    }
  };
  const play = async (id: string) => {
    await initAudio();
    const r = recs.find((x) => x.id === id); if (!r) return;
    setPlayingId(id);
    const start = now() + 0.1;
    const ons = new Map<number, { t: number; vel: number }>();
    r.events.forEach((e) => {
      if (e.on) ons.set(e.midi, { t: e.t, vel: e.vel });
      else { const o = ons.get(e.midi); if (o) { playNote(e.midi, (e.t - o.t) / 1000, start + o.t / 1000, o.vel); ons.delete(e.midi); } }
    });
    ons.forEach((o, midi) => playNote(midi, 0.5, start + o.t / 1000, o.vel));
    const end = r.events.length ? r.events[r.events.length - 1].t : 0;
    setTimeout(() => setPlayingId(null), end + 800);
  };

  return (
    <div>
      <PageHeader title="Free Play" sub="Play anything! Record yourself and listen back." right={<MidiBadge />} />
      <div className="card mb-4 flex items-center gap-4 flex-wrap">
        {!recording ? <button className="btn bg-bad" onClick={startRec}>⏺ Record</button> : <button className="btn bg-bad animate-pulse" onClick={stopRec}>⏹ Stop recording ({events.filter((e) => e.on).length} notes)</button>}
        <label className="flex items-center gap-2"><input type="checkbox" checked={metro} onChange={(e) => { void initAudio(); setMetro(e.target.checked); }} /> Metronome</label>
        <label className="flex items-center gap-2 text-sm">Tempo <input type="range" min={40} max={200} value={bpm} onChange={(e) => setBpm(Number(e.target.value))} className="accent-[#A970FF]" /> <b>{bpm} bpm</b></label>
        <button className={`btn-ghost ${pedal ? 'bg-accent/40 border-accent' : ''}`} onMouseDown={() => setPedal(true)} onMouseUp={() => setPedal(false)} onMouseLeave={() => pedal && setPedal(false)}>🦶 Sustain pedal {pedal ? '(down)' : ''}</button>
        <span className="text-xs muted">Hold Shift or use your keyboard's pedal</span>
      </div>
      <Keyboard low={36} high={96} height={190} showKeys />
      <div className="card mt-4">
        <div className="h2 mb-3">My recordings</div>
        {recs.length === 0 ? <div className="muted">No recordings yet. Press Record and play something!</div> : (
          <div className="space-y-2">
            {recs.map((r) => (
              <div key={r.id} className="flex items-center gap-3 border-b border-edge/20 pb-2">
                <button className="btn py-1" disabled={!!playingId} onClick={() => play(r.id)}>{playingId === r.id ? '🔊 Playing…' : '▶ Play'}</button>
                <input className="input py-1 flex-1" value={r.name} onChange={(e) => setRecordings(recs.map((x) => (x.id === r.id ? { ...x, name: e.target.value } : x)))} />
                <span className="text-xs muted">{r.date} · {r.events.filter((e) => e.on).length} notes · {(r.events[r.events.length - 1]?.t / 1000 || 0).toFixed(1)}s</span>
                <button className="btn-ghost py-1" onClick={() => setRecordings(recs.filter((x) => x.id !== r.id))}>🗑</button>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export { emitNote, midiName };
