// "Look it up": searches Wikipedia's public API (no account or API key) and brings the article text into the app,
// so the student reads it here instead of being sent to another website.
export type WikiSite = 'simple' | 'en';
export const WIKI_NAME: Record<WikiSite, string> = { simple: 'Simple English Wikipedia', en: 'Wikipedia' };

export interface WikiHit { title: string; snippet: string }
export interface WikiSection { heading: string; level: number; paras: string[] }
export interface WikiArticle { title: string; image?: string; sections: WikiSection[] }

// origin=* makes Wikipedia answer cross-site requests without credentials (CORS), so this works from any page, even a downloaded file.
const apiAt = (host: string, params: Record<string, string>) =>
  `https://${host}/w/api.php?${new URLSearchParams({ ...params, format: 'json', formatversion: '2', origin: '*' })}`;
const api = (site: WikiSite, params: Record<string, string>) => apiAt(`${site}.wikipedia.org`, params);
const COMMONS = 'commons.wikimedia.org';

const cache = new Map<string, unknown>();
async function getJson<T>(url: string, signal?: AbortSignal): Promise<T> {
  if (cache.has(url)) return cache.get(url) as T;
  const res = await fetch(url, { signal });
  if (!res.ok) throw new Error(`Wikipedia answered ${res.status}`);
  const json = (await res.json()) as T;
  if (cache.size > 200) cache.clear();
  cache.set(url, json);
  return json;
}

const ENTITIES: Record<string, string> = { amp: '&', lt: '<', gt: '>', quot: '"', apos: "'", nbsp: ' ' };
/** Plain text from a snippet of Wikipedia HTML (search results mark matches with <span>). */
export function stripHtml(html: string): string {
  return html.replace(/<[^>]*>/g, '').replace(/&(#x?[0-9a-f]+|\w+);/gi, (m, e: string) => {
    if (e[0] === '#') { const code = e[1] === 'x' || e[1] === 'X' ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10); return Number.isFinite(code) ? String.fromCodePoint(code) : m; }
    return ENTITIES[e.toLowerCase()] ?? m;
  });
}

export async function wikiSearch(q: string, site: WikiSite, signal?: AbortSignal): Promise<{ hits: WikiHit[]; suggestion?: string }> {
  type R = { query?: { search?: { title: string; snippet?: string }[]; searchinfo?: { suggestion?: string } } };
  const j = await getJson<R>(api(site, { action: 'query', list: 'search', srsearch: q, srlimit: '8', srinfo: 'suggestion', srprop: 'snippet' }), signal);
  return { hits: (j.query?.search ?? []).map((h) => ({ title: h.title, snippet: stripHtml(h.snippet ?? '') })), suggestion: j.query?.searchinfo?.suggestion };
}

export async function wikiArticle(title: string, site: WikiSite, signal?: AbortSignal): Promise<WikiArticle> {
  type R = { query?: { pages?: { title: string; missing?: boolean; extract?: string; thumbnail?: { source: string } }[] } };
  const j = await getJson<R>(api(site, { action: 'query', prop: 'extracts|pageimages', titles: title, explaintext: '1', exsectionformat: 'wiki', redirects: '1', piprop: 'thumbnail', pithumbsize: '480' }), signal);
  const p = j.query?.pages?.[0];
  if (!p || p.missing || !p.extract?.trim()) throw new Error('That article was not found.');
  return { title: p.title, image: p.thumbnail?.source, sections: parseExtract(p.extract) };
}

/** Articles on similar subjects, so the student can keep reading without leaving the app. */
export async function wikiRelated(title: string, site: WikiSite, signal?: AbortSignal): Promise<string[]> {
  type R = { query?: { search?: { title: string }[] } };
  const j = await getJson<R>(api(site, { action: 'query', list: 'search', srsearch: `morelike:${title}`, srlimit: '6', srprop: '' }), signal);
  return (j.query?.search ?? []).map((h) => h.title).filter((t) => t !== title);
}

// Reference lists and outside links aren't useful inside the app.
const SKIP = /^(references|notes|sources|citations|footnotes|bibliography|external links|other websites|further reading|related pages|see also|gallery)$/i;

/** Removes the LaTeX copies of math formulas that plain-text extracts include, e.g. "{\displaystyle a^{2}+b^{2}}". */
function dropLatex(s: string): string {
  let out = '';
  for (let i = 0; i < s.length; i++) {
    if (s.startsWith('{\\displaystyle', i)) {
      let depth = 0, j = i;
      for (; j < s.length; j++) { if (s[j] === '{') depth++; else if (s[j] === '}' && --depth === 0) break; }
      i = j;
      continue;
    }
    out += s[i];
  }
  return out;
}

/** Splits a plain-text extract ("== Heading ==" lines between paragraphs) into sections. */
export function parseExtract(text: string): WikiSection[] {
  const sections: WikiSection[] = [{ heading: '', level: 1, paras: [] }];
  let skipBelow = 0; // while > 0, we're inside a skipped section of that level
  for (const raw of dropLatex(text).split('\n')) {
    const line = raw.replace(/[ \t]+/g, ' ').replace(/ ([.,;:!?)])/g, '$1').trim();
    if (!line) continue;
    const h = line.match(/^(={2,6})\s*(.*?)\s*=+$/);
    if (h) {
      const level = h[1].length;
      if (skipBelow && level > skipBelow) continue;
      skipBelow = SKIP.test(h[2]) ? level : 0;
      if (!skipBelow) sections.push({ heading: h[2], level, paras: [] });
      continue;
    }
    if (!skipBelow) sections[sections.length - 1].paras.push(line);
  }
  // keep a heading only if it or a subsection under it has text
  const hasText = (i: number) => {
    for (let j = i; j < sections.length; j++) {
      if (j > i && sections[j].level <= sections[i].level) return false;
      if (sections[j].paras.length) return true;
    }
    return false;
  };
  return sections.filter((_, i) => hasText(i));
}

// ---------------------------------------------------------------- Pictures
// Wikimedia Commons (100+ million free pictures) and the main picture of matching Wikipedia articles. No key needed.

export interface WikiImage {
  file: string; // file name without "File:", e.g. "Leaf 1 web.jpg"
  host: string; // which Wikimedia API to ask for the big version and the credit ('' for other sources)
  thumb: string;
  large?: string; // big copy, when the source gives it directly
  source?: string; // where it's from, e.g. "NASA"
  sourceTitle?: string; // the picture's title at that source
  caption: string;
  credit: string;
  article?: string; // the Wikipedia article this is the main picture of
}
export interface ImageDetails { large: string; caption: string; credit: string }

// Commons has no safe-search switch, so pictures whose search words, name, description or categories mention
// adult or gory subjects are left out. This catches what Commons labels; it can't promise to catch everything.
// School topics (biology, health, history) are fine; only nudity, sexual content and extreme gore are filtered.
const UNSAFE = /\b(nud(e|es|ity|ism|ist|ists)|naked|sex|sexy|erotic\w*|porn\w*|genitals?|genitalia|penis(es)?|vaginas?|vulvas?|nipples?|topless|lingerie|fetish\w*|bdsm|bondage|masturbat\w*|intercourse|orgasm\w*|hentai|xxx|nsfw|gore|gory|beheading|decapitat\w*)\b/i;
const SCHOOL = /\b(a?sexual(ly)? (reproduction|selection|dimorphism|cells?)|sex(-| )(chromosomes?|cells?|linked|determination|organs?)|sex education)\b/gi;
export const unsafeText = (s: string) => UNSAFE.test(s.replace(/_/g, ' ').replace(SCHOOL, ' '));

export const imageName = (file: string) => file.replace(/^File:/i, '').replace(/\.[a-z0-9]+$/i, '').replace(/_/g, ' ');
const sameFile = (a: string) => a.replace(/^File:/i, '').replace(/_/g, ' ').toLowerCase();

type Meta = Record<string, { value?: unknown } | undefined>;
const metaText = (m: Meta | undefined, k: string) => stripHtml(String(m?.[k]?.value ?? '')).replace(/\s+/g, ' ').trim();
const creditOf = (m: Meta | undefined) => [metaText(m, 'Artist'), metaText(m, 'LicenseShortName')].filter(Boolean).join(' · ');
const META_FIELDS = 'ImageDescription|ObjectName|Artist|LicenseShortName';

type CommonsJson = { query?: { pages?: { title: string; index?: number; categories?: { title: string }[]; imageinfo?: { thumburl?: string; extmetadata?: Meta }[] }[] } };
/** Picture results from a Commons file search, minus anything flagged by the safety filter. */
export function parseCommons(j: CommonsJson, filter = true): WikiImage[] {
  return (j.query?.pages ?? []).slice().sort((a, b) => (a.index ?? 0) - (b.index ?? 0)).flatMap((p) => {
    const info = p.imageinfo?.[0];
    if (!info?.thumburl || (filter && !p.categories)) return []; // no categories returned: can't check it, so skip it
    const caption = metaText(info.extmetadata, 'ImageDescription') || metaText(info.extmetadata, 'ObjectName');
    if (filter && unsafeText([p.title, caption, ...(p.categories ?? []).map((c) => c.title)].join(' | '))) return [];
    return [{ file: p.title.replace(/^File:/i, ''), host: COMMONS, thumb: info.thumburl, caption, credit: creditOf(info.extmetadata) }];
  });
}

type LeadJson = { query?: { pages?: { title: string; index?: number; pageimage?: string; thumbnail?: { source: string } }[] } };
/** The main picture of each Wikipedia article that matches the search. */
export function parseLeadImages(j: LeadJson, site: WikiSite, filter = true): WikiImage[] {
  return (j.query?.pages ?? []).slice().sort((a, b) => (a.index ?? 0) - (b.index ?? 0)).flatMap((p) => {
    if (!p.thumbnail?.source || !p.pageimage || (filter && unsafeText(`${p.title} | ${p.pageimage}`))) return [];
    return [{ file: p.pageimage, host: `${site}.wikipedia.org`, thumb: p.thumbnail.source, caption: p.title, credit: '', article: p.title }];
  });
}

/** Pictures for a search: article pictures first (most on-topic), then Commons. `blocked` when the search words themselves are filtered.
 *  `filter` false (teacher PIN in Settings) shows everything. */
export async function imageSearch(q: string, site: WikiSite, signal?: AbortSignal, filter = true): Promise<{ items: WikiImage[]; blocked: boolean }> {
  if (filter && unsafeText(q)) return { items: [], blocked: true };
  const [lead, commons] = await Promise.allSettled([
    getJson<LeadJson>(api(site, { action: 'query', generator: 'search', gsrsearch: q, gsrlimit: '12', prop: 'pageimages', piprop: 'thumbnail|name', pithumbsize: '400' }), signal),
    getJson<CommonsJson>(apiAt(COMMONS, { action: 'query', generator: 'search', gsrsearch: `${q} filetype:bitmap`, gsrnamespace: '6', gsrlimit: '36', prop: 'imageinfo|categories', iiprop: 'url|extmetadata', iiurlwidth: '400', iiextmetadatafilter: META_FIELDS, iiextmetadatalanguage: 'en', clshow: '!hidden', cllimit: 'max' }), signal),
  ]);
  if (lead.status === 'rejected' && commons.status === 'rejected') throw lead.reason;
  const all = [...(lead.status === 'fulfilled' ? parseLeadImages(lead.value, site, filter) : []), ...(commons.status === 'fulfilled' ? parseCommons(commons.value, filter) : [])];
  const seen = new Set<string>();
  return { items: all.filter((i) => (seen.has(sameFile(i.file)) ? false : (seen.add(sameFile(i.file)), true))), blocked: false };
}

/** A bigger copy of a picture, with its description and who made it. */
export async function imageDetails(img: WikiImage, signal?: AbortSignal): Promise<ImageDetails> {
  type R = { query?: { pages?: { imageinfo?: { url?: string; thumburl?: string; extmetadata?: Meta }[] }[] } };
  const j = await getJson<R>(apiAt(img.host, { action: 'query', titles: `File:${img.file}`, prop: 'imageinfo', iiprop: 'url|extmetadata', iiurlwidth: '1000', iiextmetadatafilter: META_FIELDS, iiextmetadatalanguage: 'en' }), signal);
  const info = j.query?.pages?.[0]?.imageinfo?.[0];
  if (!info) throw new Error('Picture details not found.');
  return { large: info.thumburl ?? info.url ?? img.thumb, caption: metaText(info.extmetadata, 'ImageDescription') || metaText(info.extmetadata, 'ObjectName'), credit: creditOf(info.extmetadata) };
}

/** One-way scramble of the teacher PIN, so it isn't saved as plain text. */
export function pinHash(pin: string): string {
  let h = 2166136261;
  for (const c of `study+piano:${pin}`) { h ^= c.charCodeAt(0); h = Math.imul(h, 16777619); }
  return (h >>> 0).toString(36);
}
