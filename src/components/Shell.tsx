import { useEffect, useState } from 'react';
import { Link, NavLink } from 'react-router-dom';
import { useApp, useUi, dismissCelebration, liveStreak, levelInfo } from '../lib/store';
import { beep } from '../piano/audio';
import { Flame, ProgressBar } from './ui';
import { SaveBadge } from './SaveFile';
import { JazzToggle } from './Jazz';
import { SidebarSearch } from '../pages/Search';

const NAV = [
  { to: '/search', icon: '🔍', label: 'Search', phone: true },
  { to: '/', icon: '🏠', label: 'Home', end: true },
  { to: '/plan', icon: '🎯', label: 'A+ Plan' },
  { to: '/calendar', icon: '📅', label: 'Homework', sub: true },
  { to: '/study', icon: '📚', label: 'Study Zone' },
  { to: '/advanced', icon: '🚀', label: 'Super Advanced', sub: true },
  { to: '/piano', icon: '🎹', label: 'Piano Course' },
  { to: '/practice', icon: '🎯', label: 'Practice Games', sub: true },
  { to: '/songs', icon: '🎼', label: 'Song Player' },
  { to: '/sheets', icon: '📜', label: 'Sheet Music', sub: true },
  { to: '/free', icon: '🎶', label: 'Free Play' },
  { to: '/progress', icon: '📈', label: 'Progress' },
  { to: '/settings', icon: '⚙️', label: 'Settings' },
];

export function Sidebar({ drawer, onNavigate }: { drawer?: boolean; onNavigate?: () => void } = {}) {
  const s = useApp((st) => st);
  const streak = liveStreak(s);
  const lv = levelInfo(s.xp);
  return (
    <nav className={drawer ? 'w-72 max-w-[85vw] h-full bg-navy border-r border-edge/25 flex flex-col p-4 overflow-y-auto' : 'hidden md:flex w-60 shrink-0 h-screen sticky top-0 bg-navy/80 border-r border-edge/25 flex-col p-4 overflow-y-auto'}>
      <div className={`text-xl font-extrabold flex items-center gap-2 ${drawer ? 'mb-6' : 'mb-4'}`}><span className="text-2xl">✨</span> Study<span className="text-accent">+</span>Piano</div>
      {!drawer && <SidebarSearch />}
      <div className="space-y-1 flex-1">
        {NAV.filter((n) => drawer || !n.phone).map((n) => (
          <NavLink key={n.to} to={n.to} end={n.end} onClick={onNavigate}
            className={({ isActive }) => `flex items-center gap-3 px-3 py-2 rounded-xl font-semibold transition ${n.sub ? 'ml-4 text-sm py-1.5' : ''} ${isActive ? 'bg-accent/25 text-ink border border-edge/60 shadow-[0_0_12px_rgba(127,211,255,.25)]' : 'text-muted hover:text-ink hover:bg-white/5 border border-transparent'}`}>
            <span className="text-lg">{n.icon}</span>{n.label}
          </NavLink>
        ))}
      </div>
      <div className="card p-3 space-y-2">
        <div className="flex items-center gap-2"><Flame n={streak} size={28} /><div><div className="font-extrabold text-streak">{streak} day{streak === 1 ? '' : 's'}</div><div className="text-xs muted">🧊 {s.streak.freezes} freeze{s.streak.freezes === 1 ? '' : 's'}</div></div></div>
        <div className="text-xs muted">Level {lv.level} · {lv.into}/{lv.need} XP</div>
        <ProgressBar value={lv.into} max={lv.need} />
        <JazzToggle />
        <div className="pt-1 border-t border-edge/20"><SaveBadge /></div>
      </div>
    </nav>
  );
}

const CONFETTI = ['#A970FF', '#7FD3FF', '#FF9F43', '#4ADE80', '#F472B6', '#FACC15'];
export function Celebrations() {
  const list = useUi((u) => u.celebrations);
  const cur = list[0];
  useEffect(() => {
    if (!cur) return;
    void beep('done');
    const id = setTimeout(() => dismissCelebration(cur.id), 3200);
    return () => clearTimeout(id);
  }, [cur]);
  if (!cur) return null;
  return (
    <div className="fixed inset-0 z-[60] pointer-events-none flex items-center justify-center" key={cur.id}>
      {Array.from({ length: 60 }, (_, i) => (
        <div key={i} className="absolute top-0 w-2.5 h-4 rounded-sm animate-fall" style={{ left: `${(i * 37) % 100}%`, background: CONFETTI[i % CONFETTI.length], animationDelay: `${(i % 12) * 0.08}s`, animationDuration: `${2 + (i % 5) * 0.3}s` }} />
      ))}
      <div className="pointer-events-auto card2 border-2 border-edge px-10 py-8 text-center animate-pop shadow-[0_0_60px_rgba(169,112,255,.6)]" onClick={() => dismissCelebration(cur.id)}>
        <div className="text-7xl mb-2">{cur.icon}</div>
        <div className="text-3xl font-extrabold">{cur.title}</div>
        {cur.subtitle && <div className="muted mt-1">{cur.subtitle}</div>}
      </div>
    </div>
  );
}

/** Phone layout: a top bar with a ☰ menu button that slides the sidebar in. */
export function MobileTopBar() {
  const [open, setOpen] = useState(false);
  const s = useApp((st) => st);
  const streak = liveStreak(s);
  return (
    <>
      <header className="md:hidden sticky top-0 z-30 -mx-3 -mt-3 mb-3 flex items-center gap-3 px-3 py-2 bg-navy/95 backdrop-blur border-b border-edge/25">
        <button className="text-2xl w-11 h-11 rounded-xl border border-edge/40 flex items-center justify-center" onClick={() => setOpen(true)} aria-label="Open menu">☰</button>
        <div className="font-extrabold flex-1">✨ Study<span className="text-accent">+</span>Piano</div>
        <Link to="/search" className="text-xl w-11 h-11 rounded-xl border border-edge/40 flex items-center justify-center" aria-label="Search">🔍</Link>
        <div className="flex items-center gap-1 font-extrabold text-streak"><Flame n={streak} size={22} />{streak}</div>
      </header>
      {open && (
        <div className="md:hidden fixed inset-0 z-50 bg-black/60" onClick={() => setOpen(false)}>
          <div className="h-full animate-[pop_.2s_ease-out]" onClick={(e) => e.stopPropagation()}>
            <Sidebar drawer onNavigate={() => setOpen(false)} />
          </div>
        </div>
      )}
    </>
  );
}
