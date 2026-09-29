import { ask } from '../lib/embed';
import { useEffect, useMemo, useRef, useState } from 'react';
import { Link, useSearchParams } from 'react-router-dom';
import { recordUnitTest10, recordFinalExam, setMistakes, useApp, recordMistake, setDecks, recordQuiz, recordActivity, addFocusRound, addMinutes, setNote, openChat, getState } from '../lib/store';
import { usePage } from '../lib/hooks';
import { PageHeader, Tabs, Rich } from '../components/ui';
import { QuestionView, Graded, correctAnswerText } from './QuestionView';
import type { Question, Subject } from './types';
import { useSubjects } from './pages';
import { shuffle } from './rand';
import { beep } from '../piano/audio';

// ---------------- Flashcards ----------------
export function FlashcardDeck({ cards, title }: { cards: { front: string; back: string }[]; title: string }) {
  const [queue, setQueue] = useState(() => cards.map((_, i) => i));
  const [flipped, setFlipped] = useState(false);
  const [known, setKnown] = useState(0);
  const [missed, setMissed] = useState(0);
  const [reverse, setReverse] = useState(false);
  useEffect(() => { setQueue(cards.map((_, i) => i)); setKnown(0); setMissed(0); setFlipped(false); }, [cards]);
  if (!cards.length) return <div className="card muted">This deck has no cards yet.</div>;
  const cur = queue[0];
  const card = cur !== undefined ? cards[cur] : null;
  const answer = (knew: boolean) => {
    setFlipped(false);
    setTimeout(() => {
      if (knew) { setKnown((k) => k + 1); setQueue((q) => q.slice(1)); }
      else { setMissed((m) => m + 1); setQueue((q) => [...q.slice(1), q[0]]); } // missed cards come back
    }, 150);
  };
  return (
    <div className="card">
      <div className="flex justify-between items-center mb-4">
        <div className="font-bold">{title}</div>
        <div className="flex gap-2 items-center text-sm">
          <span className="text-good">✓ {known}</span><span className="text-bad">↻ {missed}</span><span className="muted">{queue.length} left</span>
          <button className="btn-ghost py-1" onClick={() => { setQueue(shuffle(queue)); setFlipped(false); }}>🔀 Shuffle</button>
          <button className="btn-ghost py-1" onClick={() => setReverse((r) => !r)}>{reverse ? 'Definition first' : 'Term first'}</button>
          <button className="btn-ghost py-1" onClick={() => { setQueue(cards.map((_, i) => i)); setKnown(0); setMissed(0); }}>Restart</button>
        </div>
      </div>
      {card ? (
        <>
          <div className="mx-auto w-[560px] h-[280px] cursor-pointer [perspective:1200px]" onClick={() => setFlipped((f) => !f)}>
            <div className="relative w-full h-full transition-transform duration-500 [transform-style:preserve-3d]" style={{ transform: flipped ? 'rotateY(180deg)' : undefined }}>
              <div className="absolute inset-0 card2 flex items-center justify-center text-center text-2xl font-bold [backface-visibility:hidden] border-2 border-edge">
                {reverse ? card.back : card.front}
              </div>
              <div className="absolute inset-0 card2 flex items-center justify-center text-center text-xl [backface-visibility:hidden] [transform:rotateY(180deg)] border-2 border-accent">
                {reverse ? card.front : card.back}
              </div>
            </div>
          </div>
          <div className="text-center muted text-sm mt-2">Click the card to flip it</div>
          <div className="flex justify-center gap-3 mt-4">
            <button className="btn-ghost border-bad text-bad" onClick={() => answer(false)}>✗ I didn't know it</button>
            <button className="btn bg-good text-navy" onClick={() => answer(true)}>✓ I knew it</button>
          </div>
        </>
      ) : (
        <div className="text-center py-10 animate-pop">
          <div className="text-5xl mb-2">🎉</div>
          <div className="h2">Deck complete!</div>
          <div className="muted">You needed {missed} extra review{missed === 1 ? '' : 's'}.</div>
        </div>
      )}
    </div>
  );
}

export function FlashcardsPage() {
  const subjects = useSubjects();
  const decks = useApp((s) => s.decks);
  const [source, setSource] = useState<string>('');
  const [editing, setEditing] = useState<string | null>(null);
  usePage({ label: 'Flashcards', subject: 'general' });

  const autoDecks = useMemo(() => subjects.flatMap((s) => s.topics.map((t) => ({ id: `auto:${t.id}`, name: `${s.icon} ${t.title}`, cards: t.vocab.map((v) => ({ front: v.term, back: v.def })) }))), [subjects]);
  const all = [...decks.map((d) => ({ ...d, name: `⭐ ${d.name}` })), ...autoDecks];
  const deck = all.find((d) => d.id === source);
  const editDeck = decks.find((d) => d.id === editing);

  return (
    <div>
      <PageHeader title="Flashcards" sub="Auto-made decks for every topic, plus your own." right={
        <button className="btn" onClick={() => { const id = `deck-${Date.now()}`; setDecks([...decks, { id, name: 'My new deck', cards: [] }]); setEditing(id); }}>+ New deck</button>
      } />
      <div className="flex gap-3 mb-4">
        <select className="input w-[420px]" value={source} onChange={(e) => { setSource(e.target.value); setEditing(null); }}>
          <option value="">— Choose a deck —</option>
          {decks.length > 0 && <optgroup label="My decks">{decks.map((d) => <option key={d.id} value={d.id}>{d.name} ({d.cards.length})</option>)}</optgroup>}
          {subjects.map((s) => <optgroup key={s.id} label={s.name}>{s.topics.map((t) => <option key={t.id} value={`auto:${t.id}`}>{t.title} ({t.vocab.length})</option>)}</optgroup>)}
        </select>
        {deck && decks.some((d) => d.id === deck.id) && <button className="btn-ghost" onClick={() => setEditing(deck.id)}>✏️ Edit deck</button>}
      </div>
      {editDeck ? (
        <div className="card space-y-3">
          <input className="input w-80 font-bold" value={editDeck.name} onChange={(e) => setDecks(decks.map((d) => (d.id === editDeck.id ? { ...d, name: e.target.value } : d)))} />
          {editDeck.cards.map((c, i) => (
            <div key={i} className="flex gap-2">
              <input className="input flex-1" placeholder="Front" value={c.front} onChange={(e) => setDecks(decks.map((d) => (d.id === editDeck.id ? { ...d, cards: d.cards.map((x, j) => (j === i ? { ...x, front: e.target.value } : x)) } : d)))} />
              <input className="input flex-[2]" placeholder="Back" value={c.back} onChange={(e) => setDecks(decks.map((d) => (d.id === editDeck.id ? { ...d, cards: d.cards.map((x, j) => (j === i ? { ...x, back: e.target.value } : x)) } : d)))} />
              <button className="btn-ghost" onClick={() => setDecks(decks.map((d) => (d.id === editDeck.id ? { ...d, cards: d.cards.filter((_, j) => j !== i) } : d)))}>🗑</button>
            </div>
          ))}
          <div className="flex gap-2">
            <button className="btn-ghost" onClick={() => setDecks(decks.map((d) => (d.id === editDeck.id ? { ...d, cards: [...d.cards, { front: '', back: '' }] } : d)))}>+ Add card</button>
            <button className="btn" onClick={() => { setSource(editDeck.id); setEditing(null); }}>Done — study it</button>
            <button className="btn-ghost text-bad border-bad ml-auto" onClick={() => { if (ask('Delete this deck?')) { setDecks(decks.filter((d) => d.id !== editDeck.id)); setEditing(null); setSource(''); } }}>Delete deck</button>
          </div>
        </div>
      ) : deck ? (
        <FlashcardDeck cards={deck.cards.filter((c) => c.front && c.back)} title={deck.name} />
      ) : (
        <div className="card muted">Pick a deck above to start studying.</div>
      )}
    </div>
  );
}

// ---------------- Quiz ----------------
export function QuizPage() {
  const subjects = useSubjects();
  const [params] = useSearchParams();
  const [subjectId, setSubjectId] = useState(params.get('subject') ?? 'geometry');
  const [topicId, setTopicId] = useState(params.get('final') ? 'final' : params.get('unit') ? `unit:${params.get('unit')}` : 'all');
  const [timed, setTimed] = useState(false);
  const [qs, setQs] = useState<Question[] | null>(null);
  const [i, setI] = useState(0);
  const [results, setResults] = useState<Graded[]>([]);
  const [timeLeft, setTimeLeft] = useState(0);
  const [done, setDone] = useState(false);
  const subject = subjects.find((s) => s.id === subjectId)!;
  const N = topicId === 'final' ? 25 : 10;
  usePage({ label: `Practice Quiz – ${subject.name}`, detail: qs && !done ? `Quiz question ${i + 1} of ${N} (the student must answer this themself — do not give the answer)` : undefined, subject: subjectId });

  const start = () => {
    const unitIdx = topicId.startsWith('unit:') ? Number(topicId.slice(5)) : -1;
    const unitIds = unitIdx >= 0 ? subject.units?.[unitIdx]?.topicIds ?? [] : [];
    const pool = topicId === 'all' || topicId === 'final' ? [...subject.topics].sort(() => Math.random() - 0.5) : unitIdx >= 0 ? subject.topics.filter((t) => unitIds.includes(t.id)) : subject.topics.filter((t) => t.id === topicId);
    setQs(Array.from({ length: N }, (_, k) => { const t = pool[k % pool.length]; return t.generate((topicId === 'final' ? 1 : Math.min(2, (getState().topics[t.id]?.difficulty ?? 0))) as 0 | 1 | 2); }).sort(() => Math.random() - 0.5));
    setI(0); setResults([]); setDone(false); setTimeLeft(N * 60);
  };
  const finish = (res: Graded[]) => {
    setDone(true);
    const score = res.filter((r) => r.correct).length;
    recordQuiz(subject.name, score, N);
    if (topicId === 'final') recordFinalExam(subject.id, Math.round((score / N) * 100));
    if (topicId.startsWith('unit:')) recordUnitTest10(`${subject.id}:${topicId.slice(5)}`, score);
    qs?.forEach((q, k) => { const r = res[k]; if (r && !r.correct) recordMistake({ topicId: `quiz-${subject.id}`, topic: `${subject.name} quiz`, prompt: q.prompt, given: r.given || '(blank)', correct: correctAnswerText(q), why: r.feedback || 'Time ran out before you answered.', solution: q.explanation }); });
    recordActivity('study');
  };
  useEffect(() => {
    if (!timed || !qs || done) return;
    const id = setInterval(() => setTimeLeft((t) => {
      if (t <= 1) { clearInterval(id); const filled = [...results]; while (filled.length < N) filled.push({ correct: false, given: '(time ran out)', feedback: '' }); setResults(filled); finish(filled); return 0; }
      return t - 1;
    }), 1000);
    return () => clearInterval(id);
  }, [timed, qs, done, results]); // eslint-disable-line react-hooks/exhaustive-deps

  const onDone = (g: Graded) => {
    const r = [...results, g];
    setResults(r);
    if (r.length >= N) finish(r); else setI(i + 1);
  };

  if (!qs) return (
    <div>
      <PageHeader title="Practice Quiz" sub="10 questions, then a full review of anything you missed." />
      <div className="card space-y-4 w-[640px]">
        <label className="block"><span className="muted text-sm">Subject</span>
          <select className="input w-full" value={subjectId} onChange={(e) => { setSubjectId(e.target.value); setTopicId('all'); }}>{subjects.map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}</select>
        </label>
        <label className="block"><span className="muted text-sm">Topic</span>
          <select className="input w-full" value={topicId} onChange={(e) => setTopicId(e.target.value)}>
            <option value="final">🏆 FINAL EXAM — 25 questions, every unit</option>
            <option value="all">Mixed — all topics</option>
            {subject.units?.map((u, k) => <option key={k} value={`unit:${k}`}>🎓 Unit {k + 1} test: {u.title}</option>)}
            {subject.topics.map((t) => <option key={t.id} value={t.id}>{t.title}</option>)}
          </select>
        </label>
        <label className="flex items-center gap-2"><input type="checkbox" checked={timed} onChange={(e) => setTimed(e.target.checked)} /> Timed ({N} minutes)</label>
        <button className="btn" onClick={start}>Start quiz</button>
      </div>
    </div>
  );

  if (done) {
    const score = results.filter((r) => r.correct).length;
    return (
      <div>
        <PageHeader title="Quiz results" right={<button className="btn" onClick={() => setQs(null)}>New quiz</button>} />
        <div className="card text-center mb-6 animate-pop">
          <div className="text-6xl font-extrabold" style={{ color: score === N ? '#4ADE80' : score >= N * 0.7 ? '#7FD3FF' : '#FF9F43' }}>{score}/{N}</div>
          <div className="muted">{topicId === 'final' ? (score >= N * 0.7 ? `You PASSED the ${subject.name} final (${Math.round((score / N) * 100)}%)! 🏆` : `${Math.round((score / N) * 100)}% — you need 70% to pass. Review the misses and the units they came from.`) : score === N ? 'Perfect score! 💯' : score >= 7 ? 'Nice work! Review the misses below.' : 'Keep practicing — review each miss below.'}</div>
        </div>
        <h2 className="h2 mb-3">Review</h2>
        <div className="space-y-3">
          {qs.map((q, k) => {
            const r = results[k];
            return (
              <div key={k} className={`card ${r?.correct ? 'border-good/50' : 'border-bad/60'}`}>
                <div className="flex gap-4">
                  <div className={`text-2xl ${r?.correct ? 'text-good' : 'text-bad'}`}>{r?.correct ? '✓' : '✗'}</div>
                  <div className="flex-1">
                    <Rich text={q.prompt} className="font-semibold" />
                    {!r?.correct && <>
                      <div className="text-sm mt-2"><span className="muted">Your answer:</span> <span className="text-bad">{r?.given || '(blank)'}</span></div>
                      {r?.feedback && <div className="text-sm text-streak">{r.feedback}</div>}
                      <div className="text-sm"><span className="muted">Correct answer:</span> <b className="text-good">{correctAnswerText(q)}</b></div>
                      <Rich text={`**Why:** ${q.explanation}`} className="text-sm mt-1" />
                    </>}
                  </div>
                  {!r?.correct && q.diagram && <div className="w-[260px]">{q.diagram}</div>}
                </div>
              </div>
            );
          })}
        </div>
      </div>
    );
  }

  return (
    <div>
      <PageHeader title={`Quiz: ${subject.name}`} sub={`${topicId === 'final' ? 'FINAL EXAM · ' : ''}Question ${i + 1} of ${N}`} right={timed ? <div className={`text-2xl font-extrabold ${timeLeft < 60 ? 'text-bad' : 'text-edge'}`}>⏱ {Math.floor(timeLeft / 60)}:{String(timeLeft % 60).padStart(2, '0')}</div> : undefined} />
      <div className="flex gap-1 mb-4">{Array.from({ length: N }, (_, k) => <div key={k} className={`h-2 flex-1 rounded ${k < i ? 'bg-accent' : k === i ? 'bg-edge' : 'bg-navy'}`} />)}</div>
      <div className="card"><QuestionView q={qs[i]} mode="quiz" onDone={onDone} /></div>
    </div>
  );
}

// ---------------- Focus timer ----------------
export function FocusTimer() {
  const s = useApp((st) => st.settings);
  const [phase, setPhase] = useState<'work' | 'short' | 'long'>('work');
  const [round, setRound] = useState(1);
  const [left, setLeft] = useState(s.focusWork * 60);
  const [running, setRunning] = useState(false);
  const minuteAcc = useRef(0);
  usePage({ label: 'Focus Timer', subject: 'general' });
  const total = (phase === 'work' ? s.focusWork : phase === 'short' ? s.focusShort : s.focusLong) * 60;
  useEffect(() => { if (!running) setLeft(total); }, [s.focusWork, s.focusShort, s.focusLong]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!running) return;
    const id = setInterval(() => {
      setLeft((l) => {
        if (phase === 'work') { minuteAcc.current++; if (minuteAcc.current >= 60) { minuteAcc.current = 0; addMinutes('study', 1); } }
        if (l > 1) return l - 1;
        beep(phase === 'work' ? 'done' : 'start');
        if (phase === 'work') {
          addFocusRound(); recordActivity('study');
          const next = round % 4 === 0 ? 'long' : 'short';
          setPhase(next);
          if (typeof Notification !== 'undefined' && Notification.permission === 'granted') new Notification('Focus round done! Take a break.');
          return (next === 'long' ? s.focusLong : s.focusShort) * 60;
        }
        setPhase('work'); setRound((r) => r + 1);
        return s.focusWork * 60;
      });
    }, 1000);
    return () => clearInterval(id);
  }, [running, phase, round, s]);

  const pct = 1 - left / total, R = 130, C = 2 * Math.PI * R;
  const color = phase === 'work' ? '#A970FF' : '#4ADE80';
  return (
    <div>
      <PageHeader title="Focus Timer" sub={`Pomodoro: ${s.focusWork} min focus / ${s.focusShort} min break, long ${s.focusLong}-min break after 4 rounds. Change times in Settings.`} />
      <div className="card flex flex-col items-center py-10">
        <Tabs value={phase} onChange={(p) => { setPhase(p); setRunning(false); setLeft((p === 'work' ? s.focusWork : p === 'short' ? s.focusShort : s.focusLong) * 60); }} tabs={[{ id: 'work', label: '🍅 Focus' }, { id: 'short', label: '☕ Short break' }, { id: 'long', label: '🌙 Long break' }]} />
        <div className="relative my-8" style={{ width: 300, height: 300 }}>
          <svg width={300} height={300} className="-rotate-90">
            <circle cx={150} cy={150} r={R} stroke="#0B1026" strokeWidth={14} fill="none" />
            <circle cx={150} cy={150} r={R} stroke={color} strokeWidth={14} fill="none" strokeDasharray={C} strokeDashoffset={C * (1 - pct)} strokeLinecap="round" style={{ filter: `drop-shadow(0 0 10px ${color})`, transition: 'stroke-dashoffset 1s linear' }} />
          </svg>
          <div className="absolute inset-0 flex flex-col items-center justify-center">
            <div className="text-6xl font-extrabold tabular-nums">{Math.floor(left / 60)}:{String(left % 60).padStart(2, '0')}</div>
            <div className="muted">Round {round} · {phase === 'work' ? 'Focus' : 'Break'}</div>
          </div>
        </div>
        <div className="flex gap-3">
          <button className="btn w-32" onClick={() => { setRunning((r) => !r); if (typeof Notification !== 'undefined' && Notification.permission === 'default') Notification.requestPermission(); }}>{running ? '⏸ Pause' : '▶ Start'}</button>
          <button className="btn-ghost" onClick={() => { setRunning(false); setLeft(total); }}>↺ Reset</button>
          <button className="btn-ghost" onClick={() => setLeft(1)}>⏭ Skip</button>
        </div>
        <div className="flex gap-2 mt-6">{[1, 2, 3, 4].map((k) => <div key={k} className={`w-4 h-4 rounded-full border border-edge ${((round - 1) % 4) + 1 > k || (((round - 1) % 4) + 1 === k && phase !== 'work') ? 'bg-accent' : ''}`} />)}</div>
      </div>
    </div>
  );
}

// ---------------- Notes ----------------
export function NotesPage() {
  const subjects = useSubjects();
  const notes = useApp((s) => s.notes);
  const [key, setKey] = useState(`${subjects[0].id}/general`);
  usePage({ label: 'Notes', subject: 'general' });
  const label = (k: string) => {
    const [sid, tid] = k.split('/');
    const s = subjects.find((x) => x.id === sid);
    return `${s?.icon ?? ''} ${s?.name ?? sid} › ${tid === 'general' ? 'General' : s?.topics.find((t) => t.id === tid)?.title ?? tid}`;
  };
  return (
    <div>
      <PageHeader title="Notes" sub="A notes page for every subject and unit. Saves automatically." />
      <div className="flex gap-4">
        <div className="w-72 card p-2 max-h-[70vh] overflow-auto shrink-0">
          {subjects.map((s: Subject) => (
            <div key={s.id} className="mb-2">
              <div className="px-2 py-1 font-bold" style={{ color: s.color }}>{s.icon} {s.name}</div>
              {[{ id: 'general', title: 'General' }, ...s.topics].map((t) => {
                const k = `${s.id}/${t.id}`;
                return <button key={k} onClick={() => setKey(k)} className={`block w-full text-left px-3 py-1 rounded-lg text-sm ${key === k ? 'bg-accent/30 text-ink' : 'muted hover:text-ink'}`}>{notes[k] ? '📝 ' : ''}{t.title}</button>;
              })}
            </div>
          ))}
        </div>
        <div className="flex-1 card">
          <div className="font-bold mb-2">{label(key)}</div>
          <textarea className="input w-full h-[60vh] font-mono text-sm" value={notes[key] ?? ''} onChange={(e) => setNote(key, e.target.value)} placeholder="Start typing…" />
        </div>
      </div>
    </div>
  );
}

// ---------------- Essay coach ----------------
const TRANSITIONS = ['first', 'second', 'next', 'then', 'finally', 'however', 'therefore', 'furthermore', 'moreover', 'for example', 'for instance', 'in addition', 'as a result', 'in conclusion', 'on the other hand', 'because', 'although', 'also', 'consequently', 'in contrast'];
export function EssayCoach() {
  const text = useApp((s) => s.notes['english/essay-draft'] ?? '');
  usePage({ label: 'English > Essay Coach', detail: 'The student is drafting an essay. Give feedback only — never write sentences or paragraphs for them.', subject: 'english' });
  const paras = text.split(/\n\s*\n/).map((p) => p.trim()).filter(Boolean);
  const sentences = text.split(/(?<=[.!?])\s+/).filter((s) => s.trim().length > 2);
  const words = text.split(/\s+/).filter(Boolean);
  const avg = sentences.length ? Math.round(words.length / sentences.length) : 0;
  const lower = text.toLowerCase();
  const usedT = TRANSITIONS.filter((t) => lower.includes(t));
  const firstPara = paras[0] ?? '';
  const lastSentIntro = firstPara.split(/(?<=[.!?])\s+/).pop() ?? '';
  const longS = sentences.filter((s) => s.split(/\s+/).length > 35).length;
  const iCount = (text.match(/\bI\b/g) ?? []).length;
  const checks: [boolean, string][] = [
    [paras.length >= 3, `Paragraphs: ${paras.length}. A basic essay has an intro, at least one body paragraph, and a conclusion (5 is classic).`],
    [lastSentIntro.split(/\s+/).length >= 10, 'Thesis check: does the last sentence of your intro state your main claim AND your reasons?'],
    [usedT.length >= 3, `Transitions used: ${usedT.length ? usedT.join(', ') : 'none yet'}. Aim for at least 3 to connect ideas.`],
    [avg >= 10 && avg <= 25, `Average sentence length: ${avg} words. 10–25 words is a good range; mix short and long sentences.`],
    [longS === 0, longS ? `${longS} very long sentence(s) (35+ words) — could any be split?` : 'No overly long sentences.'],
    [iCount <= 3, iCount > 3 ? `You used "I" ${iCount} times. Formal essays usually avoid "I think" — state claims directly.` : 'Formal voice looks good.'],
  ];
  return (
    <div>
      <PageHeader title="Essay Coach" sub="Write YOUR essay here. The coach gives feedback on your writing — it never writes it for you." />
      <div className="flex gap-4">
        <div className="flex-1 card">
          <textarea className="input w-full h-[60vh] leading-relaxed" value={text} onChange={(e) => setNote('english/essay-draft', e.target.value)} placeholder="Paste or type your draft. Leave a blank line between paragraphs." />
          <div className="text-sm muted mt-2">{words.length} words · {sentences.length} sentences · {paras.length} paragraphs</div>
        </div>
        <div className="w-96 space-y-3 shrink-0">
          <div className="card">
            <div className="h2 mb-3">Checklist</div>
            <ul className="space-y-2 text-sm">{checks.map(([ok, msg], i) => <li key={i} className="flex gap-2"><span className={ok ? 'text-good' : 'text-streak'}>{ok ? '✓' : '•'}</span><span>{msg}</span></li>)}</ul>
          </div>
          <div className="card">
            <div className="font-bold mb-2">Get AI feedback</div>
            <p className="text-sm muted mb-3">Study Buddy will point out strengths and what to improve, with questions to guide your revision.</p>
            <button className="btn w-full" disabled={words.length < 30} onClick={() => openChat(`Please give me feedback on my essay draft. Don't rewrite it — tell me what's working, what to improve, and ask me questions to help me revise.\n\n---\n${text}`)}>🤖 Review my draft</button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ---------------- My Mistakes ----------------
/** Every question answered wrong, with your answer, why it was wrong, the correct answer and the full solution. */
export function MistakesPage() {
  const mistakes = useApp((s) => s.mistakes);
  const subjects = useSubjects();
  const [show, setShow] = useState<'todo' | 'all'>('todo');
  const [topic, setTopic] = useState('all');
  usePage({ label: 'Study > My Mistakes', detail: 'The student is reviewing questions they got wrong. The answers are already revealed; help them understand the method.', subject: 'general' });
  const topics = [...new Map(mistakes.map((m) => [m.topicId, m.topic])).entries()];
  const list = mistakes.filter((m) => (show === 'all' || !m.reviewed) && (topic === 'all' || m.topicId === topic));
  const todo = mistakes.filter((m) => !m.reviewed).length;
  const linkFor = (id: string) => { const s = subjects.find((x) => x.topics.some((t) => t.id === id)); return s ? `/study/${s.id}/${id}` : null; };
  return (
    <div>
      <PageHeader title="📖 My Mistakes" sub="Every question you got wrong, explained. Read why, then mark it as understood." right={
        mistakes.some((m) => m.reviewed) ? <button className="btn-ghost" onClick={() => setMistakes(mistakes.filter((m) => !m.reviewed))}>🧹 Clear understood ones</button> : undefined
      } />
      <div className="flex gap-3 items-center mb-4 flex-wrap">
        <Tabs value={show} onChange={setShow} tabs={[{ id: 'todo', label: `To review (${todo})` }, { id: 'all', label: `All (${mistakes.length})` }]} />
        <select className="input" value={topic} onChange={(e) => setTopic(e.target.value)}>
          <option value="all">All topics</option>
          {topics.map(([id, name]) => <option key={id} value={id}>{name}</option>)}
        </select>
      </div>
      {list.length === 0 ? (
        <div className="card text-center py-10"><div className="text-5xl mb-2">🎉</div><div className="h2">{mistakes.length ? 'All caught up!' : 'No mistakes yet'}</div><div className="muted">{mistakes.length ? 'You reviewed every mistake.' : 'Any question you miss in practice or quizzes will show up here with an explanation.'}</div></div>
      ) : (
        <div className="space-y-3">
          {list.map((m) => {
            const link = linkFor(m.topicId);
            return (
              <div key={m.id} className={`card ${m.reviewed ? 'opacity-60' : 'border-bad/50'}`}>
                <div className="flex justify-between text-xs muted mb-1"><span>{m.topic}</span><span>{m.date}</span></div>
                <Rich text={m.prompt} className="font-semibold mb-2" />
                <div className="grid grid-cols-2 gap-3 text-sm">
                  <div className="rounded-lg bg-bad/10 border border-bad/40 p-2"><div className="text-xs muted">You answered</div><div className="text-bad font-bold">{m.given || '(blank)'}</div></div>
                  <div className="rounded-lg bg-good/10 border border-good/40 p-2"><div className="text-xs muted">Correct answer</div><div className="text-good font-bold">{m.correct}</div></div>
                </div>
                {m.why && <div className="text-sm mt-2"><b className="text-streak">Why yours was wrong:</b> {m.why}</div>}
                <Rich text={`**Step by step:** ${m.solution}`} className="text-sm mt-1" />
                <div className="flex gap-2 mt-3">
                  {!m.reviewed ? <button className="btn py-1" onClick={() => setMistakes(mistakes.map((x) => (x.id === m.id ? { ...x, reviewed: true } : x)))}>✓ Got it</button>
                    : <button className="btn-ghost py-1" onClick={() => setMistakes(mistakes.map((x) => (x.id === m.id ? { ...x, reviewed: false } : x)))}>↺ Review again</button>}
                  <button className="btn-ghost py-1" onClick={() => openChat(`I got this wrong and I want to understand it better:\n"${m.prompt}"\nI answered ${m.given}; the correct answer is ${m.correct}. Can you explain the method a different way and give me a similar problem to try?`)}>🤖 Explain it differently</button>
                  {link && <Link to={link} className="btn-ghost py-1">📝 Practice this topic</Link>}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
