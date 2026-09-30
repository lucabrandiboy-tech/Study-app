// Geometry side tools: an openable notepad and a TI-84–style calculator.
import { useEffect, useRef, useState } from 'react';
import { useLocation } from 'react-router-dom';

// ---------------- calculator engine (safe parser, no eval) ----------------
type Tok = { t: 'num'; v: number } | { t: 'id'; v: string } | { t: 'op'; v: string };
const FUNCS = ['asin', 'acos', 'atan', 'sin', 'cos', 'tan', 'sqrt', 'log', 'ln', 'abs'];

function tokenize(src: string): Tok[] {
  const s = src.replace(/\s+/g, '').replace(/×/g, '*').replace(/÷/g, '/').replace(/−/g, '-').replace(/π/g, 'pi').replace(/√/g, 'sqrt').replace(/sin⁻¹/g, 'asin').replace(/cos⁻¹/g, 'acos').replace(/tan⁻¹/g, 'atan').replace(/²/g, '^2').replace(/⁻¹/g, '^(-1)').replace(/⁻/g, '~');
  const out: Tok[] = [];
  let i = 0;
  while (i < s.length) {
    const c = s[i];
    const num = s.slice(i).match(/^(\d+\.?\d*|\.\d+)(E-?\d+)?/);
    if (num) { out.push({ t: 'num', v: parseFloat(num[0]) }); i += num[0].length; continue; }
    const id = s.slice(i).match(/^(asin|acos|atan|sin|cos|tan|sqrt|log|ln|abs|pi|ans|e)/i);
    if (id) { out.push({ t: 'id', v: id[0].toLowerCase() }); i += id[0].length; continue; }
    if ('+-*/^()!~,'.includes(c)) { out.push({ t: 'op', v: c }); i++; continue; }
    throw new Error('SYNTAX');
  }
  // implicit multiplication: 2pi, 3(4), )(, 2sin(30)
  const res: Tok[] = [];
  out.forEach((tk, k) => {
    const prev = out[k - 1];
    const prevEnds = prev && (prev.t === 'num' || (prev.t === 'id' && !FUNCS.includes(prev.v)) || (prev.t === 'op' && (prev.v === ')' || prev.v === '!')));
    const starts = tk.t === 'num' || tk.t === 'id' || (tk.t === 'op' && tk.v === '(');
    if (prevEnds && starts) res.push({ t: 'op', v: '*' });
    res.push(tk);
  });
  return res;
}

function evaluate(src: string, ans: number, deg: boolean): number {
  const toks = tokenize(src);
  let p = 0;
  const peek = () => toks[p];
  const eat = (v?: string) => { const t = toks[p++]; if (!t || (v && t.v !== v)) throw new Error('SYNTAX'); return t; };
  const toRad = (x: number) => (deg ? (x * Math.PI) / 180 : x);
  const fromRad = (x: number) => (deg ? (x * 180) / Math.PI : x);
  const fact = (n: number) => { if (n < 0 || !Number.isInteger(n) || n > 170) throw new Error('DOMAIN'); let r = 1; for (let k = 2; k <= n; k++) r *= k; return r; };
  function expr(): number { let v = term(); while (peek()?.t === 'op' && (peek().v === '+' || peek().v === '-')) { const o = eat().v; const r = term(); v = o === '+' ? v + r : v - r; } return v; }
  function term(): number { let v = unary(); while (peek()?.t === 'op' && (peek().v === '*' || peek().v === '/')) { const o = eat().v; const r = unary(); if (o === '/' && r === 0) throw new Error('DIVIDE BY 0'); v = o === '*' ? v * r : v / r; } return v; }
  function unary(): number { if (peek()?.t === 'op' && (peek().v === '~' || peek().v === '-')) { eat(); return -unary(); } return power(); }
  function power(): number { const b = postfix(); if (peek()?.t === 'op' && peek().v === '^') { eat(); return Math.pow(b, unary()); } return b; }
  function postfix(): number { let v = atom(); while (peek()?.t === 'op' && peek().v === '!') { eat(); v = fact(v); } return v; }
  function atom(): number {
    const t = eat();
    if (t.t === 'num') return t.v;
    if (t.t === 'op' && t.v === '(') { const v = expr(); if (peek()?.v === ')') eat(')'); return v; }
    if (t.t === 'id') {
      if (t.v === 'pi') return Math.PI;
      if (t.v === 'e') return Math.E;
      if (t.v === 'ans') return ans;
      let x: number;
      if (peek()?.v === '(') { eat('('); x = expr(); if (peek()?.v === ')') eat(')'); } else x = power();
      switch (t.v) {
        case 'sin': return Math.sin(toRad(x));
        case 'cos': return Math.cos(toRad(x));
        case 'tan': { if (deg && Math.abs(((x % 180) + 180) % 180 - 90) < 1e-12) throw new Error('DOMAIN'); return Math.tan(toRad(x)); }
        case 'asin': if (Math.abs(x) > 1) throw new Error('DOMAIN'); return fromRad(Math.asin(x));
        case 'acos': if (Math.abs(x) > 1) throw new Error('DOMAIN'); return fromRad(Math.acos(x));
        case 'atan': return fromRad(Math.atan(x));
        case 'sqrt': if (x < 0) throw new Error('NONREAL ANS'); return Math.sqrt(x);
        case 'log': if (x <= 0) throw new Error('DOMAIN'); return Math.log10(x);
        case 'ln': if (x <= 0) throw new Error('DOMAIN'); return Math.log(x);
        case 'abs': return Math.abs(x);
      }
    }
    throw new Error('SYNTAX');
  }
  const v = expr();
  if (p < toks.length) throw new Error('SYNTAX');
  if (!Number.isFinite(v)) throw new Error('OVERFLOW');
  return v;
}

const fmt = (v: number) => {
  if (Math.abs(v) < 1e-10) return '0';
  const r = parseFloat(v.toPrecision(10));
  return Math.abs(r) >= 1e10 || Math.abs(r) < 1e-4 ? r.toExponential(6).replace('e', 'E') : String(r);
};

// ---------------- UI ----------------
type Key = { l: string; ins?: string; act?: string; cls?: string; second?: { l: string; ins: string } };
const K = (l: string, ins = l, cls = '', second?: Key['second']): Key => ({ l, ins, cls, second });
const A = (l: string, act: string, cls = ''): Key => ({ l, act, cls });
const FN = 'bg-[#3a3f47] text-white', NUM = 'bg-[#e8e8e8] text-black font-bold', OP = 'bg-[#3a3f47] text-white', BLUE = 'bg-[#4a90d9] text-white', GREEN = 'bg-[#51b56b] text-white';
const KEYS: Key[][] = [
  [A('2nd', 'second', BLUE), A('MODE', 'mode', FN), A('DEL', 'del', FN), A('CLEAR', 'clear', FN), A('ANS', 'ans', FN)],
  [K('x⁻¹', '^(-1)', FN, { l: '!', ins: '!' }), K('sin', 'sin(', FN, { l: 'sin⁻¹', ins: 'sin⁻¹(' }), K('cos', 'cos(', FN, { l: 'cos⁻¹', ins: 'cos⁻¹(' }), K('tan', 'tan(', FN, { l: 'tan⁻¹', ins: 'tan⁻¹(' }), K('^', '^', OP, { l: 'π', ins: 'π' })],
  [K('x²', '²', FN, { l: '√', ins: '√(' }), K(',', ',', FN), K('(', '(', FN), K(')', ')', FN), K('÷', '÷', OP, { l: 'e', ins: 'e' })],
  [K('log', 'log(', FN, { l: '10^', ins: '10^(' }), K('7', '7', NUM), K('8', '8', NUM), K('9', '9', NUM), K('×', '×', OP)],
  [K('ln', 'ln(', FN, { l: 'e^', ins: 'e^(' }), K('4', '4', NUM), K('5', '5', NUM), K('6', '6', NUM), K('−', '−', OP)],
  [K('√', '√(', FN, { l: 'abs', ins: 'abs(' }), K('1', '1', NUM), K('2', '2', NUM), K('3', '3', NUM), K('+', '+', OP)],
  [K('π', 'π', FN), K('0', '0', NUM), K('.', '.', NUM), K('(−)', '⁻', NUM), A('ENTER', 'enter', GREEN)],
];

function Calculator() {
  const [line, setLine] = useState('');
  const [hist, setHist] = useState<{ q: string; a: string }[]>([]);
  const [ans, setAns] = useState(0);
  const [deg, setDeg] = useState(true);
  const [second, setSecond] = useState(false);
  const [recall, setRecall] = useState(-1);
  const screen = useRef<HTMLDivElement>(null);
  useEffect(() => { screen.current?.scrollTo({ top: 1e9 }); }, [hist, line]);

  const enter = () => {
    const q = line.trim() || hist[hist.length - 1]?.q;
    if (!q) return;
    let src = q;
    if (/^[+\-×÷*/^²]/.test(src) && !src.startsWith('−') ) src = 'Ans' + src; // TI-84: operator first uses Ans
    try { const v = evaluate(src.replace(/Ans/g, 'ans'), ans, deg); setAns(v); setHist((h) => [...h, { q: src, a: fmt(v) }].slice(-50)); }
    catch (e) { setHist((h) => [...h, { q: src, a: `ERR:${(e as Error).message}` }].slice(-50)); }
    setLine(''); setRecall(-1);
  };
  const press = (k: Key) => {
    if (k.act) {
      if (k.act === 'second') return setSecond((s) => !s);
      if (k.act === 'mode') setDeg((d) => !d);
      if (k.act === 'del') setLine((l) => l.slice(0, -1));
      if (k.act === 'clear') { if (line) setLine(''); else setHist([]); }
      if (k.act === 'ans') setLine((l) => l + 'Ans');
      if (k.act === 'enter') enter();
      setSecond(false);
      return;
    }
    const ins = second && k.second ? k.second.ins : k.ins!;
    setLine((l) => (!l && /^[+×÷^²!]|^\^/.test(ins) ? 'Ans' + ins : l + ins));
    setSecond(false);
  };
  const onKey = (e: React.KeyboardEvent) => {
    e.stopPropagation(); // don't trigger the piano computer-keyboard
    if (e.key === 'Enter') { e.preventDefault(); enter(); }
    else if (e.key === 'Backspace') { e.preventDefault(); setLine((l) => l.slice(0, -1)); }
    else if (e.key === 'Escape') setLine('');
    else if (e.key === 'ArrowUp') { e.preventDefault(); const i = recall < 0 ? hist.length - 1 : Math.max(0, recall - 1); if (hist[i]) { setRecall(i); setLine(hist[i].q); } }
    else if (e.key.length === 1 && /[0-9.+\-*/^()!,a-zA-Z]/.test(e.key)) { e.preventDefault(); const map: Record<string, string> = { '*': '×', '/': '÷', '-': '−' }; setLine((l) => (!l && /[+*/^!]/.test(e.key) ? 'Ans' : '') + l + (map[e.key] ?? e.key)); }
  };

  return (
    <div className="rounded-[28px] bg-[#1f2226] p-3 pb-4 shadow-[0_10px_30px_rgba(0,0,0,.6)] border border-black w-[300px] select-none" tabIndex={0} onKeyDown={onKey}>
      <div className="flex justify-between items-center px-1 mb-1"><span className="text-[11px] font-bold text-[#9aa4b1] tracking-widest">TI-84 Plus <span className="text-[#4a90d9]">CE</span> style</span><span className="text-[10px] text-[#9aa4b1]">click here, then type</span></div>
      <div ref={screen} className="bg-[#c7d3c0] text-black font-mono text-[14px] rounded-md p-2 h-[150px] overflow-y-auto border-4 border-[#111]">
        <div className="flex justify-between text-[10px] opacity-70 mb-1"><span>{deg ? 'DEGREE' : 'RADIAN'} {second && <b className="text-[#1d4ed8]">2ND</b>}</span><span>NORMAL FLOAT</span></div>
        {hist.map((h, i) => (<div key={i}><div className="truncate">{h.q}</div><div className="text-right font-bold">{h.a}</div></div>))}
        <div className="break-all">{line}<span className="animate-pulse">▌</span></div>
      </div>
      <div className="grid grid-cols-5 gap-1.5 mt-3">
        {KEYS.flat().map((k, i) => (
          <button key={i} onClick={() => press(k)} className={`relative rounded-lg text-[12px] py-2 leading-none active:translate-y-px shadow-[0_2px_0_rgba(0,0,0,.5)] ${k.cls}`}>
            {k.second && <span className="absolute -top-0.5 left-1 text-[8px] text-[#7fb3ff]">{k.second.l}</span>}
            <span className={k.second ? 'block mt-1' : ''}>{k.l}</span>
          </button>
        ))}
      </div>
      <div className="text-[10px] text-[#9aa4b1] mt-2 leading-snug">MODE = degrees/radians · 2nd (blue) = sin⁻¹, √, π… · ↑ recalls last entry · start with + × ÷ to use Ans</div>
    </div>
  );
}

function Notepad() {
  const KEY = 'geo-notepad';
  const [text, setText] = useState(() => { try { return localStorage.getItem(KEY) ?? ''; } catch { return ''; } });
  useEffect(() => { try { localStorage.setItem(KEY, text); } catch { /* storage blocked */ } }, [text]);
  return (
    <div className="w-[300px] rounded-2xl bg-[#fffbe6] text-[#1f2937] shadow-[0_10px_30px_rgba(0,0,0,.5)] border border-[#e8dca0] overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 bg-[#fde68a] font-bold text-sm">📝 Scratch Notepad<button className="text-xs font-semibold underline" onClick={() => { if (confirm('Clear the notepad?')) setText(''); }}>Clear</button></div>
      <textarea value={text} onChange={(e) => setText(e.target.value)} onKeyDown={(e) => e.stopPropagation()} placeholder="Work out problems here… (saves automatically)"
        className="w-full h-[360px] p-3 bg-transparent outline-none resize-none font-mono text-[14px] leading-[24px]"
        style={{ backgroundImage: 'repeating-linear-gradient(transparent, transparent 23px, #e5d9a3 24px)', backgroundAttachment: 'local' }} />
    </div>
  );
}

/** Floating tool buttons on the left edge, shown on geometry pages. */
export function GeoTools() {
  const loc = useLocation();
  const [open, setOpen] = useState<{ calc: boolean; notes: boolean }>({ calc: false, notes: false });
  const onGeo = /^\/(study|advanced|plan)/.test(loc.pathname); // every study page
  if (!onGeo) return null;
  return (
    <div className="fixed left-[250px] bottom-4 z-40 flex items-end gap-3 pointer-events-none">
      <div className="flex flex-col gap-2 pointer-events-auto">
        <button onClick={() => setOpen((o) => ({ ...o, notes: !o.notes }))} className={`w-12 h-12 rounded-full text-2xl border-2 shadow-lg ${open.notes ? 'bg-[#fde68a] border-[#f59e0b]' : 'bg-card2 border-edge/60'}`} title="Notepad">📝</button>
        <button onClick={() => setOpen((o) => ({ ...o, calc: !o.calc }))} className={`w-12 h-12 rounded-full text-2xl border-2 shadow-lg ${open.calc ? 'bg-[#4a90d9] border-[#93c5fd]' : 'bg-card2 border-edge/60'}`} title="TI-84 calculator">🧮</button>
      </div>
      {open.notes && <div className="pointer-events-auto"><Notepad /></div>}
      {open.calc && <div className="pointer-events-auto"><Calculator /></div>}
    </div>
  );
}

export const _test = { evaluate, fmt };
