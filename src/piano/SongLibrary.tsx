import { ask } from '../lib/embed';
import { useMemo, useState } from 'react';
import { useSearchParams } from 'react-router-dom';
import { useApp, recordSong, recordActivity, addImportedSong, removeImportedSong } from '../lib/store';
import { importMusicFile } from './importer';
import { useActiveMinutes, usePage } from '../lib/hooks';
import { songLibrary, LEVELS, CATEGORIES, type SongEntry, type Level, type Category } from './songs';
import type { Piece } from './notation';
import { SimplyPlayer } from './SimplyPlayer';
import type { PlayResult } from './engine';

const tagBg: Record<Level, string> = { Beginner: '#16A34A', Intermediate: '#0EA5E9', 'Pre-Advanced': '#F97316', Advanced: '#DC2626' };

function SongCard({ s, stars, onOpen }: { s: SongEntry; stars?: number; onOpen: () => void }) {
  const cat = CATEGORIES.find((c) => c.id === s.category)!;
  const tag = LEVELS.find((l) => l.id === s.level)!.tag;
  return (
    <button onClick={onOpen} className="w-44 shrink-0 text-left group">
      <div className="relative aspect-square rounded-2xl overflow-hidden shadow-[0_4px_14px_rgba(15,23,42,.12)] group-hover:-translate-y-1 group-hover:shadow-[0_10px_24px_rgba(15,23,42,.18)] transition"
        style={{ background: `linear-gradient(135deg, ${cat.from}, ${cat.to})` }}>
        <div className="absolute inset-0 flex items-center justify-center text-6xl drop-shadow">{cat.icon}</div>
        <span className="absolute top-2 left-2 text-[11px] font-extrabold text-white px-2 py-0.5 rounded-md" style={{ background: tagBg[s.level] }}>{tag}</span>
        {stars ? <span className="absolute bottom-2 right-2 bg-white/90 rounded-full px-2 text-sm text-[#EAB308]">{'★'.repeat(stars)}</span> : null}
        <span className="absolute inset-0 flex items-center justify-center opacity-0 group-hover:opacity-100 transition">
          <span className="w-14 h-14 rounded-full bg-white/95 text-[#16A34A] text-2xl flex items-center justify-center shadow-lg">▶</span>
        </span>
      </div>
      <div className="font-bold text-[#111827] mt-2 leading-tight line-clamp-2">{s.title}</div>
      <div className="text-sm text-[#6B7280] truncate">{s.composer}</div>
    </button>
  );
}

/** Song Player: a bright, Simply-Piano-style song library that opens a full-screen player. */
export function SongPlayer() {
  const base = useMemo(songLibrary, []);
  const importedSongs = useApp((s) => s.importedSongs);
  const lib = useMemo<SongEntry[]>(() => [...importedSongs.map((m) => ({ id: m.id, title: m.title, composer: m.composer, level: m.level, category: 'Imported' as Category, piece: () => m.piece })), ...base], [base, importedSongs]);
  const [dragging, setDragging] = useState(false);
  const [msg, setMsg] = useState<string | null>(null);
  const done = useApp((s) => s.piano.songs);
  const [params] = useSearchParams();
  const [open, setOpen] = useState<SongEntry | null>(() => lib.find((s) => s.id === params.get('id')) ?? null);
  const [imported, setImported] = useState<Piece | null>(null);
  const [level, setLevel] = useState<Level | 'all'>('all');
  const [cat, setCat] = useState<Category | 'all'>('all');
  const [q, setQ] = useState('');
  const [err, setErr] = useState<string | null>(null);
  const piece = useMemo(() => imported ?? open?.piece() ?? null, [open, imported]);
  useActiveMinutes('piano');
  usePage({ label: piece ? `Piano > Song Player: ${piece.title}` : 'Piano > Song Player', subject: 'piano' }, { kind: 'piano', path: '/songs' });

  const onFile = async (f: File) => {
    setErr(null); setMsg(null);
    try {
      const song = await importMusicFile(f);
      addImportedSong(song);
      setMsg(`✅ Imported "${song.title}" (${song.piece.events.length} notes, rated ${song.level}). It's saved in the 📂 Imported row.`);
      setImported(song.piece);
    } catch (e) { setErr(`Couldn't import ${f.name}: ${(e as Error).message}`); }
  };
  const onFinish = (r: PlayResult) => {
    if (r.mode !== 'perform' || !piece) return;
    recordSong(piece.id, r.stars, r.accuracy, r.missed.slice(0, 10));
    if (r.stars >= 1) recordActivity('piano');
  };
  const filtered = lib.filter((s) => (level === 'all' || s.level === level) && (cat === 'all' || s.category === cat)
    && `${s.title} ${s.composer}`.toLowerCase().includes(q.toLowerCase()));
  const browsing = level === 'all' && cat === 'all' && !q;
  const next = open ? filtered[(filtered.findIndex((s) => s.id === open.id) + 1) % filtered.length] : undefined;

  return (
    <div className={`-m-8 min-h-screen bg-[#F7F7FB] text-[#111827] p-8 ${dragging ? 'ring-4 ring-inset ring-[#22C55E]' : ''}`}
      onDragOver={(e) => { e.preventDefault(); setDragging(true); }} onDragLeave={() => setDragging(false)}
      onDrop={(e) => { e.preventDefault(); setDragging(false); Array.from(e.dataTransfer.files).forEach((f) => void onFile(f)); }}>
      <div className="max-w-[1400px] mx-auto">
        <div className="flex items-end justify-between gap-4 flex-wrap mb-6">
          <div>
            <h1 className="text-4xl font-black">Songs</h1>
            <p className="text-[#6B7280] mt-1">{lib.length} songs to play with your piano. Pick one and press Start — or drag a MusicXML / MIDI file here to import it.</p>
          </div>
          <div className="flex gap-2 items-center">
            <input className="w-72 rounded-full border border-[#E5E7EB] bg-white px-4 py-2.5 outline-none focus:border-[#22C55E] focus:ring-2 focus:ring-[#22C55E]/30" placeholder="🔍 Search songs or composers" value={q} onChange={(e) => setQ(e.target.value)} />
            <label className="rounded-full border border-[#E5E7EB] bg-white px-4 py-2.5 font-bold cursor-pointer hover:bg-[#F1F5F9]">📂 Import MusicXML / MIDI
              <input type="file" multiple accept=".xml,.musicxml,.mid,.midi,.mxl" className="hidden" onChange={(e) => { const fs = Array.from(e.target.files ?? []); e.target.value = ''; fs.forEach((f) => void onFile(f)); }} /></label>
          </div>
        </div>
        {msg && <div className="rounded-xl bg-[#F0FDF4] text-[#166534] px-4 py-3 mb-4">{msg}</div>}
        {err && <div className="rounded-xl bg-[#FEF2F2] text-[#B91C1C] px-4 py-3 mb-4">{err}</div>}

        <div className="flex gap-2 flex-wrap mb-3">
          <button onClick={() => setLevel('all')} className={`px-4 py-1.5 rounded-full font-bold border ${level === 'all' ? 'bg-[#111827] text-white border-[#111827]' : 'bg-white border-[#E5E7EB]'}`}>All levels</button>
          {LEVELS.map((l) => (
            <button key={l.id} onClick={() => setLevel(l.id)} className="px-4 py-1.5 rounded-full font-bold border-2 transition"
              style={{ borderColor: tagBg[l.id], color: level === l.id ? '#fff' : tagBg[l.id], background: level === l.id ? tagBg[l.id] : '#fff' }}>{l.tag} · {l.id}</button>
          ))}
        </div>
        <div className="flex gap-2 flex-wrap mb-8">
          <button onClick={() => setCat('all')} className={`px-3 py-1 rounded-full text-sm font-semibold border ${cat === 'all' ? 'bg-[#111827] text-white border-[#111827]' : 'bg-white border-[#E5E7EB]'}`}>All categories</button>
          {CATEGORIES.filter((c) => c.id !== 'Imported' || importedSongs.length).map((c) => (
            <button key={c.id} onClick={() => setCat(c.id)} className={`px-3 py-1 rounded-full text-sm font-semibold border ${cat === c.id ? 'bg-[#111827] text-white border-[#111827]' : 'bg-white border-[#E5E7EB]'}`}>{c.icon} {c.id}</button>
          ))}
        </div>

        {browsing ? CATEGORIES.filter((c) => lib.some((s) => s.category === c.id)).map((c) => {
          const row = lib.filter((s) => s.category === c.id);
          return (
            <section key={c.id} className="mb-8">
              <div className="flex items-baseline justify-between mb-3">
                <h2 className="text-2xl font-black">{c.icon} {c.id} <span className="text-base font-semibold text-[#9CA3AF]">{row.length}</span></h2>
                <button onClick={() => setCat(c.id)} className="text-sm font-bold text-[#16A34A] hover:underline">See all</button>
              </div>
              <div className="flex items-start gap-5 overflow-x-auto pb-3 pt-1 -mx-1 px-1">
                {row.map((s) => (
                  <div key={s.id} className="relative">
                    <SongCard s={s} stars={done[s.id]?.stars} onOpen={() => { setImported(null); setOpen(s); }} />
                    {c.id === 'Imported' && <button title="Remove from library" onClick={() => { if (ask(`Remove "${s.title}" from your library?`)) removeImportedSong(s.id); }} className="absolute top-1 right-1 w-7 h-7 rounded-full bg-white/90 border border-[#E5E7EB] text-sm hover:bg-[#FEE2E2]">✕</button>}
                  </div>
                ))}
              </div>
            </section>
          );
        }) : (
          <div className="grid grid-cols-[repeat(auto-fill,minmax(176px,1fr))] gap-6">
            {filtered.map((s) => <SongCard key={s.id} s={s} stars={done[s.id]?.stars} onOpen={() => { setImported(null); setOpen(s); }} />)}
            {!filtered.length && <div className="text-[#6B7280]">No songs match. Try another level or search word.</div>}
          </div>
        )}
      </div>

      {piece && (
        <SimplyPlayer key={piece.id} piece={piece} title={imported ? imported.title : open!.title} composer={imported ? imported.composer ?? 'Imported' : open!.composer}
          level={open && !imported ? open.level : undefined} best={done[piece.id]?.stars}
          onClose={() => { setOpen(null); setImported(null); }} onFinish={onFinish}
          onNext={!imported && next ? () => setOpen(next) : undefined} />
      )}
    </div>
  );
}
