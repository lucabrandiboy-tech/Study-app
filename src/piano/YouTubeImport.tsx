import { useEffect, useRef, useState } from 'react';
import { addImportedSong } from '../lib/store';
import { midiName, type Piece } from './notation';
import { startListening, framesToNotes, notesToPiece, pieceToSong, youTubeId } from './listen';

/** Song Player panel: play a YouTube video inside the app, and turn its melody into a song by listening to it. */
export function YouTubeImport({ onDone, onClose }: { onDone: (p: Piece, msg: string) => void; onClose: () => void }) {
  const [link, setLink] = useState('');
  const [vid, setVid] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [bpm, setBpm] = useState(90);
  const [hearing, setHearing] = useState<number | null>(null);
  const [listening, setListening] = useState(false);
  const [secs, setSecs] = useState(0);
  const [err, setErr] = useState<string | null>(null);
  const stopRef = useRef<(() => ReturnType<Awaited<ReturnType<typeof startListening>>['stop']>) | null>(null);
  useEffect(() => () => { stopRef.current?.(); }, []);
  useEffect(() => {
    if (!listening) return;
    const t0 = Date.now();
    const id = setInterval(() => setSecs(Math.floor((Date.now() - t0) / 1000)), 500);
    return () => clearInterval(id);
  }, [listening]);

  const load = () => { const id = youTubeId(link); setErr(id ? null : "That doesn't look like a YouTube link. Copy the link from the video's Share button."); setVid(id); };
  const start = async () => {
    setErr(null);
    try {
      const l = await startListening(setHearing);
      stopRef.current = l.stop; setSecs(0); setListening(true);
    } catch { setErr('The app needs the microphone to listen. Allow microphone access, then try again.'); }
  };
  const stop = () => {
    const frames = stopRef.current?.() ?? [];
    stopRef.current = null; setListening(false); setHearing(null);
    const notes = framesToNotes(frames);
    if (notes.length < 4) { setErr("I didn't hear a clear melody. Turn the volume up, hold the phone near the speaker, and play a part where the tune stands out."); return; }
    const piece = notesToPiece(notes, name.trim() || 'Song from YouTube', bpm);
    addImportedSong(pieceToSong(piece));
    onDone(piece, `✅ Made "${piece.title}" from what I heard (${piece.events.length} notes). It's saved in the 📂 Imported row.`);
  };

  return (
    <div className="rounded-2xl bg-white border border-[#E5E7EB] p-4 mb-6 shadow-sm">
      <div className="flex items-center justify-between gap-2 mb-2">
        <h2 className="text-xl font-black">▶️ Learn a song from YouTube</h2>
        <button className="text-[#6B7280] px-2 py-1 rounded-lg hover:bg-[#F1F5F9]" onClick={() => { stopRef.current?.(); onClose(); }} aria-label="Close">✕</button>
      </div>
      <p className="text-sm text-[#6B7280] mb-3">Paste a YouTube link to watch it here. Then press <b>Listen</b> while it plays out loud: the app hears the melody and turns it into a song you can practice.</p>
      <div className="flex gap-2 flex-wrap mb-3">
        <input className="flex-1 min-w-[220px] rounded-full border border-[#E5E7EB] px-4 py-2.5 outline-none focus:border-[#22C55E]" placeholder="https://www.youtube.com/watch?v=…" value={link}
          onChange={(e) => setLink(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') load(); }} autoComplete="off" />
        <button className="rounded-full bg-[#111827] text-white font-bold px-5 py-2.5" onClick={load}>Load video</button>
      </div>
      {vid && (
        <div className="relative w-full max-w-[720px] aspect-video rounded-xl overflow-hidden bg-black mb-3">
          {/* YouTube's own player. No allow-popups in the sandbox, so its links can't open YouTube outside the app. */}
          <iframe className="absolute inset-0 w-full h-full" src={`https://www.youtube-nocookie.com/embed/${vid}?rel=0&playsinline=1&modestbranding=1`} title="YouTube video"
            sandbox="allow-scripts allow-same-origin allow-presentation" allow="autoplay; encrypted-media; picture-in-picture; fullscreen" referrerPolicy="strict-origin-when-cross-origin" allowFullScreen />
        </div>
      )}
      <div className="flex gap-3 flex-wrap items-end">
        <label className="text-sm font-semibold">Song name<br /><input className="mt-1 w-56 rounded-lg border border-[#E5E7EB] px-3 py-2" value={name} onChange={(e) => setName(e.target.value)} placeholder="Song from YouTube" autoComplete="off" /></label>
        <label className="text-sm font-semibold">Speed (BPM)<br /><input type="number" min={40} max={200} className="mt-1 w-24 rounded-lg border border-[#E5E7EB] px-3 py-2" value={bpm} onChange={(e) => setBpm(Math.max(40, Math.min(200, Number(e.target.value) || 90)))} /></label>
        {!listening
          ? <button className="rounded-full bg-[#22C55E] text-white font-bold px-5 py-2.5" onClick={() => void start()}>🎤 Listen</button>
          : <button className="rounded-full bg-[#DC2626] text-white font-bold px-5 py-2.5" onClick={stop}>■ Stop &amp; make song</button>}
        {listening && <div className="font-bold text-[#111827]">Listening {Math.floor(secs / 60)}:{String(secs % 60).padStart(2, '0')} · hearing <span className="text-[#16A34A]">{hearing !== null ? midiName(hearing) : '…'}</span></div>}
      </div>
      {err && <div className="rounded-xl bg-[#FEF2F2] text-[#B91C1C] px-4 py-3 mt-3">{err}</div>}
      <p className="text-xs text-[#6B7280] mt-3">Works best when one tune stands out (a singer or a solo instrument). It makes the right-hand melody; chords and drums are left out, and it can miss or add a few notes. Use 30 to 60 seconds of the song at a time. Listening stops on its own after 3 minutes.</p>
    </div>
  );
}
