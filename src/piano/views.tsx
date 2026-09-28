import { forwardRef, useEffect, useImperativeHandle, useRef, useState } from 'react';
import type { OpenSheetMusicDisplay } from 'opensheetmusicdisplay';
import { Piece, measureLen, pitchClass } from './notation';
import { RH_COLOR, LH_COLOR } from './Keyboard';

export interface SheetHandle {
  setBeat: (b: number) => void;
  /** Color one note on the sheet (e.g. green when played, red when missed). */
  colorNote?: (beat: number, midi: number, color: string) => void;
  resetColors?: () => void;
}

type GNote = { sourceNote?: { halfTone: number; isRest(): boolean }; getSVGGElement?: () => SVGGElement };

/** Map "beat:midi" → the SVG groups of those notes, by walking the OSMD cursor once through the score. */
function buildNoteMap(osmd: OpenSheetMusicDisplay) {
  const map = new Map<string, SVGGElement[]>();
  const c = osmd.cursor;
  c.reset();
  let guard = 0;
  while (!c.Iterator.EndReached && guard++ < 20000) {
    const beat = c.Iterator.currentTimeStamp.RealValue * 4;
    for (const g of c.GNotesUnderCursor() as unknown as GNote[]) {
      const n = g.sourceNote;
      if (!n || n.isRest()) continue;
      const el = g.getSVGGElement?.();
      if (!el) continue;
      const k = `${beat.toFixed(3)}:${n.halfTone + 12}`; // OSMD halfTone + 12 = MIDI number
      map.set(k, [...(map.get(k) ?? []), el]);
    }
    c.next();
  }
  c.reset();
  return map;
}
function paint(el: SVGGElement, color: string) {
  el.querySelectorAll('path, ellipse, rect').forEach((x) => {
    const f = x.getAttribute('fill'), st = x.getAttribute('stroke');
    if (f !== 'none') x.setAttribute('fill', color);
    if (st && st !== 'none') x.setAttribute('stroke', color);
  });
}

/** Renders MusicXML with OpenSheetMusicDisplay and moves a cursor to follow the current beat. */
export const SheetMusic = forwardRef<SheetHandle, { xml: string; height?: number; zoom?: number; light?: boolean; singleLine?: boolean }>(
  function SheetMusic({ xml, height = 260, zoom = 0.9, light = false, singleLine = false }, ref) {
    const box = useRef<HTMLDivElement>(null);
    const scroller = useRef<HTMLDivElement>(null);
    const osmdRef = useRef<OpenSheetMusicDisplay | null>(null);
    const notes = useRef(new Map<string, SVGGElement[]>());
    const cur = useRef(-1);
    const [err, setErr] = useState<string | null>(null);
    const [loading, setLoading] = useState(true);
    const ink = light ? '#111827' : '#E8ECFF';

    useEffect(() => {
      let cancelled = false;
      setLoading(true); setErr(null);
      (async () => {
        const { OpenSheetMusicDisplay } = await import('opensheetmusicdisplay');
        if (cancelled || !box.current) return;
        box.current.innerHTML = '';
        const osmd = new OpenSheetMusicDisplay(box.current, {
          autoResize: true, backend: 'svg', drawTitle: false, drawSubtitle: false, drawComposer: false, drawCredits: false, drawPartNames: false,
          autoBeam: true, defaultColorMusic: ink, followCursor: false, renderSingleHorizontalStaffline: singleLine,
          cursorsOptions: [{ type: 0, color: light ? '#22C55E' : '#A970FF', alpha: light ? 0.35 : 0.55, follow: false }],
        });
        try {
          await osmd.load(xml);
          if (cancelled) return;
          osmd.zoom = zoom;
          osmd.render();
          try { notes.current = buildNoteMap(osmd); } catch { notes.current = new Map(); }
          osmd.cursor.show();
          osmd.cursor.reset();
          osmdRef.current = osmd;
          cur.current = 0;
          if (scroller.current) scroller.current.scrollLeft = 0;
        } catch (e) {
          setErr('Could not display this sheet music.');
          console.error(e);
        }
        setLoading(false);
      })();
      return () => { cancelled = true; };
    }, [xml, zoom, light, singleLine]); // eslint-disable-line react-hooks/exhaustive-deps

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
              if (singleLine) {
                const left = el.offsetLeft - sc.clientWidth * 0.3;
                if (Math.abs(left - sc.scrollLeft) > 8) sc.scrollTo({ left: Math.max(0, left), behavior: 'smooth' });
              } else {
                const top = el.offsetTop - 30;
                if (top < sc.scrollTop || top > sc.scrollTop + sc.clientHeight - el.clientHeight - 20) sc.scrollTo({ top, behavior: 'smooth' });
              }
            }
          }
        } catch { /* ignore cursor errors on odd files */ }
      },
      colorNote(beat: number, midi: number, color: string) {
        notes.current.get(`${beat.toFixed(3)}:${midi}`)?.forEach((el) => paint(el, color));
      },
      resetColors() {
        notes.current.forEach((els) => els.forEach((el) => paint(el, ink)));
      },
    }), [singleLine, ink]);

    return (
      <div ref={scroller} className={`relative overflow-auto rounded-xl ${light ? 'bg-white' : 'bg-navy/80 border border-edge/30'}`} style={{ height }}>
        {loading && <div className={`absolute inset-0 flex items-center justify-center ${light ? 'text-gray-400' : 'muted'}`}>Loading sheet music…</div>}
        {err && <div className="p-4 text-bad">{err}</div>}
        <div ref={box} />
      </div>
    );
  },
);

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
export const FallingNotes = forwardRef<SheetHandle, { piece: Piece; low: number; high: number; activeHands: 'both' | 'R' | 'L'; showNames: boolean; height?: number; light?: boolean }>(
  function FallingNotes({ piece, low, high, activeHands, showNames, height = 300, light = false }, ref) {
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
        if (m % 12 === 0 || m % 12 === 5) { const k = keyX(m, low, high, W); g.fillStyle = light ? (m % 12 === 0 ? 'rgba(17,24,39,.10)' : 'rgba(17,24,39,.04)') : m % 12 === 0 ? 'rgba(127,211,255,.12)' : 'rgba(127,211,255,.05)'; g.fillRect(k.x - 1, 0, 1, H); }
      }
      // measure lines
      const ml = measureLen(piece);
      for (let mb = Math.floor(b / ml) * ml; mb < b + H / ppb; mb += ml) {
        const y = H - (mb - b) * ppb;
        g.fillStyle = light ? 'rgba(17,24,39,.12)' : 'rgba(169,168,214,.25)'; g.fillRect(0, y, W, 1);
        g.fillStyle = light ? 'rgba(17,24,39,.45)' : 'rgba(169,168,214,.6)'; g.font = '11px Nunito'; g.fillText(String(Math.round(mb / ml) + 1), 4, y - 3);
      }
      for (const e of piece.events) {
        const yb = H - (e.beat - b) * ppb, h = Math.max(6, e.dur * ppb - 3);
        if (yb < 0) break;
        if (yb - h > H) continue;
        const k = keyX(e.midi, low, high, W);
        const active = activeHands === 'both' || activeHands === e.hand;
        g.globalAlpha = active ? 1 : 0.3;
        g.fillStyle = e.hand === 'R' ? (light ? '#3B82F6' : RH_COLOR) : light ? '#F59E0B' : LH_COLOR;
        g.shadowColor = g.fillStyle; g.shadowBlur = active && !light ? 8 : 0;
        const r = 5, x = k.x + 1, w = k.w - 2, y = yb - h;
        g.beginPath(); g.roundRect(x, y, w, h, r); g.fill();
        g.shadowBlur = 0;
        if (showNames && h > 14) { g.fillStyle = light ? '#FFFFFF' : '#0B1026'; g.font = 'bold 10px Nunito'; g.textAlign = 'center'; g.fillText(pitchClass(e.midi), x + w / 2, yb - 5); g.textAlign = 'start'; }
        g.globalAlpha = 1;
      }
      g.fillStyle = light ? '#22C55E' : '#A970FF'; g.fillRect(0, H - 2, W, 2);
    };
    useImperativeHandle(ref, () => ({ setBeat(b: number) { beatRef.current = b; draw(); } }));
    useEffect(() => { draw(); const ro = new ResizeObserver(draw); if (canvas.current) ro.observe(canvas.current); return () => ro.disconnect(); });
    return <canvas ref={canvas} className={`w-full block rounded-t-xl ${light ? 'bg-[#F8FAFC]' : 'bg-navy/80'}`} style={{ height }} />;
  },
);
