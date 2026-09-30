// Reminders popup shown every time the app opens: homework, tests, reviews and streak status.
import { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { getState, homeworkBlocking, liveStreak, useApp } from '../lib/store';
import { addDays, dayKey, daysBetween } from '../lib/date';

type Item = { icon: string; text: string; tone: 'bad' | 'warn' | 'ok'; to: string };

function buildReminders(): Item[] {
  const s = getState(), today = dayKey();
  const out: Item[] = [];
  const pending = s.homework.filter((h) => !h.done);
  const late = pending.filter((h) => h.due < today);
  const dueToday = pending.filter((h) => h.due === today);
  const dueTmr = pending.filter((h) => h.due === addDays(today, 1));
  const soon = pending.filter((h) => h.due > addDays(today, 1) && h.due <= addDays(today, 3));
  const list = (hs: typeof pending) => hs.map((h) => `${h.title} (${h.cls})`).join(', ');
  if (late.length) out.push({ icon: '🚨', tone: 'bad', to: '/calendar', text: `${late.length} overdue: ${list(late)}` });
  if (dueToday.length) out.push({ icon: '📌', tone: 'bad', to: '/calendar', text: `Due TODAY: ${list(dueToday)}` });
  if (dueTmr.length) out.push({ icon: '⏰', tone: 'warn', to: '/calendar', text: `Due tomorrow (finish today to keep your streak!): ${list(dueTmr)}` });
  if (soon.length) out.push({ icon: '📝', tone: 'ok', to: '/calendar', text: `Coming up: ${soon.map((h) => `${h.title} (in ${daysBetween(today, h.due)} days)`).join(', ')}` });
  s.tests.filter((t) => t.date >= today && daysBetween(today, t.date) <= 5).sort((a, b) => a.date.localeCompare(b.date)).forEach((t) => {
    const n = daysBetween(today, t.date);
    out.push({ icon: '🧪', tone: n <= 1 ? 'bad' : 'warn', to: '/plan', text: `${t.title} is ${n === 0 ? 'TODAY' : n === 1 ? 'TOMORROW' : `in ${n} days`} — do a practice test` });
  });
  const due = Object.values(s.review).filter((r) => r.due <= today).length;
  if (due) out.push({ icon: '🧠', tone: 'ok', to: '/plan', text: `${due} topic${due > 1 ? 's' : ''} due for Daily Review (~8 min)` });
  s.grades.filter((g) => g.grade !== null && g.grade < 90).forEach((g) => out.push({ icon: '📈', tone: 'warn', to: '/plan', text: `${g.name} is at ${g.grade}% — a little practice gets it to an A` }));
  const streak = liveStreak(s);
  if (s.streak.last !== today) {
    const hold = homeworkBlocking(s).length;
    out.push({ icon: '🔥', tone: hold ? 'warn' : 'ok', to: hold ? '/calendar' : '/plan', text: hold ? `${streak ? `Your ${streak}-day streak` : "Today's streak"} is on hold until homework due tomorrow is done` : streak ? `Keep your ${streak}-day streak alive — study or practice today` : 'Start a streak today — study or practice for a few minutes' });
  }
  return out;
}

const TONE = { bad: 'border-bad/70 bg-bad/10', warn: 'border-streak/60 bg-streak/10', ok: 'border-edge/40 bg-navy' };

export function Reminders() {
  useApp((s) => s); // recompute when progress changes (e.g. after the cloud save loads)
  const items = buildReminders();
  const [open, setOpen] = useState(() => { try { return !sessionStorage.getItem('reminders-shown'); } catch { return true; } });
  const nav = useNavigate();
  const close = () => { setOpen(false); try { sessionStorage.setItem('reminders-shown', '1'); } catch { /* storage blocked */ } };
  if (!open || !items.length) return null;
  const name = getState().settings.name;
  const hour = new Date().getHours();
  return (
    <div className="fixed inset-0 z-[60] bg-black/60 flex items-center justify-center p-8" onClick={close}>
      <div className="card w-[560px] max-h-[85vh] overflow-auto animate-pop border-accent" onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-1">
          <div className="text-2xl font-extrabold">{hour < 12 ? 'Good morning' : hour < 18 ? 'Good afternoon' : 'Good evening'}{name ? `, ${name}` : ''}! 👋</div>
          <button className="btn-ghost py-1 px-2" onClick={close} aria-label="Close">✕</button>
        </div>
        <div className="muted text-sm mb-4">Here are your reminders for today:</div>
        <div className="space-y-2">
          {items.map((it, k) => (
            <button key={k} onClick={() => { close(); nav(it.to); }} className={`w-full text-left flex items-start gap-3 rounded-xl border px-3 py-2.5 hover:brightness-125 ${TONE[it.tone]}`}>
              <span className="text-xl">{it.icon}</span><span className="flex-1 text-sm">{it.text}</span><span className="text-edge text-sm font-bold">Go →</span>
            </button>
          ))}
        </div>
        <div className="flex gap-2 mt-5">
          <button className="btn flex-1" onClick={() => { close(); nav('/calendar'); }}>📅 Open homework</button>
          <button className="btn-ghost flex-1" onClick={close}>Got it</button>
        </div>
      </div>
    </div>
  );
}
