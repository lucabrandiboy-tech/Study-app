import { ReactNode } from 'react';

type P = [number, number];
const EDGE = '#7FD3FF', INK = '#E8ECFF', ACC = '#A970FF', MUTED = '#A9A8D6', ORANGE = '#FF9F43';

export function Fig({ children, w = 320, h = 230 }: { children: ReactNode; w?: number; h?: number }) {
  return (
    <svg viewBox={`0 0 ${w} ${h}`} className="w-full max-w-[420px] bg-navy/70 rounded-xl border border-edge/30" style={{ fontFamily: 'Nunito, sans-serif' }}>
      {children}
    </svg>
  );
}

export const T = ({ x, y, children, color = INK, size = 14, anchor = 'middle', bold }: { x: number; y: number; children: ReactNode; color?: string; size?: number; anchor?: 'start' | 'middle' | 'end'; bold?: boolean }) => (
  <text x={x} y={y} fill={color} fontSize={size} textAnchor={anchor} dominantBaseline="middle" fontWeight={bold ? 800 : 600}>{children}</text>
);
export const Seg = ({ a, b, color = EDGE, w = 2, dash }: { a: P; b: P; color?: string; w?: number; dash?: boolean }) => (
  <line x1={a[0]} y1={a[1]} x2={b[0]} y2={b[1]} stroke={color} strokeWidth={w} strokeDasharray={dash ? '5 4' : undefined} strokeLinecap="round" />
);
export const Dot = ({ p, color = ACC, r = 4 }: { p: P; color?: string; r?: number }) => <circle cx={p[0]} cy={p[1]} r={r} fill={color} />;

const sub = (a: P, b: P): P => [a[0] - b[0], a[1] - b[1]];
const len = (v: P) => Math.hypot(v[0], v[1]);
const unit = (v: P): P => { const l = len(v) || 1; return [v[0] / l, v[1] / l]; };
const mid = (a: P, b: P): P => [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
const centroid = (ps: P[]): P => [ps.reduce((s, p) => s + p[0], 0) / ps.length, ps.reduce((s, p) => s + p[1], 0) / ps.length];

/** Label placed just outside a polygon side. */
function sideLabel(a: P, b: P, c: P, text: string, key: string) {
  const m = mid(a, b), out = unit(sub(m, c));
  return <T key={key} x={m[0] + out[0] * 16} y={m[1] + out[1] * 16} color={ORANGE}>{text}</T>;
}
/** Angle arc + label at vertex v between rays to p and q. */
export function AngleMark({ v, p, q, label, r = 22, color = ACC, right }: { v: P; p: P; q: P; label?: string; r?: number; color?: string; right?: boolean }) {
  const u1 = unit(sub(p, v)), u2 = unit(sub(q, v));
  if (right) {
    const s = 12, a: P = [v[0] + u1[0] * s, v[1] + u1[1] * s], b: P = [v[0] + u2[0] * s, v[1] + u2[1] * s], c: P = [a[0] + u2[0] * s, a[1] + u2[1] * s];
    return <path d={`M${a[0]},${a[1]} L${c[0]},${c[1]} L${b[0]},${b[1]}`} stroke={color} fill="none" strokeWidth={1.5} />;
  }
  const s: P = [v[0] + u1[0] * r, v[1] + u1[1] * r], e: P = [v[0] + u2[0] * r, v[1] + u2[1] * r];
  const cross = u1[0] * u2[1] - u1[1] * u2[0];
  const bis = unit([u1[0] + u2[0], u1[1] + u2[1]]);
  return (
    <g>
      <path d={`M${s[0]},${s[1]} A${r},${r} 0 0 ${cross > 0 ? 1 : 0} ${e[0]},${e[1]}`} stroke={color} fill="none" strokeWidth={1.8} />
      {label && <T x={v[0] + bis[0] * (r + 14)} y={v[1] + bis[1] * (r + 14)} color={color} size={13}>{label}</T>}
    </g>
  );
}

export function Polygon({ pts, vLabels, sides, angles, rights, fill = 'rgba(169,112,255,.08)', ticks }: {
  pts: P[]; vLabels?: string[]; sides?: (string | undefined)[]; angles?: (string | undefined)[]; rights?: number[]; fill?: string; ticks?: number[];
}) {
  const c = centroid(pts);
  return (
    <g>
      <polygon points={pts.map((p) => p.join(',')).join(' ')} fill={fill} stroke={EDGE} strokeWidth={2} strokeLinejoin="round" />
      {pts.map((p, i) => {
        const prev = pts[(i - 1 + pts.length) % pts.length], next = pts[(i + 1) % pts.length];
        const out = unit(sub(p, c));
        return (
          <g key={i}>
            {rights?.includes(i) ? <AngleMark v={p} p={prev} q={next} right /> : angles?.[i] ? <AngleMark v={p} p={prev} q={next} label={angles[i]} /> : null}
            {vLabels?.[i] && <T x={p[0] + out[0] * 14} y={p[1] + out[1] * 14} bold>{vLabels[i]}</T>}
            {sides?.[i] && sideLabel(p, next, c, sides[i]!, `s${i}`)}
            {ticks?.[i] ? (() => {
              const m = mid(p, next), d = unit(sub(next, p)), n: P = [-d[1], d[0]];
              return Array.from({ length: ticks[i] }, (_, k) => {
                const o = (k - (ticks[i] - 1) / 2) * 5;
                const cx = m[0] + d[0] * o, cy = m[1] + d[1] * o;
                return <Seg key={k} a={[cx - n[0] * 6, cy - n[1] * 6]} b={[cx + n[0] * 6, cy + n[1] * 6]} color={ACC} w={2} />;
              });
            })() : null}
          </g>
        );
      })}
    </g>
  );
}

/** Rays from a vertex at given angles (degrees, standard position). */
export function Rays({ v = [160, 150] as P, dirs, labels, arcs, lines }: {
  v?: P; dirs: number[]; labels?: string[]; arcs?: { from: number; to: number; label: string; color?: string; right?: boolean; r?: number }[]; lines?: boolean;
}) {
  const L = 115;
  const end = (deg: number): P => [v[0] + Math.cos((deg * Math.PI) / 180) * L, v[1] - Math.sin((deg * Math.PI) / 180) * L];
  return (
    <g>
      {dirs.map((d, i) => {
        const e = end(d);
        return (
          <g key={i}>
            <Seg a={lines ? end(d + 180) : v} b={e} />
            {!lines && <polygon points={arrow(v, e)} fill={EDGE} />}
            {labels?.[i] && <T x={v[0] + (e[0] - v[0]) * 1.1} y={v[1] + (e[1] - v[1]) * 1.1} bold>{labels[i]}</T>}
          </g>
        );
      })}
      {arcs?.map((a, i) => <AngleMark key={i} v={v} p={end(a.from)} q={end(a.to)} label={a.label} color={a.color ?? ACC} right={a.right} r={a.r ?? 26 + i * 6} />)}
      <Dot p={v} />
    </g>
  );
}
function arrow(a: P, b: P) {
  const d = unit(sub(b, a)), n: P = [-d[1], d[0]], s = 8;
  const p1: P = [b[0] - d[0] * s + n[0] * 4, b[1] - d[1] * s + n[1] * 4], p2: P = [b[0] - d[0] * s - n[0] * 4, b[1] - d[1] * s - n[1] * 4];
  return `${b[0]},${b[1]} ${p1[0]},${p1[1]} ${p2[0]},${p2[1]}`;
}

/** Two horizontal parallel lines cut by a transversal; angles numbered 1–8. */
export function ParallelLines({ tilt = 60, marks }: { tilt?: number; marks: Record<number, string> }) {
  const y1 = 75, y2 = 160, cx1 = 150 + (y2 - y1) / 2 / Math.tan((tilt * Math.PI) / 180), cx2 = 150 - (y2 - y1) / 2 / Math.tan((tilt * Math.PI) / 180);
  const P1: P = [cx1, y1], P2: P = [cx2, y2];
  const d = unit(sub(P1, P2));
  const ext = 70;
  // angle positions around each intersection: 1 upper-left,2 upper-right,3 lower-left,4 lower-right (top), 5-8 same at bottom
  const spots = (p: P, base: number) => {
    const up: P = [d[0], d[1]], down: P = [-d[0], -d[1]], left: P = [-1, 0], right: P = [1, 0];
    const pos = (a: P, b: P) => { const u = unit([a[0] + b[0], a[1] + b[1]]); return [p[0] + u[0] * 26, p[1] + u[1] * 26] as P; };
    return [[base + 1, pos(up, left)], [base + 2, pos(up, right)], [base + 3, pos(down, left)], [base + 4, pos(down, right)]] as [number, P][];
  };
  return (
    <Fig>
      <Seg a={[20, y1]} b={[300, y1]} /><Seg a={[20, y2]} b={[300, y2]} />
      <polygon points={arrow([20, y1], [300, y1])} fill={EDGE} /><polygon points={arrow([20, y2], [300, y2])} fill={EDGE} />
      <T x={308} y={y1 - 12} color={MUTED} size={12}>m</T><T x={308} y={y2 - 12} color={MUTED} size={12}>n</T>
      <Seg a={[P1[0] + d[0] * ext, P1[1] + d[1] * ext]} b={[P2[0] - d[0] * ext, P2[1] - d[1] * ext]} color={ACC} />
      <T x={P1[0] + d[0] * (ext + 10)} y={P1[1] + d[1] * (ext + 10)} color={ACC} size={12}>t</T>
      {[...spots(P1, 0), ...spots(P2, 4)].map(([n, p]) => (
        <T key={n} x={p[0]} y={p[1]} size={marks[n] ? 13 : 11} color={marks[n] ? ORANGE : MUTED} bold={!!marks[n]}>{marks[n] ?? n}</T>
      ))}
      <T x={160} y={215} size={11} color={MUTED}>m ∥ n</T>
    </Fig>
  );
}

/** Coordinate grid (static). */
export function Grid({ min = -6, max = 6, points = [], segments = [], polys = [] }: {
  min?: number; max?: number; points?: { x: number; y: number; label?: string; color?: string }[]; segments?: [number, number, number, number][]; polys?: { pts: [number, number][]; color?: string }[];
}) {
  const S = 260, pad = 20, n = max - min, step = S / n;
  const X = (x: number) => pad + (x - min) * step, Y = (y: number) => pad + (max - y) * step;
  return (
    <Fig w={S + pad * 2} h={S + pad * 2}>
      {Array.from({ length: n + 1 }, (_, i) => (
        <g key={i}>
          <line x1={X(min + i)} y1={Y(min)} x2={X(min + i)} y2={Y(max)} stroke="#2a3366" strokeWidth={1} />
          <line x1={X(min)} y1={Y(min + i)} x2={X(max)} y2={Y(min + i)} stroke="#2a3366" strokeWidth={1} />
        </g>
      ))}
      <Seg a={[X(min), Y(0)]} b={[X(max), Y(0)]} color={MUTED} w={1.5} /><Seg a={[X(0), Y(min)]} b={[X(0), Y(max)]} color={MUTED} w={1.5} />
      {polys.map((p, i) => <polygon key={i} points={p.pts.map(([x, y]) => `${X(x)},${Y(y)}`).join(' ')} fill="rgba(169,112,255,.15)" stroke={p.color ?? ACC} strokeWidth={2} />)}
      {segments.map(([a, b, c, d], i) => <Seg key={i} a={[X(a), Y(b)]} b={[X(c), Y(d)]} color={ACC} />)}
      {points.map((p, i) => (
        <g key={i}><Dot p={[X(p.x), Y(p.y)]} color={p.color ?? ORANGE} r={5} />{p.label && <T x={X(p.x) + 10} y={Y(p.y) - 10} size={12} anchor="start" bold>{p.label}</T>}</g>
      ))}
    </Fig>
  );
}

export function CircleFigL({ r = 80, parts, labels }: { r?: number; parts: string[]; labels: Record<string, string> }) {
  const c: P = [160, 115];
  const at = (deg: number): P => [c[0] + Math.cos((deg * Math.PI) / 180) * r, c[1] - Math.sin((deg * Math.PI) / 180) * r];
  const A = at(20), B = at(110), C = at(250), D = at(200);
  return (
    <Fig>
      {parts.includes('sector') && <path d={`M${c[0]},${c[1]} L${A[0]},${A[1]} A${r},${r} 0 0 0 ${B[0]},${B[1]} Z`} fill="rgba(169,112,255,.3)" />}
      <circle cx={c[0]} cy={c[1]} r={r} fill="rgba(169,112,255,.06)" stroke={EDGE} strokeWidth={2} />
      <Dot p={c} /> <T x={c[0] - 10} y={c[1] + 12} size={12}>O</T>
      {parts.includes('radius') && <><Seg a={c} b={A} color={ORANGE} /><T x={(c[0] + A[0]) / 2} y={(c[1] + A[1]) / 2 - 12} color={ORANGE}>{labels.radius ?? 'r'}</T></>}
      {parts.includes('diameter') && <><Seg a={at(160)} b={at(340)} color={ORANGE} /><T x={c[0] + 5} y={c[1] + 26} color={ORANGE}>{labels.diameter ?? 'd'}</T></>}
      {parts.includes('chord') && <><Seg a={D} b={C} color={ORANGE} /><T x={(C[0] + D[0]) / 2 - 18} y={(C[1] + D[1]) / 2 + 4} color={ORANGE}>{labels.chord ?? ''}</T></>}
      {(parts.includes('central') || parts.includes('sector')) && <><Seg a={c} b={A} /><Seg a={c} b={B} /><AngleMark v={c} p={A} q={B} label={labels.central} r={24} /><Dot p={A} color={EDGE} /><Dot p={B} color={EDGE} /><T x={A[0] + 12} y={A[1]} bold>A</T><T x={B[0] - 8} y={B[1] - 12} bold>B</T></>}
      {parts.includes('inscribed') && <><Seg a={C} b={A} color={ACC} /><Seg a={C} b={B} color={ACC} /><AngleMark v={C} p={A} q={B} label={labels.inscribed} color={ORANGE} r={30} /><T x={C[0]} y={C[1] + 14} bold>C</T><T x={A[0] + 12} y={A[1]} bold>A</T><T x={B[0] - 8} y={B[1] - 12} bold>B</T></>}
      {parts.includes('arc') && <T x={(A[0] + B[0]) / 2 + 22} y={(A[1] + B[1]) / 2 - 22} color={ORANGE}>{labels.arc}</T>}
      {parts.includes('tangent') && (() => { const T0 = at(-40), d = unit([Math.sin((-40 * Math.PI) / 180), Math.cos((-40 * Math.PI) / 180)]); return <><Seg a={[T0[0] - d[0] * 90, T0[1] - d[1] * 90]} b={[T0[0] + d[0] * 60, T0[1] + d[1] * 60]} color={ACC} /><Seg a={c} b={T0} dash /><AngleMark v={T0} p={c} q={[T0[0] + d[0] * 30, T0[1] + d[1] * 30]} right /></>; })()}
    </Fig>
  );
}

export function Solid({ kind, labels }: { kind: 'prism' | 'cylinder' | 'cone' | 'sphere' | 'pyramid' | 'cube'; labels: Record<string, string> }) {
  const lab = (x: number, y: number, k: string) => labels[k] ? <T x={x} y={y} color={ORANGE}>{labels[k]}</T> : null;
  return (
    <Fig>
      {(kind === 'prism' || kind === 'cube') && (
        <g fill="none" stroke={EDGE} strokeWidth={2}>
          <rect x={70} y={90} width={140} height={100} fill="rgba(169,112,255,.08)" />
          <path d="M70,90 L120,50 L260,50 L210,90 M260,50 L260,150 L210,190" />
          <path d="M70,190 L120,150 L260,150 M120,150 L120,50" strokeDasharray="5 4" opacity={0.5} />
          {lab(140, 205, 'l')}{lab(248, 178, 'w')}{lab(52, 140, 'h')}
        </g>
      )}
      {kind === 'cylinder' && (
        <g fill="none" stroke={EDGE} strokeWidth={2}>
          <ellipse cx={160} cy={55} rx={70} ry={20} fill="rgba(169,112,255,.12)" />
          <path d="M90,55 L90,175 M230,55 L230,175" />
          <path d="M90,175 A70,20 0 0 0 230,175" /><path d="M90,175 A70,20 0 0 1 230,175" strokeDasharray="5 4" opacity={0.5} />
          <Seg a={[160, 55]} b={[230, 55]} color={ORANGE} />{lab(195, 42, 'r')}{lab(248, 115, 'h')}
        </g>
      )}
      {kind === 'cone' && (
        <g fill="none" stroke={EDGE} strokeWidth={2}>
          <path d="M160,30 L90,175 M160,30 L230,175" /><path d="M90,175 A70,20 0 0 0 230,175" /><path d="M90,175 A70,20 0 0 1 230,175" strokeDasharray="5 4" opacity={0.5} />
          <Seg a={[160, 30]} b={[160, 175]} dash color={MUTED} /><Seg a={[160, 175]} b={[230, 175]} color={ORANGE} />
          {lab(195, 190, 'r')}{lab(148, 110, 'h')}{lab(212, 95, 'l')}
        </g>
      )}
      {kind === 'sphere' && (
        <g fill="none" stroke={EDGE} strokeWidth={2}>
          <circle cx={160} cy={115} r={85} fill="rgba(169,112,255,.08)" /><ellipse cx={160} cy={115} rx={85} ry={22} strokeDasharray="5 4" opacity={0.6} />
          <Seg a={[160, 115]} b={[245, 115]} color={ORANGE} />{lab(202, 102, 'r')}
        </g>
      )}
      {kind === 'pyramid' && (
        <g fill="none" stroke={EDGE} strokeWidth={2}>
          <path d="M80,180 L200,180 L250,140 M80,180 L160,40 L200,180 M160,40 L250,140" /><path d="M80,180 L130,140 L250,140 M130,140 L160,40" strokeDasharray="5 4" opacity={0.5} />
          <Seg a={[160, 40]} b={[165, 160]} dash color={MUTED} />{lab(140, 195, 's')}{lab(176, 110, 'h')}
        </g>
      )}
    </Fig>
  );
}

/** A number line segment with labeled points (for distance/midpoint in 1D). */
export function NumberLine({ min, max, points }: { min: number; max: number; points: { v: number; label: string }[] }) {
  const X = (v: number) => 20 + ((v - min) / (max - min)) * 280;
  return (
    <Fig h={90}>
      <Seg a={[10, 45]} b={[310, 45]} />
      {Array.from({ length: max - min + 1 }, (_, i) => min + i).map((v) => (
        <g key={v}><Seg a={[X(v), 40]} b={[X(v), 50]} color={MUTED} w={1} />{(max - min <= 20 || v % 2 === 0) && <T x={X(v)} y={64} size={10} color={MUTED}>{v}</T>}</g>
      ))}
      {points.map((p) => <g key={p.label}><Dot p={[X(p.v), 45]} color={ORANGE} r={5} /><T x={X(p.v)} y={24} bold>{p.label}</T></g>)}
    </Fig>
  );
}

export { EDGE, INK, ACC, MUTED, ORANGE };
export type { P };
