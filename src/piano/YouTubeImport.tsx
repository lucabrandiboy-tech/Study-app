import { useEffect, useRef, useState } from 'react';
import { addImportedSong } from '../lib/store';
import { midiName, type Piece } from './notation';
import { playYouTube } from '../components/MiniPlayer';
import { startListening, framesToNotes, notesToPiece, pieceToSong, youTubeId, canHearComputer, NoSoundShared, type Frame, type ListenSource } from './listen';

/** Song Player panel: play a YouTube video inside the app, and turn its melody into a song by listening to it
 *  (through the computer's own sound, which works with headphones, or through the microphone). */
export function YouTubeImport({ onDone, onClose }: { onDone: (p: Piece, msg: string) => void; onClose: () => void }) {
  const [link, setLink] = useState('');
  const [vid, setVid] = useState<string | null>(null);
  const [name, setName] = useState('');
  const [bpm, setBpm] = useState(90);
  const [hearing, setHearing] = useState<number | null>(null);
  const [listening, setListening] = useState<ListenSource | null>(null);
  const [ended, setEnded] = useState(false);
  const [secs, setSecs] = useState(0);
  const [err, setErr] = useState<string | null>(null);
  const computer = canHearComputer();
  const stopRef = useRef<(() => Frame[]) | null>(null);
  const starting = useRef(false);
  const alive = useRef(true);
  useEffect(() => { alive.current = true; return () => { alive.current = false; stopRef.current?.(); stopRef.current = null; }; }, []);
  useEffect(() => {
    if (!listening) return;
    const t0 = Date.now();
    const id = setInterval(() => setSecs(Math.floor((Date.now() - t0) / 1000)), 500);
    return () => clearInterval(id);
  }, [listening]);

  const load = () => { const id = youTubeId(link); setErr(id ? null : "That doesn't look like a YouTube link. Copy the link from the video's Share button."); setVid(id); if (id) playYouTube(id); };
  const start = async (source: ListenSource) => {
    if (starting.current || stopRef.current) return;
    starting.current = true;
    setErr(null);
    try {
      const l = await startListening(source, setHearing, () => setEnded(true));
      if (!alive.current) { l.stop(); return; } // the panel was closed while the browser was asking
      stopRef.current = l.stop; setSecs(0); setListening(source);
    } catch (e) {
      setErr(source === 'mic' ? 'The app needs the microphone to listen. Allow microphone access, then try again.'
        : e instanceof NoSoundShared ? "No sound came through. Press 🎧 again, pick the tab that's playing the song, and switch on Also share tab audio before you press Share. This needs Chrome or Edge."
        : "Sharing was cancelled or blocked. Press 🎧 again, pick the tab that's playing the song, and press Share.");
    } finally { starting.current = false; }
  };
  const stop = () => {
    const source = listening;
    const frames = stopRef.current?.() ?? [];
    stopRef.current = null; setListening(null); setHearing(null);
    const notes = framesToNotes(frames);
    if (notes.length < 4) {
      setErr(source === 'computer'
        ? "I didn't hear a clear melody. Make sure the song is playing in the tab you shared, and pick a part where the tune stands out."
        : "I didn't hear a clear melody. The mic can't hear headphones: play the song out loud, keep the mic near the speaker, and pick a part where the tune stands out.");
      return;
    }
    const piece = notesToPiece(notes, name.trim() || 'Song from YouTube', bpm);
    addImportedSong(pieceToSong(piece));
    onDone(piece, `✅ Made "${piece.title}" from what I heard (${piece.events.length} notes). It's saved in the 📂 Imported row.`);
  };
  // Listening ended on its own (the browser's "Stop sharing" button, or 3 minutes): make the song from what was heard.
  useEffect(() => { if (ended) { setEnded(false); if (stopRef.current) stop(); } }, [ended]);

  return (
    <div className="rounded-2xl bg-white border border-[#E5E7EB] p-4 mb-6 shadow-sm">
      <div className="flex items-center justify-between gap-2 mb-2">
        <h2 className="text-xl font-black">▶️ Learn a song from YouTube</h2>
        <button className="text-[#6B7280] px-2 py-1 rounded-lg hover:bg-[#F1F5F9]" onClick={() => { stopRef.current?.(); onClose(); }} aria-label="Close">✕</button>
      </div>
      <p className="text-sm text-[#6B7280] mb-3">Paste a YouTube link to watch it here. Then press <b>Listen</b> while it plays: the app hears the melody and turns it into a song you can practice.</p>
      <div className="flex gap-2 flex-wrap mb-3">
        <input className="flex-1 min-w-[220px] rounded-full border border-[#E5E7EB] px-4 py-2.5 outline-none focus:border-[#22C55E]" placeholder="https://www.youtube.com/watch?v=…" value={link}
          onChange={(e) => setLink(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter') load(); }} autoComplete="off" />
        <button className="rounded-full bg-[#111827] text-white font-bold px-5 py-2.5" onClick={load}>Load video</button>
      </div>
      {vid && <div className="rounded-xl bg-[#F0FDF4] text-[#166534] px-4 py-3 mb-3">▶️ Playing in the mini player (bottom-right corner). It keeps playing while you go to other pages; tap ▾ Hide to shrink it or ✕ to stop.</div>}
      <div className="flex gap-3 flex-wrap items-end">
        <label className="text-sm font-semibold">Song name<br /><input className="mt-1 w-56 rounded-lg border border-[#E5E7EB] px-3 py-2" value={name} onChange={(e) => setName(e.target.value)} placeholder="Song from YouTube" autoComplete="off" /></label>
        <label className="text-sm font-semibold">Speed (BPM)<br /><input type="number" min={40} max={200} className="mt-1 w-24 rounded-lg border border-[#E5E7EB] px-3 py-2" value={bpm} onChange={(e) => setBpm(Math.max(40, Math.min(200, Number(e.target.value) || 90)))} /></label>
        {!listening
          ? <>
              {computer && <button className="rounded-full bg-[#22C55E] text-white font-bold px-5 py-2.5" onClick={() => void start('computer')}>🎧 Listen to this computer</button>}
              <button className={computer ? 'rounded-full border border-[#E5E7EB] bg-white font-bold px-5 py-2.5 hover:bg-[#F1F5F9]' : 'rounded-full bg-[#22C55E] text-white font-bold px-5 py-2.5'}
                onClick={() => void start('mic')}>🎤 {computer ? 'Listen with mic' : 'Listen'}</button>
            </>
          : <button className="rounded-full bg-[#DC2626] text-white font-bold px-5 py-2.5" onClick={stop}>■ Stop &amp; make song</button>}
        {listening && <div className="font-bold text-[#111827]">Listening {Math.floor(secs / 60)}:{String(secs % 60).padStart(2, '0')} · hearing <span className="text-[#16A34A]">{hearing !== null ? midiName(hearing) : '…'}</span></div>}
      </div>
      {computer && !listening && <p className="text-sm text-[#374151] mt-3">🎧 <b>Wearing headphones?</b> Use <b>Listen to this computer</b>. A sharing box opens: pick the tab that's playing the song (this tab, if it's in the mini player), switch on <b>Also share tab audio</b>, then press <b>Share</b>. You keep hearing the song in your headphones. 🎤 The mic only works when the song comes out of speakers.</p>}
      {listening === 'computer' && <p className="text-sm text-[#374151] mt-3">Press <b>■ Stop &amp; make song</b> when you're done (the browser's <b>Stop sharing</b> button does the same).</p>}
      {err && <div className="rounded-xl bg-[#FEF2F2] text-[#B91C1C] px-4 py-3 mt-3">{err}</div>}
      <p className="text-xs text-[#6B7280] mt-3">Works best when one tune stands out (a singer or a solo instrument). It makes the right-hand melody; chords and drums are left out, and it can miss or add a few notes. Use 30 to 60 seconds of the song at a time. Listening stops on its own after 3 minutes.</p>
    </div>
  );
}
