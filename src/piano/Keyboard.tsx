import { useEffect, useState } from 'react';
import { onNote, pressed, emitNote, keyLabelFor, onStatus } from './input';
import { initAudio } from './audio';
import { pitchClass } from './notation';

export const RH_COLOR = '#7FD3FF';
export const LH_COLOR = '#FF9F43';
const isBlack = (m: number) => [1, 3, 6, 8, 10].includes(m % 12);

export function usePressed() {
  const [, set] = useState(0);
  useEffect(() => onNote(() => set((n) => n + 1)), []);
  return pressed;
}

/** On-screen piano. Click/tap to play. Highlights show which keys to press (blue = right hand, orange = left hand). */
export function Keyboard({ low = 48, high = 84, highlight = new Map(), marks = new Map(), showNames = false, showKeys = false, height = 150, light = false }: {
  low?: number; high?: number; highlight?: Map<number, 'R' | 'L'>; marks?: Map<number, 'good' | 'bad'>; showNames?: boolean; showKeys?: boolean; height?: number;
  /** Simply-Piano look: pure white keys, true black keys, bright hint colors. */
  light?: boolean;
}) {
  const down = usePressed();
  const [, setTick] = useState(0);
  useEffect(() => onStatus(() => setTick((t) => t + 1)), []);
  // snap to white keys
  while (isBlack(low)) low--;
  while (isBlack(high)) high++;
  const whites: number[] = [];
  for (let m = low; m <= high; m++) if (!isBlack(m)) whites.push(m);
  const W = 100 / whites.length;
  const press = (m: number) => { void initAudio(); emitNote({ type: 'on', midi: m, vel: 0.75, source: 'mouse' }); };
  const release = (m: number) => emitNote({ type: 'off', midi: m, vel: 0, source: 'mouse' });

  const color = (m: number, black: boolean) => {
    const mk = marks.get(m);
    if (down.has(m)) return mk === 'bad' ? (light ? '#EF4444' : '#F87171') : mk === 'good' ? (light ? '#22C55E' : '#4ADE80') : light ? '#A78BFA' : '#A970FF';
    const h = highlight.get(m);
    if (h) return h === 'R' ? (light ? '#3B82F6' : RH_COLOR) : light ? '#F59E0B' : LH_COLOR;
    if (light) return black ? 'linear-gradient(#3a3a3a, #050505 70%)' : '#FFFFFF';
    return black ? '#1a1a2e' : '#EEF0FF';
  };
  const handlers = (m: number) => ({
    onMouseDown: (e: React.MouseEvent) => { e.preventDefault(); press(m); },
    onMouseUp: () => release(m), onMouseLeave: () => down.has(m) && release(m),
    onMouseEnter: (e: React.MouseEvent) => { if (e.buttons === 1) press(m); },
  });

  return (
    <div className={`relative select-none rounded-b-xl overflow-hidden border-t-4 ${light ? 'border-[#1F2937] bg-white shadow-[0_8px_24px_rgba(15,23,42,.18)]' : 'border-accent shadow-[0_6px_24px_rgba(0,0,0,.5)]'}`} style={{ height }}>
      {whites.map((m, i) => {
        const c = color(m, false);
        const lbl = showKeys ? keyLabelFor(m) : null;
        return (
          <div key={m} {...handlers(m)} className={`absolute top-0 bottom-0 border-r rounded-b-md cursor-pointer transition-colors duration-75 flex flex-col justify-end items-center pb-1 ${light ? 'border-[#D1D5DB]' : 'border-[#9aa0c8]'}`}
            style={{ left: `${i * W}%`, width: `${W}%`, background: c, boxShadow: highlight.has(m) ? `inset 0 0 12px ${c}` : light ? 'inset 0 -6px 0 #E5E7EB' : undefined }}>
            {lbl && <span className="text-[10px] font-bold text-[#5b5f8a]">{lbl}</span>}
            {(showNames || m % 12 === 0) && <span className={`text-[11px] font-bold ${m === 60 ? (light ? 'text-[#2563EB]' : 'text-accent') : light ? 'text-[#6B7280]' : 'text-[#333a66]'}`}>{showNames ? pitchClass(m) : ''}{m % 12 === 0 ? Math.floor(m / 12) - 1 : ''}</span>}
          </div>
        );
      })}
      {Array.from({ length: high - low + 1 }, (_, k) => low + k).filter(isBlack).map((m) => {
        const i = whites.findIndex((w) => w > m);
        const c = color(m, true);
        const lbl = showKeys ? keyLabelFor(m) : null;
        return (
          <div key={m} {...handlers(m)} className="absolute top-0 rounded-b-md cursor-pointer z-10 flex flex-col justify-end items-center pb-1 transition-colors duration-75"
            style={{ left: `${i * W - W * 0.32}%`, width: `${W * 0.64}%`, height: '62%', background: c, boxShadow: light ? '0 4px 6px rgba(0,0,0,.35)' : '0 3px 4px rgba(0,0,0,.6)' }}>
            {lbl && <span className="text-[9px] font-bold text-[#9aa0c8]">{lbl}</span>}
            {showNames && <span className="text-[9px] font-bold" style={{ color: highlight.has(m) || down.has(m) ? '#0B1026' : '#9aa0c8' }}>{pitchClass(m)}</span>}
          </div>
        );
      })}
    </div>
  );
}
