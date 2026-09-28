import { useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { songLibrary, LEVELS, Level, SongEntry } from './songs';
import { toMusicXML, pieceLength, measureLen } from './notation';
import { SheetMusic } from './views';
import { PageHeader } from '../components/ui';
import { usePage } from '../lib/hooks';
import { embedded } from '../lib/embed';

const KEYS: Record<number, string> = { [-5]: 'D♭ major', [-3]: 'E♭ major', [-2]: 'B♭ major', [-1]: 'F major', 0: 'C major / A minor', 1: 'G major / E minor', 2: 'D major', 4: 'E major / C♯ minor' };

export function LevelTag({ level, big }: { level: Level; big?: boolean }) {
  const l = LEVELS.find((x) => x.id === level)!;
  return <span className={`inline-block rounded-md font-extrabold tracking-wide border ${big ? 'text-sm px-2.5 py-1' : 'text-[11px] px-2 py-0.5'}`} style={{ color: l.color, borderColor: l.color, background: `${l.color}1a` }}>{l.tag}</span>;
}

/** Every song's sheet music in one place, labeled BEG / INT / PRE-ADV / ADV. */
export function SheetLibrary() {
  const lib = useMemo(songLibrary, []);
  const nav = useNavigate();
  const [filter, setFilter] = useState<Level | 'all'>('all');
  const [q, setQ] = useState('');
  const [open, setOpen] = useState<SongEntry | null>(null);
  usePage({ label: open ? `Piano > Sheet Music: ${open.title}` : 'Piano > Sheet Music', subject: 'piano' });
  const piece = useMemo(() => open?.piece() ?? null, [open]);
  const xml = useMemo(() => (piece ? toMusicXML(piece) : ''), [piece]);

  if (open && piece) {
    const bars = Math.round(pieceLength(piece) / measureLen(piece));
    const download = () => {
      const a = document.createElement('a');
      a.href = URL.createObjectURL(new Blob([xml], { type: 'application/vnd.recordare.musicxml+xml' }));
      a.download = `${open.title.replace(/[^\w]+/g, '-')}.musicxml`;
      a.click();
    };
    return (
      <div>
        <PageHeader title={<span className="flex items-center gap-3"><LevelTag level={open.level} big />{open.title}</span>}
          sub={`${open.composer} · ${KEYS[piece.key ?? 0] ?? ''} · ${piece.beats}/${piece.beatUnit} · ${piece.bpm} bpm · ${bars} measures`}
          right={<div className="flex gap-2 shrink-0 whitespace-nowrap">
            <button className="btn" onClick={() => nav(`/songs?id=${open.id}`)}>▶ Play along</button>
            {!embedded && <button className="btn-ghost" onClick={download} title="Open in MuseScore or other notation apps">⬇️ MusicXML</button>}
            <button className="btn-ghost" onClick={() => setOpen(null)}>← All sheets</button>
          </div>} />
        <SheetMusic xml={xml} height={640} zoom={1.1} />
      </div>
    );
  }

  const shown = lib.filter((s) => (filter === 'all' || s.level === filter) && `${s.title} ${s.composer}`.toLowerCase().includes(q.toLowerCase()));
  return (
    <div>
      <PageHeader title="Sheet Music" sub={`${lib.length} public-domain pieces, from first songs to concert repertoire.`} />
      <div className="flex items-center gap-2 mb-6 flex-wrap">
        <button onClick={() => setFilter('all')} className={`px-3 py-1.5 rounded-lg border font-bold ${filter === 'all' ? 'bg-accent border-accent' : 'border-edge/40'}`}>All</button>
        {LEVELS.map((l) => (
          <button key={l.id} onClick={() => setFilter(l.id)} className="px-3 py-1.5 rounded-lg border-2 font-extrabold transition"
            style={{ color: filter === l.id ? '#0B1026' : l.color, borderColor: l.color, background: filter === l.id ? l.color : 'transparent' }}>
            {l.tag} <span className="font-semibold opacity-80">· {l.id}</span>
          </button>
        ))}
        <input className="input ml-auto w-64" placeholder="🔍 Search title or composer" value={q} onChange={(e) => setQ(e.target.value)} />
      </div>
      {LEVELS.filter((l) => filter === 'all' || filter === l.id).map((l) => {
        const items = shown.filter((s) => s.level === l.id);
        if (!items.length) return null;
        return (
          <div key={l.id} className="mb-8">
            <h2 className="h2 mb-3 flex items-center gap-2"><LevelTag level={l.id} big />{l.id}</h2>
            <div className="grid grid-cols-4 gap-3">
              {items.map((s) => (
                <button key={s.id} onClick={() => setOpen(s)} className="card text-left p-4 flex flex-col gap-2 hover:-translate-y-0.5 transition">
                  <div className="flex justify-between items-start"><LevelTag level={s.level} /><span className="text-2xl">📜</span></div>
                  <div className="font-bold leading-tight">{s.title}</div>
                  <div className="text-xs muted">{s.composer}</div>
                </button>
              ))}
            </div>
          </div>
        );
      })}
      {!shown.length && <div className="card muted">No pieces match your search.</div>}
    </div>
  );
}
