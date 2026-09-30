// 📅 Homework calendar — track assignments; unfinished homework due tomorrow (or overdue) blocks today's streak.
import { useState } from 'react';
import { useApp, addHomework, toggleHomework, removeHomework, homeworkBlocking } from '../lib/store';
import { addDays, dayKey, daysBetween, parseDay } from '../lib/date';
import { usePage } from '../lib/hooks';
import { PageHeader } from '../components/ui';
import { useSubjects } from './pages';

const PALETTE = ['#A970FF', '#7FD3FF', '#4ADE80', '#FF9F43', '#F472B6', '#FACC15', '#60A5FA', '#F87171'];
const colorFor = (cls: string) => PALETTE[[...cls].reduce((a, c) => a + c.charCodeAt(0), 0) % PALETTE.length];

export function HomeworkCalendar() {
  usePage({ label: 'Homework Calendar', subject: 'general' });
  const homework = useApp((s) => s.homework);
  const tests = useApp((s) => s.tests);
  const grades = useApp((s) => s.grades);
  const streakLast = useApp((s) => s.streak.last);
  const state = useApp((s) => s);
  const subjects = useSubjects();
  const today = dayKey();
  const [month, setMonth] = useState(() => today.slice(0, 7)); // YYYY-MM
  const [sel, setSel] = useState(today);
  const [title, setTitle] = useState('');
  const [cls, setCls] = useState('');
  const [qTitle, setQTitle] = useState('');
  const [qCls, setQCls] = useState('');
  const [qDue, setQDue] = useState(() => addDays(dayKey(), 1));
  const [flash, setFlash] = useState<string | null>(null);
  const quickAdd = () => {
    if (!qTitle.trim() || !qDue) return;
    addHomework({ title: qTitle.trim(), cls: qCls.trim() || 'General', due: qDue });
    setFlash(`✅ Added "${qTitle.trim()}" — due ${parseDay(qDue).toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })}`);
    setSel(qDue); setMonth(qDue.slice(0, 7)); setQTitle('');
    setTimeout(() => setFlash(null), 3000);
  };
  const classes = [...new Set([...grades.map((g) => g.name), ...homework.map((h) => h.cls), ...subjects.filter((s) => !s.advanced).map((s) => s.name)])].filter(Boolean);

  const first = parseDay(`${month}-01`);
  const startOffset = (first.getDay() + 6) % 7; // Monday-first
  const gridStart = addDays(`${month}-01`, -startOffset);
  const cells = Array.from({ length: 42 }, (_, i) => addDays(gridStart, i));
  const shift = (n: number) => { const d = parseDay(`${month}-01`); d.setMonth(d.getMonth() + n); setMonth(dayKey(d).slice(0, 7)); };

  const blocking = homeworkBlocking(state);
  const earnedToday = streakLast === today;
  const dayHw = homework.filter((h) => h.due === sel).sort((a, b) => Number(a.done) - Number(b.done));
  const dayTests = tests.filter((t) => t.date === sel);
  const pending = homework.filter((h) => !h.done).sort((a, b) => a.due.localeCompare(b.due));
  const add = () => { if (!title.trim()) return; addHomework({ title: title.trim(), cls: cls.trim() || 'General', due: sel }); setTitle(''); };
  const dueLabel = (d: string) => { const n = daysBetween(today, d); return n < 0 ? `${-n} day${n < -1 ? 's' : ''} late` : n === 0 ? 'due today' : n === 1 ? 'due tomorrow' : `in ${n} days`; };

  return (
    <div className="space-y-5">
      <PageHeader title="📅 Homework Calendar" sub="Add every assignment the day you get it. Rule: homework must be finished at least 1 day before it's due — or you don't get that day's streak." />

      <div className="card border-accent/70">
        <div className="font-bold mb-2">➕ Add homework</div>
        <div className="grid grid-cols-[2fr_1fr_auto_auto] gap-2 items-end">
          <label className="text-sm"><span className="muted">Assignment</span><input className="input w-full" placeholder="e.g. Math worksheet p. 42" value={qTitle} onChange={(e) => setQTitle(e.target.value)} onKeyDown={(e) => { e.stopPropagation(); if (e.key === 'Enter') quickAdd(); }} /></label>
          <label className="text-sm"><span className="muted">Class</span><input className="input w-full" list="hw-classes" placeholder="e.g. Science" value={qCls} onChange={(e) => setQCls(e.target.value)} onKeyDown={(e) => { e.stopPropagation(); if (e.key === 'Enter') quickAdd(); }} /></label>
          <label className="text-sm"><span className="muted">Due date</span><input type="date" className="input" value={qDue} onChange={(e) => setQDue(e.target.value)} /></label>
          <button className="btn" onClick={quickAdd} disabled={!qTitle.trim() || !qDue}>+ Add to calendar</button>
        </div>
        <div className="flex gap-2 mt-2 text-xs">
          <span className="muted">Due:</span>
          {[['Tomorrow', 1], ['In 2 days', 2], ['In 3 days', 3], ['Next week', 7]].map(([l, n]) => (
            <button key={l as string} className={`px-2 py-0.5 rounded-full border ${qDue === addDays(today, n as number) ? 'bg-accent border-accent' : 'border-edge/40 hover:border-edge'}`} onClick={() => setQDue(addDays(today, n as number))}>{l}</button>
          ))}
          {flash && <span className="text-good font-bold ml-auto">{flash}</span>}
        </div>
      </div>

      <div className={`card flex items-center gap-4 ${blocking.length ? 'border-bad/70' : 'border-good/60'}`}>
        <div className="text-4xl">{blocking.length ? '⏳' : '🔥'}</div>
        <div className="flex-1">
          {blocking.length ? <>
            <div className="font-bold text-bad">Today's streak is ON HOLD</div>
            <div className="text-sm">Finish {blocking.length === 1 ? 'this' : `these ${blocking.length}`} to unlock it: {blocking.map((h) => <b key={h.id} className="mr-2">{h.title} <span className="muted font-normal">({h.cls}, {dueLabel(h.due)})</span></b>)}</div>
          </> : <>
            <div className="font-bold text-good">{earnedToday ? 'Streak earned today ✓' : 'Homework is on track — study or practice to earn today\'s streak'}</div>
            <div className="text-sm muted">Nothing due tomorrow is unfinished. {pending.length ? `Next up: ${pending[0].title} (${dueLabel(pending[0].due)}).` : 'No pending homework.'}</div>
          </>}
        </div>
      </div>

      <div className="grid grid-cols-[1fr_360px] gap-5 items-start">
        <div className="card">
          <div className="flex items-center justify-between mb-3">
            <button className="btn-ghost" onClick={() => shift(-1)}>←</button>
            <div className="h2">{first.toLocaleDateString(undefined, { month: 'long', year: 'numeric' })}</div>
            <div className="flex gap-2"><button className="btn-ghost text-sm" onClick={() => { setMonth(today.slice(0, 7)); setSel(today); }}>Today</button><button className="btn-ghost" onClick={() => shift(1)}>→</button></div>
          </div>
          <div className="grid grid-cols-7 gap-1 text-center text-xs muted mb-1">{['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'].map((d) => <div key={d}>{d}</div>)}</div>
          <div className="grid grid-cols-7 gap-1">
            {cells.map((d) => {
              const hw = homework.filter((h) => h.due === d);
              const ts = tests.filter((t) => t.date === d);
              const inMonth = d.slice(0, 7) === month;
              const late = hw.some((h) => !h.done && h.due < today);
              return (
                <button key={d} onClick={() => { setSel(d); setQDue(d); }} title="Click to pick this day as the due date" className={`h-24 rounded-lg border p-1 text-left align-top flex flex-col overflow-hidden transition ${sel === d ? 'border-edge ring-2 ring-edge/50' : 'border-edge/20 hover:border-edge/60'} ${inMonth ? 'bg-navy' : 'bg-navy/30 opacity-50'} ${late ? 'border-bad/80' : ''}`}>
                  <div className={`text-xs font-bold ${d === today ? 'bg-accent text-white rounded-full w-6 h-6 flex items-center justify-center' : ''}`}>{parseDay(d).getDate()}</div>
                  <div className="space-y-0.5 mt-0.5 w-full">
                    {ts.map((t) => <div key={t.id} className="text-[10px] truncate rounded px-1 bg-bad/30 text-white">🧪 {t.title}</div>)}
                    {hw.slice(0, 3).map((h) => <div key={h.id} className={`text-[10px] truncate rounded px-1 ${h.done ? 'line-through opacity-50' : ''}`} style={{ background: colorFor(h.cls) + '44', borderLeft: `3px solid ${colorFor(h.cls)}` }}>{h.title}</div>)}
                    {hw.length > 3 && <div className="text-[10px] muted">+{hw.length - 3} more</div>}
                  </div>
                </button>
              );
            })}
          </div>
        </div>

        <div className="space-y-4">
          <div className="card">
            <div className="font-bold mb-1">{parseDay(sel).toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric' })}</div>
            <div className="text-xs muted mb-3">Assignments due this day</div>
            <div className="space-y-2 mb-3">
              {dayTests.map((t) => <div key={t.id} className="text-sm rounded-lg bg-bad/20 px-2 py-1">🧪 {t.title} (test — prep on the A+ Plan page)</div>)}
              {dayHw.map((h) => (
                <div key={h.id} className="flex items-center gap-2 text-sm">
                  <input type="checkbox" className="w-5 h-5" checked={h.done} onChange={() => toggleHomework(h.id)} />
                  <span className="w-2 h-2 rounded-full" style={{ background: colorFor(h.cls) }} />
                  <span className={`flex-1 ${h.done ? 'line-through muted' : ''}`}>{h.title} <span className="muted">· {h.cls}</span></span>
                  <button className="text-xs muted hover:text-bad" onClick={() => removeHomework(h.id)}>✕</button>
                </div>
              ))}
              {!dayHw.length && !dayTests.length && <div className="text-sm muted">Nothing due. Add an assignment below.</div>}
            </div>
            <input className="input w-full mb-2" placeholder="Assignment (e.g. Worksheet p. 42)" value={title} onChange={(e) => setTitle(e.target.value)} onKeyDown={(e) => { e.stopPropagation(); if (e.key === 'Enter') add(); }} />
            <input className="input w-full mb-2" list="hw-classes" placeholder="Class" value={cls} onChange={(e) => setCls(e.target.value)} onKeyDown={(e) => e.stopPropagation()} />
            <datalist id="hw-classes">{classes.map((c) => <option key={c} value={c} />)}</datalist>
            <button className="btn w-full" onClick={add} disabled={!title.trim()}>+ Add homework due {sel === today ? 'today' : parseDay(sel).toLocaleDateString(undefined, { month: 'short', day: 'numeric' })}</button>
            {sel <= addDays(today, 1) && sel >= today && <div className="text-xs text-streak mt-2">Heads up: homework due {sel === today ? 'today' : 'tomorrow'} must be checked off to keep today's streak.</div>}
          </div>

          <div className="card">
            <div className="font-bold mb-2">📋 To do ({pending.length})</div>
            <div className="space-y-1.5 max-h-[320px] overflow-auto">
              {pending.map((h) => {
                const n = daysBetween(today, h.due);
                return (
                  <div key={h.id} className="flex items-center gap-2 text-sm">
                    <input type="checkbox" className="w-4 h-4" checked={false} onChange={() => toggleHomework(h.id)} />
                    <span className="flex-1 truncate" onClick={() => { setSel(h.due); setMonth(h.due.slice(0, 7)); }}>{h.title} <span className="muted">· {h.cls}</span></span>
                    <span className={`text-xs font-bold ${n < 0 ? 'text-bad' : n <= 1 ? 'text-streak' : 'muted'}`}>{dueLabel(h.due)}</span>
                  </div>
                );
              })}
              {!pending.length && <div className="text-sm muted">All caught up! 🎉</div>}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
