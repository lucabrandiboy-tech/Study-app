import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from 'react';
import type { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import { Piece, measureLen, pitchClass } from './notation';
import { RH_COLOR, LH_COLOR } from './Keyboard';

export interface SheetHandle { setBeat: (b: number) => void }

/** Renders MusicXML with OpenSheetMusicDisplay and moves a cursor to follow the current beat. */
export const SheetMusic = forwardRef<SheetHandle, { xml: string; height?: number; zoom?: number }>(function SheetMusic({ xml, height = 260, zoom = 0.9 }, ref) {
  const box = useRef<HTMLDivElement>(null);
  const scroller = useRef<HTMLDivElement>(null);
  const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);
  const cur = useRef(-1);
  const [err, setErr] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    setLoading(true); setErr(null);
    (async () => {
      const { OpenSheetMusicDisplay } = await import('opensheetmusicdisplay');
      if (cancelled || !box.current) return;
      box.current.innerHTML = '';
      const osmd = new OpenSheetMusicDisplay(box.current, {
        autoResize: true, backend: 'svg', drawTitle: false, drawSubtitle: false, drawComposer: false, drawCredits: false, drawPartNames: false,
        autoBeam: true, defaultColorMusic: '#E8ECFF', followCursor: false,
        cursorsOptions: [{ type: 0, color: '#A970FF', alpha: 0.55, follow: false }],
      });
      try {
        await osmd.load(xml);
        if (cancelled) return;
        osmd.zoom = zoom;
        osmd.render();
        osmd.cursor.show();
        osmd.cursor.reset();
        osmdRef.current = osmd;
        cur.current = 0;
      } catch (e) {
        setErr('Could not display this sheet music.');
        console.error(e);
      }
      setLoading(false);
    })();
    return () => { cancelled = true; };
  }, [xml, zoom]);

  useImperativeHandle(ref, () => ({
    setBeat(b: number) {
      const o = osmdRef.current; if (!o) return;
      try {
        const c = o.cursor;
        const ts = () => c.Iterator.currentTimeStamp.RealValue * 4;
        if (b + 1e-3 < ts()) c.reset();
        let guard = 0;
        while (!c.Iterator.EndReached && guard++ < 64) {
          const it = c.Iterator.clone();
          it.moveToNext();
          if (it.EndReached || it.currentTimeStamp.RealValue * 4 > b + 1e-3) break;
          c.next();
        }
        const t = ts();
        if (t !== cur.current) {
          cur.current = t;
          const el = c.cursorElement as HTMLElement | undefined;
          const sc = scroller.current;
          if (el && sc) {
            const top = el.offsetTop - 30;
            if (top < sc.scrollTop || top > sc.scrollTop + sc.clientHeight - el.clientHeight - 20) sc.scrollTo({ top, behavior: 'smooth' });
          }
        }
      } catch { /* ignore cursor errors on odd files */ }
    },
  }), []);

  return (
    <div ref={scroller} className="relative overflow-auto rounded-xl bg-navy/80 border border-edge/30" style={{ height }}>
      {loading && <div className="absolute inset-0 flex items-center justify-center muted">Loading sheet music…</div>}
      {err && <div className="p-4 text-bad">{err}</div>}
      <div ref={box} />
    </div>
  );
});

const isBlack = (m: number) => [1, 3, 6, 8, 10].includes(m % 12);
export function keyX(m: number, low: number, high: number, width: number) {
  while (isBlack(low)) low--;
  while (isBlack(high)) high++;
  let whites = 0;
  for (let k = low; k <= high; k++) if (!isBlack(k)) whites++;
  const W = width / whites;
  let idx = 0;
  for (let k = low; k < m; k++) if (!isBlack(k)) idx++;
  return isBlack(m) ? { x: idx * W - W * 0.32, w: W * 0.64 } : { x: idx * W + 1, w: W - 2 };
}

/** Falling-notes view that lines up with the on-screen keyboard. Draws on a canvas every frame. */
export const FallingNotes = forwardRef<SheetHandle, { piece: Piece; low: number; high: number; activeHands: 'both' | 'R' | 'L'; showNames: boolean; height?: number }>(
  function FallingNotes({ piece, low, high, activeHands, showNames, height = 300 }, ref) {
    const canvas = useRef<HTMLCanvasElement>(null);
    const beatRef = useRef(-1);
    const draw = () => {
      const c = canvas.current; if (!c) return;
      const dpr = window.devicePixelRatio || 1;
      const W = c.clientWidth, H = c.clientHeight;
      if (c.width !== W * dpr) { c.width = W * dpr; c.height = H * dpr; }
      const g = c.getContext('2d')!;
      g.setTransform(dpr, 0, 0, dpr, 0, 0);
      g.clearRect(0, 0, W, H);
      const b = beatRef.current;
      const ppb = 90; // pixels per beat
      // lanes for C and F
      for (let m = low; m <= high; m++) {
        if (m % 12 === 0 || m % 12 === 5) { const k = keyX(m, low, high, W); g.fillStyle = m % 12 === 0 ? 'rgba(127,211,255,.12)' : 'rgba(127,211,255,.05)'; g.fillRect(k.x - 1, 0, 1, H); }
      }
      // measure lines
      const ml = measureLen(piece);
      for (let mb = Math.floor(b / ml) * ml; mb < b + H / ppb; mb += ml) {
        const y = H - (mb - b) * ppb;
        g.fillStyle = 'rgba(169,168,214,.25)'; g.fillRect(0, y, W, 1);
        g.fillStyle = 'rgba(169,168,214,.6)'; g.font = '11px Nunito'; g.fillText(String(Math.round(mb / ml) + 1), 4, y - 3);
      }
      for (const e of piece.events) {
        const yb = H - (e.beat - b) * ppb, h = Math.max(6, e.dur * ppb - 3);
        if (yb < 0) break;
        if (yb - h > H) continue;
        const k = keyX(e.midi, low, high, W);
        const active = activeHands === 'both' || activeHands === e.hand;
        g.globalAlpha = active ? 1 : 0.3;
        g.fillStyle = e.hand === 'R' ? RH_COLOR : LH_COLOR;
        g.shadowColor = g.fillStyle; g.shadowBlur = active ? 8 : 0;
        const r = 5, x = k.x + 1, w = k.w - 2, y = yb - h;
        g.beginPath(); g.roundRect(x, y, w, h, r); g.fill();
        g.shadowBlur = 0;
        if (showNames && h > 14) { g.fillStyle = '#0B1026'; g.font = 'bold 10px Nunito'; g.textAlign = 'center'; g.fillText(pitchClass(e.midi), x + w / 2, yb - 5); g.textAlign = 'start'; }
        g.globalAlpha = 1;
      }
      g.fillStyle = '#A970FF'; g.fillRect(0, H - 2, W, 2);
    };
    useImperativeHandle(ref, () => ({ setBeat(b: number) { beatRef.current = b; draw(); } }));
    useEffect(() => { draw(); const ro = new ResizeObserver(draw); if (canvas.current) ro.observe(canvas.current); return () => ro.disconnect(); });
    return <canvas ref={canvas} className="w-full block bg-navy/80 rounded-t-xl" style={{ height }} />;
  },
);
