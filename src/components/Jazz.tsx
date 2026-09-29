import { useEffect, useState } from 'react';
import { useLocation } from 'react-router-dom';
import { useApp, setSettings } from '../lib/store';
import { startJazz, stopJazz, setJazzVolume, onJazz, isJazzPlaying } from '../lib/jazz';

/** Pages where you play the piano (or take a timed test) — the jazz pauses there so it never clashes. */
const QUIET = [/^\/piano\/(lesson|unit)/, /^\/songs/, /^\/free/, /^\/practice/, /^\/study\/quiz/, /^\/study\/timer/];

/** Plays smooth jazz on the menus. Browsers only allow sound after a click, so it starts on the first click. */
export function JazzController() {
  const { pathname } = useLocation();
  const on = useApp((s) => s.settings.jazz);
  const vol = useApp((s) => s.settings.jazzVolume);
  const [gesture, setGesture] = useState(false);
  useEffect(() => {
    if (gesture) return;
    const go = () => setGesture(true);
    window.addEventListener('pointerdown', go, { once: true });
    window.addEventListener('keydown', go, { once: true });
    return () => { window.removeEventListener('pointerdown', go); window.removeEventListener('keydown', go); };
  }, [gesture]);
  useEffect(() => { setJazzVolume(vol); }, [vol]);
  useEffect(() => {
    const quiet = QUIET.some((r) => r.test(pathname));
    if (on && gesture && !quiet) void startJazz();
    else stopJazz();
  }, [on, gesture, pathname]);
  return null;
}

export function JazzToggle() {
  const on = useApp((s) => s.settings.jazz);
  const [playing, setPlaying] = useState(isJazzPlaying());
  useEffect(() => onJazz(setPlaying), []);
  return (
    <button onClick={() => setSettings({ jazz: !on })} title="Smooth jazz on the menus (pauses while you play piano)"
      className={`w-full flex items-center gap-2 text-xs px-2 py-1.5 rounded-lg border transition ${on ? 'border-streak/60 text-streak' : 'border-edge/20 text-muted'}`}>
      <span className={`text-base ${playing ? 'animate-flicker' : ''}`}>🎷</span>
      <span className="flex-1 text-left">Smooth jazz: {on ? (playing ? 'playing' : 'on (paused here)') : 'off'}</span>
    </button>
  );
}
