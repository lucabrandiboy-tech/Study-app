// Cloud save for the online link: progress is stored privately in the viewer's claude.ai account,
// so it is saved automatically (on every change and when the app is closed) and the same progress
// opens on the phone and the computer. Only runs inside the claude.ai page viewer; elsewhere it stays off.
import { useSyncExternalStore } from 'react';
import { getState, replaceState, subscribe, lastChangeAt, setLastChangeAt, stashPrevious, celebrate } from './store';

type Snap = { exists: boolean; data(): Record<string, unknown> | undefined };
type DocRef = { get(): Promise<Snap>; set(body: Record<string, unknown>): Promise<unknown>; delete(): Promise<unknown> };
type Db = { doc(path: string): DocRef };
type ClaudeRuntime = { use(name: string): Promise<unknown> };
type UserNs = { id(): Promise<string | null> };
type DownloadsNs = { save(r: { filename: string; data: string | Blob }): Promise<{ status: string }> };

export type CloudStatus = 'off' | 'connecting' | 'synced' | 'saving' | 'error';
interface Info { status: CloudStatus; lastSaved: Date | null; error: string | null }
let info: Info = { status: 'off', lastSaved: null, error: null };
const listeners = new Set<() => void>();
const set = (p: Partial<Info>) => { info = { ...info, ...p }; listeners.forEach((l) => l()); };
export const useCloudInfo = () => useSyncExternalStore((l) => { listeners.add(l); return () => listeners.delete(l); }, () => info);

const runtime = () => (window as unknown as { claude?: ClaudeRuntime }).claude;
const CHUNK = 60_000; // characters per document (stays under the 256 KiB document limit even for 4-byte characters)
type Meta = { savedAt: number; gen: string; chunks: number };

let db: Db | null = null;
let base = '';
let meta: Meta | null = null;
let lastJson = '';
let busy: Promise<void> = Promise.resolve();
let timer: ReturnType<typeof setTimeout> | undefined;

const json = () => JSON.stringify(getState());
const metaRef = () => db!.doc(`${base}/save`);
const chunkRef = (gen: string, k: number) => db!.doc(`${base}/save-${gen}-${k}`);

async function readMeta(): Promise<Meta | null> {
  const snap = await metaRef().get();
  const d = snap.exists ? (snap.data() as Partial<Meta>) : undefined;
  return d && typeof d.gen === 'string' && typeof d.chunks === 'number' ? { savedAt: Number(d.savedAt) || 0, gen: d.gen, chunks: d.chunks } : null;
}

/** Upload this device's progress if it changed since the last upload. Safe to call often. */
function push(): Promise<void> {
  busy = busy.then(async () => {
    if (!db) return;
    const body = json();
    if (body === lastJson) return;
    set({ status: 'saving' });
    try {
      const gen = Date.now().toString(36);
      const parts: string[] = [];
      for (let i = 0; i < body.length; i += CHUNK) parts.push(body.slice(i, i + CHUNK));
      for (let k = 0; k < parts.length; k++) await chunkRef(gen, k).set({ d: parts[k] });
      const old = meta;
      meta = { savedAt: lastChangeAt() || Date.now(), gen, chunks: parts.length };
      await metaRef().set(meta); // the new copy only becomes "the save" once every piece is written
      lastJson = body;
      if (old && old.gen !== gen) for (let k = 0; k < old.chunks; k++) await chunkRef(old.gen, k).delete().catch(() => {});
      set({ status: 'synced', lastSaved: new Date(), error: null });
    } catch (e) {
      set({ status: 'error', error: (e as { message?: string }).message ?? 'Could not reach your account' });
    }
  });
  return busy;
}

/** Download the cloud copy and use it if it is newer than this device's progress. */
function pull(): Promise<void> {
  busy = busy.then(async () => {
    if (!db) return;
    try {
      const m = await readMeta();
      if (!m) { meta = null; return; }
      meta = m;
      if (m.savedAt <= lastChangeAt()) return; // this device is up to date (or newer: push handles it)
      const parts: string[] = [];
      for (let k = 0; k < m.chunks; k++) {
        const snap = await chunkRef(m.gen, k).get();
        const d = snap.exists ? snap.data()?.d : undefined;
        if (typeof d !== 'string') throw new Error('The cloud save is incomplete; keeping this device\'s progress.');
        parts.push(d);
      }
      const body = parts.join('');
      if (body === json()) { lastJson = body; setLastChangeAt(m.savedAt); return; }
      if (getState().xp > 0 || Object.keys(getState().topics).length) stashPrevious();
      replaceState(JSON.parse(body));
      lastJson = body;
      setLastChangeAt(m.savedAt);
      celebrate('Progress loaded', '☁️', 'Your latest saved progress from your account is here.');
      set({ status: 'synced', lastSaved: new Date(m.savedAt), error: null });
    } catch (e) {
      set({ status: 'error', error: (e as { message?: string }).message ?? 'Could not load the cloud save' });
    }
  });
  return busy;
}

/** Start cloud saving (online link only). */
export async function initCloud() {
  const c = runtime();
  if (!c?.use) return;
  set({ status: 'connecting' });
  const [dbNs, user] = (await Promise.all([c.use('db'), c.use('user')])) as [Db | null, UserNs | null];
  const id = user ? await user.id().catch(() => null) : null;
  if (!dbNs || !id) { set({ status: 'off' }); return; }
  db = dbNs;
  base = `data/users/${id}`;
  await pull();
  await push(); // first upload (or nothing, if the cloud copy was just loaded)
  subscribe(() => { clearTimeout(timer); timer = setTimeout(() => void push(), 4000); });
  // Save the moment the app is closed, minimized, or the phone switches apps; check for newer progress on return.
  document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') { clearTimeout(timer); void push(); } else void pull(); });
  window.addEventListener('pagehide', () => { clearTimeout(timer); void push(); });
  setInterval(() => void push(), 60_000);
}

export const cloudSaveNow = () => push();

/** Offer a save file through the viewer (online link). Resolves a message for the UI. */
export async function downloadViaViewer(filename: string, data: string): Promise<string | null> {
  const c = runtime();
  const dl = c?.use ? ((await c.use('downloads')) as DownloadsNs | null) : null;
  if (!dl) return null;
  try { await dl.save({ filename, data }); return '✓ Save file downloaded.'; }
  catch (e) { const code = (e as { code?: string }).code; return code === 'declined' ? 'Download cancelled.' : `✗ Download failed (${code ?? 'unknown'}). Use the backup code instead.`; }
}
