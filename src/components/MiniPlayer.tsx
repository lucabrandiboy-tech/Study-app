import { useState, useSyncExternalStore } from 'react';

// A YouTube video that keeps playing while you move around the app (it lives outside the pages, so changing page doesn't stop it).
let current: string | null = null;
const subs = new Set<() => void>();
export function playYouTube(id: string | null) { current = id; subs.forEach((f) => f()); }
export const useYouTube = () => useSyncExternalStore((f) => { subs.add(f); return () => { subs.delete(f); }; }, () => current);

export function MiniPlayer() {
  const vid = useYouTube();
  const [small, setSmall] = useState(false);
  if (!vid) return null;
  return (
    <div className="fixed right-3 bottom-24 z-40 w-[320px] max-w-[85vw] card2 p-2 shadow-[0_0_24px_rgba(0,0,0,.6)]">
      <div className="flex items-center gap-2 mb-1">
        <div className="flex-1 text-sm font-bold">▶️ Now playing</div>
        <button className="btn-ghost py-0.5 px-2 text-sm min-h-0" onClick={() => setSmall((s) => !s)} aria-label={small ? 'Show video' : 'Hide video'}>{small ? '▴ Show' : '▾ Hide'}</button>
        <button className="btn-ghost py-0.5 px-2 text-sm min-h-0" onClick={() => playYouTube(null)} aria-label="Stop and close">✕</button>
      </div>
      {/* Hidden (not removed) when small, so the sound keeps going. No allow-popups: YouTube's links can't open outside the app. */}
      <div className={small ? 'relative w-px h-px overflow-hidden opacity-0 pointer-events-none' : 'relative w-full aspect-video rounded-lg overflow-hidden bg-black'}>
        <iframe className="absolute inset-0 w-full h-full" src={`https://www.youtube-nocookie.com/embed/${vid}?rel=0&playsinline=1&modestbranding=1&autoplay=1`} title="YouTube video"
          sandbox="allow-scripts allow-same-origin allow-presentation" allow="autoplay; encrypted-media; picture-in-picture; fullscreen" referrerPolicy="strict-origin-when-cross-origin" allowFullScreen />
      </div>
    </div>
  );
}
