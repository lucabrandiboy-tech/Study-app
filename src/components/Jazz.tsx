import { useEffect, useState } from 'react';
import { useApp, setSettings } from '../lib/store';
import { startJazz, stopJazz, setJazzVolume, onJazz, isJazzPlaying } from '../lib/jazz';
import { useYouTube } from './MiniPlayer';

/** Plays smooth jazz all the time, on every page (on by default; Settings > Sound turns it off). It only pauses while a
 *  YouTube video is open, so two songs never play at once. Browsers only allow sound after a tap, so it starts on the first tap. */
export function JazzController() {
  const youTube = useYouTube();
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
    if (on && gesture && !youTube) void startJazz();
    else stopJazz();
  }, [on, gesture, youTube]);
  return null;
}

export function JazzToggle() {
  const on = useApp((s) => s.settings.jazz);
  const [playing, setPlaying] = useState(isJazzPlaying());
  useEffect(() => onJazz(setPlaying), []);
  return (
    <button onClick={() => setSettings({ jazz: !on })} title="Background jazz on every page (turn it off here or in Settings)"
      className={`w-full flex items-center gap-2 text-xs px-2 py-1.5 rounded-lg border transition ${on ? 'border-streak/60 text-streak' : 'border-edge/20 text-muted'}`}>
      <span className={`text-base ${playing ? 'animate-flicker' : ''}`}>🎷</span>
      <span className="flex-1 text-left">Smooth jazz: {on ? (playing ? 'playing' : 'on (paused for video)') : 'off'}</span>
    </button>
  );
}
