// "Look it up": searches Wikipedia's public API (no account or API key) and brings the article text into the app,
// so the student reads it here instead of being sent to another website.
export type WikiSite = 'simple' | 'en';
export const WIKI_NAME: Record<WikiSite, string> = { simple: 'Simple English Wikipedia', en: 'Wikipedia' };

export interface WikiHit { title: string; snippet: string }
export interface WikiSection { heading: string; level: number; paras: string[] }
export interface WikiArticle { title: string; image?: string; sections: WikiSection[] }

// origin=* makes Wikipedia answer cross-site requests without credentials (CORS), so this works from any page, even a downloaded file.
const api = (site: WikiSite, params: Record<string, string>) =>
  `https://${site}.wikipedia.org/w/api.php?${new URLSearchParams({ ...params, format: 'json', formatversion: '2', origin: '*' })}`;

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
