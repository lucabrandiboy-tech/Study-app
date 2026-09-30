// 🎯 A+ Plan — the "get straight A's" hub: daily spaced review, test countdown + study plan, grade tracker.
import { useMemo, useState } from 'react';
import { Link } from 'react-router-dom';
import { useApp, getState, recordAnswer, recordReview, recordMistake, recordActivity, addXp, setGrades, setTests, type GradeEntry, type UpcomingTest, type Difficulty } from '../lib/store';
import { addDays, dayKey, daysBetween, parseDay } from '../lib/date';
import { usePage } from '../lib/hooks';
import { PageHeader, ProgressBar } from '../components/ui';
import { useSubjects } from './pages';
import { QuestionView, correctAnswerText, type Graded } from './QuestionView';
import { GEOMETRY_IDS } from './geometry';
import type { Question, Subject, Topic } from './types';

type PoolItem = { topic: Topic; subject: Subject };
const letter = (g: number) => (g >= 93 ? 'A' : g >= 90 ? 'A−' : g >= 87 ? 'B+' : g >= 83 ? 'B' : g >= 80 ? 'B−' : g >= 77 ? 'C+' : g >= 73 ? 'C' : g >= 70 ? 'C−' : g >= 60 ? 'D' : 'F');
const gradeColor = (g: number) => (g >= 90 ? '#4ADE80' : g >= 80 ? '#7FD3FF' : g >= 70 ? '#FF9F43' : '#F87171');
const uid = () => Math.random().toString(36).slice(2, 9);

/** Topics most worth reviewing today: due on the spaced schedule, then weakest, then recent mistakes. */
function reviewPool(subjects: Subject[], onlySubjects?: string[]): PoolItem[] {
  const s = getState(), today = dayKey();
  const all: PoolItem[] = subjects.filter((sub) => !onlySubjects?.length || onlySubjects.includes(sub.id)).flatMap((subject) => subject.topics.map((topic) => ({ topic, subject })));
  const acc = (id: string) => { const t = s.topics[id]; return t && t.attempts ? t.correct / t.attempts : 1; };
  const due = all.filter((p) => s.review[p.topic.id] && s.review[p.topic.id].due <= today).sort((a, b) => acc(a.topic.id) - acc(b.topic.id));
  const weak = all.filter((p) => (s.topics[p.topic.id]?.attempts ?? 0) >= 3 && acc(p.topic.id) < 0.8 && !due.includes(p)).sort((a, b) => acc(a.topic.id) - acc(b.topic.id));
  const missedIds = new Set(s.mistakes.slice(0, 40).map((m) => m.topicId));
  const missed = all.filter((p) => missedIds.has(p.topic.id) && !due.includes(p) && !weak.includes(p));
  return [...due, ...weak, ...missed];
}

function QuizRunner({ pool, n, title, onExit }: { pool: PoolItem[]; n: number; title: string; onExit: () => void }) {
  const [qs] = useState<{ q: Question; p: PoolItem }[]>(() => Array.from({ length: n }, (_, k) => {
    const p = pool[k % pool.length];
    const d = Math.min(2, getState().topics[p.topic.id]?.difficulty ?? 0) as Difficulty;
    return { q: p.topic.generate(d), p };
  }));
  const [i, setI] = useState(0);
  const [answered, setAnswered] = useState(false);
  const [right, setRight] = useState(0);
  const [done, setDone] = useState(false);
  const cur = qs[i];
  const onDone = (g: Graded) => {
    setAnswered(true);
    if (g.correct) setRight((r) => r + 1);
    recordAnswer(cur.p.topic.id, g.correct, GEOMETRY_IDS);
    recordReview(cur.p.topic.id, g.correct);
    if (!g.correct) recordMistake({ topicId: cur.p.topic.id, topic: `${cur.p.subject.name} > ${cur.p.topic.title}`, prompt: cur.q.prompt, given: g.given, correct: correctAnswerText(cur.q), why: g.feedback, solution: cur.q.explanation });
  };
  const next = () => {
    if (i + 1 >= qs.length) { setDone(true); recordActivity('study'); addXp(right * 3 + 10); return; }
    setI(i + 1); setAnswered(false);
  };
  if (done) {
    const pct = Math.round((right / qs.length) * 100);
    return (
      <div className="card text-center space-y-3 animate-pop">
        <div className="text-5xl font-extrabold" style={{ color: gradeColor(pct) }}>{right}/{qs.length} · {letter(pct)}</div>
        <div className="muted">{pct >= 90 ? 'A-level work! Topics you got right come back in a few days so they stick.' : 'Every miss is scheduled to come back tomorrow — that repetition is what turns B\'s into A\'s.'}</div>
        <div className="text-sm">+{right * 3 + 10} XP</div>
        <button className="btn" onClick={onExit}>Done</button>
      </div>
    );
  }
  return (
    <div className="card">
      <div className="flex items-center justify-between mb-2"><div className="font-bold">{title}</div><button className="btn-ghost text-sm" onClick={onExit}>✕ Stop</button></div>
      <div className="flex gap-1 mb-3">{qs.map((_, k) => <div key={k} className={`h-2 flex-1 rounded ${k < i ? 'bg-accent' : k === i ? 'bg-edge' : 'bg-navy'}`} />)}</div>
      <div className="text-xs muted mb-2">{cur.p.subject.icon} {cur.p.subject.name} › {cur.p.topic.title}</div>
      <QuestionView key={i} q={cur.q} mode="practice" onDone={onDone} context={`${cur.p.subject.name} > ${cur.p.topic.title}`} />
      {answered && <button className="btn mt-4" onClick={next}>{i + 1 >= qs.length ? 'See results' : 'Next →'}</button>}
    </div>
  );
}

export function APlanPage() {
  usePage({ label: 'A+ Plan', subject: 'general' });
  const subjects = useSubjects();
  const grades = useApp((s) => s.grades);
  const tests = useApp((s) => s.tests);
  const review = useApp((s) => s.review);
  const topicsState = useApp((s) => s.topics);
  const [running, setRunning] = useState<{ pool: PoolItem[]; n: number; title: string } | null>(null);
  const today = dayKey();
  const pool = useMemo(() => reviewPool(subjects), [subjects, review, topicsState]); // eslint-disable-line react-hooks/exhaustive-deps
  const dueCount = Object.values(review).filter((r) => r.due <= today).length;
  const upcoming = [...tests].filter((t) => t.date >= today).sort((a, b) => a.date.localeCompare(b.date));
  const subjName = (id: string) => subjects.find((s) => s.id === id)?.name ?? id;

  const startReview = () => {
    let p = pool;
    if (!p.length) p = subjects.filter((s) => !s.advanced).flatMap((subject) => subject.topics.filter((t) => !topicsState[t.id]?.attempts).slice(0, 2).map((topic) => ({ topic, subject })));
    setRunning({ pool: p, n: 10, title: '🧠 Daily Review' });
  };
  const startTest = (t: UpcomingTest) => {
    const sub = subjects.find((s) => s.id === t.subjectId);
    if (!sub) return;
    const ids = t.topicIds.length ? t.topicIds : sub.topics.map((x) => x.id);
    const p = sub.topics.filter((x) => ids.includes(x.id)).map((topic) => ({ topic, subject: sub }));
    const acc = (id: string) => { const s = topicsState[id]; return s && s.attempts ? s.correct / s.attempts : 0; };
    p.sort((a, b) => acc(a.topic.id) - acc(b.topic.id)); // weakest first
    setRunning({ pool: p, n: 12, title: `📝 Prep: ${t.title}` });
  };

  if (running) return <div className="max-w-[900px]"><PageHeader title="🎯 A+ Plan" /><QuizRunner {...running} onExit={() => setRunning(null)} /></div>;

  const missions = [
    { done: false, text: dueCount ? `Daily Review — ${dueCount} topic${dueCount > 1 ? 's' : ''} due today (10 questions, ~8 min)` : 'Daily Review — 10 mixed questions (~8 min)', action: startReview },
    ...upcoming.slice(0, 2).map((t) => ({ done: false, text: `Prep for ${t.title} (${subjName(t.subjectId)}) — ${daysBetween(today, t.date) === 0 ? 'TODAY' : `in ${daysBetween(today, t.date)} day${daysBetween(today, t.date) > 1 ? 's' : ''}`}`, action: () => startTest(t) })),
    ...grades.filter((g) => g.grade !== null && g.grade < 90 && g.subjectId).slice(0, 2).map((g) => ({ done: false, text: `Boost ${g.name} (${g.grade}%) — practice its weakest topic`, action: () => { const sub = subjects.find((s) => s.id === g.subjectId); if (sub) setRunning({ pool: reviewPool(subjects, [sub.id]).length ? reviewPool(subjects, [sub.id]) : sub.topics.map((topic) => ({ topic, subject: sub })), n: 10, title: `📈 Boost ${g.name}` }); } })),
  ];

  return (
    <div className="space-y-6 max-w-[1100px]">
      <PageHeader title="🎯 A+ Plan" sub="Your system for straight A's: review a little every day, prep early for tests, and track your real grades." />

      <div className="card">
        <div className="h2 mb-3">✅ Today's missions</div>
        <div className="space-y-2">
          {missions.map((m, k) => (
            <button key={k} onClick={m.action} className="w-full text-left flex items-center gap-3 rounded-xl border border-edge/40 hover:border-edge bg-navy px-4 py-3">
              <span className="text-xl">{k === 0 ? '🧠' : '🎯'}</span><span className="flex-1">{m.text}</span><span className="text-edge font-bold">Start →</span>
            </button>
          ))}
        </div>
        <div className="text-xs muted mt-3">Why this works: quizzing yourself (retrieval practice) and spacing reviews over days are the two study habits with the strongest research behind them. The app schedules it for you — right answers come back in 2, 4, 8, 16 days; misses come back tomorrow.</div>
      </div>

      <TestsCard subjects={subjects} tests={tests} onPrep={startTest} />
      <GradesCard subjects={subjects} grades={grades} />
      <TipsCard />
    </div>
  );
}

function TestsCard({ subjects, tests, onPrep }: { subjects: Subject[]; tests: UpcomingTest[]; onPrep: (t: UpcomingTest) => void }) {
  const today = dayKey();
  const [subjectId, setSubjectId] = useState(subjects[0]?.id ?? '');
  const [title, setTitle] = useState('');
  const [date, setDate] = useState('');
  const [picked, setPicked] = useState<string[]>([]);
  const sub = subjects.find((s) => s.id === subjectId);
  const add = () => {
    if (!date || !subjectId) return;
    setTests([...tests, { id: uid(), subjectId, title: title.trim() || `${sub?.name} test`, date, topicIds: picked }]);
    setTitle(''); setDate(''); setPicked([]);
  };
  const sorted = [...tests].sort((a, b) => a.date.localeCompare(b.date));
  return (
    <div className="card">
      <div className="h2 mb-3">📅 Upcoming tests & quizzes</div>
      {sorted.length === 0 && <div className="muted text-sm mb-3">Add your next test — the app builds a day-by-day plan and a practice test from exactly the topics it covers.</div>}
      <div className="space-y-3 mb-4">
        {sorted.map((t) => {
          const days = daysBetween(today, t.date);
          const s = subjects.find((x) => x.id === t.subjectId);
          const ts = s ? s.topics.filter((x) => !t.topicIds.length || t.topicIds.includes(x.id)) : [];
          const plan = days > 0 ? Array.from({ length: Math.min(days, 7) }, (_, d) => {
            const per = Math.max(1, Math.ceil(ts.length / Math.max(1, Math.min(days, 7) - 1)));
            const chunk = d === Math.min(days, 7) - 1 ? 'Full practice test of everything' : ts.slice(d * per, d * per + per).map((x) => x.title).join(', ') || 'Mixed review';
            return `${parseDay(addDays(t.date, d - Math.min(days, 7))).toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })}: ${chunk}`;
          }) : [];
          return (
            <div key={t.id} className={`rounded-xl border p-3 ${days < 0 ? 'opacity-50 border-edge/20' : days <= 2 ? 'border-bad/70' : 'border-edge/40'}`}>
              <div className="flex items-center gap-3">
                <div className="text-center w-16"><div className="text-2xl font-extrabold" style={{ color: days <= 2 ? '#F87171' : '#7FD3FF' }}>{days < 0 ? '✓' : days}</div><div className="text-[10px] muted">{days < 0 ? 'past' : days === 1 ? 'day' : 'days'}</div></div>
                <div className="flex-1"><div className="font-bold">{t.title}</div><div className="text-xs muted">{s?.icon} {s?.name} · {parseDay(t.date).toLocaleDateString(undefined, { weekday: 'long', month: 'short', day: 'numeric' })} · {ts.length} topic{ts.length === 1 ? '' : 's'}</div></div>
                {days >= 0 && <button className="btn" onClick={() => onPrep(t)}>Practice test</button>}
                <button className="btn-ghost text-sm" onClick={() => setTests(tests.filter((x) => x.id !== t.id))}>✕</button>
              </div>
              {plan.length > 0 && <details className="mt-2 text-sm"><summary className="cursor-pointer text-edge">Study plan until test day</summary><ol className="list-decimal pl-5 mt-1 space-y-0.5">{plan.map((p, k) => <li key={k}>{p}</li>)}<li><b>Test day:</b> quick 5-minute warm-up, good breakfast, you've got this 💪</li></ol></details>}
            </div>
          );
        })}
      </div>
      <div className="grid grid-cols-[1fr_1.4fr_auto_auto] gap-2 items-end">
        <label className="text-sm"><span className="muted">Subject</span><select className="input w-full" value={subjectId} onChange={(e) => { setSubjectId(e.target.value); setPicked([]); }}>{subjects.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}</select></label>
        <label className="text-sm"><span className="muted">Name</span><input className="input w-full" placeholder="e.g. Unit 3 test" value={title} onChange={(e) => setTitle(e.target.value)} /></label>
        <label className="text-sm"><span className="muted">Date</span><input type="date" className="input" min={today} value={date} onChange={(e) => setDate(e.target.value)} /></label>
        <button className="btn" onClick={add} disabled={!date}>+ Add test</button>
      </div>
      {sub && <details className="mt-2 text-sm"><summary className="cursor-pointer muted">Which topics are on it? (optional — leave blank for all {sub.topics.length})</summary>
        <div className="grid grid-cols-2 gap-1 mt-2">{sub.topics.map((t) => <label key={t.id} className="flex items-center gap-2"><input type="checkbox" checked={picked.includes(t.id)} onChange={(e) => setPicked(e.target.checked ? [...picked, t.id] : picked.filter((x) => x !== t.id))} />{t.title}</label>)}</div>
      </details>}
    </div>
  );
}

function GradesCard({ subjects, grades }: { subjects: Subject[]; grades: GradeEntry[] }) {
  const [calc, setCalc] = useState({ current: '', weight: '20', target: '90' });
  const edit = (id: string, p: Partial<GradeEntry>) => setGrades(grades.map((g) => (g.id === id ? { ...g, ...p } : g)));
  const known = grades.filter((g) => g.grade !== null) as (GradeEntry & { grade: number })[];
  const gpa = known.length ? known.reduce((a, g) => a + (g.grade >= 90 ? 4 : g.grade >= 80 ? 3 : g.grade >= 70 ? 2 : g.grade >= 60 ? 1 : 0), 0) / known.length : null;
  const cur = parseFloat(calc.current), w = parseFloat(calc.weight) / 100, tgt = parseFloat(calc.target);
  const need = Number.isFinite(cur) && w > 0 && w <= 1 && Number.isFinite(tgt) ? (tgt - cur * (1 - w)) / w : null;
  return (
    <div className="card">
      <div className="flex items-center justify-between mb-3"><div className="h2">📊 My grades</div>{gpa !== null && <div className="text-sm">GPA: <b style={{ color: gpa >= 3.9 ? '#4ADE80' : '#7FD3FF' }}>{gpa.toFixed(2)}</b> {known.every((g) => g.grade >= 90) && known.length > 0 && '· 🏆 Straight A\'s!'}</div>}</div>
      <div className="space-y-2">
        {grades.map((g) => (
          <div key={g.id} className="flex items-center gap-3">
            <input className="input flex-1" value={g.name} onChange={(e) => edit(g.id, { name: e.target.value })} />
            <select className="input w-44" value={g.subjectId} onChange={(e) => edit(g.id, { subjectId: e.target.value })}><option value="">(no app subject)</option>{subjects.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}</select>
            <input className="input w-20 text-right" type="number" min={0} max={110} placeholder="%" value={g.grade ?? ''} onChange={(e) => edit(g.id, { grade: e.target.value === '' ? null : Number(e.target.value) })} />
            <div className="w-12 text-center font-extrabold text-lg" style={{ color: g.grade !== null ? gradeColor(g.grade) : undefined }}>{g.grade !== null ? letter(g.grade) : '—'}</div>
            <div className="w-40">{g.grade !== null && <ProgressBar value={Math.min(g.grade, 100)} color={gradeColor(g.grade)} />}</div>
            {g.subjectId && <Link to={`/study/${g.subjectId}`} className="btn-ghost text-sm">Study</Link>}
            <button className="btn-ghost text-sm" onClick={() => setGrades(grades.filter((x) => x.id !== g.id))}>✕</button>
          </div>
        ))}
      </div>
      <button className="btn-ghost mt-3" onClick={() => setGrades([...grades, { id: uid(), name: `Class ${grades.length + 1}`, subjectId: '', grade: null }])}>+ Add class</button>

      <div className="mt-5 rounded-xl bg-navy border border-edge/30 p-4">
        <div className="font-bold mb-2">🧮 What do I need on the next test?</div>
        <div className="flex flex-wrap items-end gap-3 text-sm">
          <label><span className="muted block">Current grade %</span><input className="input w-24" type="number" value={calc.current} onChange={(e) => setCalc({ ...calc, current: e.target.value })} /></label>
          <label><span className="muted block">Test counts for (% of grade)</span><input className="input w-24" type="number" value={calc.weight} onChange={(e) => setCalc({ ...calc, weight: e.target.value })} /></label>
          <label><span className="muted block">Goal %</span><input className="input w-24" type="number" value={calc.target} onChange={(e) => setCalc({ ...calc, target: e.target.value })} /></label>
          {need !== null && <div className="text-base">→ You need <b style={{ color: need <= 100 ? '#4ADE80' : '#F87171' }}>{Math.max(0, Math.ceil(need))}%</b> {need > 100 ? '(not possible from this test alone — ask about extra credit or retakes!)' : need <= 0 ? '(you already have it — just don\'t skip it!)' : 'on the test.'}</div>}
        </div>
      </div>
    </div>
  );
}

function TipsCard() {
  const tips = [
    ['🧠', 'Quiz, don\'t reread', 'Rereading notes feels productive but barely helps. Testing yourself (Practice tab, flashcards, Daily Review) is what makes it stick.'],
    ['📆', 'Space it out', '20 minutes a day for 5 days beats 2 hours the night before. Start test prep as soon as a test is announced.'],
    ['❌', 'Mine your mistakes', 'Every wrong answer goes to My Mistakes with an explanation. Redo them until they\'re right — that\'s where the points are.'],
    ['🗣️', 'Teach it back', 'Explain a topic out loud like you\'re the teacher. If you get stuck, that\'s exactly what to study.'],
    ['📝', 'Turn everything in', 'Homework and missing assignments are the easiest points in any class. Check your grade portal every week.'],
    ['😴', 'Sleep = memory', 'Your brain saves what you learned while you sleep. An all-nighter erases the benefit of studying.'],
    ['🙋', 'Ask early', 'If you\'re confused, ask your teacher the same week — not the day before the test. Teachers notice kids who ask.'],
    ['⏱️', 'Focus sprints', 'Phone in another room + the Focus Timer (25 min on, 5 off). Multitasking cuts learning roughly in half.'],
  ];
  return (
    <div className="card">
      <div className="h2 mb-3">🏆 Straight-A habits</div>
      <div className="grid grid-cols-2 gap-3">
        {tips.map(([i, t, d]) => <div key={t} className="flex gap-3"><div className="text-2xl">{i}</div><div><div className="font-bold">{t}</div><div className="text-sm muted">{d}</div></div></div>)}
      </div>
    </div>
  );
}
