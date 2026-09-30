// "Get it on your phone": a QR code for the app's link plus Add-to-Home-Screen steps for iPhone and Android.
import { useMemo, useState } from 'react';
import qrcode from 'qrcode-generator';

export const APP_URL = 'https://claude.ai/artifact/SKAmGM25aGdXY6jge3JyGg';
const ua = typeof navigator !== 'undefined' ? navigator.userAgent : '';
export const device: 'ios' | 'android' | 'desktop' = /iPhone|iPad|iPod/i.test(ua) ? 'ios' : /Android/i.test(ua) ? 'android' : 'desktop';

function QrCode({ text, size = 180 }: { text: string; size?: number }) {
  const { n, path } = useMemo(() => {
    const qr = qrcode(0, 'M');
    qr.addData(text);
    qr.make();
    const n = qr.getModuleCount();
    let d = '';
    for (let r = 0; r < n; r++) for (let c = 0; c < n; c++) if (qr.isDark(r, c)) d += `M${c} ${r}h1v1h-1z`;
    return { n, path: d };
  }, [text]);
  return (
    <svg width={size} height={size} viewBox={`-2 -2 ${n + 4} ${n + 4}`} className="rounded-xl bg-white shrink-0" role="img" aria-label="QR code for the app link">
      <rect x={-2} y={-2} width={n + 4} height={n + 4} fill="#fff" />
      <path d={path} fill="#111827" />
    </svg>
  );
}

const STEPS = {
  ios: ['Open the link in Safari and sign in to Claude.', 'Tap the Share button (the square with an arrow ⬆️) at the bottom.', 'Scroll down and tap "Add to Home Screen", then "Add".', 'Open Study + Piano from its new icon on your home screen.'],
  android: ['Open the link in Chrome and sign in to Claude.', 'Tap the ⋮ menu in the top-right corner.', 'Tap "Add to Home screen" (or "Install app"), then "Add".', 'Open Study + Piano from its new icon on your home screen.'],
};

export function PhoneSetupCard({ compact = false }: { compact?: boolean }) {
  const [copied, setCopied] = useState<string | null>(null);
  const [tab, setTab] = useState<'ios' | 'android'>(device === 'android' ? 'android' : 'ios');
  const copy = async () => {
    try { await navigator.clipboard.writeText(APP_URL); setCopied('✓ Link copied — send it to your phone (text or email it to yourself).'); }
    catch { const el = document.getElementById('app-link'); if (el) { const r = document.createRange(); r.selectNodeContents(el); window.getSelection()?.removeAllRanges(); window.getSelection()?.addRange(r); } setCopied('Link selected — copy it with Ctrl+C.'); }
  };
  return (
    <div className="card space-y-3">
      <div className="h2">📲 Get Study + Piano on your phone</div>
      <div className="flex gap-5 items-start flex-wrap">
        {device === 'desktop' && (
          <div className="text-center space-y-1">
            <QrCode text={APP_URL} size={compact ? 140 : 180} />
            <div className="text-xs muted">Scan with your phone's camera</div>
          </div>
        )}
        <div className="flex-1 min-w-0 space-y-2 text-sm">
          <div>Your app lives at this link. It updates itself automatically, and your progress follows you between phone and computer (☁️ cloud save).</div>
          <div className="flex items-center gap-2 flex-wrap">
            <code id="app-link" className="rounded-lg bg-navy border border-edge/30 px-2 py-1 text-edge break-all select-all">{APP_URL}</code>
            <button className="btn-ghost text-sm" onClick={copy}>📋 Copy link</button>
          </div>
          {copied && <div className="text-good text-xs">{copied}</div>}
          {!compact && <>
            <div className="flex gap-1 pt-1">
              {(['ios', 'android'] as const).map((t) => <button key={t} onClick={() => setTab(t)} className={`px-3 py-1 rounded-full text-xs font-bold border ${tab === t ? 'bg-accent border-accent' : 'border-edge/40'}`}>{t === 'ios' ? '📱 iPhone / iPad' : '🤖 Android'}</button>)}
            </div>
            <ol className="list-decimal pl-5 space-y-1">{STEPS[tab].map((s) => <li key={s}>{s}</li>)}</ol>
            <div className="text-xs muted">Tip: the icon opens straight into the app, like any other app on your phone. Keep your phone signed in to Claude so cloud save can sync.</div>
          </>}
        </div>
      </div>
    </div>
  );
}

/** Small dismissible banner on the Home page (computer only). */
export function PhoneBanner() {
  const [hidden, setHidden] = useState(() => { try { return localStorage.getItem('phone-banner-hidden') === '1'; } catch { return false; } });
  if (hidden || device !== 'desktop') return null;
  return (
    <div className="relative mb-6">
      <PhoneSetupCard compact />
      <button className="absolute top-3 right-3 btn-ghost py-1 px-2 text-xs" onClick={() => { setHidden(true); try { localStorage.setItem('phone-banner-hidden', '1'); } catch { /* blocked */ } }}>Hide</button>
    </div>
  );
}
