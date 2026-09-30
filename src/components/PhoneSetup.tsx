// "Get it on your phone": install the app (offline, own icon) or use the cloud-sync link.
import { useEffect, useMemo, useState } from 'react';
import qrcode from 'qrcode-generator';

export const APP_URL = 'https://claude.ai/artifact/SKAmGM25aGdXY6jge3JyGg'; // online link with cloud sync
export const INSTALL_URL = 'https://lucabrandiboy-tech.github.io/Study-app/'; // installable app (works offline)
const installed = typeof window !== 'undefined' && (window.matchMedia?.('(display-mode: standalone)').matches || (navigator as unknown as { standalone?: boolean }).standalone === true);

// Android/Chrome: the browser offers a one-tap install; keep the offer so a button can use it.
type InstallPrompt = Event & { prompt(): Promise<void>; userChoice: Promise<{ outcome: string }> };
let deferred: InstallPrompt | null = null;
const promptListeners = new Set<() => void>();
if (typeof window !== 'undefined') window.addEventListener('beforeinstallprompt', (e) => { e.preventDefault(); deferred = e as InstallPrompt; promptListeners.forEach((l) => l()); });
function useInstallPrompt() {
  const [p, setP] = useState(deferred);
  useEffect(() => { const l = () => setP(deferred); promptListeners.add(l); return () => { promptListeners.delete(l); }; }, []);
  return p;
}
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
  ios: ['Open the install page in Safari (scan the QR code or tap the link).', 'Tap the Share button (the square with an arrow ⬆️).', 'Tap "Add to Home Screen", then "Add".', 'Open Study + Piano from its new icon. It works offline too.'],
  android: ['Open the install page in Chrome (scan the QR code or tap the link).', 'Tap "Install" when Chrome offers it — or tap ⋮ then "Install app".', 'Tap "Install" to confirm.', 'Open Study + Piano from its new icon. It works offline too.'],
};

export function PhoneSetupCard({ compact = false }: { compact?: boolean }) {
  const [copied, setCopied] = useState<string | null>(null);
  const [tab, setTab] = useState<'ios' | 'android'>(device === 'android' ? 'android' : 'ios');
  const prompt = useInstallPrompt();
  const copy = async (url: string, id: string) => {
    try { await navigator.clipboard.writeText(url); setCopied('✓ Link copied — text or email it to yourself, then open it on your phone.'); }
    catch { const el = document.getElementById(id); if (el) { const r = document.createRange(); r.selectNodeContents(el); window.getSelection()?.removeAllRanges(); window.getSelection()?.addRange(r); } setCopied('Link selected — copy it with Ctrl+C.'); }
  };
  if (installed) return compact ? null : <div className="card text-sm">✅ You're using the installed Study + Piano app. Your progress is saved on this device — use <b>Download save file</b> below to keep a copy or move it.</div>;
  return (
    <div className="card space-y-3">
      <div className="h2">📲 Get Study + Piano on your phone</div>
      <div className="flex gap-5 items-start flex-wrap">
        {device === 'desktop' && (
          <div className="text-center space-y-1">
            <QrCode text={INSTALL_URL} size={compact ? 140 : 180} />
            <div className="text-xs muted">Scan with your phone's camera</div>
          </div>
        )}
        <div className="flex-1 min-w-0 space-y-2 text-sm">
          <div className="font-bold">📥 Install the app — it gets its own icon and works offline</div>
          <div className="flex items-center gap-2 flex-wrap">
            {prompt ? <button className="btn" onClick={async () => { await prompt.prompt(); deferred = null; }}>📥 Install Study + Piano</button>
              : <a className="btn" href={INSTALL_URL} target="_blank" rel="noreferrer">📥 Open the install page</a>}
            <code id="install-link" className="rounded-lg bg-navy border border-edge/30 px-2 py-1 text-edge break-all select-all">{INSTALL_URL}</code>
            <button className="btn-ghost text-sm" onClick={() => copy(INSTALL_URL, 'install-link')}>📋 Copy</button>
          </div>
          {copied && <div className="text-good text-xs">{copied}</div>}
          {!compact && <>
            <div className="flex gap-1 pt-1">
              {(['ios', 'android'] as const).map((t) => <button key={t} onClick={() => setTab(t)} className={`px-3 py-1 rounded-full text-xs font-bold border ${tab === t ? 'bg-accent border-accent' : 'border-edge/40'}`}>{t === 'ios' ? '📱 iPhone / iPad' : '🤖 Android'}</button>)}
            </div>
            <ol className="list-decimal pl-5 space-y-1">{STEPS[tab].map((st) => <li key={st}>{st}</li>)}</ol>
            <div className="rounded-xl bg-navy border border-edge/30 p-3 space-y-1">
              <div className="font-bold">☁️ Want the same progress on phone and computer?</div>
              <div className="muted">The installed app saves on the phone itself. To sync automatically between devices instead, use your private link while signed in to Claude: <code id="cloud-link" className="text-edge break-all select-all">{APP_URL}</code> <button className="btn-ghost text-xs" onClick={() => copy(APP_URL, 'cloud-link')}>📋 Copy</button></div>
              <div className="muted">Or move progress by hand any time: <b>Download save file</b> on one device, <b>Load save file</b> on the other (Settings).</div>
            </div>
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
