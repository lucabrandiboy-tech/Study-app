// More places to search, all free and keyless, all shown inside the app (never linked out):
// Openverse (free photos), NASA's image library, the Art Institute of Chicago, Wiktionary and Open Library.
import { stripHtml, unsafeText, type WikiImage } from './wiki';

async function getJson<T>(url: string, signal?: AbortSignal): Promise<T> {
  const res = await fetch(url, { signal, cache: 'no-store', referrerPolicy: 'no-referrer' }); // nothing kept in the browser cache
  if (!res.ok) throw new Error(`${new URL(url).host} answered ${res.status}`);
  return (await res.json()) as T;
}
const clean = (s: unknown) => stripHtml(String(s ?? '')).replace(/\s+/g, ' ').trim();
const ok = (filter: boolean, ...text: string[]) => !filter || !unsafeText(text.join(' | '));

// ---- pictures
type OV = { results?: { id: string; title?: string; creator?: string; license?: string; license_version?: string; thumbnail?: string; url?: string; source?: string; tags?: { name: string }[] }[] };
export function parseOpenverse(j: OV, filter: boolean): WikiImage[] {
  return (j.results ?? []).flatMap((r) => {
    const title = clean(r.title), tags = (r.tags ?? []).map((t) => t.name).join(' ');
    if (!r.thumbnail || !ok(filter, title, tags)) return [];
    return [{ file: `ov-${r.id}`, host: '', thumb: r.thumbnail, large: r.url, source: `Openverse${r.source ? ` (${r.source})` : ''}`, sourceTitle: title || 'Photo', caption: title, credit: [clean(r.creator), r.license ? `CC ${r.license.toUpperCase()} ${r.license_version ?? ''}`.trim() : ''].filter(Boolean).join(' · ') }];
  });
}
type NASA = { collection?: { items?: { data?: { title?: string; description?: string; nasa_id?: string; date_created?: string; center?: string }[]; links?: { href: string; rel?: string }[] }[] } };
export function parseNasa(j: NASA, filter: boolean): WikiImage[] {
  return (j.collection?.items ?? []).flatMap((it) => {
    const d = it.data?.[0], thumb = it.links?.find((l) => l.rel === 'preview')?.href ?? it.links?.[0]?.href;
    if (!d?.nasa_id || !thumb || !ok(filter, d.title ?? '', d.description ?? '')) return [];
    return [{ file: `nasa-${d.nasa_id}`, host: '', thumb, large: thumb.replace('~thumb', '~medium'), source: 'NASA', sourceTitle: clean(d.title), caption: clean(d.description).slice(0, 800), credit: `NASA${d.center ? ` (${d.center})` : ''}${d.date_created ? ` · ${d.date_created.slice(0, 10)}` : ''} · public domain` }];
  });
}
type AIC = { config?: { iiif_url?: string }; data?: { id: number; title?: string; artist_display?: string; date_display?: string; image_id?: string | null; medium_display?: string }[] };
export function parseArt(j: AIC, filter: boolean): WikiImage[] {
  const iiif = j.config?.iiif_url ?? 'https://www.artic.edu/iiif/2';
  return (j.data ?? []).flatMap((a) => {
    if (!a.image_id || !ok(filter, a.title ?? '', a.medium_display ?? '')) return [];
    return [{ file: `art-${a.id}`, host: '', thumb: `${iiif}/${a.image_id}/full/400,/0/default.jpg`, large: `${iiif}/${a.image_id}/full/843,/0/default.jpg`, source: 'Art Institute of Chicago', sourceTitle: clean(a.title), caption: [clean(a.title), clean(a.date_display), clean(a.medium_display)].filter(Boolean).join(' · '), credit: clean(a.artist_display) }];
  });
}

/** Pictures from Openverse, NASA and the Art Institute of Chicago. Sources that fail are skipped. */
export async function morePictures(q: string, filter: boolean, signal?: AbortSignal): Promise<WikiImage[][]> {
  if (filter && unsafeText(q)) return [];
  const e = encodeURIComponent(q);
  const r = await Promise.allSettled([
    getJson<OV>(`https://api.openverse.org/v1/images/?q=${e}&page_size=20&mature=${filter ? 'false' : 'true'}`, signal).then((j) => parseOpenverse(j, filter)),
    getJson<NASA>(`https://images-api.nasa.gov/search?q=${e}&media_type=image&page_size=12`, signal).then((j) => parseNasa(j, filter)),
    getJson<AIC>(`https://api.artic.edu/api/v1/artworks/search?q=${e}&limit=12&fields=id,title,artist_display,date_display,image_id,medium_display`, signal).then((j) => parseArt(j, filter)),
  ]);
  return r.map((x) => (x.status === 'fulfilled' ? x.value : []));
}

/** Mixes lists so each source gets a turn: a1 b1 c1 a2 b2 c2… */
export function interleave<T>(lists: T[][]): T[] {
  const out: T[] = [];
  for (let i = 0; lists.some((l) => i < l.length); i++) for (const l of lists) if (i < l.length) out.push(l[i]);
  return out;
}

// ---- dictionary
export interface Definition { word: string; parts: { pos: string; defs: string[] }[] }
type WT = Record<string, { partOfSpeech?: string; language?: string; definitions?: { definition?: string }[] }[]>;
export function parseWiktionary(word: string, j: WT): Definition | null {
  const parts = (j.en ?? []).map((p) => ({ pos: p.partOfSpeech ?? '', defs: (p.definitions ?? []).map((d) => clean(d.definition)).filter(Boolean).slice(0, 3) })).filter((p) => p.defs.length).slice(0, 4);
  return parts.length ? { word, parts } : null;
}
/** Dictionary meaning of a single word (or short phrase), from Wiktionary. */
export async function define(q: string, signal?: AbortSignal): Promise<Definition | null> {
  const word = q.trim();
  if (!word || word.split(/\s+/).length > 3) return null;
  for (const w of [word, word.toLowerCase()]) {
    try { const d = parseWiktionary(w, await getJson<WT>(`https://en.wiktionary.org/api/rest_v1/page/definition/${encodeURIComponent(w.replace(/ /g, '_'))}`, signal)); if (d) return d; }
    catch (e) { if ((e as Error).name === 'AbortError') throw e; }
  }
  return null;
}

// ---- books
export interface Book { key: string; title: string; author: string; year?: number; cover?: string; coverLarge?: string; subjects: string[]; firstSentence?: string; pages?: number }
type OL = { docs?: { key: string; title?: string; author_name?: string[]; first_publish_year?: number; cover_i?: number; subject?: string[]; first_sentence?: string[]; number_of_pages_median?: number }[] };
export function parseBooks(j: OL, filter: boolean): Book[] {
  return (j.docs ?? []).flatMap((d) => {
    const subjects = (d.subject ?? []).slice(0, 8);
    if (!d.title || !ok(filter, d.title, subjects.join(' '))) return [];
    return [{ key: d.key, title: d.title, author: (d.author_name ?? []).slice(0, 2).join(', ') || 'Unknown author', year: d.first_publish_year, subjects, firstSentence: d.first_sentence?.[0], pages: d.number_of_pages_median,
      cover: d.cover_i ? `https://covers.openlibrary.org/b/id/${d.cover_i}-M.jpg` : undefined, coverLarge: d.cover_i ? `https://covers.openlibrary.org/b/id/${d.cover_i}-L.jpg` : undefined }];
  });
}
/** Books about or titled like the search, from Open Library. */
export async function findBooks(q: string, filter: boolean, signal?: AbortSignal): Promise<Book[]> {
  const j = await getJson<OL>(`https://openlibrary.org/search.json?q=${encodeURIComponent(q)}&limit=8&fields=key,title,author_name,first_publish_year,cover_i,subject,first_sentence,number_of_pages_median`, signal);
  return parseBooks(j, filter);
}
