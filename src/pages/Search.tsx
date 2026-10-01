import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { useLocation, useNavigate, useSearchParams } from 'react-router-dom';
import { useApp, openChat } from '../lib/store';
import { embedded } from '../lib/embed';
import { usePage } from '../lib/hooks';
import { useSubjects } from '../study/pages';
import { findTopic } from '../study/subjects';
import { songLibrary } from '../piano/songs';
import { staticIndex, userIndex, runSearch, snippet, matchRanges, queryTerms, KIND_GROUP, KIND_LABEL, type SearchItem, type Kind } from '../lib/search';
import { wikiSearch, wikiArticle, wikiRelated, imageSearch, imageDetails, imageName, WIKI_NAME, type WikiSite, type WikiHit, type WikiArticle, type WikiImage, type ImageDetails } from '../lib/wiki';
import { PageHeader, Rich, Stars } from '../components/ui';

const INPUT_ID = 'search-input';
const isPhone = () => window.matchMedia('(max-width: 767px)').matches;
/** Puts the cursor at the end of the Search page's box. */
export function focusSearch() {
  const el = document.getElementById(INPUT_ID) as HTMLInputElement | null;
  if (!el) return;
  el.focus();
  el.setSelectionRange(el.value.length, el.value.length);
}

/** Ctrl+K / ⌘K anywhere opens Search. */
export function SearchHotkey() {
  const nav = useNavigate();
  const here = useLocation().pathname === '/search';
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.ctrlKey || e.metaKey) || e.altKey || e.shiftKey || e.key.toLowerCase() !== 'k') return;
      e.preventDefault();
      if (!here) nav('/search');
      setTimeout(focusSearch, 0);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [nav, here]);
  return null;
}

/** The search box at the top of the sidebar: typing jumps to the Search page, which keeps the text and takes over. */
export function SidebarSearch() {
  const nav = useNavigate();
  const here = useLocation().pathname === '/search';
  const [v, setV] = useState(''); // keeps fast typing until the Search page's box takes the focus
  return (
    <div className="relative mb-4">
      <span className="absolute left-3 top-1/2 -translate-y-1/2 text-sm pointer-events-none">🔍</span>
      <input className="input w-full pl-9 pr-14 text-sm" placeholder="Search…" value={v} aria-label="Search the app"
        onFocus={() => { if (here) focusSearch(); }}
        onBlur={() => setV('')}
        onChange={(e) => { setV(e.target.value); if (e.target.value) nav(`/search?q=${encodeURIComponent(e.target.value)}`, { replace: here }); }}
        onKeyDown={(e) => { if (e.key === 'Enter') nav('/search'); }} />
      <kbd className="absolute right-2 top-1/2 -translate-y-1/2 text-[10px] muted border border-edge/30 rounded px-1 pointer-events-none">Ctrl K</kbd>
    </div>
  );
}

/** Text with the searched words highlighted. */
function Hl({ text, terms }: { text: string; terms: string[] }) {
  const ranges = matchRanges(text, terms);
  if (!ranges.length) return <>{text}</>;
  const parts: ReactNode[] = [];
  let at = 0;
  ranges.forEach(([a, b], i) => {
    if (a > at) parts.push(text.slice(at, a));
    parts.push(<mark key={i} className="bg-accent/40 text-ink rounded px-0.5">{text.slice(a, b)}</mark>);
    at = b;
  });
  if (at < text.length) parts.push(text.slice(at));
  return <>{parts}</>;
}

function Row({ icon, title, sub, text, label, active, terms, onClick }: { icon: string; title: string; sub: string; text?: string; label: string; active: boolean; terms: string[]; onClick: () => void }) {
  return (
    <button onClick={onClick} className={`w-full text-left rounded-xl px-3 py-2.5 border transition flex gap-3 items-start ${active ? 'bg-accent/20 border-edge/60' : 'border-transparent hover:bg-white/5'}`}>
      <span className="text-xl leading-7 w-7 text-center shrink-0">{icon}</span>
      <span className="min-w-0 flex-1">
        <span className="flex items-baseline gap-2"><span className="font-bold truncate"><Hl text={title} terms={terms} /></span><span className="chip shrink-0">{label}</span></span>
        <span className="block text-xs muted truncate">{sub}</span>
        {text && <span className="block text-sm text-ink/80 mt-0.5 line-clamp-2"><Hl text={text} terms={terms} /></span>}
      </span>
    </button>
  );
}

const OPEN: Record<Kind, string> = {
  page: 'Go there →', topic: 'Open the lesson & practice →', vocab: 'Open the lesson →', piano: 'Open in the Piano Course →', song: '▶ Play it',
  homework: 'Open the calendar →', test: 'Open the A+ Plan →', note: 'Open to edit →', deck: 'Study this deck →',
};

function TopicBody({ refId, terms }: { refId: string; terms: string[] }) {
  const subjects = useSubjects();
  const found = findTopic(subjects, refId.split('/')[1] ?? '');
  if (!found) return null;
  const { topic } = found;
  const hit = (v: { term: string; def: string }) => (matchRanges(`${v.term} ${v.def}`, terms).length ? 1 : 0);
  const vocab = [...topic.vocab].sort((a, b) => hit(b) - hit(a));
  return (
    <div className="space-y-4">
      <Rich text={topic.lesson} />
      {!!topic.formulas?.length && (
        <div><div className="font-bold mb-1">Formulas</div>
          {topic.formulas.map((f) => <div key={f.name} className="text-sm"><span className="muted">{f.name}:</span> <span className="font-mono">{f.f}</span></div>)}</div>
      )}
      {!!vocab.length && (
        <div><div className="font-bold mb-1">Vocabulary</div>
          <div className="space-y-1.5">{vocab.map((v) => <div key={v.term} className="text-sm"><span className="font-bold text-edge"><Hl text={v.term} terms={terms} /></span> — <Hl text={v.def} terms={terms} /></div>)}</div></div>
      )}
    </div>
  );
}

function SongBody({ item }: { item: SearchItem }) {
  const best = useApp((s) => s.piano.songs[item.ref]?.stars);
  const song = useMemo(() => songLibrary().find((s) => s.id === item.ref), [item.ref]);
  return (
    <div className="space-y-1">
      <div><span className="muted">By</span> {song?.composer ?? item.sub.split(' · ')[0]}</div>
      {song && <div><span className="muted">Level:</span> {song.level} · <span className="muted">Category:</span> {song.category}</div>}
      <div><span className="muted">Your best:</span> {best ? <Stars n={best} size="text-base" /> : 'not played yet'}</div>
    </div>
  );
}

function Preview({ item, terms }: { item: SearchItem; terms: string[] }) {
  const nav = useNavigate();
  const isStudy = item.kind === 'topic' || item.kind === 'vocab';
  return (
    <div>
      <div className="flex items-start gap-3 mb-3">
        <div className="text-4xl leading-none">{item.icon}</div>
        <div className="min-w-0 flex-1">
          <div className="text-xs muted">{KIND_LABEL[item.kind]} · {item.sub}</div>
          <h2 className="h2">{item.title}</h2>
        </div>
      </div>
      {item.kind === 'vocab' && <div className="rounded-xl bg-navy border border-edge/40 px-4 py-3 mb-4 text-lg"><Hl text={item.body} terms={terms} /></div>}
      <div className="flex gap-2 flex-wrap mb-4">
        <button className="btn" onClick={() => nav(item.path)}>{OPEN[item.kind]}</button>
        {isStudy && <button className="btn-ghost" onClick={() => openChat(`Can you explain "${item.title}" in simple words?`)}>🤖 Ask Study Buddy</button>}
      </div>
      {item.kind === 'vocab' && <div className="text-xs muted uppercase tracking-wide mb-2">From the lesson</div>}
      {isStudy ? <TopicBody refId={item.ref} terms={terms} /> : item.kind === 'song' ? <SongBody item={item} /> : item.body ? <Rich text={item.body} /> : null}
    </div>
  );
}

const OFFLINE = "You're offline, so looking things up doesn't work right now. Search inside the app still works.";
function netMessage(e: unknown): string {
  if (!navigator.onLine) return OFFLINE;
  if (e instanceof TypeError) return embedded
    ? "Couldn't reach Wikipedia from inside this viewer. The phone app and the downloaded app can look things up. Search inside the app still works."
    : "Couldn't reach Wikipedia. Check your internet connection and try again. Search inside the app still works.";
  return (e as Error)?.message || 'Something went wrong. Try again.';
}

function WikiReader({ title, site, terms, onOpen }: { title: string; site: WikiSite; terms: string[]; onOpen: (t: string) => void }) {
  const [art, setArt] = useState<WikiArticle | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const [related, setRelated] = useState<string[]>([]);
  useEffect(() => {
    const ac = new AbortController();
    setArt(null); setErr(null); setRelated([]);
    wikiArticle(title, site, ac.signal).then(setArt).catch((e) => { if (!ac.signal.aborted) setErr(netMessage(e)); });
    wikiRelated(title, site, ac.signal).then(setRelated).catch(() => { /* optional */ });
    return () => ac.abort();
  }, [title, site]);
  return (
    <div>
      <div className="flex items-start gap-3 mb-3">
        <div className="text-4xl leading-none">🌐</div>
        <div className="min-w-0 flex-1">
          <div className="text-xs muted">From {WIKI_NAME[site]} · reading it right here in the app</div>
          <h2 className="h2">{art?.title ?? title}</h2>
        </div>
      </div>
      {!art && !err && <div className="muted animate-pulse">Loading the article…</div>}
      {err && <div className="text-bad">{err}</div>}
      {art && (
        <article className="leading-relaxed">
          {art.image && <img src={art.image} alt="" referrerPolicy="no-referrer" loading="lazy" className="float-right ml-4 mb-2 w-36 md:w-52 rounded-xl border border-edge/30 bg-white" />}
          {art.sections.map((s, i) => (
            <section key={i}>
              {s.heading && (s.level <= 2 ? <h3 className="text-lg font-bold mt-5 mb-1 text-edge">{s.heading}</h3> : <h4 className="font-bold mt-3 mb-1">{s.heading}</h4>)}
              {s.paras.map((p, j) => <p key={j} className="mb-2"><Hl text={p} terms={terms} /></p>)}
            </section>
          ))}
          <div className="clear-both" />
        </article>
      )}
      {!!related.length && (
        <div className="mt-5">
          <div className="font-bold mb-2">Keep reading</div>
          <div className="flex flex-wrap gap-2">{related.map((t) => <button key={t} className="btn-ghost text-sm py-1 px-3" onClick={() => onOpen(t)}>{t}</button>)}</div>
        </div>
      )}
      {art && <p className="text-xs muted mt-5">Text from {WIKI_NAME[site]}, shared under the Creative Commons Attribution-ShareAlike license. Wikipedia is written by volunteers, so check important facts against your textbook or teacher.</p>}
    </div>
  );
}

function ImageGrid({ items, active, onPick }: { items: WikiImage[]; active?: string; onPick: (img: WikiImage) => void }) {
  return (
    <div className="keep-cols grid grid-cols-3 sm:grid-cols-4 gap-2">
      {items.map((img) => (
        <button key={img.file} onClick={() => onPick(img)} title={imageName(img.file)}
          className={`aspect-square rounded-lg overflow-hidden bg-navy border transition ${active === img.file ? 'border-edge ring-2 ring-edge' : 'border-edge/20 hover:border-edge/70'}`}>
          <img src={img.thumb} alt={img.caption || imageName(img.file)} loading="lazy" referrerPolicy="no-referrer" className="w-full h-full object-cover"
            onError={(e) => { (e.currentTarget.parentElement as HTMLElement).style.display = 'none'; }} />
        </button>
      ))}
    </div>
  );
}

/** A picture shown big, right here, with what it shows and who made it. */
function ImageViewer({ img, onArticle }: { img: WikiImage; onArticle: (title: string) => void }) {
  const [d, setD] = useState<ImageDetails | null>(null);
  useEffect(() => {
    const ac = new AbortController();
    setD(null);
    imageDetails(img, ac.signal).then(setD).catch(() => { /* keep the small copy */ });
    return () => ac.abort();
  }, [img]);
  const caption = d?.caption || img.caption;
  const credit = d?.credit || img.credit;
  return (
    <div>
      <div className="flex items-start gap-3 mb-3">
        <div className="text-4xl leading-none">🖼️</div>
        <div className="min-w-0 flex-1">
          <div className="text-xs muted">{img.article ? `Picture from the Wikipedia article "${img.article}"` : 'Picture from Wikimedia Commons'}</div>
          <h2 className="h2 break-words">{imageName(img.file)}</h2>
        </div>
      </div>
      <div className="rounded-xl overflow-hidden bg-navy border border-edge/30 flex items-center justify-center min-h-40">
        <img src={d?.large ?? img.thumb} alt={caption || imageName(img.file)} referrerPolicy="no-referrer" className="max-h-[60vh] max-w-full object-contain" />
      </div>
      {caption && caption !== img.article && <p className="mt-3 leading-relaxed">{caption.length > 600 ? `${caption.slice(0, 600)}…` : caption}</p>}
      {credit && <p className="text-xs muted mt-2">📷 {credit}</p>}
      {img.article && <button className="btn mt-3" onClick={() => onArticle(img.article!)}>📖 Read about {img.article}</button>}
      <p className="text-xs muted mt-4">These are free pictures shared by their makers. If you use one in a school project, give credit like the line above. Pictures are filtered for school, but if you ever see something that isn't okay, tell a parent or teacher.</p>
    </div>
  );
}

type Tab = 'all' | 'study' | 'piano' | 'yours' | 'images' | 'web';
type Sel = { kind: 'app'; item: SearchItem } | { kind: 'web'; title: string } | { kind: 'image'; img: WikiImage } | null;
interface Web { q: string; hits: WikiHit[]; suggestion?: string; loading: boolean; err?: string }
interface Pics { q: string; items: WikiImage[]; loading: boolean; err?: string; blocked?: boolean }
let lastSite: WikiSite = 'simple';
let builtIn: { lang: string; items: SearchItem[] } | null = null; // the app's own content, indexed once per language
const EXAMPLES = ['photosynthesis', 'Pythagorean theorem', 'Civil War', 'verbs', 'Beethoven', 'Moon'];

/** Search: finds anything in the app, and looks things up on Wikipedia. Results open in a preview here instead of sending you away. */
export function SearchPage() {
  const [params, setParams] = useSearchParams();
  const [text, setText] = useState(() => params.get('q') ?? '');
  const [tab, setTab] = useState<Tab>('all');
  const [sel, setSel] = useState<Sel>(null);
  const [more, setMore] = useState(0);
  const [site, setSiteState] = useState<WikiSite>(lastSite);
  const setSite = (s: WikiSite) => { lastSite = s; setSiteState(s); if (sel && sel.kind !== 'app') setSel(null); };
  const [web, setWeb] = useState<Web>({ q: '', hits: [], loading: false });
  const [pics, setPics] = useState<Pics>({ q: '', items: [], loading: false });
  const picFilter = useApp((s) => s.settings.picFilter);
  const previewBox = useRef<HTMLDivElement>(null);
  usePage({ label: 'Search', subject: 'general' });

  // keep the box and the address (?q=) in step, both ways
  const urlQ = params.get('q') ?? '';
  useEffect(() => { if (urlQ !== text) setParams(text ? { q: text } : {}, { replace: true }); }, [text]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { if (urlQ !== text) setText(urlQ); }, [urlQ]); // eslint-disable-line react-hooks/exhaustive-deps
  useEffect(() => { focusSearch(); }, []);
  useEffect(() => { setSel(null); setMore(0); }, [text]);
  useEffect(() => { previewBox.current?.scrollTo(0, 0); if (sel && isPhone()) window.scrollTo(0, 0); }, [sel]);

  const subjects = useSubjects();
  const lang = useApp((s) => s.settings.language);
  const homework = useApp((s) => s.homework), tests = useApp((s) => s.tests), notes = useApp((s) => s.notes), decks = useApp((s) => s.decks), importedSongs = useApp((s) => s.importedSongs);
  const fixed = useMemo(() => {
    if (builtIn?.lang !== lang) builtIn = { lang, items: staticIndex(subjects) };
    return builtIn.items;
  }, [subjects, lang]);
  const mine = useMemo(() => userIndex({ homework, tests, notes, decks, importedSongs }, subjects), [homework, tests, notes, decks, importedSongs, subjects]);
  const terms = useMemo(() => queryTerms(text), [text]);
  const hits = useMemo(() => runSearch([...mine, ...fixed], text), [mine, fixed, text]);
  const count = (g: Tab) => hits.filter((h) => KIND_GROUP[h.kind] === g).length;
  const songCount = useMemo(() => fixed.filter((i) => i.kind === 'song').length, [fixed]);

  // Wikipedia look-up, a moment after typing stops
  useEffect(() => {
    const q = text.trim();
    if (q.length < 2) { setWeb({ q: '', hits: [], loading: false }); return; }
    if (!navigator.onLine) { setWeb({ q, hits: [], loading: false, err: OFFLINE }); return; }
    setWeb((w) => ({ ...w, loading: true, err: undefined }));
    const ac = new AbortController();
    const id = setTimeout(() => {
      wikiSearch(q, site, ac.signal).then((r) => setWeb({ q, ...r, loading: false })).catch((e) => { if (!ac.signal.aborted) setWeb({ q, hits: [], loading: false, err: netMessage(e) }); });
    }, 400);
    return () => { clearTimeout(id); ac.abort(); };
  }, [text, site]);

  // Pictures, the same way
  useEffect(() => {
    const q = text.trim();
    if (q.length < 2) { setPics({ q: '', items: [], loading: false }); return; }
    if (!navigator.onLine) { setPics({ q, items: [], loading: false, err: OFFLINE }); return; }
    setPics((p) => ({ ...p, loading: true, err: undefined }));
    const ac = new AbortController();
    const id = setTimeout(() => {
      imageSearch(q, site, ac.signal, picFilter).then((r) => setPics({ q, ...r, loading: false })).catch((e) => { if (!ac.signal.aborted) setPics({ q, items: [], loading: false, err: netMessage(e) }); });
    }, 450);
    return () => { clearTimeout(id); ac.abort(); };
  }, [text, site, picFilter]);

  const appTab = tab !== 'web' && tab !== 'images';
  const shown = tab === 'all' ? hits.slice(0, 8 + more) : !appTab ? [] : hits.filter((h) => KIND_GROUP[h.kind] === tab).slice(0, 40 + more);
  const total = tab === 'all' ? hits.length : !appTab ? 0 : count(tab);
  const pickFirst = () => {
    if (shown[0]) setSel({ kind: 'app', item: shown[0] });
    else if (tab === 'images' && pics.items[0]) setSel({ kind: 'image', img: pics.items[0] });
    else if ((tab === 'all' || tab === 'web') && web.hits[0]) setSel({ kind: 'web', title: web.hits[0].title });
  };

  const chips: [Tab, string, number | null][] = [['all', 'All', null], ['study', '📚 Study', count('study')], ['piano', '🎹 Piano & songs', count('piano')], ['yours', '🗂️ Your stuff', count('yours')], ['images', '🖼️ Pictures', pics.q ? pics.items.length : null], ['web', '🌐 Look it up', web.q ? web.hits.length : null]];
  const picsSection = (
    <div className="p-1">
      <div className="flex items-center justify-between gap-2 px-1 mb-1">
        <div className="font-bold">🖼️ Pictures</div>
        {tab === 'all' && pics.items.length > 8 && <button className="text-sm font-bold text-edge hover:underline" onClick={() => setTab('images')}>See all {pics.items.length} →</button>}
      </div>
      <div className="text-xs muted px-1 mb-2">Free pictures from Wikipedia and Wikimedia Commons. Tap one to see it big, right here.{!picFilter && ' Picture filter is off.'}</div>
      {pics.loading && <div className="muted text-sm px-1 animate-pulse">Finding pictures…</div>}
      {!pics.loading && pics.err && <div className="text-sm text-bad px-1">{pics.err}</div>}
      {!pics.loading && pics.blocked && <div className="muted text-sm px-1">Pictures are turned off for this search.</div>}
      {!pics.loading && !pics.err && !pics.blocked && pics.q && !pics.items.length && <div className="muted text-sm px-1">No pictures found.</div>}
      {!pics.loading && !!pics.items.length && (
        <ImageGrid items={tab === 'all' ? pics.items.slice(0, 8) : pics.items} active={sel?.kind === 'image' ? sel.img.file : undefined} onPick={(img) => setSel({ kind: 'image', img })} />
      )}
    </div>
  );
  const webSection = (
    <div className="p-1">
      <div className="flex items-center gap-2 flex-wrap mb-1 px-1">
        <div className="font-bold">🌐 Look it up</div>
        <div className="flex gap-1 p-0.5 bg-navy rounded-lg border border-edge/30 text-xs">
          {(['simple', 'en'] as const).map((s) => <button key={s} onClick={() => setSite(s)} className={`px-2 py-1 rounded-md font-semibold ${site === s ? 'bg-accent text-white' : 'muted hover:text-ink'}`}>{s === 'simple' ? 'Simple English' : 'Full Wikipedia'}</button>)}
        </div>
      </div>
      <div className="text-xs muted px-1 mb-2">Articles from {WIKI_NAME[site]} open here in the app. No website opens.</div>
      {web.loading && <div className="muted text-sm px-1 animate-pulse">Looking it up…</div>}
      {!web.loading && web.err && <div className="text-sm text-bad px-1">{web.err}</div>}
      {!web.loading && !web.err && web.suggestion && <div className="text-sm px-1 mb-1">Did you mean <button className="text-edge font-bold hover:underline" onClick={() => setText(web.suggestion!)}>{web.suggestion}</button>?</div>}
      {!web.loading && !web.err && web.q && !web.hits.length && <div className="muted text-sm px-1">No articles found.</div>}
      {!web.loading && web.hits.map((h) => (
        <Row key={h.title} icon="📖" title={h.title} sub={WIKI_NAME[site]} text={h.snippet} label="Article" terms={terms}
          active={sel?.kind === 'web' && sel.title === h.title} onClick={() => setSel({ kind: 'web', title: h.title })} />
      ))}
    </div>
  );

  return (
    <div>
      <PageHeader title="🔍 Search" sub="Find anything in the app, look something up, or find pictures. Results open right here." />
      <div className="relative mb-3">
        <span className="absolute left-4 top-1/2 -translate-y-1/2 pointer-events-none">🔍</span>
        <input id={INPUT_ID} className="input w-full text-lg py-3 pl-11 pr-12" placeholder="Search lessons, vocab, songs, homework… or look anything up" value={text} autoComplete="off" enterKeyHint="search"
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => { if (e.key === 'Enter') { e.preventDefault(); pickFirst(); if (isPhone()) (e.target as HTMLInputElement).blur(); } else if (e.key === 'Escape') { if (sel) setSel(null); else setText(''); } }} />
        {text && <button className="absolute right-2 top-1/2 -translate-y-1/2 w-9 h-9 rounded-lg muted hover:text-ink" onClick={() => { setText(''); setTimeout(focusSearch, 0); }} aria-label="Clear search">✕</button>}
      </div>

      {!terms.length ? (
        <div className="card">
          <div className="h2 mb-2">What can I search?</div>
          <ul className="space-y-1 mb-4">
            <li>📚 Every lesson, vocab word and formula in your courses</li>
            <li>🎹 Piano lessons and all {songCount.toLocaleString()} songs</li>
            <li>🗂️ Your homework, tests, notes and flashcard decks</li>
            <li>🖼️ Pictures of almost anything, from Wikipedia and Wikimedia Commons</li>
            <li>🌐 Anything else, looked up on Wikipedia and shown right here. No website opens and there's no API key.</li>
          </ul>
          <div className="text-sm muted mb-2">Try:</div>
          <div className="flex flex-wrap gap-2">{EXAMPLES.map((x) => <button key={x} className="btn-ghost text-sm py-1 px-3" onClick={() => setText(x)}>{x}</button>)}</div>
          <div className="hidden md:block text-xs muted mt-4">Tip: press Ctrl K (⌘K on a Mac) on any page to jump here.</div>
        </div>
      ) : (
        <>
          <div className="flex flex-wrap gap-2 mb-3">
            {chips.map(([id, label, n]) => (
              <button key={id} onClick={() => { setTab(id); setMore(0); }} className={`px-3 py-1.5 rounded-full text-sm font-semibold border transition ${tab === id ? 'bg-accent text-white border-accent' : 'border-edge/40 muted hover:text-ink'}`}>
                {label}{n !== null && <span className="opacity-70"> {n}</span>}
              </button>
            ))}
          </div>
          <div className="grid md:grid-cols-[minmax(0,1fr)_minmax(0,1.25fr)] gap-4 items-start">
            <div className={`space-y-3 ${sel ? 'hidden md:block' : ''}`}>
              {appTab && (
                <div className="card p-2">
                  {!shown.length && (
                    <div className="p-3">
                      <div className="muted mb-2">Nothing in the app matches "{text.trim()}".</div>
                      <button className="btn-ghost text-sm" onClick={() => openChat(`Can you explain ${text.trim()}?`)}>🤖 Ask Study Buddy</button>
                    </div>
                  )}
                  {shown.map((h) => (
                    <Row key={h.id} icon={h.icon} title={h.title} sub={h.sub} text={h.body ? snippet(h.body, terms) : undefined} label={KIND_LABEL[h.kind]} terms={terms}
                      active={sel?.kind === 'app' && sel.item.id === h.id} onClick={() => setSel({ kind: 'app', item: h })} />
                  ))}
                  {shown.length < total && <button className="w-full text-sm font-bold text-edge py-2 hover:underline" onClick={() => setMore((m) => m + (tab === 'all' ? 12 : 40))}>Show more ({total - shown.length} left)</button>}
                </div>
              )}
              {(tab === 'all' || tab === 'images') && <div className="card p-2">{picsSection}</div>}
              {(tab === 'all' || tab === 'web') && <div className="card p-2">{webSection}</div>}
            </div>
            <div className={`${sel ? '' : 'hidden md:block'} md:sticky md:top-4`}>
              <div ref={previewBox} className="card2 md:max-h-[calc(100vh-2rem)] md:overflow-auto">
                {sel && <button className="md:hidden btn-ghost mb-3" onClick={() => setSel(null)}>← Back to results</button>}
                {!sel && <div className="muted text-center py-16">Pick a result to see it here.</div>}
                {sel?.kind === 'app' && <Preview item={sel.item} terms={terms} />}
                {sel?.kind === 'web' && <WikiReader title={sel.title} site={site} terms={terms} onOpen={(t) => setSel({ kind: 'web', title: t })} />}
                {sel?.kind === 'image' && <ImageViewer img={sel.img} onArticle={(t) => setSel({ kind: 'web', title: t })} />}
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
