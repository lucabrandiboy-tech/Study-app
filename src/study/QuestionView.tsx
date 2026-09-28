import { useEffect, useState } from 'react';
import type { Question } from './types';
import { parseNum, close } from './rand';
import { Rich } from '../components/ui';
import { openChat } from '../lib/store';

export interface Graded { correct: boolean; given: string; feedback: string }

export function grade(q: Question, raw: string, sel: number | null, point: [number, number] | null, proof: string[]): Graded {
  const a = q.answer;
  switch (a.kind) {
    case 'number': {
      const v = parseNum(raw);
      if (isNaN(v)) return { correct: false, given: raw, feedback: "I couldn't read that as a number. Try something like 12, 3.5, 3/4, or 5√2." };
      if (close(v, a.value)) return { correct: true, given: raw, feedback: 'Correct!' };
      const m = a.mistakes?.find((x) => close(v, x.value));
      if (m) return { correct: false, given: raw, feedback: m.msg };
      const t = a.value;
      const why = t !== 0 && close(-v, t) ? 'You have the right size but the wrong sign (+/−). Check where a negative sign should flip.'
        : t !== 0 && (close(v, t * 10) || close(v, t / 10) || close(v, t * 100) || close(v, t / 100)) ? 'The digits look right, but the decimal point is in the wrong place.'
        : Math.abs(v - t) <= Math.max(0.6, Math.abs(t) * 0.02) ? 'Very close! This looks like a rounding slip. Check how many decimal places the question asks for, and round only at the end.'
        : `Your answer is too ${v > t ? 'high' : 'low'}. Re-check each step, especially signs, squaring, and which formula applies.`;
      return { correct: false, given: raw, feedback: why };
    }
    case 'choice': {
      if (sel === null) return { correct: false, given: '', feedback: 'Pick an answer first.' };
      const ok = sel === a.correct;
      return { correct: ok, given: a.choices[sel], feedback: ok ? 'Correct!' : a.why?.[sel] || `"${a.choices[sel]}" isn't right. Compare it carefully with the key idea in the hint.` };
    }
    case 'point': {
      let p = point;
      if (!p && raw) {
        const m = raw.match(/\(?\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*\)?/);
        if (m) p = [Number(m[1]), Number(m[2])];
      }
      if (!p) return { correct: false, given: raw, feedback: 'Click a point on the grid or type it like (2, -3).' };
      const ok = p[0] === a.x && p[1] === a.y;
      const swapped = p[0] === a.y && p[1] === a.x && a.x !== a.y;
      const signs = Math.abs(p[0]) === Math.abs(a.x) && Math.abs(p[1]) === Math.abs(a.y) && !ok;
      return { correct: ok, given: `(${p[0]}, ${p[1]})`, feedback: ok ? 'Correct!' : swapped ? 'Your x and y look swapped. Remember (x, y) — x is left/right first.' : signs ? 'The numbers are right but a sign is off. Check which coordinate should change sign.' : 'Not quite. Apply the rule step by step to x and y.' };
    }
    case 'proof': {
      const wrong = a.steps.map((s, i) => (proof[i] === s.reason ? -1 : i)).filter((i) => i >= 0);
      return { correct: wrong.length === 0, given: proof.join(' | '), feedback: wrong.length === 0 ? 'Every reason is correct!' : `Check step${wrong.length > 1 ? 's' : ''} ${wrong.map((i) => i + 1).join(', ')}. Ask: is it given, a definition, a property, or a theorem?` };
    }
    case 'text': {
      const ok = a.accept.some((x) => x.toLowerCase().trim() === raw.toLowerCase().trim());
      return { correct: ok, given: raw, feedback: ok ? 'Correct!' : 'Not quite.' };
    }
  }
}

/** The correct answer written out in words. */
export function correctAnswerText(q: Question): string {
  const a = q.answer;
  switch (a.kind) {
    case 'number': { const v = Math.round(a.value * 100) / 100; return `${v}${a.unit ? ` ${a.unit}` : ''}`; }
    case 'choice': return a.choices[a.correct];
    case 'point': return `(${a.x}, ${a.y})`;
    case 'proof': return a.steps.map((st, i) => `${i + 1}. ${st.reason}`).join('; ');
    case 'text': return a.accept[0];
  }
}

export function ClickGrid({ min = -6, max = 6, value, onPick, shown = [], answer, disabled }: {
  min?: number; max?: number; value: [number, number] | null; onPick: (p: [number, number]) => void; shown?: { x: number; y: number; label: string }[]; answer?: { x: number; y: number } | null; disabled?: boolean;
}) {
  const S = 300, pad = 20, n = max - min, step = S / n;
  const X = (x: number) => pad + (x - min) * step, Y = (y: number) => pad + (max - y) * step;
  const [hover, setHover] = useState<[number, number] | null>(null);
  const toPt = (e: React.MouseEvent<SVGSVGElement>): [number, number] => {
    const r = e.currentTarget.getBoundingClientRect();
    const sx = ((e.clientX - r.left) / r.width) * (S + 2 * pad), sy = ((e.clientY - r.top) / r.height) * (S + 2 * pad);
    return [Math.max(min, Math.min(max, Math.round((sx - pad) / step + min))), Math.max(min, Math.min(max, Math.round(max - (sy - pad) / step)))];
  };
  return (
    <svg viewBox={`0 0 ${S + 2 * pad} ${S + 2 * pad}`} className={`w-[340px] bg-navy/70 rounded-xl border border-edge/40 ${disabled ? '' : 'cursor-crosshair'}`}
      onMouseMove={(e) => !disabled && setHover(toPt(e))} onMouseLeave={() => setHover(null)} onClick={(e) => !disabled && onPick(toPt(e))}>
      {Array.from({ length: n + 1 }, (_, i) => (
        <g key={i}>
          <line x1={X(min + i)} y1={Y(min)} x2={X(min + i)} y2={Y(max)} stroke="#2a3366" />
          <line x1={X(min)} y1={Y(min + i)} x2={X(max)} y2={Y(min + i)} stroke="#2a3366" />
          {(min + i) % 2 === 0 && min + i !== 0 && <text x={X(min + i)} y={Y(0) + 12} fontSize={9} fill="#A9A8D6" textAnchor="middle">{min + i}</text>}
          {(min + i) % 2 === 0 && min + i !== 0 && <text x={X(0) - 8} y={Y(min + i) + 3} fontSize={9} fill="#A9A8D6" textAnchor="middle">{min + i}</text>}
        </g>
      ))}
      <line x1={X(min)} y1={Y(0)} x2={X(max)} y2={Y(0)} stroke="#A9A8D6" strokeWidth={1.5} />
      <line x1={X(0)} y1={Y(min)} x2={X(0)} y2={Y(max)} stroke="#A9A8D6" strokeWidth={1.5} />
      {shown.map((p) => <g key={p.label}><circle cx={X(p.x)} cy={Y(p.y)} r={5} fill="#FF9F43" /><text x={X(p.x) + 8} y={Y(p.y) - 8} fill="#E8ECFF" fontSize={13} fontWeight={800}>{p.label}</text></g>)}
      {hover && !disabled && <circle cx={X(hover[0])} cy={Y(hover[1])} r={5} fill="none" stroke="#7FD3FF" />}
      {value && <g><circle cx={X(value[0])} cy={Y(value[1])} r={6} fill="#A970FF" /><text x={X(value[0]) + 8} y={Y(value[1]) + 16} fill="#A970FF" fontSize={12} fontWeight={700}>({value[0]}, {value[1]})</text></g>}
      {answer && <circle cx={X(answer.x)} cy={Y(answer.y)} r={8} fill="none" stroke="#4ADE80" strokeWidth={3} />}
    </svg>
  );
}

/** Shows one question with the right input type. In practice mode: hints + instant feedback. In quiz mode: submit only. */
export function QuestionView({ q, mode, onDone, context }: { q: Question; mode: 'practice' | 'quiz'; onDone: (g: Graded) => void; context?: string }) {
  const [raw, setRaw] = useState('');
  const [sel, setSel] = useState<number | null>(null);
  const [point, setPoint] = useState<[number, number] | null>(null);
  const [proof, setProof] = useState<string[]>([]);
  const [hintsShown, setHintsShown] = useState(0);
  const [result, setResult] = useState<Graded | null>(null);
  const [tries, setTries] = useState(0);

  useEffect(() => { setRaw(''); setSel(null); setPoint(null); setProof([]); setHintsShown(0); setResult(null); setTries(0); }, [q]);

  const a = q.answer;
  const submit = () => {
    const g = grade(q, raw, sel, point, proof);
    if (mode === 'quiz') { onDone(g); return; }
    if (!g.correct && (g.feedback.startsWith("I couldn't") || g.feedback.startsWith('Pick') || g.feedback.startsWith('Click'))) { setResult(g); return; }
    const t = tries + 1;
    // A second try is allowed, but only a first-try correct answer counts toward the 5-in-a-row streak.
    const shown = g.correct && t > 1 ? { ...g, feedback: 'Correct on the second try! (Only first tries count toward mastery.)' } : g;
    setResult(shown);
    setTries(t);
    if (g.correct || t >= 2) onDone({ ...shown, correct: g.correct && t === 1 });
  };
  const locked = mode === 'practice' && result !== null && (result.correct || tries >= 2);
  const canRetry = mode === 'practice' && result && !result.correct && tries < 2 && !locked;

  return (
    <div className="space-y-4">
      <Rich text={q.prompt} className="text-lg font-semibold" />
      <div className="flex gap-6 items-start flex-wrap">
        {q.diagram && <div className="w-[400px]">{q.diagram}</div>}
        <div className="flex-1 min-w-[320px] space-y-3">
          {a.kind === 'number' || a.kind === 'text' ? (
            <input className="input w-64 text-lg" value={raw} disabled={locked} placeholder="Your answer" autoFocus
              onChange={(e) => setRaw(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && raw && !locked && submit()} />
          ) : null}
          {a.kind === 'choice' && (
            <div className="grid gap-2">
              {a.choices.map((c, i) => {
                const show = locked || (mode === 'practice' && result && i === sel);
                const col = show && i === a.correct && locked ? 'border-good bg-good/10' : show && i === sel && !result?.correct ? 'border-bad bg-bad/10' : sel === i ? 'border-accent bg-accent/15' : 'border-edge/30';
                return (
                  <button key={i} disabled={locked} onClick={() => setSel(i)} className={`text-left px-4 py-2.5 rounded-xl border-2 transition hover:border-edge ${col}`}>
                    <span className="font-bold text-edge mr-2">{String.fromCharCode(65 + i)}.</span>{c}
                  </button>
                );
              })}
            </div>
          )}
          {a.kind === 'point' && (
            <div className="space-y-2">
              <ClickGrid min={a.grid?.min} max={a.grid?.max} value={point} onPick={(p) => { setPoint(p); setRaw(''); }} shown={a.shown} answer={locked ? a : null} disabled={locked} />
              <input className="input w-48" placeholder="or type (x, y)" value={raw} disabled={locked} onChange={(e) => { setRaw(e.target.value); setPoint(null); }} />
            </div>
          )}
          {a.kind === 'proof' && (
            <table className="w-full text-sm">
              <thead><tr className="text-muted text-left"><th className="p-2">#</th><th className="p-2">Statement</th><th className="p-2">Reason</th></tr></thead>
              <tbody>
                {a.steps.map((s, i) => {
                  const ok = locked && proof[i] === s.reason;
                  return (
                    <tr key={i} className="border-t border-edge/20">
                      <td className="p-2 text-muted">{i + 1}</td>
                      <td className="p-2">{s.statement}</td>
                      <td className="p-2">
                        <select className={`input w-full ${locked ? (ok ? 'border-good' : 'border-bad') : ''}`} disabled={locked} value={proof[i] ?? ''}
                          onChange={(e) => { const p = [...proof]; p[i] = e.target.value; setProof(p); }}>
                          <option value="">— choose a reason —</option>
                          {a.reasonBank.map((r) => <option key={r}>{r}</option>)}
                        </select>
                        {locked && !ok && <div className="text-good text-xs mt-1">✓ {s.reason}</div>}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )}

          <div className="flex gap-2 flex-wrap">
            {!locked && <button className="btn" onClick={submit}>{mode === 'quiz' ? 'Submit' : canRetry ? 'Try again' : 'Check my work'}</button>}
            {mode === 'practice' && !locked && hintsShown < q.hints.length && (
              <button className="btn-ghost" onClick={() => setHintsShown((h) => h + 1)}>💡 Hint {hintsShown + 1}/{q.hints.length}</button>
            )}
            {mode === 'practice' && (
              <button className="btn-ghost" onClick={() => openChat(`I'm stuck on this practice problem: "${q.prompt}"${context ? ` (${context})` : ''}. Can you give me a hint without the answer?`)}>🤖 Ask Study Buddy</button>
            )}
          </div>

          {mode === 'practice' && hintsShown > 0 && (
            <div className="space-y-2">
              {q.hints.slice(0, hintsShown).map((h, i) => (
                <div key={i} className="animate-pop rounded-xl border border-streak/50 bg-streak/10 px-3 py-2 text-sm"><b className="text-streak">Hint {i + 1}:</b> {h}</div>
              ))}
            </div>
          )}

          {mode === 'practice' && result && (
            <div className={`animate-pop rounded-xl border-2 px-4 py-3 ${result.correct ? 'border-good bg-good/10' : 'border-bad bg-bad/10'}`}>
              <div className={`font-bold ${result.correct ? 'text-good' : 'text-bad'}`}>{result.correct ? '✓ ' : '✗ '}{result.feedback}</div>
              {canRetry && <div className="text-sm muted mt-1">You get one more try. Use a hint if you need one.</div>}
              {locked && !result.correct && (
                <div className="mt-3 rounded-lg bg-navy/60 border border-edge/30 p-3 space-y-1.5 text-sm">
                  <div className="font-bold text-edge">📖 Here's why</div>
                  {result.given && <div><span className="muted">You answered:</span> <span className="text-bad">{result.given}</span> — {result.feedback}</div>}
                  <div><span className="muted">Correct answer:</span> <b className="text-good">{correctAnswerText(q)}</b></div>
                  <Rich text={`**Step by step:** ${q.explanation}`} />
                  <div className="muted text-xs">Saved to My Mistakes so you can review it later.</div>
                </div>
              )}
              {locked && result.correct && <Rich text={`**Solution:** ${q.explanation}`} className="text-sm mt-2" />}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
