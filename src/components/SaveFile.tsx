import { ask } from '../lib/embed';
import { useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import { embedded } from '../lib/embed';
import { useCloudInfo, cloudSaveNow } from '../lib/cloud';
import { useApp, setSettings, hasPrevious, restorePrevious } from '../lib/store';
import { backupText, restoreFromText, useSaveInfo, fileSaveSupported, createSaveFile, openSaveFile, reconnectSaveFile, forgetSaveFile, saveNow, exportBackup, importBackup } from '../lib/filesave';

const time = (d: Date | null) => (d ? d.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }) : '');

/** Compact status + one-click reconnect, shown in the sidebar. */
export function SaveBadge() {
  const s = useSaveInfo();
  const c = useCloudInfo();
  if (embedded && c.status !== 'off') {
    return (
      <Link to="/settings" className={`block text-xs truncate ${c.status === 'error' ? 'text-bad' : 'muted'}`} title={c.error ?? ''}>
        {c.status === 'connecting' ? '☁️ Connecting to your account…' : c.status === 'saving' ? '☁️ Saving…' : c.status === 'error' ? '⚠️ Cloud save problem — tap for help' : `☁️ Saved to your account ${time(c.lastSaved)}`}
      </Link>
    );
  }
  if (s.status === 'needs-permission') {
    return <button className="btn w-full text-sm py-1.5 animate-pulse" onClick={reconnectSaveFile} title={s.fileName ?? ''}>💾 Reconnect save file</button>;
  }
  if (s.status === 'none' || s.status === 'unsupported') {
    return <Link to="/settings" className="block text-xs text-streak hover:underline">⚠️ Only saved in this browser — set up a save file</Link>;
  }
  return (
    <div className="text-xs muted truncate" title={s.error ?? s.fileName ?? ''}>
      {s.status === 'error' ? <span className="text-bad">⚠️ Save failed — see Settings</span> : s.status === 'saving' ? '💾 Saving…' : `💾 Saved to ${s.fileName} ${time(s.lastSaved)}`}
    </div>
  );
}

export function SaveFileCard() {
  const s = useSaveInfo();
  const c = useCloudInfo();
  const autoBackup = useApp((st) => st.settings.autoBackup);
  const input = useRef<HTMLInputElement>(null);
  const [msg, setMsg] = useState<string | null>(null);
  const [prev, setPrev] = useState(hasPrevious);
  return (
    <div className="card space-y-3 col-span-2">
      <div className="h2">💾 Saving your progress</div>
      <div className="text-sm rounded-xl bg-navy border border-edge/30 p-3">✅ Every change is saved on this device instantly, and again the moment you close or leave the app.</div>
      {embedded && (
        <div className="rounded-xl border border-edge/40 p-3 space-y-2">
          <div className="flex items-center gap-3">
            <div className={`w-4 h-4 rounded-full ${c.status === 'synced' ? 'bg-good shadow-[0_0_10px_#4ADE80]' : c.status === 'error' ? 'bg-bad' : c.status === 'off' ? 'bg-muted/40' : 'bg-streak animate-pulse'}`} />
            <div className="flex-1">
              <div className="font-bold">☁️ Cloud save {c.status === 'off' ? 'is off' : c.status === 'error' ? 'had a problem' : 'is on'}</div>
              <div className="text-sm muted">{c.status === 'off' ? 'Sign in to Claude and open the app from your link to save to your account.' : c.status === 'error' ? c.error : `Progress is saved to your Claude account automatically, so it follows you between your phone and computer.${c.lastSaved ? ` Last saved ${time(c.lastSaved)}.` : ''}`}</div>
            </div>
            {c.status !== 'off' && <button className="btn-ghost" onClick={() => void cloudSaveNow()}>Save now</button>}
          </div>
          {prev && <div className="text-sm flex items-center gap-2 flex-wrap"><span className="muted">This device's older progress was replaced by your newer cloud save.</span><button className="btn-ghost text-sm" onClick={() => { restorePrevious(); setPrev(false); }}>Undo — use this device's old progress</button></div>}
        </div>
      )}
      <div className="h2 text-lg">💾 Save file (outside the browser)</div>
      {fileSaveSupported ? (
        <>
          <p className="text-sm muted">Keep your progress in a real file on your computer (for example in Documents or on a USB stick). Every change is saved to it automatically. Next time, open the app and click <b>Reconnect save file</b> in the sidebar.</p>
          <div className="flex items-center gap-3">
            <div className={`w-4 h-4 rounded-full ${s.status === 'saved' || s.status === 'saving' ? 'bg-good shadow-[0_0_10px_#4ADE80]' : s.status === 'needs-permission' ? 'bg-streak' : s.status === 'error' ? 'bg-bad' : 'bg-muted/40'}`} />
            <div className="flex-1">
              {s.fileName ? <>Save file: <b className="text-edge">{s.fileName}</b> {s.status === 'saved' && <span className="muted text-sm">· last saved {time(s.lastSaved)}</span>}{s.status === 'needs-permission' && <span className="text-streak text-sm"> · click Reconnect to allow saving</span>}</> : <span className="muted">No save file yet — progress is only in this browser.</span>}
              {s.error && <div className="text-bad text-sm">{s.error}</div>}
            </div>
          </div>
          <div className="flex gap-2 flex-wrap">
            {s.status === 'needs-permission' && <button className="btn" onClick={reconnectSaveFile}>🔓 Reconnect</button>}
            <button className={s.fileName ? 'btn-ghost' : 'btn'} onClick={createSaveFile}>➕ Create new save file</button>
            <button className="btn-ghost" onClick={() => { if (!s.fileName || ask('Opening a save file replaces the progress shown now with the progress in that file. Continue?')) void openSaveFile(); }}>📂 Open existing save file</button>
            {s.fileName && s.status !== 'needs-permission' && <button className="btn-ghost" onClick={saveNow}>Save now</button>}
            {s.fileName && <button className="btn-ghost" onClick={forgetSaveFile}>Stop using this file</button>}
          </div>
        </>
      ) : (
        <p className="text-sm muted">{embedded ? 'Download a save file any time to keep your own copy (you confirm each download).' : 'Automatic file saving needs Chrome or Edge. In this browser, the app downloads a backup file once a day, and you can export or import one below any time.'}</p>
      )}
      {embedded && <BackupCode />}
      <div className="border-t border-edge/20 pt-3 flex items-center gap-2 flex-wrap">
        <span className="text-sm muted mr-2">Save file (any browser):</span>
        <button className="btn" onClick={async () => setMsg(await exportBackup() ?? '✗ Downloads are not available here. Use the backup code instead.')}>⬇️ Download save file</button>
        <button className="btn-ghost" onClick={() => input.current?.click()}>⬆️ Load save file</button>
        <input ref={input} type="file" accept=".json,application/json" className="hidden" onChange={async (e) => {
          const f = e.target.files?.[0]; e.target.value = '';
          if (!f || !ask('Importing replaces your current progress with the backup. Continue?')) return;
          try { await importBackup(f); setMsg('✓ Backup imported.'); } catch (err) { setMsg(`✗ ${(err as Error).message}`); }
        }} />
        {msg && <span className={`text-sm ${msg.startsWith('✓') ? 'text-good' : 'text-bad'}`}>{msg}</span>}
      </div>
      {!embedded && <label className="flex items-center gap-2 text-sm"><input type="checkbox" checked={autoBackup} onChange={(e) => setSettings({ autoBackup: e.target.checked })} /> Download a backup file automatically once a day (when no save file is connected)</label>}
    </div>
  );
}

/** Online-link backup: copy a backup code to the clipboard, paste it back to restore. */
function BackupCode() {
  const [text, setText] = useState('');
  const [msg, setMsg] = useState<string | null>(null);
  const copy = async () => {
    const code = backupText();
    try { await navigator.clipboard.writeText(code); setMsg('✓ Backup code copied. Paste it into a note or email to yourself.'); }
    catch { setText(code); setMsg('Select all the text in the box and copy it.'); }
  };
  return (
    <div className="border-t border-edge/20 pt-3 space-y-2">
      <div className="flex gap-2 flex-wrap items-center">
        <button className="btn-ghost" onClick={copy}>📋 Copy backup code</button>
        <button className="btn-ghost" disabled={!text.trim()} onClick={() => { try { restoreFromText(text); setMsg('✓ Progress restored.'); setText(''); } catch (e) { setMsg(`✗ ${(e as Error).message === 'Unexpected end of JSON input' ? 'That backup code is incomplete.' : (e as Error).message}`); } }}>⬆️ Restore from pasted code</button>
        {msg && <span className={`text-sm ${msg.startsWith('✗') ? 'text-bad' : 'text-good'}`}>{msg}</span>}
      </div>
      <textarea id="backup-code" className="input w-full h-24 font-mono text-xs" placeholder="Paste a backup code here to restore your progress (this replaces the current progress)." value={text} onChange={(e) => setText(e.target.value)} />
    </div>
  );
}
