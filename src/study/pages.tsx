import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useNavigate, useParams } from 'react-router-dom';
import { useApp, recordMistake, recordAnswer, recordActivity, setTopicDifficulty, topicStats, DIFF_NAMES, Difficulty, setNote, celebrate, openChat } from '../lib/store';
import { useActiveMinutes, usePage } from '../lib/hooks';
import { getSubjects, findTopic } from './subjects';
import { GEOMETRY_IDS, GEOMETRY_TOPICS } from './geometry';
import { QuestionView, Graded, correctAnswerText } from './QuestionView';
import { PageHeader, ProgressBar, Rich, Tabs, Modal } from '../components/ui';
import type { Question, Subject } from './types';
import { FlashcardDeck } from './tools';

export function useSubjects(): Subject[] {
  const lang = useApp((s) => s.settings.language);
  return useMemo(() => getSubjects(lang), [lang]);
}

export function masteryLabel(id: string, topics: Record<string, { masteredLevel: number }>) {
  const m = topics[id]?.masteredLevel ?? -1;
  return m < 0 ? null : `Mastered: ${DIFF_NAMES[m]}`;
}

export function FormulaSheet({ open, onClose }: { open: boolean; onClose: () => void }) {
  return (
    <Modal open={open} onClose={onClose} title="📐 Geometry Formula Sheet" wide>
      <div className="grid grid-cols-2 gap-4">
        {GEOMETRY_TOPICS.filter((t) => t.formulas?.length).map((t) => (
          <div key={t.id} className="card py-3">
            <div className="font-bold text-edge mb-1 text-sm">{t.title}</div>
            {t.formulas!.map((f) => (
              <div key={f.name} className="flex justify-between gap-3 text-sm py-0.5"><span className="muted">{f.name}</span><span className="font-mono">{f.f}</span></div>
            ))}
          </div>
        ))}
      </div>
    </Modal>
  );
}

export function StudyHome() {
  const subjects = useSubjects();
  const topics = useApp((s) => s.topics);
  const [formulas, setFormulas] = useState(false);
  usePage({ label: 'Study Zone', subject: 'general' });
  const weak = Object.entries(topics).filter(([, t]) => t.attempts >= 5 && t.correct / t.attempts < 0.7).map(([id, t]) => ({ id, t, f: findTopic(subjects, id) })).filter((x) => x.f);
  return (
    <div>
      <PageHeader title="Study Zone" sub="Learn it, practice it, master it. The app teaches — you do the solving." right={<button className="btn-ghost" onClick={() => setFormulas(true)}>📐 Formula sheet</button>} />
      <div className="grid grid-cols-3 gap-4 mb-8">
        {subjects.filter((s) => !s.advanced).map((s) => {
          const mastered = s.topics.filter((t) => (topics[t.id]?.masteredLevel ?? -1) >= 0).length;
          return (
            <Link key={s.id} to={`/study/${s.id}`} className="card hover:-translate-y-0.5 transition">
              <div className="text-4xl mb-2">{s.icon}</div>
              <div className="h2" style={{ color: s.color }}>{s.name}</div>
              <div className="muted text-sm mb-3">{s.blurb}</div>
              <ProgressBar value={mastered} max={s.topics.length} color={s.color} />
              <div className="text-xs muted mt-1">{mastered}/{s.topics.length} topics mastered</div>
            </Link>
          );
        })}
      </div>
      <h2 className="h2 mb-3">Study tools</h2>
      <div className="grid grid-cols-6 gap-3 mb-8">
        {[['/study/mistakes', '📖', 'My Mistakes'], ['/study/flashcards', '🃏', 'Flashcards'], ['/study/quiz', '📝', 'Practice Quiz'], ['/study/timer', '🍅', 'Focus Timer'], ['/study/notes', '🗒️', 'Notes'], ['/study/essay', '✍️', 'Essay Coach']].map(([to, i, l]) => (
          <Link key={to} to={to} className="card text-center py-4"><div className="text-3xl">{i}</div><div className="font-bold mt-1">{l}</div></Link>
        ))}
      </div>
      {weak.length > 0 && (
        <>
          <h2 className="h2 mb-3">Weak spots to review</h2>
          <div className="grid grid-cols-3 gap-3">
            {weak.map(({ id, t, f }) => (
              <Link key={id} to={`/study/${f!.subject.id}/${id}`} className="card py-3 border-bad/50">
                <div className="font-bold">{f!.topic.title}</div>
                <div className="text-sm text-bad">{Math.round((t.correct / t.attempts) * 100)}% accuracy</div>
              </Link>
            ))}
          </div>
        </>
      )}
      <FormulaSheet open={formulas} onClose={() => setFormulas(false)} />
    </div>
  );
}

export function SubjectPage() {
  const { subjectId } = useParams();
  const subjects = useSubjects();
  const topics = useApp((s) => s.topics);
  const unitsBest = useApp((s) => s.studyUnits);
  const subject = subjects.find((s) => s.id === subjectId);
  usePage({ label: `${subject?.name ?? 'Study'}`, subject: subjectId ?? 'general' });
  if (!subject) return <div>Subject not found. <Link to="/study" className="text-edge">Back</Link></div>;
  const units = subject.units?.length ? subject.units : [{ title: 'All Topics', topicIds: subject.topics.map((t) => t.id) }];
  const byId = new Map(subject.topics.map((t) => [t.id, t]));
  const isMastered = (id: string) => (topics[id]?.masteredLevel ?? -1) >= 0;
  const done = subject.topics.filter((t) => isMastered(t.id)).length;
  const nextUp = subject.topics.find((t) => !isMastered(t.id))?.id;
  const finalBest = unitsBest[`${subject.id}:final`];
  let n = 0;
  return (
    <div>
      <PageHeader title={<span>{subject.icon} {subject.name}</span>} sub={subject.blurb} right={<Link to={subject.advanced ? '/advanced' : '/study'} className="btn-ghost">← {subject.advanced ? 'Super Advanced' : 'Study Zone'}</Link>} />
      <div className="card mb-4 flex items-center gap-6">
        <div className="flex-1">
          <div className="font-bold mb-1">Course progress: {done}/{subject.topics.length} topics mastered</div>
          <ProgressBar value={done} max={Math.max(1, subject.topics.length)} />
        </div>
        <Link to={`/study/quiz?subject=${subject.id}&final=1`} className="btn">🏆 Final Exam{finalBest !== undefined ? ` · best ${finalBest}%` : ''}</Link>
      </div>
      <RandomQuestion subject={subject} />
      {units.map((u, k) => {
        const ts = u.topicIds.map((id) => byId.get(id)).filter(Boolean) as Subject['topics'];
        const m = ts.filter((t) => isMastered(t.id)).length;
        const best = unitsBest[`${subject.id}:${k}`];
        return (
          <div key={k} className="mb-6">
            <div className="flex items-center gap-3 mb-2">
              <h2 className="h2 flex-1">Unit {k + 1}: {u.title} <span className="text-sm muted font-normal">· {m}/{ts.length} mastered</span></h2>
              {subject.units?.length ? <Link to={`/study/quiz?subject=${subject.id}&unit=${k}`} className="btn-ghost">🎓 Unit test{best !== undefined ? ` · best ${best}/10${best >= 8 ? ' ✓' : ''}` : ''}</Link> : null}
            </div>
            <div className="grid grid-cols-2 gap-3">
              {ts.map((t) => {
                n++;
                const st = topics[t.id];
                const acc = st && st.attempts ? Math.round((st.correct / st.attempts) * 100) : null;
                const ml = masteryLabel(t.id, topics);
                return (
                  <Link key={t.id} to={`/study/${subject.id}/${t.id}`} className={`card flex items-center gap-4 py-4 ${t.id === nextUp ? 'border-edge ring-2 ring-edge/40' : ''}`}>
                    <div className="w-10 h-10 rounded-full bg-card2 border border-edge/50 flex items-center justify-center font-extrabold text-edge">{n}</div>
                    <div className="flex-1">
                      <div className="font-bold">{t.title}</div>
                      <div className="text-xs muted">{t.id === nextUp ? '👉 Next up · ' : ''}{acc !== null ? `${acc}% accuracy · ${st!.attempts} answered` : 'Not started'}</div>
                    </div>
                    {ml && <span className="chip border-good text-good">✓ {ml}</span>}
                  </Link>
                );
              })}
            </div>
          </div>
        );
      })}
    </div>
  );
}

/** Random question generator — pulls a fresh generated question from any topic in the subject. No internet or API key needed. */
function RandomQuestion({ subject }: { subject: Subject }) {
  const [open, setOpen] = useState(false);
  const [pick, setPick] = useState<{ q: Question; title: string; id: string } | null>(null);
  const [answered, setAnswered] = useState(false);
  const [count, setCount] = useState({ right: 0, total: 0 });
  const roll = () => {
    const t = subject.topics[Math.floor(Math.random() * subject.topics.length)];
    setPick({ q: t.generate(Math.floor(Math.random() * 3) as Difficulty), title: t.title, id: t.id });
    setAnswered(false); setOpen(true);
  };
  const onDone = (g: Graded) => {
    setAnswered(true);
    setCount((c) => ({ right: c.right + (g.correct ? 1 : 0), total: c.total + 1 }));
    if (!pick) return;
    recordAnswer(pick.id, g.correct, GEOMETRY_IDS);
    if (!g.correct) recordMistake({ topicId: pick.id, topic: `${subject.name} > ${pick.title}`, prompt: pick.q.prompt, given: g.given, correct: correctAnswerText(pick.q), why: g.feedback, solution: pick.q.explanation });
    recordActivity('study');
  };
  return (
    <div className="card mb-6">
      <div className="flex items-center gap-3">
        <div className="flex-1"><div className="font-bold">🎲 Random Question Generator</div><div className="text-xs muted">A brand-new question from any {subject.name} topic, every time.{count.total ? ` Session: ${count.right}/${count.total}` : ''}</div></div>
        <button className="btn" onClick={roll}>{open ? '🎲 New random question' : '🎲 Give me a question'}</button>
      </div>
      {open && pick && (
        <div className="mt-4">
          <div className="text-xs muted mb-2">From: <b>{pick.title}</b></div>
          <QuestionView key={pick.q.prompt + count.total} q={pick.q} mode="practice" onDone={onDone} context={`${subject.name} > ${pick.title}`} />
          {answered && <button className="btn mt-4" onClick={roll}>Next random question →</button>}
        </div>
      )}
    </div>
  );
}

type TopicTab = 'lesson' | 'example' | 'practice' | 'cards' | 'notes';
export function TopicPage() {
  const { subjectId, topicId } = useParams();
  const subjects = useSubjects();
  const found = findTopic(subjects, topicId ?? '');
  const stats = useApp((s) => s.topics[topicId ?? '']);
  const notes = useApp((s) => s.notes[`${subjectId}/${topicId}`] ?? '');
  const [tab, setTab] = useState<TopicTab>('lesson');
  const [q, setQ] = useState<Question | null>(null);
  const [answered, setAnswered] = useState(false);
  const [sessionCount, setSessionCount] = useState(0);
  const [formulas, setFormulas] = useState(false);
  const nav = useNavigate();
  useActiveMinutes('study');
  const label = found ? `${found.subject.name} > ${found.subject.id === 'geometry' ? `Unit ${found.index + 1}: ` : ''}${found.topic.title}` : 'Study';
  usePage({ label, detail: tab === 'practice' && q ? `Practice problem: ${q.prompt}` : `Viewing ${tab}`, subject: subjectId ?? 'general' }, { kind: 'study', path: `/study/${subjectId}/${topicId}` });

  const st = stats ?? topicStats(topicId ?? '');
  const diff = st.difficulty;
  const newQ = () => { if (found) { setQ(found.topic.generate(diff)); setAnswered(false); } };
  useEffect(() => { setQ(null); setSessionCount(0); setTab('lesson'); }, [topicId]);
  useEffect(() => { if (tab === 'practice' && found) newQ(); }, [tab, diff, topicId]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!found) return <div>Topic not found.</div>;
  const { subject, topic, index } = found;

  const onDone = (g: Graded) => {
    setAnswered(true);
    const mastered = recordAnswer(topic.id, g.correct, GEOMETRY_IDS);
    if (!g.correct && q) recordMistake({ topicId: topic.id, topic: label, prompt: q.prompt, given: g.given, correct: correctAnswerText(q), why: g.feedback, solution: q.explanation });
    if (mastered) {
      celebrate(`${DIFF_NAMES[diff]} mastered!`, '🏅', diff < 3 ? `${DIFF_NAMES[diff + 1]} questions are now unlocked.` : 'You conquered the Challenge level!');
      if (diff < 3) setTopicDifficulty(topic.id, (diff + 1) as Difficulty);
    }
    const c = sessionCount + 1;
    setSessionCount(c);
    if (c === 5) recordActivity('study');
  };

  const prev = subject.topics[index - 1], next = subject.topics[index + 1];
  return (
    <div>
      <PageHeader
        title={topic.title}
        sub={<span>{subject.icon} {subject.name}{subject.id === 'geometry' ? ` · Unit ${index + 1}` : ''}</span>}
        right={<div className="flex gap-2">
          {subject.id === 'geometry' && <button className="btn-ghost" onClick={() => setFormulas(true)}>📐 Formulas</button>}
          <Link to={`/study/${subject.id}`} className="btn-ghost">← {subject.name}</Link>
        </div>}
      />
      <div className="flex items-center justify-between mb-4">
        <Tabs<TopicTab> value={tab} onChange={setTab} tabs={[
          { id: 'lesson', label: '1. Lesson' }, { id: 'example', label: '2. Worked Example' }, { id: 'practice', label: '3. Practice' },
          { id: 'cards', label: 'Flashcards' }, { id: 'notes', label: 'Notes' },
        ]} />
        <div className="flex items-center gap-3 text-sm">
          <span className="muted">In a row:</span>
          <div className="flex gap-1">{[0, 1, 2, 3, 4].map((i) => <div key={i} className={`w-4 h-4 rounded-full border border-edge/50 ${i < st.inARow ? 'bg-good shadow-[0_0_8px_#4ADE80]' : ''}`} />)}</div>
          {st.masteredLevel >= 0 && <span className="chip border-good text-good">✓ Mastered {DIFF_NAMES[st.masteredLevel]}</span>}
        </div>
      </div>

      {tab === 'lesson' && (
        <div className="card flex gap-8">
          <div className="flex-1"><Rich text={topic.lesson} className="text-[17px]" /></div>
          {topic.lessonDiagram && <div className="w-[400px] shrink-0">{topic.lessonDiagram}</div>}
        </div>
      )}
      {tab === 'lesson' && <div className="mt-4 flex gap-2"><button className="btn" onClick={() => setTab('example')}>Next: Worked Example →</button><button className="btn-ghost" onClick={() => openChat()}>🤖 Explain it differently</button></div>}

      {tab === 'example' && (
        <div className="card">
          <div className="text-lg font-bold mb-3">📘 {topic.example.problem}</div>
          <div className="flex gap-8">
            <ol className="flex-1 space-y-3">
              {topic.example.steps.map((s, i) => (
                <li key={i} className="flex gap-3">
                  <div className="w-8 h-8 rounded-full bg-accent flex items-center justify-center font-bold shrink-0">{i + 1}</div>
                  <div><div className="font-semibold">{s.step}</div><div className="muted text-sm">Why: {s.why}</div></div>
                </li>
              ))}
            </ol>
            {topic.example.diagram && <div className="w-[380px] shrink-0">{topic.example.diagram}</div>}
          </div>
          <button className="btn mt-5" onClick={() => setTab('practice')}>Now you try →</button>
        </div>
      )}

      {tab === 'practice' && (
        <div className="card">
          <div className="flex items-center justify-between mb-4">
            <div className="flex gap-1">
              {DIFF_NAMES.map((n, i) => {
                const unlocked = i === 0 || st.masteredLevel >= i - 1;
                return (
                  <button key={n} disabled={!unlocked} onClick={() => setTopicDifficulty(topic.id, i as Difficulty)}
                    className={`px-3 py-1 rounded-lg text-sm font-bold border ${diff === i ? 'bg-accent border-accent' : 'border-edge/40'} disabled:opacity-40`}>
                    {unlocked ? '' : '🔒 '}{n}
                  </button>
                );
              })}
            </div>
            <div className="text-sm muted">{st.attempts ? `${Math.round((st.correct / st.attempts) * 100)}% accuracy · ` : ''}Get 5 in a row to master {DIFF_NAMES[diff]}</div>
          </div>
          {q && <QuestionView q={q} mode="practice" onDone={onDone} context={label} />}
          {answered && <button className="btn mt-4" onClick={newQ}>Next question →</button>}
        </div>
      )}

      {tab === 'cards' && <FlashcardDeck cards={topic.vocab.map((v) => ({ front: v.term, back: v.def }))} title={`${topic.title} vocabulary`} />}

      {tab === 'notes' && (
        <div className="card">
          <div className="muted text-sm mb-2">Your notes for {topic.title} (saved automatically)</div>
          <textarea className="input w-full h-80 font-mono text-sm" value={notes} onChange={(e) => setNote(`${subjectId}/${topicId}`, e.target.value)} placeholder="Write key ideas, formulas, and questions to ask your teacher…" />
        </div>
      )}

      <div className="flex justify-between mt-6">
        {prev ? <button className="btn-ghost" onClick={() => nav(`/study/${subject.id}/${prev.id}`)}>← {prev.title}</button> : <span />}
        {next ? <button className="btn-ghost" onClick={() => nav(`/study/${subject.id}/${next.id}`)}>{next.title} →</button> : <span />}
      </div>
      <FormulaSheet open={formulas} onClose={() => setFormulas(false)} />
    </div>
  );
}

// re-export for convenience
export { useRef };

/** Super Advanced: everything one grade ahead of the normal Study Zone. */
export function AdvancedHome() {
  const subjects = useSubjects().filter((s) => s.advanced);
  const topics = useApp((s) => s.topics);
  usePage({ label: 'Super Advanced (9th grade level)', subject: 'general' });
  return (
    <div>
      <PageHeader title="🚀 Super Advanced" sub="One grade ahead: 9th grade material. Same learn, practice, master flow. Challenge yourself!" />
      <div className="card2 mb-6 flex items-center gap-4 border-streak/60">
        <div className="text-4xl">🔥</div>
        <div className="text-sm"><b>Heads up:</b> these topics are harder than your grade level. It is totally fine to get things wrong here. Use hints, the worked examples, and Study Buddy. Mastering a topic still earns XP and badges.</div>
      </div>
      <div className="grid grid-cols-3 gap-4">
        {subjects.map((s) => {
          const mastered = s.topics.filter((t) => (topics[t.id]?.masteredLevel ?? -1) >= 0).length;
          return (
            <Link key={s.id} to={`/study/${s.id}`} className="card hover:-translate-y-0.5 transition">
              <div className="flex justify-between"><div className="text-4xl mb-2">{s.icon}</div><span className="chip border-streak text-streak h-fit">GRADE 9</span></div>
              <div className="h2" style={{ color: s.color }}>{s.name}</div>
              <div className="muted text-sm mb-3">{s.blurb}</div>
              <ProgressBar value={mastered} max={s.topics.length} color={s.color} />
              <div className="text-xs muted mt-1">{mastered}/{s.topics.length} topics mastered</div>
            </Link>
          );
        })}
      </div>
    </div>
  );
}
