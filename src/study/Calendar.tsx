// 📅 Homework calendar — a real month calendar. Tap a day to add an assignment and its subject.
// Rule: unfinished homework due tomorrow (or overdue) blocks today's streak.
import { useState } from 'react';
import { useApp, addHomework, toggleHomework, removeHomework, homeworkBlocking } from '../lib/store';
import { addDays, dayKey, daysBetween, parseDay } from '../lib/date';
import { usePage } from '../lib/hooks';
import { PageHeader, Modal } from '../components/ui';

const PALETTE = ['#A970FF', '#7FD3FF', '#4ADE80', '#FF9F43', '#F472B6', '#FACC15', '#60A5FA', '#F87171'];
const FIXED: Record<string, string> = { Math: '#A970FF', Science: '#4ADE80', English: '#7FD3FF', History: '#FF9F43', Spanish: '#F472B6', French: '#F472B6' };
export const colorFor = (cls: string) => FIXED[cls] ?? PALETTE[[...cls].reduce((a, c) => a + c.charCodeAt(0), 0) % PALETTE.length];
const WEEKDAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const longDay = (d: string) => parseDay(d).toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' });
const shortDate = (d: string) => parseDay(d).toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' });

/** The pop-up for one day: what's due, plus a form to add an assignment with its subject. */
function DayEditor({ day, subjects, onClose }: { day: string; subjects: string[]; onClose: () => void }) {
  const allHw = useApp((s) => s.homework), allTests = useApp((s) => s.tests);
  const homework = allHw.filter((h) => h.due === day);
  const tests = allTests.filter((t) => t.date === day);
  const today = dayKey();
  const [name, setName] = useState('');
  const [subject, setSubject] = useState(lastSubject);
  const [other, setOther] = useState(lastSubject && !subjects.includes(lastSubject) ? lastSubject : '');
  const [added, setAdded] = useState<string | null>(null);
  const pick = (s: string) => { setSubject(s); lastSubject = s; };
  const finalSubject = subject === '__other' ? other.trim() : subject;
  const add = () => {
    if (!name.trim() || !finalSubject) return;
    addHomework({ title: name.trim(), cls: finalSubject, due: day });
    lastSubject = finalSubject;
    setAdded(`✅ Added "${name.trim()}" (${finalSubject})`);
    setName('');
  };
  return (
    <Modal open onClose={onClose} title={`📅 ${longDay(day)}`}>
      <div className="space-y-2 mb-4">
        {tests.map((t) => <div key={t.id} className="text-sm rounded-lg bg-bad/20 px-3 py-2">🧪 {t.title} <span className="muted">(test, prep on the A+ Plan page)</span></div>)}
        {homework.map((h) => (
          <div key={h.id} className="flex items-center gap-2 rounded-lg bg-navy border border-edge/20 px-3 py-2" style={{ borderLeft: `4px solid ${colorFor(h.cls)}` }}>
            <input type="checkbox" className="w-5 h-5" checked={h.done} onChange={() => toggleHomework(h.id)} aria-label={`Done: ${h.title}`} />
            <span className={`flex-1 ${h.done ? 'line-through muted' : ''}`}>{h.title}</span>
            <span className="chip" style={{ color: colorFor(h.cls), borderColor: colorFor(h.cls) }}>{h.cls}</span>
            <button className="muted hover:text-bad px-1" onClick={() => removeHomework(h.id)} aria-label={`Delete ${h.title}`}>✕</button>
          </div>
        ))}
        {!homework.length && !tests.length && <div className="text-sm muted">Nothing due this day yet.</div>}
      </div>

      <div className="border-t border-edge/20 pt-4 space-y-3">
        <div className="font-bold">➕ New assignment</div>
        <label className="block text-sm"><span className="muted">Assignment</span>
          <input className="input w-full mt-1" autoFocus placeholder="e.g. Worksheet p. 42" value={name} autoComplete="off"
            onChange={(e) => { setName(e.target.value); setAdded(null); }} onKeyDown={(e) => { e.stopPropagation(); if (e.key === 'Enter') add(); }} />
        </label>
        <div className="text-sm">
          <div className="muted mb-1">Subject</div>
          <div className="flex flex-wrap gap-2">
            {subjects.map((s) => (
              <button key={s} onClick={() => pick(s)} className="px-3 py-1.5 rounded-full text-sm font-semibold border-2 transition"
                style={{ borderColor: colorFor(s), background: subject === s ? colorFor(s) : 'transparent', color: subject === s ? '#0B1026' : colorFor(s) }}>{s}</button>
            ))}
            <button onClick={() => pick('__other')} className={`px-3 py-1.5 rounded-full text-sm font-semibold border-2 border-edge/50 ${subject === '__other' ? 'bg-edge text-navy' : 'text-ink'}`}>Other…</button>
          </div>
          {subject === '__other' && <input className="input w-full mt-2" placeholder="Subject name (e.g. Art, P.E.)" value={other} autoComplete="off" onChange={(e) => setOther(e.target.value)} onKeyDown={(e) => { e.stopPropagation(); if (e.key === 'Enter') add(); }} />}
        </div>
        <div className="flex gap-2">
          <button className="btn flex-1" onClick={add} disabled={!name.trim() || !finalSubject}>+ Add to {shortDate(day)}</button>
          <button className="btn-ghost" onClick={onClose}>Done</button>
        </div>
        {added && <div className="text-sm text-good font-bold">{added}</div>}
        {!finalSubject && name.trim() && <div className="text-xs muted">Pick a subject to add it.</div>}
        {day >= today && day <= addDays(today, 1) && <div className="text-xs text-streak">Heads up: homework due {day === today ? 'today' : 'tomorrow'} must be checked off to keep today's streak.</div>}
      </div>
    </Modal>
  );
}
let lastSubject = ''; // remembers the subject you picked last, for adding several in a row

export function HomeworkCalendar() {
  usePage({ label: 'Homework Calendar', subject: 'general' });
  const homework = useApp((s) => s.homework);
  const tests = useApp((s) => s.tests);
  const grades = useApp((s) => s.grades);
  const lang = useApp((s) => s.settings.language);
  const streakLast = useApp((s) => s.streak.last);
  const state = useApp((s) => s);
  const today = dayKey();
  const [month, setMonth] = useState(() => today.slice(0, 7)); // YYYY-MM
  const [open, setOpen] = useState<string | null>(null);

  // the student's real classes (A+ Plan grades) first, then the usual school subjects, then any used before
  const subjects = [...new Set([...grades.map((g) => g.name.trim()), 'Math', 'Science', 'English', 'History', lang === 'french' ? 'French' : 'Spanish', ...homework.map((h) => h.cls)])]
    .filter((s) => s && s !== 'General');

  const first = parseDay(`${month}-01`);
  const gridStart = addDays(`${month}-01`, -first.getDay()); // Sunday-first, like a wall calendar
  const weeks = Math.ceil((first.getDay() + new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate()) / 7);
  const cells = Array.from({ length: weeks * 7 }, (_, i) => addDays(gridStart, i));
  const shift = (n: number) => { const d = parseDay(`${month}-01`); d.setMonth(d.getMonth() + n); setMonth(dayKey(d).slice(0, 7)); };

  const blocking = homeworkBlocking(state);
  const earnedToday = streakLast === today;
  const pending = homework.filter((h) => !h.done).sort((a, b) => a.due.localeCompare(b.due));
  const dueLabel = (d: string) => { const n = daysBetween(today, d); return n < 0 ? `${-n} day${n < -1 ? 's' : ''} late` : n === 0 ? 'due today' : n === 1 ? 'due tomorrow' : `in ${n} days`; };

  return (
    <div className="space-y-5">
      <PageHeader title="📅 Homework Calendar" sub="Tap any day to add an assignment and its subject. Finish homework at least 1 day before it's due to keep your streak."
        right={<button className="btn" onClick={() => setOpen(addDays(today, 1))}>+ Add homework</button>} />

      <div className={`card flex items-center gap-4 ${blocking.length ? 'border-bad/70' : 'border-good/60'}`}>
        <div className="text-4xl">{blocking.length ? '⏳' : '🔥'}</div>
        <div className="flex-1">
          {blocking.length ? <>
            <div className="font-bold text-bad">Today's streak is ON HOLD</div>
            <div className="text-sm">Finish {blocking.length === 1 ? 'this' : `these ${blocking.length}`} to unlock it: {blocking.map((h) => <b key={h.id} className="mr-2">{h.title} <span className="muted font-normal">({h.cls}, {dueLabel(h.due)})</span></b>)}</div>
          </> : <>
            <div className="font-bold text-good">{earnedToday ? 'Streak earned today ✓' : "Homework is on track. Study or practice to earn today's streak."}</div>
            <div className="text-sm muted">Nothing due tomorrow is unfinished. {pending.length ? `Next up: ${pending[0].title} (${dueLabel(pending[0].due)}).` : 'No pending homework.'}</div>
          </>}
        </div>
      </div>

      <div className="grid lg:grid-cols-[minmax(0,1fr)_300px] gap-5 items-start">
        <div className="card p-3 md:p-5">
          <div className="flex items-center gap-2 mb-3">
            <h2 className="text-2xl md:text-3xl font-extrabold flex-1">{first.toLocaleDateString(undefined, { month: 'long' })} <span className="muted font-bold">{first.getFullYear()}</span></h2>
            <button className="btn-ghost text-sm px-3" onClick={() => setMonth(today.slice(0, 7))}>Today</button>
            <button className="btn-ghost px-3" onClick={() => shift(-1)} aria-label="Previous month">‹</button>
            <button className="btn-ghost px-3" onClick={() => shift(1)} aria-label="Next month">›</button>
          </div>
          <div className="rounded-xl overflow-hidden border border-edge/30">
            <div className="grid grid-cols-7 bg-card2">
              {WEEKDAYS.map((d, i) => <div key={d} className={`py-2 text-center text-xs font-bold uppercase tracking-wide ${i === 0 || i === 6 ? 'text-muted' : 'text-edge'}`}>{d}</div>)}
            </div>
            <div className="grid grid-cols-7">
              {cells.map((d, i) => {
                const hw = homework.filter((h) => h.due === d).sort((a, b) => Number(a.done) - Number(b.done));
                const ts = tests.filter((t) => t.date === d);
                const inMonth = d.slice(0, 7) === month;
                const late = hw.some((h) => !h.done && d < today);
                const weekend = i % 7 === 0 || i % 7 === 6;
                return (
                  <button key={d} onClick={() => setOpen(d)} aria-label={`${longDay(d)}: ${hw.length} assignment${hw.length === 1 ? '' : 's'}. Tap to add.`}
                    className={`group relative min-h-[64px] md:min-h-[112px] p-1 md:p-1.5 text-left flex flex-col border-t border-edge/20 ${i % 7 ? 'border-l' : ''} transition hover:bg-accent/10
                      ${inMonth ? (weekend ? 'bg-navy/60' : 'bg-navy') : 'bg-deep/40'} ${late ? 'ring-1 ring-inset ring-bad/70' : ''}`}>
                    <div className="flex items-center justify-between">
                      <span className={`text-xs md:text-sm font-bold w-6 h-6 md:w-7 md:h-7 flex items-center justify-center rounded-full ${d === today ? 'bg-accent text-white' : inMonth ? '' : 'text-muted/50'}`}>{parseDay(d).getDate()}</span>
                      <span className="hidden md:inline text-xs text-edge opacity-0 group-hover:opacity-100">＋ Add</span>
                    </div>
                    {/* computer: assignment names; phone: colored dots */}
                    <div className="hidden md:block space-y-0.5 mt-1 w-full">
                      {ts.map((t) => <div key={t.id} className="text-[11px] truncate rounded px-1 bg-bad/30 text-white">🧪 {t.title}</div>)}
                      {hw.slice(0, 3).map((h) => <div key={h.id} className={`text-[11px] truncate rounded px-1 ${h.done ? 'line-through opacity-50' : ''}`} style={{ background: colorFor(h.cls) + '33', borderLeft: `3px solid ${colorFor(h.cls)}` }}>{h.title}</div>)}
                      {hw.length > 3 && <div className="text-[11px] muted">+{hw.length - 3} more</div>}
                    </div>
                    <div className="md:hidden flex flex-wrap gap-0.5 mt-1">
                      {ts.map((t) => <span key={t.id} className="w-2 h-2 rounded-full bg-bad" />)}
                      {hw.map((h) => <span key={h.id} className={`w-2 h-2 rounded-full ${h.done ? 'opacity-30' : ''}`} style={{ background: colorFor(h.cls) }} />)}
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
          <div className="flex flex-wrap gap-3 mt-3 text-xs">
            {subjects.map((s) => <span key={s} className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded-full" style={{ background: colorFor(s) }} />{s}</span>)}
            <span className="flex items-center gap-1"><span className="w-2.5 h-2.5 rounded-full bg-bad" />Test</span>
          </div>
        </div>

        <div className="card">
          <div className="font-bold mb-2">📋 To do ({pending.length})</div>
          <div className="space-y-1.5 max-h-[480px] overflow-auto">
            {pending.map((h) => {
              const n = daysBetween(today, h.due);
              return (
                <div key={h.id} className="flex items-center gap-2 text-sm">
                  <input type="checkbox" className="w-4 h-4" checked={false} onChange={() => toggleHomework(h.id)} aria-label={`Done: ${h.title}`} />
                  <span className="w-2 h-2 rounded-full shrink-0" style={{ background: colorFor(h.cls) }} />
                  <button className="flex-1 truncate text-left hover:text-edge" onClick={() => { setMonth(h.due.slice(0, 7)); setOpen(h.due); }}>{h.title} <span className="muted">· {h.cls}</span></button>
                  <span className={`text-xs font-bold whitespace-nowrap ${n < 0 ? 'text-bad' : n <= 1 ? 'text-streak' : 'muted'}`}>{dueLabel(h.due)}</span>
                </div>
              );
            })}
            {!pending.length && <div className="text-sm muted">All caught up! 🎉</div>}
          </div>
        </div>
      </div>

      {open && <DayEditor key={open} day={open} subjects={subjects} onClose={() => setOpen(null)} />}
    </div>
  );
}
