import { ask } from '../lib/embed';
import { useEffect, useMemo, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { BarChart, Bar, XAxis, YAxis, Tooltip, ResponsiveContainer, LineChart, Line, CartesianGrid, Legend } from 'recharts';
import { useApp, liveStreak, levelInfo, openChat, setSettings, resetProgress, AppState, DIFF_NAMES } from '../lib/store';
import { dayKey, lastNDays, shortDay, parseDay, weekStart, addDays } from '../lib/date';
import { BADGES } from '../lib/badges';
import { usePage } from '../lib/hooks';
import { Flame, ProgressBar, Ring, Stat, PageHeader, Stars } from '../components/ui';
import { useSubjects } from '../study/pages';
import { COURSE } from '../piano/course';
import { MidiSetupPanel } from '../piano/MidiSetup';
import { PhoneBanner, PhoneSetupCard } from '../components/PhoneSetup';
import { setVolume, initAudio, click } from '../piano/audio';
import { SaveFileCard } from '../components/SaveFile';
import { pinHash } from '../lib/wiki';

/** Teacher-only switch for the picture-search filter, locked with a PIN. */
function PictureFilterCard() {
  const { picFilter, picPin } = useApp((s) => s.settings);
  const [pin, setPin] = useState('');
  const [pin2, setPin2] = useState('');
  const [msg, setMsg] = useState<string | null>(null);
  const turnOff = () => {
    if (!picPin) {
      if (!/^\d{4,8}$/.test(pin)) return setMsg('Pick a PIN of 4 to 8 digits.');
      if (pin !== pin2) return setMsg("The two PINs don't match.");
      setSettings({ picPin: pinHash(pin), picFilter: false });
    } else {
      if (pinHash(pin) !== picPin) return setMsg('Wrong PIN.');
      setSettings({ picFilter: false });
    }
    setPin(''); setPin2(''); setMsg(null);
  };
  return (
    <div className="card space-y-3">
      <div className="h2">🖼️ Picture filter (teacher)</div>
      <p className="text-sm muted">Search hides nudity, sexual content and extreme gore from pictures and books (school topics like biology and history still show). Only someone with the teacher PIN can turn it off.</p>
      {picFilter ? (
        <>
          <div className="font-bold text-good">On</div>
          <div className="flex flex-wrap gap-2">
            <input className="input w-36" type="password" inputMode="numeric" autoComplete="off" placeholder={picPin ? 'Teacher PIN' : 'New PIN'} value={pin} onChange={(e) => setPin(e.target.value)} />
            {!picPin && <input className="input w-36" type="password" inputMode="numeric" autoComplete="off" placeholder="PIN again" value={pin2} onChange={(e) => setPin2(e.target.value)} />}
            <button className="btn-ghost" onClick={turnOff}>Turn filter off</button>
          </div>
          {!picPin && <p className="text-xs muted">The first time, choose a PIN. Keep it to yourself.</p>}
          {msg && <p className="text-sm text-bad">{msg}</p>}
        </>
      ) : (
        <>
          <div className="font-bold text-streak">Off: picture search shows everything</div>
          <button className="btn" onClick={() => setSettings({ picFilter: true })}>Turn filter back on</button>
        </>
      )}
    </div>
  );
}

const tooltipStyle = { contentStyle: { background: '#141B3D', border: '1px solid #7FD3FF', borderRadius: 10, color: '#E8ECFF' }, labelStyle: { color: '#A9A8D6' } };

export function StreakCalendar({ weeks = 12 }: { weeks?: number }) {
  const days = useApp((s) => s.days);
  const frozen = useApp((s) => s.streak.frozenDays);
  const today = dayKey();
  const start = addDays(weekStart(today), -(weeks - 1) * 7);
  const cols = Array.from({ length: weeks }, (_, w) => Array.from({ length: 7 }, (_, d) => addDays(start, w * 7 + d)));
  return (
    <div className="flex gap-1">
      <div className="flex flex-col gap-1 mr-1 text-[10px] muted">{['M', '', 'W', '', 'F', '', 'S'].map((d, i) => <div key={i} className="h-4 leading-4">{d}</div>)}</div>
      {cols.map((col, i) => (
        <div key={i} className="flex flex-col gap-1">
          {col.map((k) => {
            const a = days[k];
            const did = a && (a.study > 0 || a.piano > 0);
            const both = a && a.study > 0 && a.piano > 0;
            const future = k > today;
            const fz = frozen.includes(k);
            return <div key={k} title={`${parseDay(k).toDateString()}${did ? ` — ${a.study} study, ${a.piano} piano` : fz ? ' — streak freeze' : ''}`}
              className={`w-4 h-4 rounded ${future ? 'opacity-0' : did ? (both ? 'bg-streak shadow-[0_0_6px_#FF9F43]' : 'bg-streak/60') : fz ? 'bg-edge/60' : 'bg-navy border border-edge/15'} ${k === today ? 'ring-2 ring-edge' : ''}`} />;
          })}
        </div>
      ))}
    </div>
  );
}

function greeting() { const h = new Date().getHours(); return h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening'; }

export function Home() {
  const s = useApp((st) => st);
  const nav = useNavigate();
  usePage({ label: 'Home dashboard', subject: 'general' });
  const streak = liveStreak(s);
  const today = s.days[dayKey()];
  const goal = s.settings.goalStudy + s.settings.goalPiano;
  const done = Math.min(today?.study ?? 0, s.settings.goalStudy) + Math.min(today?.piano ?? 0, s.settings.goalPiano);
  const lv = levelInfo(s.xp);
  const week = lastNDays(7).map((k) => ({ day: shortDay(k), xp: s.days[k]?.xp ?? 0 }));
  const recentBadges = Object.entries(s.badges).sort((a, b) => b[1].localeCompare(a[1])).slice(0, 4).map(([id]) => BADGES.find((b) => b.id === id)!).filter(Boolean);
  const nextLesson = useMemo(() => {
    for (const u of COURSE) {
      if (u.n > s.piano.unlockedUnit) break;
      const l = u.lessons.find((x) => !s.piano.lessons[x.id]);
      if (l) return { path: `/piano/lesson/${l.id}`, label: `Unit ${u.n}: ${l.title}` };
      if (u.test && (s.piano.unitTests[u.n] ?? 0) < 2) return { path: `/piano/lesson/u${u.n}-test`, label: `Unit ${u.n} Test` };
    }
    return { path: '/piano', label: 'Course map' };
  }, [s.piano]);
  return (
    <div>
      <PageHeader title={`${greeting()}${s.settings.name ? `, ${s.settings.name}` : ''}! 👋`} sub={new Date().toLocaleDateString(undefined, { weekday: 'long', month: 'long', day: 'numeric', year: 'numeric' })} />
      <PhoneBanner />
      <div className="grid grid-cols-3 gap-4 mb-4">
        <div className="card flex items-center gap-5">
          <Flame n={streak} size={64} />
          <div>
            <div className="text-4xl font-extrabold text-streak">{streak}</div>
            <div className="muted">day streak · best {s.streak.best}</div>
            <div className="text-sm mt-1">🧊 {s.streak.freezes}/2 streak freezes</div>
          </div>
        </div>
        <div className="card flex items-center gap-5">
          <Ring value={done} max={goal} label={`${done}/${goal}`} sub="today" />
          <div className="text-sm space-y-1">
            <div className="font-bold">Daily goal</div>
            <div>{(today?.study ?? 0) >= s.settings.goalStudy ? '✅' : '⬜'} {s.settings.goalStudy} study session{s.settings.goalStudy > 1 ? 's' : ''}</div>
            <div>{(today?.piano ?? 0) >= s.settings.goalPiano ? '✅' : '⬜'} {s.settings.goalPiano} piano lesson{s.settings.goalPiano > 1 ? 's' : ''}</div>
          </div>
        </div>
        <div className="card">
          <div className="flex justify-between items-end"><div><div className="text-xs muted">LEVEL</div><div className="text-4xl font-extrabold text-accent">{lv.level}</div></div><div className="text-right text-sm muted">{s.xp} total XP</div></div>
          <ProgressBar value={lv.into} max={lv.need} className="mt-3" />
          <div className="text-xs muted mt-1">{lv.need - lv.into} XP to level {lv.level + 1}</div>
        </div>
      </div>
      <div className="grid grid-cols-4 gap-3 mb-4">
        <button className="btn py-3" onClick={() => nav(s.lastStudy?.path ?? '/study')}>📚 Start Studying</button>
        <button className="btn py-3" onClick={() => nav(nextLesson.path)}>🎹 Next Piano Lesson</button>
        <button className="btn-ghost py-3" onClick={() => nav('/study/timer')}>🍅 Focus Timer</button>
        <button className="btn-ghost py-3" onClick={() => openChat()}>🤖 Ask Study Buddy</button>
      </div>
      <div className="grid grid-cols-2 gap-4 mb-4">
        <Link to={s.lastStudy?.path ?? '/study'} className="card2 flex items-center gap-4 hover:brightness-110">
          <div className="text-4xl">📚</div>
          <div className="flex-1"><div className="text-xs muted">CONTINUE STUDYING</div><div className="font-bold">{s.lastStudy?.label ?? 'Pick a subject to start'}</div></div><div>→</div>
        </Link>
        <Link to={s.lastPiano?.path ?? nextLesson.path} className="card2 flex items-center gap-4 hover:brightness-110">
          <div className="text-4xl">🎹</div>
          <div className="flex-1"><div className="text-xs muted">CONTINUE PIANO</div><div className="font-bold">{s.lastPiano?.label ?? nextLesson.label}</div></div><div>→</div>
        </Link>
      </div>
      <div className="grid grid-cols-3 gap-4">
        <div className="card col-span-2">
          <div className="font-bold mb-2">XP this week</div>
          <div className="h-48">
            <ResponsiveContainer><BarChart data={week}><XAxis dataKey="day" stroke="#A9A8D6" /><YAxis stroke="#A9A8D6" allowDecimals={false} /><Tooltip {...tooltipStyle} cursor={{ fill: 'rgba(127,211,255,.08)' }} /><Bar dataKey="xp" fill="#A970FF" radius={[6, 6, 0, 0]} /></BarChart></ResponsiveContainer>
          </div>
        </div>
        <div className="card">
          <div className="font-bold mb-2">Recent badges</div>
          {recentBadges.length ? <div className="grid grid-cols-2 gap-2">{recentBadges.map((b) => <div key={b.id} className="text-center p-2 rounded-xl bg-card2 border border-edge/30"><div className="text-3xl">{b.icon}</div><div className="text-xs font-bold">{b.name}</div></div>)}</div>
            : <div className="muted text-sm">Finish your first lesson to earn a badge!</div>}
          <Link to="/progress" className="text-edge text-sm mt-3 inline-block">All badges →</Link>
        </div>
      </div>
      <div className="card mt-4"><div className="font-bold mb-3">Streak calendar</div><StreakCalendar weeks={20} /></div>
    </div>
  );
}

export function Progress() {
  const s = useApp((st) => st);
  const subjects = useSubjects();
  usePage({ label: 'Progress', subject: 'general' });
  const lv = levelInfo(s.xp);
  const weeks = Array.from({ length: 8 }, (_, i) => addDays(weekStart(dayKey()), -7 * (7 - i)));
  const weekly = weeks.map((w) => {
    const ds = Array.from({ length: 7 }, (_, d) => s.days[addDays(w, d)]);
    return { week: parseDay(w).toLocaleDateString(undefined, { month: 'short', day: 'numeric' }), study: ds.reduce((n, d) => n + (d?.studyMin ?? 0), 0), piano: ds.reduce((n, d) => n + (d?.pianoMin ?? 0), 0), xp: ds.reduce((n, d) => n + (d?.xp ?? 0), 0) };
  });
  const xpHistory = lastNDays(30).map((k) => ({ day: parseDay(k).getDate(), xp: s.days[k]?.xp ?? 0 }));
  const allTopics = subjects.flatMap((sub) => sub.topics.map((t) => ({ sub, t, st: s.topics[t.id] })));
  const mastered = allTopics.filter((x) => (x.st?.masteredLevel ?? -1) >= 0).length;
  const weak = allTopics.filter((x) => x.st && x.st.attempts >= 5 && x.st.correct / x.st.attempts < 0.7);
  const bySubject = subjects.map((sub) => {
    const ts = sub.topics.map((t) => s.topics[t.id]).filter(Boolean);
    const a = ts.reduce((n, t) => n + t!.attempts, 0), c = ts.reduce((n, t) => n + t!.correct, 0);
    return { name: sub.name, accuracy: a ? Math.round((c / a) * 100) : 0, answered: a };
  });
  const totalStars = Object.values(s.piano.lessons).reduce((n, v) => n + v, 0);
  const npm = s.piano.noteGameNPM.slice(-20).map((x, i) => ({ n: i + 1, npm: x.npm }));
  return (
    <div>
      <PageHeader title="Progress" sub="Everything you've accomplished." />
      <div className="grid grid-cols-5 gap-3 mb-4">
        <Stat label="Level" value={lv.level} color="#A970FF" /><Stat label="Total XP" value={s.xp} /><Stat label="Current streak" value={`🔥 ${liveStreak(s)}`} color="#FF9F43" />
        <Stat label="Topics mastered" value={`${mastered}/${allTopics.length}`} color="#4ADE80" /><Stat label="Piano stars" value={`⭐ ${totalStars}`} color="#FACC15" />
      </div>
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="card"><div className="font-bold mb-3">Streak calendar</div><StreakCalendar weeks={20} /><div className="text-xs muted mt-2">Bright = study AND piano that day · Blue = streak freeze used</div></div>
        <div className="card"><div className="font-bold mb-2">XP — last 30 days</div><div className="h-40"><ResponsiveContainer><LineChart data={xpHistory}><CartesianGrid stroke="#2a3366" /><XAxis dataKey="day" stroke="#A9A8D6" /><YAxis stroke="#A9A8D6" /><Tooltip {...tooltipStyle} /><Line dataKey="xp" stroke="#A970FF" strokeWidth={3} dot={false} /></LineChart></ResponsiveContainer></div></div>
      </div>
      <h2 className="h2 mb-3 mt-6">📚 Study</h2>
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="card"><div className="font-bold mb-2">Minutes practiced per week</div><div className="h-52"><ResponsiveContainer><BarChart data={weekly}><XAxis dataKey="week" stroke="#A9A8D6" fontSize={11} /><YAxis stroke="#A9A8D6" /><Tooltip {...tooltipStyle} /><Legend /><Bar dataKey="study" name="Study" fill="#7FD3FF" radius={[4, 4, 0, 0]} /><Bar dataKey="piano" name="Piano" fill="#A970FF" radius={[4, 4, 0, 0]} /></BarChart></ResponsiveContainer></div></div>
        <div className="card"><div className="font-bold mb-2">Accuracy by subject</div><div className="space-y-3 mt-3">{bySubject.map((b) => <div key={b.name}><div className="flex justify-between text-sm"><span>{b.name}</span><span className="muted">{b.answered ? `${b.accuracy}% · ${b.answered} answered` : 'not started'}</span></div><ProgressBar value={b.accuracy} color={b.accuracy >= 80 ? '#4ADE80' : b.accuracy >= 60 ? '#7FD3FF' : '#FF9F43'} /></div>)}</div></div>
      </div>
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="card max-h-80 overflow-auto">
          <div className="font-bold mb-2">Accuracy by topic</div>
          <table className="w-full text-sm"><tbody>{allTopics.filter((x) => x.st?.attempts).map(({ sub, t, st }) => (
            <tr key={t.id} className="border-t border-edge/15"><td className="py-1">{sub.icon} <Link className="hover:text-edge" to={`/study/${sub.id}/${t.id}`}>{t.title}</Link></td><td className="text-right muted">{Math.round((st!.correct / st!.attempts) * 100)}%</td><td className="text-right pl-2">{st!.masteredLevel >= 0 ? <span className="text-good">✓ {DIFF_NAMES[st!.masteredLevel]}</span> : ''}</td></tr>
          ))}</tbody></table>
          {!allTopics.some((x) => x.st?.attempts) && <div className="muted text-sm">Answer some practice questions to see stats.</div>}
        </div>
        <div className="card">
          <div className="font-bold mb-2">Weak spots to review</div>
          {weak.length ? weak.map(({ sub, t, st }) => <Link key={t.id} to={`/study/${sub.id}/${t.id}`} className="flex justify-between py-1 border-b border-edge/15 hover:text-edge"><span>{sub.icon} {t.title}</span><span className="text-bad">{Math.round((st!.correct / st!.attempts) * 100)}%</span></Link>) : <div className="muted text-sm">No weak spots yet — nice! (Topics under 70% after 5+ questions show here.)</div>}
          <div className="font-bold mt-4 mb-2">Recent quizzes</div>
          {s.quizzes.slice(-5).reverse().map((q, i) => <div key={i} className="flex justify-between text-sm"><span>{q.subject} · {q.date}</span><b className={q.score === q.total ? 'text-good' : ''}>{q.score}/{q.total}</b></div>)}
          {!s.quizzes.length && <div className="muted text-sm">No quizzes yet.</div>}
        </div>
      </div>
      <h2 className="h2 mb-3 mt-6">🎹 Piano</h2>
      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="card max-h-80 overflow-auto">
          <div className="font-bold mb-2">Course map · stars per lesson</div>
          {COURSE.map((u) => (
            <div key={u.n} className="flex items-center gap-2 py-1 border-b border-edge/10 text-sm">
              <span className={`w-16 shrink-0 `}>{u.exam ? 'Final' : `Unit ${u.n}`}</span>
              <div className="flex gap-1 flex-1 flex-wrap">{u.lessons.map((l) => <span key={l.id} title={l.title} className="text-xs"><Stars n={s.piano.lessons[l.id] ?? 0} size="text-xs" /></span>)}</div>
              {!u.exam && <span className="text-xs">{(s.piano.unitTests[u.n] ?? 0) >= 2 ? '✅' : '▶️'}</span>}
            </div>
          ))}
        </div>
        <div className="space-y-4">
          <div className="card"><div className="font-bold mb-2">Songs completed</div>
            {Object.entries(s.piano.songs).length ? Object.entries(s.piano.songs).map(([id, r]) => <div key={id} className="flex justify-between text-sm py-0.5"><span className="truncate">{id.replace(/^(song-|rep-|import-)/, '')}</span><span><Stars n={r.stars} size="text-sm" /> {r.accuracy}% · {r.plays}×</span></div>) : <div className="muted text-sm">Play a song in the Song Player!</div>}
          </div>
          <div className="card"><div className="font-bold mb-2">Note-reading speed (notes per minute)</div>
            {npm.length ? <div className="h-32"><ResponsiveContainer><LineChart data={npm}><XAxis dataKey="n" stroke="#A9A8D6" /><YAxis stroke="#A9A8D6" /><Tooltip {...tooltipStyle} /><Line dataKey="npm" stroke="#4ADE80" strokeWidth={3} /></LineChart></ResponsiveContainer></div> : <div className="muted text-sm">Play the note-reading game in Practice Games.</div>}
          </div>
        </div>
      </div>
      <h2 className="h2 mb-3 mt-6">🏅 Badges</h2>
      <div className="grid grid-cols-5 gap-3">
        {BADGES.map((b) => {
          const got = s.badges[b.id];
          return (
            <div key={b.id} className={`card text-center ${got ? 'border-streak/70 shadow-[0_0_14px_rgba(255,159,67,.3)]' : 'opacity-40 grayscale'}`}>
              <div className="text-4xl">{got ? b.icon : '🔒'}</div><div className="font-bold text-sm mt-1">{b.name}</div><div className="text-xs muted">{b.desc}</div>
              {got && <div className="text-[10px] text-streak mt-1">Earned {got}</div>}
            </div>
          );
        })}
      </div>
    </div>
  );
}

export function Settings() {
  const set = useApp((st: AppState) => st.settings);
  const [ai, setAi] = useState<'checking' | 'ok' | 'no-key' | 'offline'>('checking');
  const [model, setModel] = useState('');
  const [confirmText, setConfirmText] = useState('');
  usePage({ label: 'Settings', subject: 'general' });
  useEffect(() => { fetch('/api/status').then((r) => r.json()).then((j) => { setAi(j.configured ? 'ok' : 'no-key'); setModel(j.model); }).catch(() => setAi('offline')); }, []);
  const num = (label: string, key: keyof AppState['settings'], min: number, max: number, suffix = '') => (
    <label className="flex items-center justify-between gap-3"><span>{label}</span><span className="flex items-center gap-2"><input type="number" className="input w-20" min={min} max={max} value={set[key] as number} onChange={(e) => setSettings({ [key]: Math.max(min, Math.min(max, Number(e.target.value))) })} />{suffix}</span></label>
  );
  return (
    <div>
      <PageHeader title="Settings" />
      <div className="grid grid-cols-2 gap-4">
        <div className="col-span-2"><PhoneSetupCard /></div>
        <SaveFileCard />
        <div className="card space-y-3 col-span-2"><div className="h2">🎹 MIDI keyboard</div><MidiSetupPanel /></div>
        <div className="card space-y-3">
          <div className="h2">👤 Profile & goals</div>
          <label className="flex items-center justify-between gap-3"><span>Your name</span><input className="input w-48" value={set.name} onChange={(e) => setSettings({ name: e.target.value })} placeholder="(optional)" /></label>
          {num('Study sessions per day', 'goalStudy', 0, 10)}
          {num('Piano lessons per day', 'goalPiano', 0, 10)}
          <label className="flex items-center justify-between gap-3"><span>Foreign language</span>
            <select className="input w-48" value={set.language} onChange={(e) => setSettings({ language: e.target.value as 'spanish' | 'french' })}><option value="spanish">🇪🇸 Spanish</option><option value="french">🇫🇷 French</option></select></label>
        </div>
        <div className="card space-y-3">
          <div className="h2">🍅 Focus timer</div>
          {num('Focus length', 'focusWork', 1, 90, 'min')}
          {num('Short break', 'focusShort', 1, 30, 'min')}
          {num('Long break (after 4 rounds)', 'focusLong', 1, 60, 'min')}
        </div>
        <div className="card space-y-3">
          <div className="h2">🔊 Sound</div>
          <label className="flex items-center justify-between gap-3"><span>Volume</span><input type="range" min={0} max={1} step={0.05} value={set.volume} className="accent-[#A970FF] w-48" onChange={(e) => { setSettings({ volume: Number(e.target.value) }); setVolume(Number(e.target.value)); }} /></label>
          <label className="flex items-center justify-between gap-3"><span>🎷 Smooth jazz on menus</span><input type="checkbox" checked={set.jazz} onChange={(e) => setSettings({ jazz: e.target.checked })} /></label>
          <label className="flex items-center justify-between gap-3"><span>Jazz volume</span><input type="range" min={0} max={1} step={0.05} value={set.jazzVolume} className="accent-[#A970FF] w-48" onChange={(e) => setSettings({ jazzVolume: Number(e.target.value) })} /></label>
          <label className="flex items-center justify-between gap-3"><span>Metronome sound</span>
            <span className="flex gap-2"><select className="input" value={set.metronome} onChange={(e) => setSettings({ metronome: e.target.value as 'click' | 'wood' | 'beep' })}><option value="click">Click</option><option value="wood">Woodblock</option><option value="beep">Beep</option></select>
              <button className="btn-ghost py-1" onClick={async () => { await initAudio(); click(true); setTimeout(() => click(false), 400); }}>Test</button></span></label>
        </div>
        <PictureFilterCard />
        <div className="card space-y-3">
          <div className="h2">🤖 AI helper</div>
          <div className="flex items-center gap-3">
            <div className={`w-4 h-4 rounded-full ${ai === 'ok' ? 'bg-good shadow-[0_0_10px_#4ADE80]' : ai === 'checking' ? 'bg-muted' : 'bg-bad'}`} />
            <div>{ai === 'ok' ? <>Connected — using <b>{model}</b></> : ai === 'checking' ? 'Checking…' : ai === 'no-key' ? 'Server running, but no API key is set.' : 'Helper server is not running.'}</div>
          </div>
          {ai !== 'ok' && ai !== 'checking' && <p className="text-sm muted">Copy <code className="text-edge">.env.example</code> to <code className="text-edge">.env</code>, add your <code className="text-edge">ANTHROPIC_API_KEY</code>, then restart with <code className="text-edge">npm run dev</code>.</p>}
        </div>
        <div className="card space-y-3 col-span-2 border-bad/50">
          <div className="h2 text-bad">⚠️ Reset progress</div>
          <p className="text-sm muted">This permanently erases your streak, XP, badges, mastery, piano stars, notes, decks, recordings and chats. Type <b>RESET</b> to confirm.</p>
          <div className="flex gap-2"><input className="input w-40" value={confirmText} onChange={(e) => setConfirmText(e.target.value)} placeholder="RESET" />
            <button className="btn bg-bad" disabled={confirmText !== 'RESET'} onClick={() => { if (ask('Are you absolutely sure? This cannot be undone.')) { resetProgress(); setConfirmText(''); } }}>Erase everything</button></div>
        </div>
      </div>
    </div>
  );
}
