import { useSyncExternalStore } from 'react';
import { getState, replaceState, subscribe } from './store';

/**
 * Saving progress to a real file on the computer (outside the browser).
 * - Chrome/Edge: pick a save file once; every change is auto-saved to it (File System Access API).
 *   The file handle is remembered in IndexedDB so it reconnects next time (one click to re-allow).
 * - Any browser: Export / Import a backup .json file.
 */
export type SaveStatus = 'unsupported' | 'none' | 'needs-permission' | 'saving' | 'saved' | 'error';
interface Info { status: SaveStatus; fileName: string | null; lastSaved: Date | null; error: string | null }

type Picker = (opts: object) => Promise<FileSystemFileHandle | FileSystemFileHandle[]>;
type PermHandle = FileSystemFileHandle & {
  queryPermission(o: { mode: 'readwrite' }): Promise<PermissionState>;
  requestPermission(o: { mode: 'readwrite' }): Promise<PermissionState>;
};
const w = window as unknown as { showSaveFilePicker?: Picker; showOpenFilePicker?: Picker };
import { embedded } from './embed';
import { downloadViaViewer } from './cloud';
export const fileSaveSupported = typeof w.showSaveFilePicker === 'function' && !embedded;

let info: Info = { status: fileSaveSupported ? 'none' : 'unsupported', fileName: null, lastSaved: null, error: null };
const listeners = new Set<() => void>();
const set = (p: Partial<Info>) => { info = { ...info, ...p }; listeners.forEach((l) => l()); };
export const useSaveInfo = () => useSyncExternalStore((l) => { listeners.add(l); return () => listeners.delete(l); }, () => info);

let handle: PermHandle | null = null;
const FILE_TYPES = [{ description: 'Study + Piano save file', accept: { 'application/json': ['.json'] } }];

// ---- tiny IndexedDB key-value store for the file handle ----
function idb<T>(mode: IDBTransactionMode, fn: (s: IDBObjectStore) => IDBRequest): Promise<T> {
  return new Promise((resolve, reject) => {
    const open = indexedDB.open('study-piano-files', 1);
    open.onupgradeneeded = () => open.result.createObjectStore('kv');
    open.onerror = () => reject(open.error);
    open.onsuccess = () => {
      const req = fn(open.result.transaction('kv', mode).objectStore('kv'));
      req.onsuccess = () => resolve(req.result as T);
      req.onerror = () => reject(req.error);
    };
  });
}
const rememberHandle = (h: FileSystemFileHandle | null) => idb('readwrite', (s) => (h ? s.put(h, 'handle') : s.delete('handle'))).catch(() => {});

const snapshot = () => JSON.stringify({ ...getState(), savedAt: new Date().toISOString() }, null, 2);

async function writeNow() {
  if (!handle) return;
  set({ status: 'saving' });
  try {
    const out = await handle.createWritable();
    await out.write(snapshot());
    await out.close();
    set({ status: 'saved', lastSaved: new Date(), error: null });
  } catch (e) {
    set({ status: 'error', error: (e as Error).message });
  }
}

let timer: ReturnType<typeof setTimeout> | undefined;
subscribe(() => {
  if (!handle || info.status === 'needs-permission') return;
  clearTimeout(timer);
  timer = setTimeout(writeNow, 1200);
});
// Write the save file immediately when the app is closed or hidden.
const flush = () => { if (timer && handle && info.status !== 'needs-permission') { clearTimeout(timer); timer = undefined; void writeNow(); } };
window.addEventListener('beforeunload', flush);
window.addEventListener('pagehide', flush);
document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') flush(); });

async function readHandle(h: FileSystemFileHandle) {
  const text = await (await h.getFile()).text();
  if (!text.trim()) return false; // brand-new empty file
  replaceState(JSON.parse(text));
  return true;
}

/** Create a new save file and start auto-saving current progress into it. */
export async function createSaveFile() {
  if (!w.showSaveFilePicker) return;
  try {
    handle = (await w.showSaveFilePicker({ suggestedName: 'study-piano-progress.json', types: FILE_TYPES })) as PermHandle;
    await rememberHandle(handle);
    set({ fileName: handle.name });
    await writeNow();
  } catch (e) { if ((e as Error).name !== 'AbortError') set({ status: 'error', error: (e as Error).message }); }
}

/** Open an existing save file: loads its progress, then keeps auto-saving to it. */
export async function openSaveFile() {
  if (!w.showOpenFilePicker) return;
  try {
    const [h] = (await w.showOpenFilePicker({ types: FILE_TYPES, multiple: false })) as FileSystemFileHandle[];
    const ph = h as PermHandle;
    if ((await ph.requestPermission({ mode: 'readwrite' })) !== 'granted') throw new Error('Permission to save to the file was not given.');
    handle = null; // don't auto-save the old state while loading
    await readHandle(ph);
    handle = ph;
    await rememberHandle(ph);
    set({ fileName: ph.name, status: 'saved', lastSaved: new Date(), error: null });
  } catch (e) { if ((e as Error).name !== 'AbortError') set({ status: 'error', error: (e as Error).message }); }
}

/** After a reload the browser asks again before the app may touch the file. Must be called from a click. */
export async function reconnectSaveFile() {
  if (!handle) return;
  try {
    if ((await handle.requestPermission({ mode: 'readwrite' })) !== 'granted') return;
    const h = handle;
    handle = null;
    const had = await readHandle(h);
    handle = h;
    if (!had) await writeNow();
    set({ status: 'saved', lastSaved: new Date(), error: null });
  } catch (e) { set({ status: 'error', error: (e as Error).message }); }
}

export async function forgetSaveFile() {
  handle = null;
  await rememberHandle(null);
  set({ status: fileSaveSupported ? 'none' : 'unsupported', fileName: null, lastSaved: null, error: null });
}

export const saveNow = () => writeNow();

/** On startup: find the remembered save file and load from it if the browser still allows access. */
export async function initFileSave() {
  if (!fileSaveSupported) return;
  try {
    const h = await idb<PermHandle | undefined>('readonly', (s) => s.get('handle'));
    if (!h) return;
    handle = h;
    set({ fileName: h.name });
    if ((await h.queryPermission({ mode: 'readwrite' })) === 'granted') await reconnectSaveFile();
    else set({ status: 'needs-permission' });
  } catch { /* IndexedDB unavailable: fall back to manual export/import */ }
}

// ---- manual backup (works in every browser) ----
export async function exportBackup(): Promise<string | null> {
  const name = `study-piano-backup-${new Date().toISOString().slice(0, 10)}.json`;
  if (embedded) return downloadViaViewer(name, snapshot());
  const blob = new Blob([snapshot()], { type: 'application/json' });
  const a = document.createElement('a');
  a.href = URL.createObjectURL(blob);
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
  return '✓ Backup file downloaded.';
}

/**
 * Downloaded app file only: if no auto-save file is connected, drop a dated backup file into
 * Downloads once a day, so progress survives even if the browser's storage is cleared.
 */
export function dailyAutoBackup(enabled: boolean) {
  if (embedded || !enabled) return;
  const today = new Date().toISOString().slice(0, 10);
  try {
    if (localStorage.getItem('study-piano-last-auto-backup') === today) return;
    const st = getState();
    if (!st.xp && !Object.keys(st.topics).length && !st.homework.length) return; // nothing to back up yet
    setTimeout(() => {
      if (info.status === 'saved' || info.status === 'saving') return; // a save file already has everything
      void exportBackup();
      localStorage.setItem('study-piano-last-auto-backup', today);
    }, 4000);
  } catch { /* storage blocked */ }
}
export async function importBackup(file: File) {
  replaceState(JSON.parse(await file.text()));
}

/** Backup as text, for hosted links where downloads are blocked: copy it, keep it somewhere, paste it back later. */
export const backupText = () => snapshot();
export function restoreFromText(text: string) { replaceState(JSON.parse(text)); }
