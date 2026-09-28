import { ReactNode } from 'react';

export function ProgressBar({ value, max = 100, color = '#A970FF', className = '' }: { value: number; max?: number; color?: string; className?: string }) {
  const pct = Math.max(0, Math.min(100, (value / (max || 1)) * 100));
  return (
    <div className={`h-3 rounded-full bg-navy border border-edge/30 overflow-hidden ${className}`}>
      <div className="h-full rounded-full transition-all duration-700" style={{ width: `${pct}%`, background: color, boxShadow: `0 0 10px ${color}` }} />
    </div>
  );
}

export function Ring({ value, max, size = 120, label, sub }: { value: number; max: number; size?: number; label: ReactNode; sub?: string }) {
  const r = size / 2 - 10, c = 2 * Math.PI * r, pct = Math.min(1, value / (max || 1));
  return (
    <div className="relative inline-flex items-center justify-center" style={{ width: size, height: size }}>
      <svg width={size} height={size} className="-rotate-90">
        <circle cx={size / 2} cy={size / 2} r={r} stroke="#0B1026" strokeWidth={10} fill="none" />
        <circle cx={size / 2} cy={size / 2} r={r} stroke={pct >= 1 ? '#4ADE80' : '#A970FF'} strokeWidth={10} fill="none"
          strokeDasharray={c} strokeDashoffset={c * (1 - pct)} strokeLinecap="round" style={{ transition: 'stroke-dashoffset .8s' }} />
      </svg>
      <div className="absolute text-center">
        <div className="text-2xl font-extrabold">{label}</div>
        {sub && <div className="text-xs muted">{sub}</div>}
      </div>
    </div>
  );
}

export function Stars({ n, size = 'text-2xl' }: { n: number; size?: string }) {
  return (
    <span className={size}>
      {[0, 1, 2].map((i) => <span key={i} className={i < n ? 'text-yellow-300 drop-shadow-[0_0_6px_rgba(250,204,21,.7)]' : 'text-muted/30'}>★</span>)}
    </span>
  );
}

export function Flame({ n, size = 48 }: { n: number; size?: number }) {
  return (
    <span className={n > 0 ? 'animate-flicker inline-block' : 'inline-block grayscale opacity-50'} style={{ fontSize: size, filter: n > 0 ? 'drop-shadow(0 0 10px #FF9F43)' : undefined }}>🔥</span>
  );
}

export function Modal({ open, onClose, title, children, wide }: { open: boolean; onClose: () => void; title: string; children: ReactNode; wide?: boolean }) {
  if (!open) return null;
  return (
    <div className="fixed inset-0 z-50 bg-black/60 flex items-center justify-center p-8" onClick={onClose}>
      <div className={`card2 animate-pop max-h-[85vh] overflow-auto ${wide ? 'w-[900px]' : 'w-[560px]'}`} onClick={(e) => e.stopPropagation()}>
        <div className="flex items-center justify-between mb-3">
          <h2 className="h2">{title}</h2>
          <button className="btn-ghost px-2 py-1" onClick={onClose} aria-label="Close">✕</button>
        </div>
        {children}
      </div>
    </div>
  );
}

export function Tabs<T extends string>({ tabs, value, onChange }: { tabs: { id: T; label: ReactNode; disabled?: boolean }[]; value: T; onChange: (t: T) => void }) {
  return (
    <div className="flex gap-1 p-1 bg-navy rounded-xl border border-edge/30 w-fit">
      {tabs.map((t) => (
        <button key={t.id} disabled={t.disabled} onClick={() => onChange(t.id)}
          className={`px-4 py-1.5 rounded-lg font-semibold transition disabled:opacity-40 ${value === t.id ? 'bg-accent text-white shadow-[0_0_12px_rgba(169,112,255,.6)]' : 'text-muted hover:text-ink'}`}>
          {t.label}
        </button>
      ))}
    </div>
  );
}

export function PageHeader({ title, sub, right }: { title: ReactNode; sub?: ReactNode; right?: ReactNode }) {
  return (
    <div className="flex items-end justify-between mb-6">
      <div><h1 className="h1">{title}</h1>{sub && <p className="muted mt-1">{sub}</p>}</div>
      {right}
    </div>
  );
}

export function Stat({ label, value, color }: { label: string; value: ReactNode; color?: string }) {
  return (
    <div className="card py-3">
      <div className="text-xs muted uppercase tracking-wide">{label}</div>
      <div className="text-2xl font-extrabold" style={{ color }}>{value}</div>
    </div>
  );
}

/** Renders text with **bold** and line breaks. */
export function Rich({ text, className = '' }: { text: string; className?: string }) {
  return (
    <div className={`space-y-2 leading-relaxed ${className}`}>
      {text.split('\n').filter(Boolean).map((line, i) => (
        <p key={i} className={line.startsWith('• ') ? 'pl-3' : ''}>
          {line.split(/(\*\*[^*]+\*\*)/).map((part, j) => part.startsWith('**') ? <strong key={j} className="text-edge">{part.slice(2, -2)}</strong> : part)}
        </p>
      ))}
    </div>
  );
}
